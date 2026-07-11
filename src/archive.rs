use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

use crate::{config::ExtractDefaults, errors::ExtractionError};

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

    fn extract(
        self,
        source: &Path,
        target: &Path,
        password: Option<&str>,
    ) -> Result<(), ExtractionError> {
        match self {
            Self::Zip => extract_zip(source, target, password),
            Self::SevenZip => extract_7z(source, target, password),
            Self::Rar => extract_rar(source, target, password),
        }
    }
}

pub fn extract_recursive(
    source: &Path,
    defaults: &ExtractDefaults,
) -> Result<PathBuf, ExtractionError> {
    if source.is_dir() {
        return Ok(source.to_path_buf());
    }

    let mut current_archive = source.to_path_buf();
    let mut first_output: Option<PathBuf> = None;
    let mut last_output: Option<PathBuf> = None;
    let mut allow_password = true;

    loop {
        let target = current_archive.with_extension("");

        if target.exists() && defaults.force {
            fs::remove_dir_all(&target).map_err(|e| ExtractionError::Io {
                path: target.clone(),
                e,
            })?;
        }

        if !target.exists() {
            fs::create_dir_all(&target).map_err(|e| ExtractionError::Io {
                path: target.clone(),
                e,
            })?;

            let password = if allow_password {
                defaults.password.as_deref()
            } else {
                None
            };

            log::info!("Extracting {:?}", current_archive.file_name().unwrap_or_default());

            let archive_type = ArchiveType::from_path(&current_archive)
                .ok_or_else(|| ExtractionError::UnsupportedFormat(current_archive.clone()))?;

            archive_type.extract(&current_archive, &target, password)?;
        }

        first_output.get_or_insert_with(|| target.clone());
        last_output = Some(target.clone());
        allow_password = false;

        if defaults.recursive {
            if let Some(next_archive) = find_nested_archive(&target) {
                current_archive = next_archive;
                continue;
            }
        }

        break;
    }

    Ok(if defaults.recursive_innermost {
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

fn extract_rar(
    source: &Path,
    target: &Path,
    password: Option<&str>,
) -> Result<(), ExtractionError> {
    let archive_path = source
        .to_str()
        .ok_or(ExtractionError::InvalidPathEncoding)?;

    let archive = if let Some(pwd) = password {
        unrar::Archive::with_password(archive_path, pwd.as_bytes())
    } else {
        unrar::Archive::new(archive_path)
    };

    let mut archive = archive
        .open_for_processing()
        .map_err(|e| ExtractionError::ArchiveSpecific(e.to_string()))?;

    while let Some(header) = archive
        .read_header()
        .map_err(|e| ExtractionError::ArchiveSpecific(e.to_string()))?
    {
        let out_path = target.join(&header.entry().filename);

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| ExtractionError::Io {
                path: parent.to_path_buf(),
                e,
            })?;
        }

        if header.entry().is_file() {
            let out_str = out_path
                .to_str()
                .ok_or(ExtractionError::InvalidPathEncoding)?;

            archive = header
                .extract_to(out_str)
                .map_err(|e| ExtractionError::ArchiveSpecific(e.to_string()))?;
        } else {
            archive = header
                .skip()
                .map_err(|e| ExtractionError::ArchiveSpecific(e.to_string()))?;
        }
    }

    Ok(())
}

fn extract_zip(
    source: &Path,
    target: &Path,
    password: Option<&str>,
) -> Result<(), ExtractionError> {
    let file = fs::File::open(source).map_err(|e| ExtractionError::Io {
        path: source.to_path_buf(),
        e,
    })?;

    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| ExtractionError::ArchiveSpecific(e.to_string()))?;

    for index in 0..archive.len() {
        let mut file = if let Some(password) = password {
            archive.by_index_decrypt(index, password.as_bytes()).map_err(|_| {
                ExtractionError::DecryptionFailed {
                    path: source.to_path_buf(),
                }
            })?
        } else {
            archive
                .by_index(index)
                .map_err(|e| ExtractionError::ArchiveSpecific(e.to_string()))?
        };

        let out_path = target.join(file.mangled_name());

        if file.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| ExtractionError::Io {
                path: out_path,
                e,
            })?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| ExtractionError::Io {
                path: parent.to_path_buf(),
                e,
            })?;
        }

        let mut outfile = fs::File::create(&out_path).map_err(|e| ExtractionError::Io {
            path: out_path.clone(),
            e,
        })?;

        io::copy(&mut file, &mut outfile).map_err(| e| ExtractionError::Io {
            path: out_path,
            e,
        })?;
    }

    Ok(())
}

fn extract_7z(
    source: &Path,
    target: &Path,
    password: Option<&str>,
) -> Result<(), ExtractionError> {
    match password {
        Some(password) => {
            sevenz_rust2::decompress_file_with_password(source, target, password.into())
                .map_err(|e| ExtractionError::ArchiveSpecific(e.to_string()))?;
        }
        None => {
            sevenz_rust2::decompress_file(source, target)
                .map_err(|e| ExtractionError::ArchiveSpecific(e.to_string()))?;
        }
    }

    Ok(())
}