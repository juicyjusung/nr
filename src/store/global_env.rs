use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalEnvConfig {
    /// Last selected env file display names (e.g., [".env", ".env.local"])
    pub last_env_files: Vec<String>,
}

/// Loads global env configuration from disk.
/// Returns default config if the file doesn't exist.
pub fn load_global_env_config(config_dir: &Path) -> Result<GlobalEnvConfig> {
    let path = config_dir.join("global_env.json");

    if !path.exists() {
        return Ok(GlobalEnvConfig::default());
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read global env config from {}", path.display()))?;

    let config: GlobalEnvConfig = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse global env config from {}", path.display()))?;

    Ok(config)
}

/// Saves global env configuration to disk.
pub fn save_global_env_config(config_dir: &Path, config: &GlobalEnvConfig) -> Result<()> {
    fs::create_dir_all(config_dir)
        .with_context(|| format!("Failed to create config directory: {}", config_dir.display()))?;

    let path = config_dir.join("global_env.json");

    let content = serde_json::to_string_pretty(config)
        .context("Failed to serialize global env config")?;

    fs::write(&path, content)
        .with_context(|| format!("Failed to write global env config to {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path();

        let config = GlobalEnvConfig {
            last_env_files: vec![".env".to_string(), ".env.local".to_string()],
        };

        // Save
        save_global_env_config(config_dir, &config).unwrap();

        // Load
        let loaded = load_global_env_config(config_dir).unwrap();

        assert_eq!(loaded.last_env_files, vec![".env", ".env.local"]);
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("nonexistent");

        let config = load_global_env_config(&config_dir).unwrap();
        assert_eq!(config.last_env_files.len(), 0);
    }

    #[test]
    fn test_save_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join("nested").join("config");

        let config = GlobalEnvConfig::default();
        save_global_env_config(&config_dir, &config).unwrap();

        assert!(config_dir.exists());
        assert!(config_dir.join("global_env.json").exists());
    }
}
