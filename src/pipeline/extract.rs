use crate::archive;
use crate::config::{Config, ExtractSettings, ExtractUntil};
use crate::pipeline::executable;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn process(input: &Path, config: &Config) -> Result<Vec<PathBuf>> {
    let settings = config.extract.as_ref().context("No extract settings")?;

    if !settings.extract_recursive {
        let target_dir = extract_archive(input, settings)?;
        return Ok(vec![target_dir]);
    }

    let mut results = Vec::new();
    let mut queue = vec![input.to_path_buf()];

    while let Some(current_path) = queue.pop() {
        let target_dir = extract_archive(&current_path, settings)?;

        if branch_complete(&target_dir, settings, config) {
            results.push(target_dir);
            if !settings.extract_all_branches {
                break;
            }
            continue;
        }

        let nested_archives = find_nested_archives(&target_dir);
        let nested_folders = find_nested_folders(&target_dir);

        match settings.extract_until {
            ExtractUntil::NoArchives => {
                if nested_archives.is_empty() {
                    results.push(target_dir);
                    if !settings.extract_all_branches {
                        break;
                    }
                } else {
                    enqueue_sorted(&mut queue, nested_archives);
                }
            }
            ExtractUntil::Executable | ExtractUntil::MaxDepth => {
                if nested_archives.is_empty() && nested_folders.is_empty() {
                    results.push(target_dir);
                    if !settings.extract_all_branches {
                        break;
                    }
                } else {
                    enqueue_sorted(&mut queue, nested_archives);
                    enqueue_sorted(&mut queue, nested_folders);
                }
            }
        }
    }

    Ok(results)
}

fn extract_archive(path: &Path, settings: &ExtractSettings) -> Result<PathBuf> {
    if path.is_dir() {
        return Ok(path.to_path_buf());
    }

    let target = path.with_extension("");
    if target.exists() {
        if settings.force {
            fs::remove_dir_all(&target)?;
            log::info!("Removed existing extraction at {:?}", target);
            fs::create_dir_all(&target)?;
            archive::extract(path, &target, settings.password.as_deref())
                .context("Failed to extract archive")?;
        } else {
            log::info!("Reusing existing extraction at {:?}", target);
        }
    } else {
        fs::create_dir_all(&target)?;
        archive::extract(path, &target, settings.password.as_deref())
            .context("Failed to extract archive")?;
    }

    Ok(target)
}

fn branch_complete(target_dir: &Path, settings: &ExtractSettings, config: &Config) -> bool {
    match settings.extract_until {
        ExtractUntil::Executable => config.executable.as_ref().is_some_and(|exe_cfg| {
            executable::has_executable(target_dir, exe_cfg).unwrap_or(false)
        }),
        ExtractUntil::NoArchives => find_nested_archives(target_dir).is_empty(),
        ExtractUntil::MaxDepth => false,
    }
}

fn enqueue_sorted(queue: &mut Vec<PathBuf>, mut paths: Vec<PathBuf>) {
    paths.sort();
    for path in paths.into_iter().rev() {
        queue.push(path);
    }
}

fn find_nested_archives(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .ok()
        .map(|read_dir| {
            read_dir
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && archive::is_archive(p))
                .collect()
        })
        .unwrap_or_default()
}

fn find_nested_folders(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .ok()
        .map(|read_dir| {
            read_dir
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExecutableSettings, ExtractSettings};
    use std::fs::File;

    fn test_config(extract: ExtractSettings, executable: Option<ExecutableSettings>) -> Config {
        Config {
            app: crate::config::AppSettings {
                workdir_str: "/tmp".to_string(),
                workdir: PathBuf::from("/tmp"),
                cleanup_workdir: false,
                source_strategy: crate::config::SourceStrategy::None,
                delete_original: false,
            },
            extract: Some(extract),
            executable,
            bottles: None,
            overrides: None,
        }
    }

    #[test]
    fn noarchives_stops_when_no_archives_in_dir() {
        let dir = tempfile::tempdir().unwrap();
        File::create(dir.path().join("game.exe")).unwrap();

        let settings = ExtractSettings {
            force: false,
            password: None,
            extract_recursive: true,
            extract_until: ExtractUntil::NoArchives,
            extract_all_branches: true,
        };
        let config = test_config(settings, None);

        assert!(branch_complete(
            dir.path(),
            config.extract.as_ref().unwrap(),
            &config
        ));
    }

    #[test]
    fn executable_stops_when_exe_present() {
        let dir = tempfile::tempdir().unwrap();
        File::create(dir.path().join("game.exe")).unwrap();

        let settings = ExtractSettings {
            force: false,
            password: None,
            extract_recursive: true,
            extract_until: ExtractUntil::Executable,
            extract_all_branches: true,
        };
        let exe = ExecutableSettings {
            search_pattern: "*.exe".to_string(),
            prefer_longest: true,
            ignore_common: false,
        };
        let config = test_config(settings, Some(exe));

        assert!(branch_complete(
            dir.path(),
            config.extract.as_ref().unwrap(),
            &config
        ));
    }

    #[test]
    fn maxdepth_never_stops_early() {
        let dir = tempfile::tempdir().unwrap();
        File::create(dir.path().join("game.exe")).unwrap();

        let settings = ExtractSettings {
            force: false,
            password: None,
            extract_recursive: true,
            extract_until: ExtractUntil::MaxDepth,
            extract_all_branches: true,
        };
        let exe = ExecutableSettings {
            search_pattern: "*.exe".to_string(),
            prefer_longest: true,
            ignore_common: false,
        };
        let config = test_config(settings, Some(exe));

        assert!(!branch_complete(
            dir.path(),
            config.extract.as_ref().unwrap(),
            &config
        ));
    }
}
