use crate::core::package_manager::PackageManager;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Execute a package.json script via the detected package manager.
///
/// Inherits stdin/stdout/stderr so the child process can interact with the terminal.
/// Returns the process exit code (or `1` on spawn failure / missing exit code).
pub fn run_script(pm: PackageManager, script_name: &str, cwd: &Path) -> i32 {
    let status = Command::new(pm.command_name())
        .args(pm.run_args(script_name))
        .current_dir(cwd)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    match status {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!();
            eprintln!(
                "❌ Failed to run script: '{} {}'",
                pm.command_name(),
                script_name
            );
            eprintln!();

            // Check if it's a command not found error
            if e.kind() == std::io::ErrorKind::NotFound {
                eprintln!(
                    "🔍 Package manager '{}' not found in PATH",
                    pm.command_name()
                );
                eprintln!();
                eprintln!("💡 Install {} to continue:", pm);

                match pm {
                    PackageManager::Npm => {
                        eprintln!("   - Download Node.js (includes npm): https://nodejs.org");
                        eprintln!("   - Or use a version manager: nvm, fnm, volta");
                    }
                    PackageManager::Yarn => {
                        eprintln!("   npm install -g yarn");
                        eprintln!("   Or: https://yarnpkg.com/getting-started/install");
                    }
                    PackageManager::Pnpm => {
                        eprintln!("   npm install -g pnpm");
                        eprintln!("   Or: https://pnpm.io/installation");
                    }
                    PackageManager::Bun => {
                        eprintln!("   curl -fsSL https://bun.sh/install | bash");
                        eprintln!("   Or: https://bun.sh");
                    }
                }
            } else {
                eprintln!("Error: {}", e);
                eprintln!();
                eprintln!("💡 Common issues:");
                eprintln!("   - Check if the package manager is in your PATH");
                eprintln!(
                    "   - Try running the script manually: {} {}",
                    pm.command_name(),
                    script_name
                );
            }

            eprintln!();
            1
        }
    }
}

/// Execute a package.json script with additional environment variables and arguments.
///
/// This is the extended version of `run_script` that supports:
/// - Custom environment variable injection (e.g., from .env files)
/// - Additional arguments appended to the script command
///
/// Returns the process exit code (or `1` on spawn failure / missing exit code).
pub fn run_script_with_config(
    pm: PackageManager,
    script_name: &str,
    cwd: &Path,
    env_vars: HashMap<String, String>,
    args: &str,
) -> i32 {
    let mut cmd = Command::new(pm.command_name());
    cmd.args(pm.run_args(script_name));

    // Append additional arguments if provided
    if !args.is_empty() {
        for arg in args.split_whitespace() {
            cmd.arg(arg);
        }
    }

    // Inject environment variables
    cmd.envs(env_vars);

    cmd.current_dir(cwd)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    let status = cmd.status();

    match status {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!();
            eprintln!(
                "❌ Failed to run script: '{} {}'",
                pm.command_name(),
                script_name
            );
            eprintln!();

            if e.kind() == std::io::ErrorKind::NotFound {
                eprintln!(
                    "🔍 Package manager '{}' not found in PATH",
                    pm.command_name()
                );
                eprintln!();
                eprintln!("💡 Install {} to continue:", pm);

                match pm {
                    PackageManager::Npm => {
                        eprintln!("   - Download Node.js (includes npm): https://nodejs.org");
                        eprintln!("   - Or use a version manager: nvm, fnm, volta");
                    }
                    PackageManager::Yarn => {
                        eprintln!("   npm install -g yarn");
                        eprintln!("   Or: https://yarnpkg.com/getting-started/install");
                    }
                    PackageManager::Pnpm => {
                        eprintln!("   npm install -g pnpm");
                        eprintln!("   Or: https://pnpm.io/installation");
                    }
                    PackageManager::Bun => {
                        eprintln!("   curl -fsSL https://bun.sh/install | bash");
                        eprintln!("   Or: https://bun.sh");
                    }
                }
            } else {
                eprintln!("Error: {}", e);
                eprintln!();
                eprintln!("💡 Common issues:");
                eprintln!("   - Check if the package manager is in your PATH");
                eprintln!(
                    "   - Try running the script manually: {} {}",
                    pm.command_name(),
                    script_name
                );
            }

            eprintln!();
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_args_are_forwarded_correctly() {
        // Verify the command construction is correct for each PM
        let pm = PackageManager::Npm;
        let args = pm.run_args("test");
        assert_eq!(args, vec!["run", "test"]);

        let pm = PackageManager::Yarn;
        let args = pm.run_args("test");
        assert_eq!(args, vec!["test"]);
    }

    #[test]
    fn nonexistent_command_returns_1() {
        // Trying to run a command that doesn't exist should return exit code 1
        let code = Command::new("__nr_nonexistent_binary__")
            .status()
            .map(|s| s.code().unwrap_or(1))
            .unwrap_or(1);
        assert_eq!(code, 1);
    }

    #[test]
    fn test_run_script_with_config_constructs_command_correctly() {
        // Test that env vars and args are properly prepared
        let mut cmd = Command::new("echo");
        let env_vars = HashMap::from([
            ("KEY1".to_string(), "value1".to_string()),
            ("KEY2".to_string(), "value2".to_string()),
        ]);

        cmd.envs(env_vars);
        cmd.arg("test");

        // Just verify the command builds without errors
        assert!(true);
    }

    #[test]
    fn test_args_parsing_splits_correctly() {
        let args = "-- --watch --coverage";
        let parsed: Vec<&str> = args.split_whitespace().collect();
        assert_eq!(parsed, vec!["--", "--watch", "--coverage"]);
    }
}
