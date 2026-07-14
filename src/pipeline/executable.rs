use crate::config::ExecutableSettings;
use anyhow::{Context, Result};
use glob::Pattern;
use std::fs;
use std::path::{Path, PathBuf};

pub fn has_executable(dir: &Path, settings: &ExecutableSettings) -> Result<bool> {
    Ok(!find_executables(dir, settings)?.is_empty())
}

pub fn find_executables(dir: &Path, settings: &ExecutableSettings) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let pattern = Pattern::new(&settings.search_pattern)
        .with_context(|| format!("invalid executable search_pattern: {}", settings.search_pattern))?;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if pattern.matches(name) {
                    found.push(path);
                }
            }
        }
    }

    if settings.use_longest_name && found.len() > 1 {
        if let Some(longest) = found.iter().max_by_key(|p| p.file_name().map(|n| n.len()).unwrap_or(0)) {
            return Ok(vec![longest.clone()]);
        }
    }

    Ok(found)
}
