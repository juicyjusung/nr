use crate::core::package_json::PackageJson;
use indexmap::IndexMap;
use std::path::{Path, PathBuf};

/// A package discovered inside a monorepo workspace.
pub struct WorkspacePackage {
    /// The `name` field from `package.json` (or directory name as fallback).
    pub name: String,
    /// Path relative to the monorepo root.
    pub relative_path: String,
    /// Scripts declared in this package's `package.json`.
    pub scripts: IndexMap<String, String>,
}

/// Scan a monorepo root for workspace packages.
///
/// Reads workspace glob patterns from either `package.json` `"workspaces"` field
/// or `pnpm-workspace.yaml`, then finds matching directories containing `package.json`.
pub fn scan_workspaces(monorepo_root: &Path) -> Vec<WorkspacePackage> {
    let patterns = read_workspace_patterns(monorepo_root);
    if patterns.is_empty() {
        return Vec::new();
    }

    let mut packages = Vec::new();

    for pattern in &patterns {
        let matched_dirs = expand_glob_pattern(monorepo_root, pattern);
        for dir in matched_dirs {
            let pkg_path = dir.join("package.json");
            if !pkg_path.is_file() {
                continue;
            }

            let relative = dir
                .strip_prefix(monorepo_root)
                .unwrap_or(&dir)
                .to_string_lossy()
                .replace('\\', "/");

            let (name, scripts) = read_package_info(&dir);

            packages.push(WorkspacePackage {
                name,
                relative_path: relative,
                scripts,
            });
        }
    }

    // Sort by relative path for deterministic output
    packages.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    packages
}

/// Extract workspace patterns from package.json or pnpm-workspace.yaml.
fn read_workspace_patterns(monorepo_root: &Path) -> Vec<String> {
    // Try package.json first
    if let Some(pkg) = PackageJson::load(monorepo_root) {
        let patterns = pkg.workspace_patterns();
        if !patterns.is_empty() {
            return patterns;
        }
    }

    // Fall back to pnpm-workspace.yaml
    if let Some(patterns) = read_patterns_from_pnpm_workspace(monorepo_root) {
        return patterns;
    }

    Vec::new()
}

/// Read workspace patterns from `pnpm-workspace.yaml`.
fn read_patterns_from_pnpm_workspace(monorepo_root: &Path) -> Option<Vec<String>> {
    let contents = std::fs::read_to_string(monorepo_root.join("pnpm-workspace.yaml")).ok()?;
    let val: serde_yaml_ng::Value = serde_yaml_ng::from_str(&contents).ok()?;
    let packages = val.get("packages")?.as_sequence()?;

    Some(
        packages
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
    )
}

/// Expand a single glob pattern relative to `root` into matching directories.
///
/// Uses `globset::Glob` for matching. Since globset does not walk the filesystem,
/// we manually traverse directories and test each path against the compiled glob.
fn expand_glob_pattern(root: &Path, pattern: &str) -> Vec<PathBuf> {
    // Build glob matcher
    let glob = match globset::Glob::new(pattern) {
        Ok(g) => g.compile_matcher(),
        Err(_) => return Vec::new(),
    };

    // Calculate the maximum directory depth from the pattern
    // e.g., "packages/*" => depth 2, "apps/*/packages/*" => depth 4
    let max_depth = pattern.split('/').count();

    let mut results = Vec::new();
    collect_matching_dirs(root, root, &glob, 0, max_depth, &mut results);
    results
}

/// Recursively walk directories, checking each path against the glob matcher.
fn collect_matching_dirs(
    root: &Path,
    current: &Path,
    glob: &globset::GlobMatcher,
    depth: usize,
    max_depth: usize,
    results: &mut Vec<PathBuf>,
) {
    if depth > max_depth {
        return;
    }

    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // Skip hidden directories and node_modules
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || name == "node_modules" {
                continue;
            }
        }

        let relative = path.strip_prefix(root).unwrap_or(&path);
        let relative_str = relative.to_string_lossy();

        if glob.is_match(relative_str.as_ref()) {
            results.push(path.clone());
        }

        // Continue recursing if we haven't hit max depth
        if depth + 1 < max_depth {
            collect_matching_dirs(root, &path, glob, depth + 1, max_depth, results);
        }
    }
}

/// Read the package name and scripts from a `package.json` file.
/// Falls back to using the directory name if `name` is missing.
fn read_package_info(dir: &Path) -> (String, IndexMap<String, String>) {
    let fallback_name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let pkg = match PackageJson::load(dir) {
        Some(p) => p,
        None => return (fallback_name, IndexMap::new()),
    };

    let scripts = pkg.scripts();
    let name = pkg.name.unwrap_or(fallback_name);

    (name, scripts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, contents: &str) {
        fs::write(dir.join(name), contents).unwrap();
    }

    fn setup_monorepo_npm(tmp: &TempDir) {
        // Root package.json with workspaces
        write_file(
            tmp.path(),
            "package.json",
            r#"{"name":"monorepo","workspaces":["packages/*"]}"#,
        );

        // packages/app
        let app = tmp.path().join("packages").join("app");
        fs::create_dir_all(&app).unwrap();
        write_file(
            &app,
            "package.json",
            r#"{"name":"@mono/app","scripts":{"dev":"vite","build":"tsc"}}"#,
        );

        // packages/lib
        let lib = tmp.path().join("packages").join("lib");
        fs::create_dir_all(&lib).unwrap();
        write_file(
            &lib,
            "package.json",
            r#"{"name":"@mono/lib","scripts":{"test":"vitest"}}"#,
        );
    }

    #[test]
    fn scans_npm_workspaces() {
        let tmp = TempDir::new().unwrap();
        setup_monorepo_npm(&tmp);

        let pkgs = scan_workspaces(tmp.path());
        assert_eq!(pkgs.len(), 2);

        let names: Vec<&str> = pkgs.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"@mono/app"));
        assert!(names.contains(&"@mono/lib"));

        let app = pkgs.iter().find(|p| p.name == "@mono/app").unwrap();
        assert_eq!(app.scripts.len(), 2);
        assert_eq!(app.scripts["dev"], "vite");
    }

    #[test]
    fn scans_pnpm_workspaces() {
        let tmp = TempDir::new().unwrap();

        write_file(tmp.path(), "package.json", r#"{"name":"monorepo"}"#);
        write_file(
            tmp.path(),
            "pnpm-workspace.yaml",
            "packages:\n  - 'packages/*'\n",
        );

        let pkg_dir = tmp.path().join("packages").join("core");
        fs::create_dir_all(&pkg_dir).unwrap();
        write_file(
            &pkg_dir,
            "package.json",
            r#"{"name":"@mono/core","scripts":{"build":"tsup"}}"#,
        );

        let pkgs = scan_workspaces(tmp.path());
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "@mono/core");
        assert_eq!(pkgs[0].scripts["build"], "tsup");
    }

    #[test]
    fn scans_workspaces_object_format() {
        let tmp = TempDir::new().unwrap();

        write_file(
            tmp.path(),
            "package.json",
            r#"{"name":"monorepo","workspaces":{"packages":["packages/*"]}}"#,
        );

        let pkg_dir = tmp.path().join("packages").join("ui");
        fs::create_dir_all(&pkg_dir).unwrap();
        write_file(
            &pkg_dir,
            "package.json",
            r#"{"name":"@mono/ui","scripts":{"storybook":"sb dev"}}"#,
        );

        let pkgs = scan_workspaces(tmp.path());
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "@mono/ui");
    }

    #[test]
    fn uses_directory_name_when_name_missing() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "package.json",
            r#"{"name":"monorepo","workspaces":["packages/*"]}"#,
        );

        let pkg_dir = tmp.path().join("packages").join("unnamed");
        fs::create_dir_all(&pkg_dir).unwrap();
        write_file(&pkg_dir, "package.json", r#"{"scripts":{"dev":"node ."}}"#);

        let pkgs = scan_workspaces(tmp.path());
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "unnamed");
    }

    #[test]
    fn returns_empty_when_no_patterns() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "package.json", r#"{"name":"plain"}"#);

        let pkgs = scan_workspaces(tmp.path());
        assert!(pkgs.is_empty());
    }

    #[test]
    fn skips_dirs_without_package_json() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "package.json",
            r#"{"name":"monorepo","workspaces":["packages/*"]}"#,
        );

        // Directory without package.json
        let empty_dir = tmp.path().join("packages").join("empty");
        fs::create_dir_all(&empty_dir).unwrap();

        // Directory with package.json
        let real_dir = tmp.path().join("packages").join("real");
        fs::create_dir_all(&real_dir).unwrap();
        write_file(&real_dir, "package.json", r#"{"name":"real"}"#);

        let pkgs = scan_workspaces(tmp.path());
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "real");
    }

    #[test]
    fn relative_path_is_correct() {
        let tmp = TempDir::new().unwrap();
        setup_monorepo_npm(&tmp);

        let pkgs = scan_workspaces(tmp.path());
        let app = pkgs.iter().find(|p| p.name == "@mono/app").unwrap();
        assert_eq!(app.relative_path, "packages/app");
    }
}
