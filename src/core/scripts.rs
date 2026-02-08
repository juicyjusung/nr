use crate::core::package_json::PackageJson;
use indexmap::IndexMap;
use std::path::Path;

/// Load scripts from a `package.json` in the given directory, preserving insertion order.
///
/// Returns an empty map if the file cannot be read, parsed, or has no `scripts` field.
pub fn load_scripts(package_dir: &Path) -> IndexMap<String, String> {
    PackageJson::load(package_dir)
        .map(|pkg| pkg.scripts())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, contents: &str) {
        fs::write(dir.join(name), contents).unwrap();
    }

    #[test]
    fn loads_scripts_preserving_order() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "package.json",
            r#"{
                "name": "test",
                "scripts": {
                    "dev": "vite",
                    "build": "tsc && vite build",
                    "lint": "eslint .",
                    "test": "vitest"
                }
            }"#,
        );

        let scripts = load_scripts(tmp.path());
        let keys: Vec<&str> = scripts.keys().map(String::as_str).collect();
        assert_eq!(keys, vec!["dev", "build", "lint", "test"]);
        assert_eq!(scripts["dev"], "vite");
        assert_eq!(scripts["build"], "tsc && vite build");
    }

    #[test]
    fn returns_empty_map_when_no_scripts_field() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "package.json", r#"{"name": "no-scripts"}"#);

        let scripts = load_scripts(tmp.path());
        assert!(scripts.is_empty());
    }

    #[test]
    fn returns_empty_map_when_file_missing() {
        let tmp = TempDir::new().unwrap();
        let scripts = load_scripts(tmp.path());
        assert!(scripts.is_empty());
    }

    #[test]
    fn returns_empty_map_when_invalid_json() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "package.json", "not valid json{{{");

        let scripts = load_scripts(tmp.path());
        assert!(scripts.is_empty());
    }

    #[test]
    fn skips_non_string_script_values() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "package.json",
            r#"{"scripts": {"dev": "vite", "bad": 42, "ok": "eslint"}}"#,
        );

        let scripts = load_scripts(tmp.path());
        assert_eq!(scripts.len(), 2);
        assert_eq!(scripts["dev"], "vite");
        assert_eq!(scripts["ok"], "eslint");
    }

    #[test]
    fn handles_empty_scripts_object() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "package.json", r#"{"scripts": {}}"#);

        let scripts = load_scripts(tmp.path());
        assert!(scripts.is_empty());
    }
}
