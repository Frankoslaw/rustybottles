use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use std::fs;
use crate::config::{AppSettings, SourceStrategy};

pub fn process(input_path: &Path, config: &AppSettings) -> Result<PathBuf> {
    fs::create_dir_all(&config.workdir)
        .with_context(|| format!("Failed to create workdir at {:?}", config.workdir))?;

    let file_name = input_path.file_name().context("Input path has no filename")?;
    let dest = config.workdir.join(file_name);

    match config.source_strategy {
        SourceStrategy::Copy => {
            fs::copy(input_path, &dest)
                .with_context(|| format!("Failed to copy {:?} to {:?}", input_path, dest))?;
            log::info!("Copied source to workdir: {:?}", dest);
            Ok(dest)
        }
        SourceStrategy::Move => {
            fs::rename(input_path, &dest)
                .with_context(|| format!("Failed to move {:?} to {:?}", input_path, dest))?;
            log::info!("Moved source to workdir: {:?}", dest);
            Ok(dest)
        }
        SourceStrategy::None => {
            log::info!("Using source file in-place: {:?}", input_path);
            Ok(input_path.to_path_buf())
        }
    }
}

pub fn cleanup(
    original_path: &Path,
    staged_path: &Path,
    extracted_paths: &[PathBuf],
    config: &AppSettings,
) -> Result<()> {
    if config.delete_original
        && !matches!(config.source_strategy, SourceStrategy::Move)
        && original_path.exists()
    {
        fs::remove_file(original_path)?;
        log::debug!("Deleted original file: {:?}", original_path);
    }

    if config.cleanup_workdir {
        if staged_path.exists() && staged_path != original_path {
            fs::remove_file(staged_path)?;
            log::debug!("Cleaned up staged file: {:?}", staged_path);
        }

        for ext_path in extracted_paths {
            if ext_path.exists() && ext_path.starts_with(&config.workdir) {
                fs::remove_dir_all(ext_path)?;
                log::debug!("Cleaned up extracted folder: {:?}", ext_path);
            }
        }
    }

    Ok(())
}
