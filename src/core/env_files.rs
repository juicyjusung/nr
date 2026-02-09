use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum EnvScope {
    Package(PathBuf),
    Root(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvFile {
    pub path: PathBuf,
    pub display_name: String,
    pub scope: EnvScope,
}

#[derive(Debug, Default)]
pub struct EnvFileList {
    pub package_files: Vec<EnvFile>,
    pub root_files: Vec<EnvFile>,
}

impl EnvFileList {
    /// Returns all files in package → root order (for UI display)
    pub fn all_files(&self) -> impl Iterator<Item = &EnvFile> {
        self.package_files.iter().chain(self.root_files.iter())
    }

    /// Returns all files in root → package order (for env merging - package overrides root)
    pub fn all_files_merge_order(&self) -> impl Iterator<Item = &EnvFile> {
        self.root_files.iter().chain(self.package_files.iter())
    }
}

/// Scans for .env* files in both package directory and monorepo root (if different)
pub fn scan_env_files(cwd: &Path, monorepo_root: &Option<PathBuf>) -> EnvFileList {
    let mut list = EnvFileList::default();

    // Scan package directory
    if let Ok(entries) = fs::read_dir(cwd) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(".env") {
                    list.package_files.push(EnvFile {
                        path: entry.path(),
                        display_name: name.to_string(),
                        scope: EnvScope::Package(cwd.to_path_buf()),
                    });
                }
            }
        }
    }

    // Sort package files alphabetically
    list.package_files.sort_by(|a, b| a.display_name.cmp(&b.display_name));

    // Scan monorepo root if it exists and is different from package dir
    if let Some(root) = monorepo_root {
        if root != cwd {
            if let Ok(entries) = fs::read_dir(root) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with(".env") {
                            list.root_files.push(EnvFile {
                                path: entry.path(),
                                display_name: name.to_string(),
                                scope: EnvScope::Root(root.clone()),
                            });
                        }
                    }
                }
            }

            // Sort root files alphabetically
            list.root_files.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        }
    }

    list
}

/// Loads and merges environment variables from multiple .env files
/// Files are processed in order: later files override earlier ones
/// Expected order: root files first, then package files (so package overrides root)
pub fn load_env_files(env_file_paths: &[PathBuf]) -> Result<HashMap<String, String>> {
    let mut merged = HashMap::new();

    for path in env_file_paths {
        match load_single_env_file(path) {
            Ok(vars) => {
                for (key, value) in vars {
                    merged.insert(key, value);
                }
            }
            Err(e) => {
                eprintln!("⚠️  Failed to load {}: {}", path.display(), e);
                // Continue with other files
            }
        }
    }

    Ok(merged)
}

/// Loads a single .env file and returns its key-value pairs
fn load_single_env_file(path: &Path) -> Result<HashMap<String, String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read env file: {}", path.display()))?;

    let mut vars = HashMap::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Parse KEY=VALUE format
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            // Remove quotes if present
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value);

            if !key.is_empty() {
                vars.insert(key.to_string(), value.to_string());
            }
        } else {
            eprintln!(
                "⚠️  Invalid line {} in {}: {}",
                line_num + 1,
                path.display(),
                trimmed
            );
        }
    }

    Ok(vars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_env_files_finds_package_files() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path();

        // Create test .env files
        fs::write(package_dir.join(".env"), "KEY=value").unwrap();
        fs::write(package_dir.join(".env.local"), "LOCAL=true").unwrap();
        fs::write(package_dir.join(".env.development"), "DEV=true").unwrap();
        fs::write(package_dir.join("not-env.txt"), "ignore").unwrap();

        let list = scan_env_files(package_dir, &None);

        assert_eq!(list.package_files.len(), 3);
        assert_eq!(list.root_files.len(), 0);

        // Verify alphabetical sorting
        assert_eq!(list.package_files[0].display_name, ".env");
        assert_eq!(list.package_files[1].display_name, ".env.development");
        assert_eq!(list.package_files[2].display_name, ".env.local");
    }

    #[test]
    fn test_scan_env_files_separates_root_and_package() {
        let temp_dir = TempDir::new().unwrap();
        let root_dir = temp_dir.path();
        let package_dir = root_dir.join("apps").join("web");
        fs::create_dir_all(&package_dir).unwrap();

        // Create root .env
        fs::write(root_dir.join(".env"), "ROOT=true").unwrap();

        // Create package .env
        fs::write(package_dir.join(".env.local"), "LOCAL=true").unwrap();

        let list = scan_env_files(&package_dir, &Some(root_dir.to_path_buf()));

        assert_eq!(list.package_files.len(), 1);
        assert_eq!(list.root_files.len(), 1);
        assert_eq!(list.package_files[0].display_name, ".env.local");
        assert_eq!(list.root_files[0].display_name, ".env");
    }

    #[test]
    fn test_scan_env_files_skips_root_if_same_as_package() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path();

        fs::write(dir.join(".env"), "KEY=value").unwrap();

        let list = scan_env_files(dir, &Some(dir.to_path_buf()));

        // Should only appear in package_files, not root_files
        assert_eq!(list.package_files.len(), 1);
        assert_eq!(list.root_files.len(), 0);
    }

    #[test]
    fn test_load_env_files_merges_correctly() {
        let temp_dir = TempDir::new().unwrap();

        let file1 = temp_dir.path().join(".env");
        let file2 = temp_dir.path().join(".env.local");

        fs::write(&file1, "KEY1=root\nKEY2=root\nKEY3=root").unwrap();
        fs::write(&file2, "KEY2=package\nKEY4=package").unwrap();

        let vars = load_env_files(&[file1, file2]).unwrap();

        assert_eq!(vars.len(), 4);
        assert_eq!(vars.get("KEY1"), Some(&"root".to_string()));
        assert_eq!(vars.get("KEY2"), Some(&"package".to_string())); // Package overrides
        assert_eq!(vars.get("KEY3"), Some(&"root".to_string()));
        assert_eq!(vars.get("KEY4"), Some(&"package".to_string()));
    }

    #[test]
    fn test_load_single_env_file_parses_correctly() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join(".env");

        fs::write(
            &file,
            r#"
# Comment line
KEY1=value1
KEY2="quoted value"
KEY3='single quoted'
  KEY4  =  spaced  

EMPTY=
INVALID LINE
"#,
        )
        .unwrap();

        let vars = load_single_env_file(&file).unwrap();

        assert_eq!(vars.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(vars.get("KEY2"), Some(&"quoted value".to_string()));
        assert_eq!(vars.get("KEY3"), Some(&"single quoted".to_string()));
        assert_eq!(vars.get("KEY4"), Some(&"spaced".to_string()));
        assert_eq!(vars.get("EMPTY"), Some(&"".to_string()));
        assert!(!vars.contains_key("INVALID"));
    }

    #[test]
    fn test_load_env_files_continues_on_error() {
        let temp_dir = TempDir::new().unwrap();

        let file1 = temp_dir.path().join(".env");
        let file2 = temp_dir.path().join(".env.missing");
        let file3 = temp_dir.path().join(".env.local");

        fs::write(&file1, "KEY1=value1").unwrap();
        // file2 doesn't exist
        fs::write(&file3, "KEY3=value3").unwrap();

        let vars = load_env_files(&[file1, file2, file3]).unwrap();

        // Should load file1 and file3, skip file2
        assert_eq!(vars.len(), 2);
        assert_eq!(vars.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(vars.get("KEY3"), Some(&"value3".to_string()));
    }

    #[test]
    fn test_env_file_list_all_files() {
        let list = EnvFileList {
            package_files: vec![EnvFile {
                path: PathBuf::from(".env"),
                display_name: ".env".to_string(),
                scope: EnvScope::Package(PathBuf::from(".")),
            }],
            root_files: vec![EnvFile {
                path: PathBuf::from("../.env"),
                display_name: ".env".to_string(),
                scope: EnvScope::Root(PathBuf::from("..")),
            }],
        };

        // all_files returns package → root (UI display order)
        let all: Vec<_> = list.all_files().collect();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].path, PathBuf::from(".env")); // package first
        assert_eq!(all[1].path, PathBuf::from("../.env")); // root second
    }

    #[test]
    fn test_env_file_list_merge_order() {
        let list = EnvFileList {
            package_files: vec![EnvFile {
                path: PathBuf::from(".env"),
                display_name: ".env".to_string(),
                scope: EnvScope::Package(PathBuf::from(".")),
            }],
            root_files: vec![EnvFile {
                path: PathBuf::from("../.env"),
                display_name: ".env".to_string(),
                scope: EnvScope::Root(PathBuf::from("..")),
            }],
        };

        // all_files_merge_order returns root → package (merge order, package overrides)
        let all: Vec<_> = list.all_files_merge_order().collect();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].path, PathBuf::from("../.env")); // root first
        assert_eq!(all[1].path, PathBuf::from(".env")); // package second (overrides)
    }
}
