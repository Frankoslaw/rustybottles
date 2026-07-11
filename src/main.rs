use anyhow::{Context, Result};
use clap::Parser;
use glob::glob;
use std::{fs, path::PathBuf};

pub mod archive;
pub mod config;
pub mod engine;
pub mod errors;

#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    #[arg(required = true, num_args = 1..)]
    paths_or_globs: Vec<String>,

    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    // #[arg(short, long)]
    // profile: Option<String>,

    // #[arg(long)]
    // shortcut_override: Option<String>,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    let config_path = args.config.unwrap_or_else(|| PathBuf::from("rustybottles.toml"));
    let config = config::Config::load(&config_path).context("Failed to load config")?;

    let staging_dir = &config.settings.workdir;
    fs::create_dir_all(staging_dir)?;

    for raw_pattern in args.paths_or_globs {
        let expanded_pattern = shellexpand::full(&raw_pattern)
            .context(format!("Failed to expand shell variables in: {}", raw_pattern))?;
        log::debug!("Expanded pattern: {}", expanded_pattern);

        let escaped_pattern = escape_glob_metachars(&expanded_pattern);
        log::debug!("Escaped pattern: {}", escaped_pattern);

        let entries = glob(&escaped_pattern)
            .context(format!("Failed to read glob pattern: {}", escaped_pattern))?;

        for entry in entries {
            match entry {
                Ok(path) => {
                    if path.is_file() {
                        process_file(&path, staging_dir, &config)?;
                    }
                }
                Err(e) => log::error!("Error reading glob entry: {:?}", e),
            }
        }
    }

    Ok(())
}

fn escape_glob_metachars(path: &str) -> String {
    let mut escaped = String::new();
    for c in path.chars() {
        match c {
            // Note: we don't escape '*' because you actually WANT that to be a glob
            '[' => escaped.push_str("[[]"),
            ']' => escaped.push_str("[]]"),
            '?' => escaped.push_str("[?]"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn process_file(input_path: &PathBuf, staging_dir: &PathBuf, config: &config::Config) -> Result<()> {
    log::info!("Processing: {:?}", input_path);

    let file_name = input_path
        .file_name()
        .context("Input path has no filename")?;

    let staged_archive = staging_dir.join(file_name);
    
    fs::copy(input_path, &staged_archive)
        .with_context(|| format!("Failed to copy {:?} to staging", input_path))?;

    let extracted_path = archive::extract_recursive(
        &staged_archive,
        &config.extract_defaults,
    ).context("Extraction failed")?;

    log::info!("Extracted to: {:?}", extracted_path);
    Ok(())
}