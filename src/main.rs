use anyhow::{Context, Result};
use clap::Parser;
use glob::glob;
use rustybottles::config::Config;
use rustybottles::pipeline;
use std::{env, path::PathBuf};

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
    let config = Config::load(&config_path).context("Failed to load configuration file")?;

    for raw_pattern in args.paths_or_globs {
        let expanded_pattern = shellexpand::full(&raw_pattern)
            .context(format!("Failed to expand shell variables in: {}", raw_pattern))?;

        let escaped_pattern = escape_glob_meta_chars(&expanded_pattern);

        let entries = glob(&escaped_pattern)
            .context(format!("Failed to read glob pattern: {}", escaped_pattern))?;

        for entry in entries {
            match entry {
                Ok(path) => {
                    if path.is_file() {
                        pipeline::run(&path, &config)?;
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
