use crate::config::OverrideSettings;
use glob::Pattern;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct Labels {
    names: Vec<String>,
}

impl Labels {
    pub fn apply_overrides(
        &mut self,
        branch_path: &Path,
        overrides: &HashMap<String, OverrideSettings>,
    ) {
        for (name, setting) in overrides {
            if matches_override(branch_path, setting) && !self.names.iter().any(|n| n == name) {
                self.names.push(name.clone());
            }
        }

        if !self.names.is_empty() {
            log::info!("Matched override labels: {:?}", self.names);
        }
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }
}

fn matches_override(branch_path: &Path, setting: &OverrideSettings) -> bool {
    setting
        .search_files
        .iter()
        .any(|pattern| contains_marker(branch_path, pattern, setting.search_recursive))
}

fn contains_marker(branch_path: &Path, pattern: &Path, recursive: bool) -> bool {
    if branch_path.join(pattern).exists() {
        return true;
    }

    let pattern = pattern.to_string_lossy();
    let matches_name = |name: &str| name_matches_pattern(name, &pattern);

    if recursive {
        WalkDir::new(branch_path)
            .into_iter()
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_str().is_some_and(matches_name))
    } else {
        fs::read_dir(branch_path)
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| entry.file_name().to_str().is_some_and(matches_name))
    }
}

fn name_matches_pattern(name: &str, pattern: &str) -> bool {
    if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
        Pattern::new(pattern)
            .map(|p| p.matches(name))
            .unwrap_or(false)
    } else {
        name == pattern
    }
}
