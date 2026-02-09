use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const MAX_HISTORY_ENTRIES: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ArgsHistory {
    pub entries: Vec<String>,
}

impl ArgsHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entry to the history, removing duplicates and capping at MAX_HISTORY_ENTRIES.
    /// The most recent entry appears first in the list.
    pub fn add_entry(&mut self, entry: String) {
        // Skip empty entries
        if entry.trim().is_empty() {
            return;
        }

        // Remove existing duplicate if present
        self.entries.retain(|e| e != &entry);

        // Insert at the beginning (most recent first)
        self.entries.insert(0, entry);

        // Cap at max size
        if self.entries.len() > MAX_HISTORY_ENTRIES {
            self.entries.truncate(MAX_HISTORY_ENTRIES);
        }
    }

    /// Returns the entries in order (most recent first)
    pub fn get_entries(&self) -> &[String] {
        &self.entries
    }
}

/// Loads args history from disk.
/// Returns an empty ArgsHistory if the file doesn't exist.
pub fn load_args_history(config_dir: &Path) -> Result<ArgsHistory> {
    let path = config_dir.join("args_history.json");

    if !path.exists() {
        return Ok(ArgsHistory::new());
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read args history from {}", path.display()))?;

    let history: ArgsHistory = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse args history from {}", path.display()))?;

    Ok(history)
}

/// Saves args history to disk.
pub fn save_args_history(config_dir: &Path, history: &ArgsHistory) -> Result<()> {
    fs::create_dir_all(config_dir).with_context(|| {
        format!(
            "Failed to create config directory: {}",
            config_dir.display()
        )
    })?;

    let path = config_dir.join("args_history.json");

    let content =
        serde_json::to_string_pretty(history).context("Failed to serialize args history")?;

    fs::write(&path, content)
        .with_context(|| format!("Failed to write args history to {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_add_entry_inserts_at_beginning() {
        let mut history = ArgsHistory::new();

        history.add_entry("first".to_string());
        history.add_entry("second".to_string());
        history.add_entry("third".to_string());

        assert_eq!(history.entries.len(), 3);
        assert_eq!(history.entries[0], "third");
        assert_eq!(history.entries[1], "second");
        assert_eq!(history.entries[2], "first");
    }

    #[test]
    fn test_add_entry_removes_duplicates() {
        let mut history = ArgsHistory::new();

        history.add_entry("first".to_string());
        history.add_entry("second".to_string());
        history.add_entry("first".to_string()); // Duplicate

        assert_eq!(history.entries.len(), 2);
        assert_eq!(history.entries[0], "first"); // Most recent
        assert_eq!(history.entries[1], "second");
    }

    #[test]
    fn test_add_entry_caps_at_max() {
        let mut history = ArgsHistory::new();

        for i in 0..25 {
            history.add_entry(format!("entry_{}", i));
        }

        assert_eq!(history.entries.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(history.entries[0], "entry_24"); // Most recent
        assert_eq!(history.entries[19], "entry_5"); // 20th entry
    }

    #[test]
    fn test_add_entry_skips_empty() {
        let mut history = ArgsHistory::new();

        history.add_entry("".to_string());
        history.add_entry("   ".to_string());
        history.add_entry("valid".to_string());

        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.entries[0], "valid");
    }

    #[test]
    fn test_save_and_load_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path();

        let mut history = ArgsHistory::new();
        history.add_entry("-- --watch".to_string());
        history.add_entry("-- --coverage".to_string());

        // Save
        save_args_history(config_dir, &history).unwrap();

        // Load
        let loaded = load_args_history(config_dir).unwrap();

        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0], "-- --coverage");
        assert_eq!(loaded.entries[1], "-- --watch");
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("nonexistent");

        let history = load_args_history(&config_dir).unwrap();
        assert_eq!(history.entries.len(), 0);
    }

    #[test]
    fn test_save_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("nested").join("config");

        let history = ArgsHistory::new();
        save_args_history(&config_dir, &history).unwrap();

        assert!(config_dir.exists());
        assert!(config_dir.join("args_history.json").exists());
    }

    #[test]
    fn test_get_entries() {
        let mut history = ArgsHistory::new();
        history.add_entry("a".to_string());
        history.add_entry("b".to_string());

        let entries = history.get_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], "b");
        assert_eq!(entries[1], "a");
    }
}
