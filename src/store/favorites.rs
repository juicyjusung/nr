use std::collections::HashSet;
use std::path::Path;

/// Loads favorite scripts from the config directory.
/// Returns an empty HashSet if the file doesn't exist or is corrupted.
///
/// # Arguments
/// * `config_dir` - Path to the config directory
///
/// # Returns
/// A HashSet containing favorite script keys
pub fn load_favorites(config_dir: &Path) -> HashSet<String> {
    let path = config_dir.join("favorites.json");

    if !path.exists() {
        return HashSet::new();
    }

    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&contents) {
                Ok(map) => map
                    .into_iter()
                    .filter(|(_, v)| v.as_bool().unwrap_or(false))
                    .map(|(k, _)| k)
                    .collect(),
                Err(_) => HashSet::new(),
            }
        }
        Err(_) => HashSet::new(),
    }
}

/// Saves favorite scripts to the config directory.
///
/// # Arguments
/// * `config_dir` - Path to the config directory
/// * `favorites` - HashSet of favorite script keys
pub fn save_favorites(config_dir: &Path, favorites: &HashSet<String>) {
    let path = config_dir.join("favorites.json");

    let map: serde_json::Map<String, serde_json::Value> = favorites
        .iter()
        .map(|k| (k.clone(), serde_json::Value::Bool(true)))
        .collect();

    let json = serde_json::to_string_pretty(&map).unwrap_or_else(|_| "{}".to_string());
    std::fs::write(&path, json).ok();
}

/// Toggles a favorite script.
/// If the key exists, it is removed. If it doesn't exist, it is added.
///
/// # Arguments
/// * `favorites` - Mutable reference to the favorites HashSet
/// * `key` - The script key to toggle
///
/// # Returns
/// `true` if the key was added, `false` if it was removed
pub fn toggle_favorite(favorites: &mut HashSet<String>, key: &str) -> bool {
    if favorites.contains(key) {
        favorites.remove(key);
        false
    } else {
        favorites.insert(key.to_string());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_favorites_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let favorites = load_favorites(temp_dir.path());
        assert!(favorites.is_empty());
    }

    #[test]
    fn test_save_and_load_favorites() {
        let temp_dir = TempDir::new().unwrap();
        let mut favorites = HashSet::new();
        favorites.insert("a1b2c3d4:root:dev".to_string());
        favorites.insert("a1b2c3d4:root:build".to_string());

        save_favorites(temp_dir.path(), &favorites);
        let loaded = load_favorites(temp_dir.path());

        assert_eq!(favorites, loaded);
    }

    #[test]
    fn test_load_favorites_corrupted_json() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("favorites.json");
        fs::write(&path, "not valid json").unwrap();

        let favorites = load_favorites(temp_dir.path());
        assert!(favorites.is_empty());
    }

    #[test]
    fn test_load_favorites_filters_false_values() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("favorites.json");
        let json = r#"{
            "a1b2c3d4:root:dev": true,
            "a1b2c3d4:root:test": false,
            "a1b2c3d4:root:build": true
        }"#;
        fs::write(&path, json).unwrap();

        let favorites = load_favorites(temp_dir.path());
        assert_eq!(favorites.len(), 2);
        assert!(favorites.contains("a1b2c3d4:root:dev"));
        assert!(favorites.contains("a1b2c3d4:root:build"));
        assert!(!favorites.contains("a1b2c3d4:root:test"));
    }

    #[test]
    fn test_toggle_favorite_adds_new() {
        let mut favorites = HashSet::new();
        let added = toggle_favorite(&mut favorites, "a1b2c3d4:root:dev");

        assert!(added);
        assert!(favorites.contains("a1b2c3d4:root:dev"));
    }

    #[test]
    fn test_toggle_favorite_removes_existing() {
        let mut favorites = HashSet::new();
        favorites.insert("a1b2c3d4:root:dev".to_string());

        let removed = toggle_favorite(&mut favorites, "a1b2c3d4:root:dev");

        assert!(!removed);
        assert!(!favorites.contains("a1b2c3d4:root:dev"));
    }

    #[test]
    fn test_toggle_favorite_multiple_times() {
        let mut favorites = HashSet::new();

        assert!(toggle_favorite(&mut favorites, "key"));
        assert!(favorites.contains("key"));

        assert!(!toggle_favorite(&mut favorites, "key"));
        assert!(!favorites.contains("key"));

        assert!(toggle_favorite(&mut favorites, "key"));
        assert!(favorites.contains("key"));
    }

    #[test]
    fn test_save_empty_favorites() {
        let temp_dir = TempDir::new().unwrap();
        let favorites = HashSet::new();

        save_favorites(temp_dir.path(), &favorites);

        let path = temp_dir.path().join("favorites.json");
        assert!(path.exists());

        let loaded = load_favorites(temp_dir.path());
        assert!(loaded.is_empty());
    }
}
