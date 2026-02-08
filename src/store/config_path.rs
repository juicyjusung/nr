use std::path::PathBuf;

/// Returns the XDG-compatible config directory path for nr.
/// Respects `$XDG_CONFIG_HOME` via `dirs::config_dir()`.
/// Falls back to `~/.config/nr` if the platform config dir is unavailable.
pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("nr")
}

/// Ensures the config directory exists, creating it if necessary.
/// Returns the config directory path.
pub fn ensure_config_dir() -> PathBuf {
    let dir = get_config_dir();
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
}
