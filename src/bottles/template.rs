use chrono::Local;

pub fn render(
    template: &str,
    game_name: &str,
    exe_name: &str,
    folder_name: &str,
    folder_hash: Option<&str>,
    exe_hash: Option<&str>,
) -> String {
    let mut rendered = template
        .replace("{game_name}", game_name)
        .replace("{exe_name}", exe_name)
        .replace("{folder_name}", folder_name);

    if template.contains("{date}") {
        rendered = rendered.replace("{date}", &Local::now().format("%d.%m.%Y").to_string());
    }
    if let Some(folder_hash) = folder_hash {
        rendered = rendered.replace("{folder_hash}", folder_hash);
    }
    if let Some(exe_hash) = exe_hash {
        rendered = rendered.replace("{exe_hash}", exe_hash);
    }

    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_placeholders() {
        let result = render(
            "{game_name} - {exe_name} - {folder_name}",
            "Cool RPG",
            "Game",
            "folder",
            None,
            None,
        );
        assert_eq!(result, "Cool RPG - Game - folder");
    }

    #[test]
    fn game_name_and_exe_name_can_differ() {
        let result = render("{game_name}", "Cool RPG", "Game", "folder", None, None);
        assert_eq!(result, "Cool RPG");
    }

    #[test]
    fn substitutes_hashes() {
        let result = render(
            "{game_name}-{folder_hash}",
            "Game",
            "Game",
            "folder",
            Some("abc123"),
            None,
        );
        assert_eq!(result, "Game-abc123");
    }
}
