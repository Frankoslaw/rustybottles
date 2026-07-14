use crate::bottles::quirks::IconSource;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn create_library_icon(icon_source: &IconSource, dest_png: &Path) -> Result<()> {
    if let Some(parent) = dest_png.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create icon directory {:?}", parent))?;
    }

    match icon_source {
        IconSource::PackageIcon(src) => copy_png(src, dest_png),
        IconSource::Executable(exe_path) => extract_exe_icon(exe_path, dest_png),
    }
}

fn copy_png(src: &Path, dest: &Path) -> Result<()> {
    if !src.is_file() {
        bail!("Package icon not found at {:?}", src);
    }
    fs::copy(src, dest).with_context(|| format!("Failed to copy icon {:?} to {:?}", src, dest))?;
    Ok(())
}

fn extract_exe_icon(exe_path: &Path, dest_png: &Path) -> Result<()> {
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory for icon extraction")?;
    let ico_path = temp_dir.path().join("icon.ico");

    let status = Command::new("wrestool")
        .args([
            "-x",
            "--output",
            ico_path.to_str().context("Invalid temp ico path")?,
            "-o",
            "1",
            exe_path.to_str().context("Invalid executable path")?,
        ])
        .status()
        .with_context(|| format!("Failed to run wrestool for {:?}", exe_path))?;

    if !status.success() {
        bail!("wrestool failed to extract icon from {:?}", exe_path);
    }

    let img = image::open(&ico_path)
        .with_context(|| format!("Failed to decode icon extracted from {:?}", exe_path))?;
    img.save(dest_png)
        .with_context(|| format!("Failed to write library icon {:?}", dest_png))?;

    Ok(())
}
