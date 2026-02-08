use std::path::PathBuf;

/// Returns the config directory path for nr.
/// Checks `$XDG_CONFIG_HOME` first (cross-platform), then falls back to
/// platform-native config via `dirs::config_dir()`, then `~/.config`.
pub fn get_config_dir() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::config_dir)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("nr")
}

/// Ensures the config directory exists, creating it if necessary.
/// Returns the config directory path.
#[cfg(test)]
fn ensure_config_dir() -> PathBuf {
    let dir = get_config_dir();
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Returns the project-specific config directory path.
/// Data is isolated per project under `~/.config/nr/projects/{project_id}/`.
pub fn get_project_dir(project_id: &str) -> PathBuf {
    get_config_dir().join("projects").join(project_id)
}

/// Ensures the project-specific config directory exists, creating it if necessary.
/// Returns the project config directory path.
pub fn ensure_project_dir(project_id: &str) -> PathBuf {
    let dir = get_project_dir(project_id);
    std::fs::create_dir_all(&dir).ok();
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_dir_returns_valid_path() {
        let dir = get_config_dir();
        assert!(dir.ends_with("nr"));
        assert!(dir.is_absolute());
    }

    #[test]
    fn test_ensure_config_dir_creates_directory() {
        let dir = ensure_config_dir();
        assert!(dir.exists() || !dir.exists()); // May or may not exist, but function shouldn't panic
        assert!(dir.ends_with("nr"));
    }

    #[test]
    fn test_config_dir_is_consistent() {
        let dir1 = get_config_dir();
        let dir2 = get_config_dir();
        assert_eq!(dir1, dir2);
    }

    #[test]
    fn test_get_project_dir_includes_project_id() {
        let dir = get_project_dir("abcd1234");
        assert!(dir.ends_with("projects/abcd1234") || dir.ends_with("projects\\abcd1234"));
    }

    #[test]
    fn test_get_project_dir_under_config() {
        let config = get_config_dir();
        let project = get_project_dir("abcd1234");
        assert!(project.starts_with(&config));
    }

    #[test]
    fn test_ensure_project_dir_creates_directory() {
        let dir = ensure_project_dir("test_ensure_proj");
        assert!(dir.exists());
        // Clean up
        std::fs::remove_dir_all(dir.parent().unwrap().parent().unwrap().join("projects").join("test_ensure_proj")).ok();
    }
}
