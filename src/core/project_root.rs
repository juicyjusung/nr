use crate::core::package_json::PackageJson;
use std::path::{Path, PathBuf};

/// Result of project root discovery.
#[derive(Debug)]
pub struct ProjectRoot {
    /// Path to the directory containing the nearest `package.json`.
    pub nearest_pkg: PathBuf,
    /// Path to the monorepo root (contains `workspaces` in package.json or pnpm-workspace.yaml).
    pub monorepo_root: Option<PathBuf>,
}

/// Errors that can occur during project root discovery.
#[derive(Debug, thiserror::Error)]
pub enum ProjectRootError {
    #[error(
        "No package.json found in any parent directory.\n\n💡 To use nr, you need a Node.js project with package.json.\n\nCreate one by running:\n   npm init -y\n   # or\n   yarn init -y\n   # or\n   pnpm init\n   # or\n   bun init\n\nThen add scripts to your package.json and run 'nr' again."
    )]
    NotFound,
}

/// Two-phase upward traversal from `cwd` to locate the nearest `package.json`
/// and, optionally, a monorepo root above it.
///
/// Phase 1: Walk `cwd.ancestors()` to find the first directory containing `package.json`.
/// Phase 2: Continue upward from that directory's parent looking for a `package.json`
///           with a `"workspaces"` field, or a `pnpm-workspace.yaml` file.
pub fn find_project_root(cwd: &Path) -> Result<ProjectRoot, ProjectRootError> {
    // Phase 1: find nearest package.json
    let nearest_pkg = cwd
        .ancestors()
        .find(|dir| dir.join("package.json").is_file())
        .map(Path::to_path_buf)
        .ok_or(ProjectRootError::NotFound)?;

    // Phase 2: check if nearest_pkg itself is a monorepo root, then search above
    let monorepo_root = if is_monorepo_root(&nearest_pkg) {
        Some(nearest_pkg.clone())
    } else {
        find_monorepo_root(&nearest_pkg)
    };

    Ok(ProjectRoot {
        nearest_pkg,
        monorepo_root,
    })
}

/// Check if a directory itself is a monorepo root (has workspaces in package.json or pnpm-workspace.yaml).
fn is_monorepo_root(dir: &Path) -> bool {
    if dir.join("pnpm-workspace.yaml").is_file() {
        return true;
    }
    PackageJson::load(dir).is_some_and(|pkg| pkg.workspaces.is_some())
}

/// Starting from `start_dir`'s parent, walk upward looking for a monorepo root indicator:
/// - A `package.json` containing a `"workspaces"` key
/// - A `pnpm-workspace.yaml` file
fn find_monorepo_root(start_dir: &Path) -> Option<PathBuf> {
    let parent = start_dir.parent()?;

    for ancestor in parent.ancestors() {
        if is_monorepo_root(ancestor) {
            return Some(ancestor.to_path_buf());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a file with given contents inside `dir`.
    fn write_file(dir: &Path, name: &str, contents: &str) {
        fs::write(dir.join(name), contents).unwrap();
    }

    #[test]
    fn finds_nearest_package_json_in_cwd() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "package.json", r#"{"name":"root"}"#);

        let result = find_project_root(tmp.path()).unwrap();
        assert_eq!(result.nearest_pkg, tmp.path());
        assert!(result.monorepo_root.is_none());
    }

    #[test]
    fn finds_nearest_package_json_in_parent() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "package.json", r#"{"name":"root"}"#);

        let child = tmp.path().join("src").join("deep");
        fs::create_dir_all(&child).unwrap();

        let result = find_project_root(&child).unwrap();
        assert_eq!(result.nearest_pkg, tmp.path());
    }

    #[test]
    fn returns_not_found_when_no_package_json() {
        let tmp = TempDir::new().unwrap();
        let child = tmp.path().join("empty");
        fs::create_dir_all(&child).unwrap();

        let result = find_project_root(&child);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ProjectRootError::NotFound));
    }

    #[test]
    fn detects_monorepo_root_with_workspaces_field() {
        let tmp = TempDir::new().unwrap();

        // Monorepo root
        write_file(
            tmp.path(),
            "package.json",
            r#"{"name":"monorepo","workspaces":["packages/*"]}"#,
        );

        // Nested package
        let pkg_dir = tmp.path().join("packages").join("app");
        fs::create_dir_all(&pkg_dir).unwrap();
        write_file(&pkg_dir, "package.json", r#"{"name":"app"}"#);

        let result = find_project_root(&pkg_dir).unwrap();
        assert_eq!(result.nearest_pkg, pkg_dir);
        assert_eq!(result.monorepo_root.unwrap(), tmp.path());
    }

    #[test]
    fn detects_monorepo_root_with_pnpm_workspace_yaml() {
        let tmp = TempDir::new().unwrap();

        // Monorepo root with pnpm workspace
        write_file(tmp.path(), "package.json", r#"{"name":"monorepo"}"#);
        write_file(
            tmp.path(),
            "pnpm-workspace.yaml",
            "packages:\n  - 'packages/*'\n",
        );

        // Nested package
        let pkg_dir = tmp.path().join("packages").join("lib");
        fs::create_dir_all(&pkg_dir).unwrap();
        write_file(&pkg_dir, "package.json", r#"{"name":"lib"}"#);

        let result = find_project_root(&pkg_dir).unwrap();
        assert_eq!(result.nearest_pkg, pkg_dir);
        assert_eq!(result.monorepo_root.unwrap(), tmp.path());
    }

    #[test]
    fn no_monorepo_when_parent_has_no_workspaces() {
        let tmp = TempDir::new().unwrap();

        // Parent without workspaces
        write_file(tmp.path(), "package.json", r#"{"name":"parent"}"#);

        // Child package
        let child = tmp.path().join("sub");
        fs::create_dir_all(&child).unwrap();
        write_file(&child, "package.json", r#"{"name":"child"}"#);

        let result = find_project_root(&child).unwrap();
        assert_eq!(result.nearest_pkg, child);
        assert!(result.monorepo_root.is_none());
    }

    #[test]
    fn monorepo_root_equals_nearest_pkg_when_at_root_level() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "package.json",
            r#"{"name":"root","workspaces":["packages/*"]}"#,
        );

        // Running from the monorepo root itself — nearest_pkg IS the monorepo root,
        // so monorepo_root should equal nearest_pkg.
        let result = find_project_root(tmp.path()).unwrap();
        assert_eq!(result.nearest_pkg, tmp.path());
        assert_eq!(result.monorepo_root.unwrap(), tmp.path());
    }
}
