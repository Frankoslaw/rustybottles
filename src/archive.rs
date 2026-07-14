use anyhow::Context;
use std::{
    fs, io,
    path::Path,
};

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
}

pub fn is_archive(path: &Path) -> bool {
    ArchiveType::from_path(path).is_some()
}

pub fn extract(source: &Path, target: &Path, password: Option<&str>) -> anyhow::Result<()> {
    let archive_type = ArchiveType::from_path(source)
        .with_context(|| format!("Unsupported archive format: {:?}", source))?;

    match archive_type {
        ArchiveType::Zip => extract_zip(source, target, password),
        ArchiveType::SevenZip => extract_7z(source, target, password),
        ArchiveType::Rar => extract_rar(source, target, password),
    }
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