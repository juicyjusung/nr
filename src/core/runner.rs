use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::core::package_manager::PackageManager;

/// Execute a package.json script via the detected package manager.
///
/// Inherits stdin/stdout/stderr so the child process can interact with the terminal.
/// Returns the process exit code (or `1` on spawn failure / missing exit code).
pub fn run_script(pm: PackageManager, script_name: &str, cwd: &Path) -> i32 {
    let mut command = script_command(pm, script_name, cwd, &HashMap::new(), "");
    execute_command(&mut command, pm, script_name)
}

fn script_command(
    pm: PackageManager,
    script_name: &str,
    cwd: &Path,
    env_vars: &HashMap<String, String>,
    args: &str,
) -> Command {
    let mut command = Command::new(pm.command_name());
    command.args(pm.run_args(script_name));

    if !args.is_empty() {
        command.args(args.split_whitespace());
    }

    command.envs(env_vars).current_dir(cwd);
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    command
}

fn execute_command(command: &mut Command, pm: PackageManager, script_name: &str) -> i32 {
    match command.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            let displayed_command = match pm {
                PackageManager::Yarn => format!("{} run {}", pm.command_name(), script_name),
                _ => format!("{} {}", pm.command_name(), script_name),
            };

            eprintln!();
            eprintln!("❌ Failed to run script: '{displayed_command}'");
            eprintln!();

            if error.kind() == std::io::ErrorKind::NotFound {
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
                eprintln!("Error: {error}");
                eprintln!();
                eprintln!("💡 Common issues:");
                eprintln!("   - Check if the package manager is in your PATH");
                eprintln!("   - Try running the script manually: {displayed_command}");
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
    let mut command = script_command(pm, script_name, cwd, &env_vars, args);
    execute_command(&mut command, pm, script_name)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn script_command_sets_program_script_and_configured_arguments() {
        let command = script_command(
            PackageManager::Yarn,
            "add",
            Path::new("/tmp/project"),
            &HashMap::new(),
            "-- --watch",
        );

        assert_eq!(command.get_program(), OsStr::new("yarn"));
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["run", "add", "--", "--watch"].map(OsStr::new)
        );
    }

    #[test]
    fn script_command_sets_current_directory() {
        let cwd = Path::new("/tmp/project");

        let command = script_command(PackageManager::Npm, "test", cwd, &HashMap::new(), "");

        assert_eq!(command.get_current_dir(), Some(cwd));
    }

    #[test]
    fn script_command_injects_environment_variables() {
        let command = script_command(
            PackageManager::Npm,
            "test",
            Path::new("/tmp/project"),
            &HashMap::from([("NODE_ENV".to_string(), "test".to_string())]),
            "",
        );

        assert_eq!(
            command.get_envs().collect::<Vec<_>>(),
            vec![(OsStr::new("NODE_ENV"), Some(OsStr::new("test")))]
        );
    }

    #[test]
    fn execute_command_returns_1_when_program_does_not_exist() {
        let mut command = Command::new("__nr_test_binary_that_does_not_exist__");

        let code = execute_command(&mut command, PackageManager::Npm, "test");

        assert_eq!(code, 1);
    }
}
