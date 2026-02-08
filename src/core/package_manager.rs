use std::path::Path;

/// Supported Node.js package managers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Bun,
    Pnpm,
    Yarn,
    Npm,
}

impl PackageManager {
    /// Arguments to pass after the binary name to run a script.
    pub fn run_args<'a>(&self, script_name: &'a str) -> Vec<&'a str> {
        match self {
            Self::Bun => vec!["run", script_name],
            Self::Pnpm => vec!["run", script_name],
            Self::Yarn => vec![script_name],
            Self::Npm => vec!["run", script_name],
        }
    }

    /// The CLI binary name for this package manager.
    pub fn command_name(&self) -> &str {
        match self {
            Self::Bun => "bun",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Npm => "npm",
        }
    }
}

impl std::fmt::Display for PackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.command_name())
    }
}

/// Detect the package manager for a project by checking lockfiles and package.json.
///
/// Priority order:
/// 1. `bun.lockb` or `bun.lock` -> Bun
/// 2. `pnpm-lock.yaml` -> Pnpm
/// 3. `yarn.lock` -> Yarn
/// 4. `package-lock.json` -> Npm
/// 5. `packageManager` field in `package.json` -> parse PM name
/// 6. Fallback -> Npm
pub fn detect_package_manager(project_root: &Path) -> PackageManager {
    // Lockfile-based detection (highest priority)
    if project_root.join("bun.lockb").exists() || project_root.join("bun.lock").exists() {
        return PackageManager::Bun;
    }
    if project_root.join("pnpm-lock.yaml").exists() {
        return PackageManager::Pnpm;
    }
    if project_root.join("yarn.lock").exists() {
        return PackageManager::Yarn;
    }
    if project_root.join("package-lock.json").exists() {
        return PackageManager::Npm;
    }

    // packageManager field in package.json
    if let Some(pm) = detect_from_package_json(project_root) {
        return pm;
    }

    // Fallback
    PackageManager::Npm
}

/// Parse the `packageManager` field from `package.json` (e.g. `"pnpm@9.1.0"`).
fn detect_from_package_json(project_root: &Path) -> Option<PackageManager> {
    let pkg = crate::core::package_json::PackageJson::load(project_root)?;
    let pm_field = pkg.package_manager?;

    // Format: "name@version" or just "name"
    let name = pm_field.split('@').next().unwrap_or(&pm_field).trim();

    match name {
        "bun" => Some(PackageManager::Bun),
        "pnpm" => Some(PackageManager::Pnpm),
        "yarn" => Some(PackageManager::Yarn),
        "npm" => Some(PackageManager::Npm),
        _ => None,
    }
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
    fn detects_bun_from_lockb() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "bun.lockb", "");
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Bun);
    }

    #[test]
    fn detects_bun_from_bun_lock() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "bun.lock", "");
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Bun);
    }

    #[test]
    fn detects_pnpm_from_lockfile() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "pnpm-lock.yaml", "");
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Pnpm);
    }

    #[test]
    fn detects_yarn_from_lockfile() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "yarn.lock", "");
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Yarn);
    }

    #[test]
    fn detects_npm_from_lockfile() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "package-lock.json", "");
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Npm);
    }

    #[test]
    fn bun_lockfile_takes_priority_over_others() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "bun.lockb", "");
        write_file(tmp.path(), "yarn.lock", "");
        write_file(tmp.path(), "package-lock.json", "");
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Bun);
    }

    #[test]
    fn pnpm_takes_priority_over_yarn_and_npm() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "pnpm-lock.yaml", "");
        write_file(tmp.path(), "yarn.lock", "");
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Pnpm);
    }

    #[test]
    fn detects_pnpm_from_package_manager_field() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "package.json",
            r#"{"packageManager":"pnpm@9.1.0"}"#,
        );
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Pnpm);
    }

    #[test]
    fn detects_yarn_from_package_manager_field() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "package.json",
            r#"{"packageManager":"yarn@4.0.0"}"#,
        );
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Yarn);
    }

    #[test]
    fn detects_bun_from_package_manager_field() {
        let tmp = TempDir::new().unwrap();
        write_file(
            tmp.path(),
            "package.json",
            r#"{"packageManager":"bun@1.0.0"}"#,
        );
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Bun);
    }

    #[test]
    fn lockfile_takes_priority_over_package_manager_field() {
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "yarn.lock", "");
        write_file(
            tmp.path(),
            "package.json",
            r#"{"packageManager":"pnpm@9.0.0"}"#,
        );
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Yarn);
    }

    #[test]
    fn falls_back_to_npm_when_nothing_found() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(detect_package_manager(tmp.path()), PackageManager::Npm);
    }

    #[test]
    fn run_args_correct_for_each_pm() {
        assert_eq!(PackageManager::Bun.run_args("dev"), vec!["run", "dev"]);
        assert_eq!(PackageManager::Pnpm.run_args("dev"), vec!["run", "dev"]);
        assert_eq!(PackageManager::Yarn.run_args("dev"), vec!["dev"]);
        assert_eq!(PackageManager::Npm.run_args("dev"), vec!["run", "dev"]);
    }

    #[test]
    fn command_name_correct_for_each_pm() {
        assert_eq!(PackageManager::Bun.command_name(), "bun");
        assert_eq!(PackageManager::Pnpm.command_name(), "pnpm");
        assert_eq!(PackageManager::Yarn.command_name(), "yarn");
        assert_eq!(PackageManager::Npm.command_name(), "npm");
    }

    #[test]
    fn display_matches_command_name() {
        assert_eq!(format!("{}", PackageManager::Bun), "bun");
        assert_eq!(format!("{}", PackageManager::Npm), "npm");
    }
}
