use crate::bottles::library_yml::{self, LibraryEntry};
use crate::bottles::quirks::IconSource;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub fn library_yml_path(bottles_root: &Path) -> Result<PathBuf> {
    library_yml::library_path(bottles_root)
}

pub fn remove_for_name(path: &Path, bottle_path: &str, name: &str) -> Result<usize> {
    let mut entries = library_yml::load(path)?;
    let removed = library_yml::remove_entries_for_bottle_name(&mut entries, bottle_path, name);
    if removed > 0 {
        library_yml::save(path, &entries)?;
    }
    Ok(removed)
}

pub fn exists(path: &Path, bottle_path: &str, name: &str) -> Result<bool> {
    let entries = library_yml::load(path)?;
    Ok(library_yml::has_entry_by_name(
        &entries,
        bottle_path,
        name,
    ))
}

pub fn add(
    path: &Path,
    bottle_dir: &Path,
    bottle_name: &str,
    library_name: &str,
    icon_source: Option<&IconSource>,
) -> Result<()> {
    let mut entries = library_yml::load(path)?;
    let key = Uuid::new_v4();
    let id = Uuid::new_v4();
    let icon = library_yml::icon_path(bottle_dir, library_name);

    if let Some(icon_source) = icon_source {
        if let Err(err) = crate::bottles::icon::create_library_icon(icon_source, &icon) {
            log::warn!("Failed to create library icon at {:?}: {}", icon, err);
        }
    }

    let entry = LibraryEntry {
        key,
        id,
        name: library_name.to_string(),
        bottle_name: bottle_name.to_string(),
        bottle_path: bottle_name.to_string(),
        icon,
    };

    library_yml::add_entry(&mut entries, entry)?;
    library_yml::save(path, &entries)?;
    Ok(())
}

pub fn load_entries(path: &Path) -> Result<HashMap<Uuid, LibraryEntry>> {
    library_yml::load(path)
}
