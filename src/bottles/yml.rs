use anyhow::{Context, Result};
use chrono::Local;
use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::Path;
use uuid::Uuid;

const DATE_FIELDS: &[&str] = &["Creation_Date", "Update_Date"];

#[derive(Debug, Clone)]
pub struct ExternalProgram {
    pub id: String,
    pub name: String,
    pub executable: String,
    pub folder: String,
    pub path: String,
}

pub fn load(path: &Path) -> Result<Value> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {:?}", path))?;
    serde_yaml::from_str(&content).with_context(|| format!("Failed to parse {:?}", path))
}

pub fn save(path: &Path, doc: &mut Value) -> Result<()> {
    normalize_for_bottles(doc);
    let yaml = serde_yaml::to_string(doc).context("Failed to serialize bottle.yml")?;
    let yaml = ensure_quoted_date_fields(&yaml);
    fs::write(path, yaml).with_context(|| format!("Failed to write {:?}", path))
}

fn normalize_for_bottles(doc: &mut Value) {
    let Some(map) = doc.as_mapping_mut() else {
        return;
    };

    for field in DATE_FIELDS {
        let key = Value::from(*field);
        if let Some(value) = map.get(&key).cloned() {
            map.insert(key, Value::String(value_to_bottle_date_string(&value)));
        }
    }

    let updated = Local::now().format("%Y-%m-%d %H:%M:%S%.6f").to_string();
    map.insert(Value::from("Update_Date"), Value::String(updated));
}

fn value_to_bottle_date_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => value.as_str().unwrap_or("").to_string(),
    }
}

fn ensure_quoted_date_fields(yaml: &str) -> String {
    let mut lines: Vec<String> = yaml
        .lines()
        .map(|line| {
            for field in DATE_FIELDS {
                let prefix = format!("{field}:");
                if let Some(rest) = line.strip_prefix(&prefix) {
                    let value = rest.trim();
                    if value.starts_with('\'') {
                        return line.to_string();
                    }
                    return format!("{field}: '{value}'");
                }
            }
            line.to_string()
        })
        .collect();

    if yaml.ends_with('\n') {
        lines.push(String::new());
    }

    lines.join("\n")
}

pub fn external_programs(doc: &Value) -> Result<&Mapping> {
    doc.as_mapping()
        .and_then(|map| map.get("External_Programs"))
        .and_then(Value::as_mapping)
        .context("bottle.yml is missing External_Programs map")
}

pub fn external_programs_mut(doc: &mut Value) -> Result<&mut Mapping> {
    doc.as_mapping_mut()
        .and_then(|map| map.get_mut("External_Programs"))
        .and_then(Value::as_mapping_mut)
        .context("bottle.yml is missing External_Programs map")
}

pub fn has_program(doc: &Value, folder: &Path, exe_path: &Path) -> bool {
    find_program_key(doc, folder, exe_path).is_some()
}

pub fn remove_programs_in_folder(path: &Path, folder: &Path) -> Result<usize> {
    let mut doc = load(path)?;
    let folder_str = folder.to_string_lossy();
    let keys_to_remove: Vec<Value> = external_programs(&doc)?
        .iter()
        .filter_map(|(key, value)| {
            value.as_mapping().and_then(|entry| {
                entry
                    .get(Value::from("folder"))
                    .and_then(Value::as_str)
                    .filter(|entry_folder| *entry_folder == folder_str)
                    .map(|_| key.clone())
            })
        })
        .collect();

    if keys_to_remove.is_empty() {
        return Ok(0);
    }

    let programs = external_programs_mut(&mut doc)?;
    for key in &keys_to_remove {
        programs.remove(key);
    }

    save(path, &mut doc)?;
    Ok(keys_to_remove.len())
}

pub fn add_program(path: &Path, program: &ExternalProgram) -> Result<()> {
    let mut doc = load(path)?;

    if has_program(&doc, Path::new(&program.folder), Path::new(&program.path)) {
        anyhow::bail!(
            "Shortcut already exists for {:?} in {:?}",
            program.folder,
            path
        );
    }

    let entry = Mapping::from_iter([
        (
            Value::from("executable"),
            Value::String(program.executable.clone()),
        ),
        (
            Value::from("folder"),
            Value::String(program.folder.clone()),
        ),
        (
            Value::from("icon"),
            Value::String("com.usebottles.bottles-program".to_string()),
        ),
        (Value::from("id"), Value::String(program.id.clone())),
        (Value::from("name"), Value::String(program.name.clone())),
        (Value::from("path"), Value::String(program.path.clone())),
    ]);

    external_programs_mut(&mut doc)?.insert(Value::String(program.id.clone()), Value::Mapping(entry));
    save(path, &mut doc)?;
    Ok(())
}

fn find_program_key(doc: &Value, folder: &Path, exe_path: &Path) -> Option<String> {
    let programs = doc
        .as_mapping()
        .and_then(|map| map.get("External_Programs"))
        .and_then(Value::as_mapping)?;

    let exe_path_str = exe_path.to_string_lossy();
    let folder_str = folder.to_string_lossy();

    for (key, value) in programs {
        let Some(entry) = value.as_mapping() else {
            continue;
        };

        let same_folder = entry
            .get(Value::from("folder"))
            .and_then(Value::as_str)
            .is_some_and(|entry_folder| entry_folder == folder_str);

        let same_path = entry
            .get(Value::from("path"))
            .and_then(Value::as_str)
            .is_some_and(|path| path == exe_path_str);

        if same_folder || same_path {
            return key.as_str().map(str::to_string);
        }
    }

    None
}

pub fn new_program_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn round_trip_preserves_quoted_dates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bottle.yml");
        let content = "Creation_Date: '2024-01-01 00:00:00.000000'\nUpdate_Date: '2024-01-02 00:00:00.000000'\nExternal_Programs: {}\n";
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let mut doc = load(&path).unwrap();
        save(&path, &mut doc).unwrap();

        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("Creation_Date: '"));
        assert!(saved.contains("Update_Date: '"));
    }
}
