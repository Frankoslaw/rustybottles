use crate::config::ExecutableSettings;
use anyhow::{Context, Result};
use glob::Pattern;
use std::fs;
use std::path::{Path, PathBuf};

const COMMON_SUBSTRINGS: &[&str] = &["notification", "crash", "error", "helper"];

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

    if settings.ignore_common && found.len() > 1 {
        let filtered: Vec<_> = found
            .iter()
            .filter(|p| {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                !is_common_executable(name)
            })
            .cloned()
            .collect();

        if !filtered.is_empty() {
            found = filtered;
        }
    }

    if settings.prefer_longest && found.len() > 1 {
        if let Some(longest) = found
            .iter()
            .max_by_key(|p| p.file_name().map(|n| n.len()).unwrap_or(0))
        {
            return Ok(vec![longest.clone()]);
        }
    }

    Ok(found)
}

fn is_common_executable(name: &str) -> bool {
    let lower = name.to_lowercase();
    COMMON_SUBSTRINGS.iter().any(|s| lower.contains(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    fn settings(prefer_longest: bool, ignore_common: bool) -> ExecutableSettings {
        ExecutableSettings {
            search_pattern: "*.exe".to_string(),
            prefer_longest,
            ignore_common,
        }
    }

    #[test]
    fn prefer_longest_picks_longest_name() {
        let dir = tempfile::tempdir().unwrap();
        File::create(dir.path().join("Game.exe")).unwrap();
        File::create(dir.path().join("Game Long Title.exe")).unwrap();

        let found = find_executables(dir.path(), &settings(true, false)).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file_name().unwrap(), "Game Long Title.exe");
    }

    #[test]
    fn ignore_common_filters_helper_and_crash_handlers() {
        let dir = tempfile::tempdir().unwrap();
        File::create(dir.path().join("Game.exe")).unwrap();
        File::create(dir.path().join("UnityCrashHandler64.exe")).unwrap();

        let found = find_executables(dir.path(), &settings(false, true)).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file_name().unwrap(), "Game.exe");
    }

    #[test]
    fn ignore_common_falls_back_when_all_are_common() {
        let dir = tempfile::tempdir().unwrap();
        File::create(dir.path().join("CrashHandler.exe")).unwrap();
        File::create(dir.path().join("ErrorReporter.exe")).unwrap();

        let found = find_executables(dir.path(), &settings(false, true)).unwrap();
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn is_common_executable_matches_substrings() {
        assert!(is_common_executable("UnityCrashHandler64.exe"));
        assert!(is_common_executable("notification_helper.exe"));
        assert!(!is_common_executable("Game.exe"));
    }
}
