use anyhow::{Context, Result};
use clap::Parser;
use glob::glob;
use std::{env, fs, path::PathBuf};

pub mod archive;
pub mod config;

#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    #[arg(required = true, num_args = 1..)]
    paths_or_globs: Vec<String>,

    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    if env::var("RUST_LOG").is_err() {
        unsafe { env::set_var("RUST_LOG", "info") }
    }
    pretty_env_logger::init();
    let args = Args::parse();

    let config_path = args.config.unwrap_or_else(|| PathBuf::from("rustybottles.toml"));
    let config = config::Config::load(&config_path).context("Failed to load config")?;

    let staging_dir = &config.app.workdir;
    fs::create_dir_all(staging_dir)?;

    for raw_pattern in args.paths_or_globs {
        let expanded_pattern = shellexpand::full(&raw_pattern)
            .context(format!("Failed to expand shell variables in: {}", raw_pattern))?;
        log::debug!("Expanded pattern: {}", expanded_pattern);

        let escaped_pattern = escape_glob_meta_chars(&expanded_pattern);
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

fn escape_glob_meta_chars(path: &str) -> String {
    let mut escaped = String::new();
    for c in path.chars() {
        match c {
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

    let staged_archive = match config.app.source_strategy {
        config::SourceStrategy::Copy => {
            let dest = staging_dir.join(file_name);
            fs::copy(input_path, &dest)
                .with_context(|| format!("Failed to copy {:?} to staging", input_path))?;
            dest
        }
        config::SourceStrategy::Move => {
            let dest = staging_dir.join(file_name);
            fs::rename(input_path, &dest)
                .with_context(|| format!("Failed to move {:?} to staging", input_path))?;
            dest
        }
        config::SourceStrategy::None => input_path.clone(),
    };

    let extracted_path = if let Some(ref extract_settings) = config.extract {
        let options = archive::ExtractOptions {
            force: extract_settings.force,
            password: extract_settings.password.as_deref(),
            extract_recursive: extract_settings.extract_recursive,
            use_innermost: extract_settings.use_innermost,
        };

        let path = archive::extract_recursive(&staged_archive, &options)
            .context("Extraction failed")?;
        log::info!("Extracted to: {:?}", path);
        Some(path)
    } else {
        log::info!("Extraction stage skipped (not configured)");
        None
    };

    if config.app.delete_original && !matches!(config.app.source_strategy, config::SourceStrategy::Move) {
        if input_path.exists() {
            fs::remove_file(input_path)
                .with_context(|| format!("Failed to delete original file: {:?}", input_path))?;
            log::debug!("Deleted original file: {:?}", input_path);
        }
    }

    if config.app.cleanup_workdir {
        if extracted_path.is_some() && staged_archive.is_file() && staged_archive != *input_path {
            fs::remove_file(&staged_archive)
                .with_context(|| format!("Failed to clean up staged archive: {:?}", staged_archive))?;
            log::debug!("Cleaned up staged archive file: {:?}", staged_archive);
        }
    }

    Ok(())
}