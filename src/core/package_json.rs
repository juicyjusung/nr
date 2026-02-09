use indexmap::IndexMap;
use serde::Deserialize;
use std::path::Path;

/// Shared representation of a `package.json` file.
///
/// Only fields that the application actually uses are included.
/// Unknown fields are silently ignored by serde.
///
/// The `scripts` field is kept as raw `serde_json::Value` because
/// real-world `package.json` files may contain non-string values
/// that would cause strict `IndexMap<String, String>` deserialization to fail.
#[derive(Deserialize, Default)]
pub struct PackageJson {
    pub name: Option<String>,
    scripts: Option<serde_json::Map<String, serde_json::Value>>,
    pub workspaces: Option<serde_json::Value>,
    #[serde(rename = "packageManager")]
    pub package_manager: Option<String>,
}

impl PackageJson {
    /// Load and parse `package.json` from the given directory.
    /// Returns `None` if the file doesn't exist or cannot be parsed.
    pub fn load(dir: &Path) -> Option<Self> {
        let contents = std::fs::read_to_string(dir.join("package.json")).ok()?;
        serde_json::from_str(&contents).ok()
    }

    /// Extract scripts as an ordered map, filtering out non-string values.
    pub fn scripts(&self) -> IndexMap<String, String> {
        match &self.scripts {
            Some(obj) => obj
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect(),
            None => IndexMap::new(),
        }
    }

    /// Extract workspace glob patterns from the `workspaces` field.
    ///
    /// Supports both array format (`["packages/*"]`) and
    /// object format (`{ "packages": ["packages/*"] }`).
    pub fn workspace_patterns(&self) -> Vec<String> {
        let workspaces = match &self.workspaces {
            Some(w) => w,
            None => return Vec::new(),
        };

        let arr = if let Some(arr) = workspaces.as_array() {
            arr.clone()
        } else if let Some(obj) = workspaces.as_object() {
            match obj.get("packages").and_then(|p| p.as_array()) {
                Some(a) => a.clone(),
                None => return Vec::new(),
            }
        } else {
            return Vec::new();
        };

        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_parses_valid_json() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{
                "name": "test-package",
                "scripts": {
                    "test": "echo test",
                    "build": "echo build"
                }
            }"#,
        )
        .unwrap();

        let pkg = PackageJson::load(temp.path()).unwrap();
        assert_eq!(pkg.name, Some("test-package".to_string()));

        let scripts = pkg.scripts();
        assert_eq!(scripts.len(), 2);
        assert_eq!(scripts.get("test"), Some(&"echo test".to_string()));
        assert_eq!(scripts.get("build"), Some(&"echo build".to_string()));
    }

    #[test]
    fn test_load_returns_none_when_file_missing() {
        let temp = TempDir::new().unwrap();
        let pkg = PackageJson::load(temp.path());
        assert!(pkg.is_none());
    }

    #[test]
    fn test_load_returns_none_when_invalid_json() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("package.json"), "invalid json").unwrap();

        let pkg = PackageJson::load(temp.path());
        assert!(pkg.is_none());
    }

    #[test]
    fn test_scripts_returns_empty_when_none() {
        let pkg = PackageJson {
            name: Some("test".to_string()),
            scripts: None,
            workspaces: None,
            package_manager: None,
        };

        let scripts = pkg.scripts();
        assert!(scripts.is_empty());
    }

    #[test]
    fn test_scripts_filters_non_string_values() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{
                "scripts": {
                    "test": "echo test",
                    "invalid": 123,
                    "build": "echo build",
                    "also-invalid": true
                }
            }"#,
        )
        .unwrap();

        let pkg = PackageJson::load(temp.path()).unwrap();
        let scripts = pkg.scripts();

        // Should only include string values
        assert_eq!(scripts.len(), 2);
        assert!(scripts.contains_key("test"));
        assert!(scripts.contains_key("build"));
        assert!(!scripts.contains_key("invalid"));
        assert!(!scripts.contains_key("also-invalid"));
    }

    #[test]
    fn test_workspace_patterns_array_format() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{
                "workspaces": ["packages/*", "apps/*"]
            }"#,
        )
        .unwrap();

        let pkg = PackageJson::load(temp.path()).unwrap();
        let patterns = pkg.workspace_patterns();

        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0], "packages/*");
        assert_eq!(patterns[1], "apps/*");
    }

    #[test]
    fn test_workspace_patterns_object_format() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{
                "workspaces": {
                    "packages": ["packages/*", "tools/*"]
                }
            }"#,
        )
        .unwrap();

        let pkg = PackageJson::load(temp.path()).unwrap();
        let patterns = pkg.workspace_patterns();

        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0], "packages/*");
        assert_eq!(patterns[1], "tools/*");
    }

    #[test]
    fn test_workspace_patterns_returns_empty_when_none() {
        let pkg = PackageJson {
            name: Some("test".to_string()),
            scripts: None,
            workspaces: None,
            package_manager: None,
        };

        let patterns = pkg.workspace_patterns();
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_workspace_patterns_handles_invalid_format() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("package.json"),
            r#"{
                "workspaces": "invalid"
            }"#,
        )
        .unwrap();

        let pkg = PackageJson::load(temp.path()).unwrap();
        let patterns = pkg.workspace_patterns();
        assert!(patterns.is_empty());
    }
}
