use anyhow::Context;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default)]
pub struct ExtractOptions<'a> {
    pub force: bool,
    pub password: Option<&'a str>,
    pub extract_recursive: bool,
    pub use_innermost: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveType {
    Zip,
    SevenZip,
    Rar,
}

impl ArchiveType {
    fn from_path(path: &Path) -> Option<Self> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("zip") => Some(Self::Zip),
            Some("7z") => Some(Self::SevenZip),
            Some("rar") => Some(Self::Rar),
            _ => None,
        }
    }

    fn extract(self, source: &Path, target: &Path, password: Option<&str>) -> anyhow::Result<()> {
        match self {
            Self::Zip => extract_zip(source, target, password),
            Self::SevenZip => extract_7z(source, target, password),
            Self::Rar => extract_rar(source, target, password),
        }
    }
}

pub fn extract_recursive(source: &Path, options: &ExtractOptions) -> anyhow::Result<PathBuf> {
    if source.is_dir() {
        return Ok(source.to_path_buf());
    }

    let mut current_archive = source.to_path_buf();
    let mut first_output: Option<PathBuf> = None;
    let mut last_output: Option<PathBuf> = None;
    let mut allow_password = true;

    loop {
        let target = current_archive.with_extension("");

        if target.exists() && options.force {
            fs::remove_dir_all(&target)?;
        }

        if !target.exists() {
            fs::create_dir_all(&target)?;

            let password = if allow_password {
                options.password
            } else {
                None
            };

            log::info!(
                "Extracting {:?}",
                current_archive.file_name().unwrap_or_default()
            );

            let archive_type = ArchiveType::from_path(&current_archive)
                .with_context(|| format!("Unsupported archive format: {:?}", current_archive))?;

            archive_type.extract(&current_archive, &target, password)?;
        }

        first_output.get_or_insert_with(|| target.clone());
        last_output = Some(target.clone());
        allow_password = false;

        if options.extract_recursive {
            if let Some(next_archive) = find_nested_archive(&target) {
                current_archive = next_archive;
                continue;
            }
        }

        break;
    }

    Ok(if options.use_innermost {
        last_output.unwrap_or_else(|| source.to_path_buf())
    } else {
        first_output.unwrap_or_else(|| source.to_path_buf())
    })
}

fn find_nested_archive(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| ArchiveType::from_path(path).is_some())
}

fn extract_rar(source: &Path, target: &Path, password: Option<&str>) -> anyhow::Result<()> {
    let archive_path = source
        .to_str()
        .with_context(|| format!("Unsupported archive path: {:?}", source))?;

    let archive = if let Some(pwd) = password {
        unrar::Archive::with_password(archive_path, pwd.as_bytes())
    } else {
        unrar::Archive::new(archive_path)
    };

    let mut archive = archive.open_for_processing()?;

    while let Some(header) = archive.read_header()? {
        let out_path = target.join(&header.entry().filename);

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if header.entry().is_file() {
            let out_str = out_path
                .to_str()
                .with_context(|| format!("Unsupported archive path: {:?}", out_path))?;

            archive = header
                .extract_to(out_str)
                .with_context(|| format!("Unsupported archive path: {:?}", out_path))?;
        } else {
            archive = header
                .skip()
                .with_context(|| format!("Unsupported archive path: {:?}", out_path))?;
        }
    }

    Ok(())
}

fn extract_zip(source: &Path, target: &Path, password: Option<&str>) -> anyhow::Result<()> {
    let file = fs::File::open(source)?;

    let mut archive = zip::ZipArchive::new(file)?;

    for index in 0..archive.len() {
        let mut file = if let Some(password) = password {
            archive.by_index_decrypt(index, password.as_bytes())?
        } else {
            archive.by_index(index)?
        };

        let out_path = target.join(file.mangled_name());

        if file.is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut outfile = fs::File::create(&out_path)?;
        io::copy(&mut file, &mut outfile)?;
    }

    Ok(())
}

fn extract_7z(source: &Path, target: &Path, password: Option<&str>) -> anyhow::Result<()> {
    match password {
        Some(password) => {
            sevenz_rust2::decompress_file_with_password(source, target, password.into())?;
        }
        None => {
            sevenz_rust2::decompress_file(source, target)?;
        }
    }

    Ok(())
}