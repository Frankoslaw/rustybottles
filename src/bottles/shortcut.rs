use crate::bottles::yml::{self, ExternalProgram};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn remove_for_folder(bottle_yml_path: &Path, folder: &Path) -> Result<usize> {
    yml::remove_programs_in_folder(bottle_yml_path, folder)
}

pub fn add(
    bottle_yml_path: &Path,
    shortcut_name: &str,
    executable_name: &str,
    folder: &Path,
    exe_path: &Path,
) -> Result<()> {
    let program = ExternalProgram {
        id: yml::new_program_id(),
        name: shortcut_name.to_string(),
        executable: executable_name.to_string(),
        folder: folder.to_string_lossy().into_owned(),
        path: exe_path.to_string_lossy().into_owned(),
    };

    yml::add_program(bottle_yml_path, &program)
}

pub fn exists(bottle_yml_path: &Path, folder: &Path, exe_path: &Path) -> Result<bool> {
    let doc = yml::load(bottle_yml_path)?;
    Ok(yml::has_program(&doc, folder, exe_path))
}

pub fn bottle_yml_path(bottle_path: &Path) -> PathBuf {
    bottle_path.join("bottle.yml")
}
