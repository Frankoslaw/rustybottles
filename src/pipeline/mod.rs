use anyhow::{Context, Result};
use std::path::Path;

use crate::config::Config;

mod source;
mod extract;
mod executable;
pub mod labels;

pub fn run(input_path: &Path, config: &Config) -> Result<()> {
    log::info!("Starting pipeline for: {:?}", input_path);

    let staged_file = source::process(input_path, &config.app)
        .context("Source stage failed")?;

    let mut extracted_paths = Vec::new();

    let pipeline_result: Result<()> = (|| {
        if config.extract.is_some() {
            extracted_paths =
                extract::process(&staged_file, config).context("Extraction stage failed")?;
        } else {
            log::info!("Pipeline halted: [extract] configuration missing.");
            extracted_paths.push(staged_file.clone());
        }

        if let Some(exe_cfg) = &config.executable {
            for branch_path in &extracted_paths {
                let found_exes = executable::find_executables(branch_path, exe_cfg)?;

                if found_exes.is_empty() {
                    log::warn!("No executable found in branch: {:?}", branch_path);
                    continue;
                }

                if let Some(bottle_cfg) = &config.bottles {
                    for exe_path in found_exes {
                        log::info!("Installing bottle for executable: {:?}", exe_path);

                        let mut labels = labels::Labels::default();
                        if let Some(overrides) = &config.overrides {
                            labels.apply_overrides(branch_path, overrides);
                        }

                        crate::bottles::install::install(
                            &exe_path,
                            bottle_cfg,
                            config.overrides.as_ref(),
                            &labels,
                        )
                        .context("Bottle installation failed")?;
                    }
                }
            }
        } else {
            log::info!("Pipeline halted: [executable] configuration missing.");
        }

        Ok(())
    })();

    if pipeline_result.is_err() {
        let _ = source::cleanup(input_path, &staged_file, &extracted_paths, &config.app);
        return pipeline_result;
    }

    source::cleanup(input_path, &staged_file, &extracted_paths, &config.app)?;
    log::info!("Pipeline completed successfully.");
    Ok(())
}
