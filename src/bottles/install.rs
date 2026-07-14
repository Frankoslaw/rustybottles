use crate::bottles::{library, shortcut, template};
use crate::config::{BottlesSettings, BottlesSettingsOverride, OverrideSettings};
use crate::pipeline::labels::Labels;
use crate::bottles::quirks::{self, GameQuirks};
use crate::util::{copy, hash};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ResolvedBottlesSettings {
    pub root_path: PathBuf,
    pub bottle: String,
    pub install_path: PathBuf,
    pub create_shortcut: bool,
    pub shortcut_tmpl: String,
    pub create_library: bool,
    pub library_tmpl: String,
    pub force: bool,
    pub rpgmaker_quirks: bool,
}

pub fn resolve_settings(
    base: &BottlesSettings,
    overrides: Option<&HashMap<String, OverrideSettings>>,
    labels: &Labels,
) -> Result<ResolvedBottlesSettings> {
    let mut resolved = ResolvedBottlesSettings {
        root_path: base.root_path.clone(),
        bottle: base.bottle.clone(),
        install_path: base
            .install_path
            .clone()
            .context("bottles.install_path is required for installation")?,
        create_shortcut: base.create_shortcut,
        shortcut_tmpl: base.shortcut_tmpl.clone(),
        create_library: base.create_library,
        library_tmpl: base.library_tmpl.clone(),
        force: base.force,
        rpgmaker_quirks: base.rpgmaker_quirks,
    };

    let Some(overrides) = overrides else {
        return Ok(resolved);
    };

    let mut matched: Vec<_> = labels.names().to_vec();
    matched.sort();

    for label in matched {
        let Some(override_setting) = overrides.get(&label) else {
            continue;
        };
        if let Some(bottles) = &override_setting.bottles {
            apply_bottles_override(&mut resolved, bottles);
        }
    }

    Ok(resolved)
}

fn apply_bottles_override(resolved: &mut ResolvedBottlesSettings, override_cfg: &BottlesSettingsOverride) {
    if let Some(root) = &override_cfg.root {
        resolved.root_path = root.clone();
    }
    if let Some(bottle) = &override_cfg.bottle {
        resolved.bottle = bottle.clone();
    }
    if let Some(install_path) = &override_cfg.install_path {
        resolved.install_path = install_path.clone();
    }
    if let Some(create_shortcut) = override_cfg.create_shortcut {
        resolved.create_shortcut = create_shortcut;
    }
    if let Some(shortcut_tmpl) = &override_cfg.shortcut_tmpl {
        resolved.shortcut_tmpl = shortcut_tmpl.clone();
    }
    if let Some(create_library) = override_cfg.create_library {
        resolved.create_library = create_library;
    }
    if let Some(library_tmpl) = &override_cfg.library_tmpl {
        resolved.library_tmpl = library_tmpl.clone();
    }
    if let Some(force) = override_cfg.force {
        resolved.force = force;
    }
    if let Some(rpgmaker_quirks) = override_cfg.rpgmaker_quirks {
        resolved.rpgmaker_quirks = rpgmaker_quirks;
    }
}

pub fn install(
    exe_path: &Path,
    base_cfg: &BottlesSettings,
    overrides: Option<&HashMap<String, OverrideSettings>>,
    labels: &Labels,
) -> Result<()> {
    let settings = resolve_settings(base_cfg, overrides, labels)?;
    let bottle_path = resolve_bottle(&settings.root_path, &settings.bottle)?;

    let game_folder = exe_path
        .parent()
        .context("Executable has no parent directory")?;
    let folder_name = game_folder
        .file_name()
        .and_then(|n| n.to_str())
        .context("Game folder has no name")?;

    let install_root = bottle_path.join(&settings.install_path);
    fs::create_dir_all(&install_root)
        .with_context(|| format!("Failed to create install path {:?}", install_root))?;

    let dest_folder = install_root.join(folder_name);
    let dest_exe = dest_folder.join(
        exe_path
            .file_name()
            .context("Executable has no file name")?,
    );

    let exe_stem = exe_path
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Executable has no stem")?;
    let executable_name = exe_path
        .file_name()
        .and_then(|s| s.to_str())
        .context("Executable has no file name")?;

    let game_quirks = quirks::resolve(
        game_folder,
        exe_stem,
        folder_name,
        settings.rpgmaker_quirks,
    );

    let bottle_yml_path = shortcut::bottle_yml_path(&bottle_path);
    let library_yml_path = library::library_yml_path(&settings.root_path)?;

    if settings.force && settings.create_shortcut {
        shortcut::remove_for_folder(&bottle_yml_path, &dest_folder)?;
    }
    if settings.force && settings.create_library {
        let library_name = render_name(
            &settings.library_tmpl,
            &game_quirks,
            folder_name,
            &dest_folder,
            &dest_exe,
        )?;
        library::remove_for_name(&library_yml_path, &settings.bottle, &library_name)?;
    }

    if dest_folder.exists() {
        if settings.force {
            fs::remove_dir_all(&dest_folder)
                .with_context(|| format!("Failed to remove existing install {:?}", dest_folder))?;
            log::info!("Removed existing install at {:?}", dest_folder);
        } else {
            bail!("Install destination already exists: {:?}", dest_folder);
        }
    } else {
        if settings.create_shortcut && !settings.force {
            if shortcut::exists(&bottle_yml_path, &dest_folder, &dest_exe)? {
                bail!(
                    "Shortcut already exists for {:?} in {:?}",
                    dest_folder,
                    bottle_yml_path
                );
            }
        }
        if settings.create_library && !settings.force {
            let library_name = render_name(
                &settings.library_tmpl,
                &game_quirks,
                folder_name,
                &dest_folder,
                &dest_exe,
            )?;
            if library::exists(&library_yml_path, &settings.bottle, &library_name)? {
                bail!(
                    "Library entry '{}' already exists in {:?}",
                    library_name,
                    library_yml_path
                );
            }
        }
    }

    copy::copy_dir(game_folder, &dest_folder)
        .with_context(|| format!("Failed to copy {:?} to {:?}", game_folder, dest_folder))?;
    log::info!("Copied game folder to {:?}", dest_folder);

    if settings.create_shortcut {
        let shortcut_name = render_name(
            &settings.shortcut_tmpl,
            &game_quirks,
            folder_name,
            &dest_folder,
            &dest_exe,
        )?;
        shortcut::add(
            &bottle_yml_path,
            &shortcut_name,
            executable_name,
            &dest_folder,
            &dest_exe,
        )?;
        log::info!("Created bottle shortcut '{}' in {:?}", shortcut_name, bottle_yml_path);
    }

    if settings.create_library {
        let library_name = render_name(
            &settings.library_tmpl,
            &game_quirks,
            folder_name,
            &dest_folder,
            &dest_exe,
        )?;
        library::add(
            &library_yml_path,
            &bottle_path,
            &settings.bottle,
            &library_name,
            game_quirks.icon_source.as_ref(),
        )?;
        log::info!("Created library entry '{}' in {:?}", library_name, library_yml_path);
    }

    Ok(())
}

fn render_name(
    tmpl: &str,
    game_quirks: &GameQuirks,
    folder_name: &str,
    dest_folder: &Path,
    dest_exe: &Path,
) -> Result<String> {
    let folder_hash = if tmpl.contains("{folder_hash}") {
        Some(hash::hash_directory(dest_folder)?)
    } else {
        None
    };
    let exe_hash = if tmpl.contains("{exe_hash}") {
        Some(hash::hash_file(dest_exe)?)
    } else {
        None
    };

    Ok(template::render(
        tmpl,
        &game_quirks.game_name,
        &game_quirks.exe_name,
        folder_name,
        folder_hash.as_deref(),
        exe_hash.as_deref(),
    ))
}

fn resolve_bottle(root: &Path, bottle_name: &str) -> Result<PathBuf> {
    let mut bottles = Vec::new();

    for entry in fs::read_dir(root)
        .with_context(|| format!("Failed to read bottles root {:?}", root))?
    {
        let entry = entry?;
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                bottles.push(name.to_string());
            }
        }
    }

    bottles.sort();

    if !bottles.iter().any(|name| name == bottle_name) {
        bail!(
            "Bottle '{}' not found in {:?}. Existing bottles: {:?}",
            bottle_name,
            root,
            bottles
        );
    }

    Ok(root.join(bottle_name))
}
