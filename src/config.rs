#![allow(unused)]
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,
    pub extract_defaults: ExtractDefaults,
    pub shortcut_defaults: ShortcutDefaults,
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub workdir: PathBuf,

    #[serde(rename = "bottles_root")]
    pub bottles_root_str: String,
    #[serde(skip)]
    pub bottles_root: PathBuf,

    pub default_bottle: String,
    pub default_user: String,
}

#[derive(Debug, Deserialize)]
pub struct ExtractDefaults {
    #[serde(default)]
    pub force: bool,
    pub password: Option<String>,
    pub recursive: bool,
    pub recursive_innermost: bool,
    pub delete_source: bool,
}

#[derive(Debug, Deserialize)]
pub struct ShortcutDefaults {
    pub enabled: bool,
    pub executable_pattern: String,
    pub prefer_longest_path: bool,
    pub suffix_hash: bool,
    pub suffix_date: bool,
}

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub bottle: Option<String>,
    pub markers: Vec<String>,
    pub shortcut_prefix: Option<String>,
    #[serde(default)]
    pub use_manifest_title: bool,
}

impl Config {
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context(format!("Failed to read config at {:?}", path))?;
        
        let mut config: Config = toml::from_str(&content)
            .context("TOML syntax error")?;

        let expanded = shellexpand::full(&config.settings.bottles_root_str)
            .context("Failed to expand bottles_root path variables")?;
        config.settings.bottles_root = PathBuf::from(expanded.into_owned());

        if !config.settings.bottles_root.exists() {
            anyhow::bail!("Bottles directory not found: {:?}", config.settings.bottles_root);
        }

        Ok(config)
    }
}