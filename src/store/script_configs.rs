use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptConfig {
    pub args: String,
    #[serde(with = "systemtime_serde")]
    pub last_used: SystemTime,
}

pub type ScriptConfigs = HashMap<String, ScriptConfig>;

/// Loads script configurations from disk.
/// Returns an empty HashMap if the file doesn't exist.
pub fn load_script_configs(config_dir: &Path) -> Result<ScriptConfigs> {
    let path = config_dir.join("script_configs.json");

    if !path.exists() {
        return Ok(ScriptConfigs::new());
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read script configs from {}", path.display()))?;

    let configs: ScriptConfigs = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse script configs from {}", path.display()))?;

    Ok(configs)
}

/// Saves script configurations to disk.
pub fn save_script_configs(config_dir: &Path, configs: &ScriptConfigs) -> Result<()> {
    fs::create_dir_all(config_dir).with_context(|| {
        format!(
            "Failed to create config directory: {}",
            config_dir.display()
        )
    })?;

    let path = config_dir.join("script_configs.json");

    let content =
        serde_json::to_string_pretty(configs).context("Failed to serialize script configs")?;

    fs::write(&path, content)
        .with_context(|| format!("Failed to write script configs to {}", path.display()))?;

    Ok(())
}

// Custom serialization for SystemTime
mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path();

        let mut configs = ScriptConfigs::new();
        configs.insert(
            "project123:root:test".to_string(),
            ScriptConfig {
                args: "-- --watch".to_string(),
                last_used: SystemTime::now(),
            },
        );
        configs.insert(
            "project123:root:build".to_string(),
            ScriptConfig {
                args: "".to_string(),
                last_used: SystemTime::now(),
            },
        );

        // Save
        save_script_configs(config_dir, &configs).unwrap();

        // Load
        let loaded = load_script_configs(config_dir).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(
            loaded.get("project123:root:test").unwrap().args,
            "-- --watch"
        );
        assert_eq!(loaded.get("project123:root:build").unwrap().args, "");
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("nonexistent");

        let configs = load_script_configs(&config_dir).unwrap();
        assert_eq!(configs.len(), 0);
    }

    #[test]
    fn test_save_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("nested").join("config");

        let configs = ScriptConfigs::new();
        save_script_configs(&config_dir, &configs).unwrap();

        assert!(config_dir.exists());
        assert!(config_dir.join("script_configs.json").exists());
    }

    #[test]
    fn test_systemtime_serialization() {
        let config = ScriptConfig {
            args: "test".to_string(),
            last_used: SystemTime::now(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ScriptConfig = serde_json::from_str(&json).unwrap();

        // Times should be equal (within rounding error from secs precision)
        let original_secs = config
            .last_used
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let deserialized_secs = deserialized
            .last_used
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert_eq!(original_secs, deserialized_secs);
    }
}
