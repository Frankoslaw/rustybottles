use serde::Deserialize;
use std::path::{Path, PathBuf};

const RPGMAKER_MAIN: &str = "www/index.html";

#[derive(Debug, Deserialize)]
struct RpgMakerPackage {
    name: String,
    main: String,
    window: Option<RpgMakerWindow>,
}

#[derive(Debug, Deserialize)]
struct RpgMakerWindow {
    icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GameQuirks {
    pub game_name: String,
    pub exe_name: String,
    pub icon_source: Option<IconSource>,
}

#[derive(Debug, Clone)]
pub enum IconSource {
    PackageIcon(PathBuf),
    Executable(PathBuf),
}

pub fn resolve(
    game_folder: &Path,
    exe_stem: &str,
    folder_name: &str,
    rpgmaker_quirks: bool,
) -> GameQuirks {
    let exe_name = exe_stem.to_string();
    let mut game_name = exe_stem.to_string();

    let mut icon_source = None;

    if rpgmaker_quirks {
        if let Some(rpgmaker) = detect_rpgmaker_package(game_folder) {
            if let Some(name) = rpgmaker.game_name {
                game_name = name;
            }
            if let Some(icon_path) = rpgmaker.icon_path {
                icon_source = Some(IconSource::PackageIcon(icon_path));
            }
        }
    }

    game_name = normalize_game_name(&game_name, folder_name);

    if icon_source.is_none() {
        if let Some(exe_path) = find_executable_path(game_folder, exe_stem) {
            icon_source = Some(IconSource::Executable(exe_path));
        }
    }

    GameQuirks {
        game_name,
        exe_name,
        icon_source,
    }
}

fn find_executable_path(game_folder: &Path, exe_stem: &str) -> Option<PathBuf> {
    let exe_path = game_folder.join(format!("{exe_stem}.exe"));
    if exe_path.is_file() {
        Some(exe_path)
    } else {
        None
    }
}

fn detect_rpgmaker_package(game_folder: &Path) -> Option<RpgMakerPackageInfo> {
    let package_path = game_folder.join("package.json");
    let content = std::fs::read_to_string(&package_path).ok()?;
    let package: RpgMakerPackage = serde_json::from_str(&content).ok()?;

    if package.main != RPGMAKER_MAIN {
        return None;
    }

    let game_name = if package.name.is_empty() {
        None
    } else {
        Some(package.name)
    };

    let icon_path = package
        .window
        .and_then(|w| w.icon)
        .filter(|icon| !icon.is_empty())
        .map(|icon| game_folder.join(icon));

    Some(RpgMakerPackageInfo { game_name, icon_path })
}

struct RpgMakerPackageInfo {
    game_name: Option<String>,
    icon_path: Option<PathBuf>,
}

pub fn normalize_game_name(game_name: &str, folder_name: &str) -> String {
    if game_name.to_lowercase().starts_with("game") {
        folder_name.to_string()
    } else {
        game_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn normalize_game_name_uses_folder_when_starts_with_game() {
        assert_eq!(normalize_game_name("Game", "MyFolder"), "MyFolder");
        assert_eq!(normalize_game_name("GAME Title", "MyFolder"), "MyFolder");
        assert_eq!(normalize_game_name("game.exe", "MyFolder"), "MyFolder");
        assert_eq!(normalize_game_name("NotAGame", "MyFolder"), "NotAGame");
    }

    #[test]
    fn detects_rpgmaker_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let package = r#"{
            "name": "My RPG",
            "main": "www/index.html",
            "window": {
                "icon": "www/icon/icon.png"
            }
        }"#;
        fs::write(dir.path().join("package.json"), package).unwrap();

        let info = detect_rpgmaker_package(dir.path()).unwrap();
        assert_eq!(info.game_name.as_deref(), Some("My RPG"));
        assert_eq!(
            info.icon_path,
            Some(dir.path().join("www/icon/icon.png"))
        );
    }

    #[test]
    fn rejects_non_rpgmaker_package_json() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"name": "app", "main": "index.js"}"#,
        )
        .unwrap();

        assert!(detect_rpgmaker_package(dir.path()).is_none());
    }

    #[test]
    fn resolve_uses_package_name_with_rpgmaker_quirks() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"name": "Cool RPG", "main": "www/index.html"}"#,
        )
        .unwrap();
        fs::write(dir.path().join("Game.exe"), b"").unwrap();

        let quirks = resolve(dir.path(), "Game", "CoolFolder", true);
        assert_eq!(quirks.exe_name, "Game");
        assert_eq!(quirks.game_name, "Cool RPG");
    }

    #[test]
    fn resolve_keeps_exe_stem_without_quirks() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"name": "Cool RPG", "main": "www/index.html"}"#,
        )
        .unwrap();
        fs::write(dir.path().join("Game.exe"), b"").unwrap();

        let quirks = resolve(dir.path(), "Game", "CoolFolder", false);
        assert_eq!(quirks.exe_name, "Game");
        assert_eq!(quirks.game_name, "CoolFolder");
    }

    #[test]
    fn resolve_uses_non_game_package_name() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"name": "Cool RPG", "main": "www/index.html"}"#,
        )
        .unwrap();
        fs::write(dir.path().join("Launcher.exe"), b"").unwrap();

        let quirks = resolve(dir.path(), "Launcher", "CoolFolder", true);
        assert_eq!(quirks.exe_name, "Launcher");
        assert_eq!(quirks.game_name, "Cool RPG");
    }
}
