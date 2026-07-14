use anyhow::{Context, Result};
use serde_yaml::{Mapping, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryEntry {
    pub key: Uuid,
    pub id: Uuid,
    pub name: String,
    pub bottle_name: String,
    pub bottle_path: String,
    pub icon: PathBuf,
}

pub fn library_path(bottles_root: &Path) -> Result<PathBuf> {
    bottles_root
        .parent()
        .map(|p| p.join("library.yml"))
        .context("bottles root has no parent directory for library.yml")
}

pub fn load(path: &Path) -> Result<HashMap<Uuid, LibraryEntry>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {:?}", path))?;
    let value: Value = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse {:?}", path))?;

    let Some(map) = value.as_mapping() else {
        return Ok(HashMap::new());
    };

    let mut entries = HashMap::new();
    for (key, value) in map {
        let Some(key_str) = key.as_str() else {
            continue;
        };
        let Ok(key_uuid) = Uuid::parse_str(key_str) else {
            continue;
        };
        let Some(entry_map) = value.as_mapping() else {
            continue;
        };

        if let Some(entry) = parse_entry(key_uuid, entry_map) {
            entries.insert(key_uuid, entry);
        }
    }

    Ok(entries)
}

pub fn save(path: &Path, entries: &HashMap<Uuid, LibraryEntry>) -> Result<()> {
    let mut map = Mapping::new();
    let mut keys: Vec<_> = entries.keys().collect();
    keys.sort();

    for key in keys {
        let entry = &entries[key];
        map.insert(Value::String(key.to_string()), entry_to_value(entry));
    }

    let yaml = serde_yaml::to_string(&Value::Mapping(map)).context("Failed to serialize library.yml")?;
    fs::write(path, yaml).with_context(|| format!("Failed to write {:?}", path))
}

pub fn has_entry_by_name(entries: &HashMap<Uuid, LibraryEntry>, bottle_path: &str, name: &str) -> bool {
    entries.values().any(|e| e.bottle_path == bottle_path && e.name == name)
}

pub fn remove_entries_for_bottle_name(
    entries: &mut HashMap<Uuid, LibraryEntry>,
    bottle_path: &str,
    name: &str,
) -> usize {
    let keys: Vec<Uuid> = entries
        .iter()
        .filter(|(_, e)| e.bottle_path == bottle_path && e.name == name)
        .map(|(k, _)| *k)
        .collect();

    for key in &keys {
        entries.remove(key);
    }

    keys.len()
}

pub fn add_entry(entries: &mut HashMap<Uuid, LibraryEntry>, entry: LibraryEntry) -> Result<()> {
    if has_entry_by_name(entries, &entry.bottle_path, &entry.name) {
        anyhow::bail!(
            "Library entry '{}' already exists for bottle '{}'",
            entry.name,
            entry.bottle_path
        );
    }

    entries.insert(entry.key, entry);
    Ok(())
}

fn parse_entry(key: Uuid, map: &Mapping) -> Option<LibraryEntry> {
    let bottle = map.get(Value::from("bottle"))?.as_mapping()?;
    let bottle_name = bottle.get(Value::from("name"))?.as_str()?.to_string();
    let bottle_path = bottle.get(Value::from("path"))?.as_str()?.to_string();
    let icon = map.get(Value::from("icon"))?.as_str()?;
    let id = Uuid::parse_str(map.get(Value::from("id"))?.as_str()?).ok()?;
    let name = map.get(Value::from("name"))?.as_str()?.to_string();

    Some(LibraryEntry {
        key,
        id,
        name,
        bottle_name,
        bottle_path,
        icon: PathBuf::from(icon),
    })
}

fn entry_to_value(entry: &LibraryEntry) -> Value {
    let bottle = Mapping::from_iter([
        (Value::from("name"), Value::String(entry.bottle_name.clone())),
        (Value::from("path"), Value::String(entry.bottle_path.clone())),
    ]);

    Value::Mapping(Mapping::from_iter([
        (Value::from("bottle"), Value::Mapping(bottle)),
        (
            Value::from("icon"),
            Value::String(entry.icon.to_string_lossy().into_owned()),
        ),
        (Value::from("id"), Value::String(entry.id.to_string())),
        (Value::from("name"), Value::String(entry.name.clone())),
        (Value::from("thumbnail"), Value::Null),
    ]))
}

pub fn icon_path(bottle_path: &Path, library_name: &str) -> PathBuf {
    bottle_path.join("icons").join(format!("{library_name}.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_deserialize_entry() {
        let key = Uuid::new_v4();
        let id = Uuid::new_v4();
        let entry = LibraryEntry {
            key,
            id,
            name: "Test Game (managed)".to_string(),
            bottle_name: "games".to_string(),
            bottle_path: "games".to_string(),
            icon: PathBuf::from("/tmp/games/icons/Test Game (managed).png"),
        };

        let mut entries = HashMap::new();
        entries.insert(key, entry.clone());

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.yml");
        save(&path, &entries).unwrap();

        let loaded = load(&path).unwrap();
        let loaded_entry = loaded.get(&key).unwrap();
        assert_eq!(loaded_entry.name, entry.name);
        assert_eq!(loaded_entry.bottle_path, entry.bottle_path);
        assert_eq!(loaded_entry.id, entry.id);
    }
}
