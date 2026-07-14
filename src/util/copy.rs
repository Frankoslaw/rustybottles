use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use walkdir::WalkDir;

pub fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let relative = entry
            .path()
            .strip_prefix(src)
            .context("Failed to compute relative path during copy")?;
        let target = dst.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)
                .with_context(|| format!("Failed to create directory {:?}", target))?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create parent directory {:?}", parent))?;
            }
            fs::copy(entry.path(), &target).with_context(|| {
                format!("Failed to copy {:?} to {:?}", entry.path(), target)
            })?;
        }
    }

    Ok(())
}
