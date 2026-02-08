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
