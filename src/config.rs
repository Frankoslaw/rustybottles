use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_workdir() -> String {
    "~/.rustybottles".to_string()
}
fn default_pattern() -> String {
    "*.exe".to_string()
}
fn default_shortcut() -> String {
    "{game_name}".to_string()
}
fn default_library() -> String {
    "{game_name} ({folder_name}) (managed)".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub app: AppSettings,

    pub extract: Option<ExtractSettings>,
    pub executable: Option<ExecutableSettings>,
    pub bottles: Option<BottlesSettings>,
    pub overrides: Option<HashMap<String, OverrideSettings>>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SourceStrategy {
    #[default]
    None,
    Copy,
    Move,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppSettings {
    #[serde(rename = "workdir", default = "default_workdir")]
    pub workdir_str: String,
    #[serde(skip)]
    pub workdir: PathBuf,
    #[serde(default = "default_true")]
    pub cleanup_workdir: bool,
    #[serde(default)]
    pub source_strategy: SourceStrategy,
    #[serde(default)]
    pub delete_original: bool,
}

#[derive(Debug, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExtractUntil {
    #[default]
    Executable,
    NoArchives,
    MaxDepth,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractSettings {
    #[serde(default = "default_false")]
    pub force: bool,
    pub password: Option<String>,
    #[serde(default = "default_true")]
    pub extract_recursive: bool,
    #[serde(default)]
    pub extract_until: ExtractUntil,
    #[serde(default = "default_true")]
    pub extract_all_branches: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableSettings {
    #[serde(default = "default_pattern")]
    pub search_pattern: String,
    #[serde(default = "default_true")]
    pub use_longest_name: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BottlesSettings {
    #[serde(rename = "root")]
    pub root_path_str: String,
    #[serde(skip)]
    pub root_path: PathBuf,
    pub bottle: String,
    #[serde(default)]
    pub install_path: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub create_shortcut: bool,
    #[serde(default = "default_shortcut")]
    pub shortcut_tmpl: String,
    #[serde(default)]
    pub create_library: bool,
    #[serde(default = "default_library")]
    pub library_tmpl: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverrideSettings {
    pub search_files: Vec<PathBuf>,
    #[serde(default)]
    pub search_recursive: bool,
    pub bottles: Option<BottlesSettingsOverride>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BottlesSettingsOverride {
    #[serde(rename = "root")]
    pub root_str: Option<String>,
    #[serde(skip)]
    pub root: Option<PathBuf>,
    pub bottle: Option<String>,
    pub install_path: Option<PathBuf>,
    pub create_shortcut: Option<bool>,
    pub shortcut_tmpl: Option<String>,
    pub create_library: Option<bool>,
    pub library_tmpl: Option<String>,
    pub force: Option<bool>,
}

impl Config {
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config at {:?}", path))?;

        let mut config: Config = toml::from_str(&content)
            .context("Failed to parse TOML configuration syntax")?;

        config.app.workdir = Self::expand_path(&config.app.workdir_str)
            .context("Failed to expand app.workdir path variables")?;

        if let Some(ref mut bottles) = config.bottles {
            bottles.root_path = Self::expand_path(&bottles.root_path_str)
                .context("Failed to expand bottles.root path variables")?;
        }

        if let Some(ref mut overrides) = config.overrides {
            for (engine_name, override_setting) in overrides.iter_mut() {
                if let Some(ref mut bottles_override) = override_setting.bottles {
                    if let Some(ref root_str) = bottles_override.root_str {
                        let expanded = Self::expand_path(root_str).with_context(|| {
                            format!("Failed expanding root path override for engine: {}", engine_name)
                        })?;
                        bottles_override.root = Some(expanded);
                    }
                }
            }
        }

        Ok(config)
    }

    fn expand_path(raw_path: &str) -> Result<PathBuf> {
        let expanded = shellexpand::full(raw_path)?;
        Ok(PathBuf::from(expanded.into_owned()))
    }
}
