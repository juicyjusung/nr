use anyhow::{Context, Result};
use nr::{app, core, store};
use std::process;

fn main() -> Result<()> {
    // 0. Handle CLI arguments
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("nr {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let wants_reset = args.iter().any(|a| a == "--reset");
    let wants_reset_favorites = args.iter().any(|a| a == "--reset-favorites");
    let wants_reset_recents = args.iter().any(|a| a == "--reset-recents");
    let wants_reset_configs = args.iter().any(|a| a == "--reset-configs");
    let wants_any_reset =
        wants_reset || wants_reset_favorites || wants_reset_recents || wants_reset_configs;

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("nr — TUI-based npm script runner with fuzzy search");
        println!();
        println!("USAGE: nr");
        println!();
        println!("Run in a directory containing package.json to interactively");
        println!("browse and execute npm scripts.");
        println!();
        println!("OPTIONS:");
        println!("  -h, --help            Print this help message");
        println!("  -V, --version         Print version");
        println!("  --reset               Clear favorites and recents for current project");
        println!("  --reset-favorites     Clear favorites for current project");
        println!("  --reset-recents       Clear recents for current project");
        return Ok(());
    }

    // 1. Core discovery (before TUI)
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let root = core::project_root::find_project_root(&cwd)?;

    let pm_root = root.monorepo_root.as_ref().unwrap_or(&root.nearest_pkg);
    let proj_id = store::project_id::project_id(pm_root);

    // Handle reset commands (no TUI needed)
    if wants_any_reset {
        let project_dir = store::config_path::get_project_dir(&proj_id);
        return handle_reset(
            &project_dir,
            wants_reset,
            wants_reset_favorites,
            wants_reset_recents,
            wants_reset_configs,
        );
    }

    let package_manager = core::package_manager::detect_package_manager(pm_root);
    let scripts = core::scripts::load_scripts(&root.nearest_pkg);

    if scripts.is_empty() {
        eprintln!(
            "❌ No scripts found in {}/package.json",
            root.nearest_pkg.display()
        );
        eprintln!();
        eprintln!("💡 To use nr, add scripts to your package.json:");
        eprintln!("   {{");
        eprintln!("     \"scripts\": {{");
        eprintln!("       \"dev\": \"vite\",");
        eprintln!("       \"build\": \"vite build\",");
        eprintln!("       \"test\": \"vitest\"");
        eprintln!("     }}");
        eprintln!("   }}");
        eprintln!();
        eprintln!("📖 Learn more: https://docs.npmjs.com/cli/v10/using-npm/scripts");
        process::exit(1);
    }

    let workspace_packages = root
        .monorepo_root
        .as_ref()
        .map(|r| core::workspaces::scan_workspaces(r))
        .unwrap_or_default();

    let project_dir = store::config_path::ensure_project_dir(&proj_id);

    let project_name = core::package_json::PackageJson::load(&root.nearest_pkg)
        .and_then(|pkg| pkg.name)
        .unwrap_or_else(|| "unknown".to_string());
    let project_path = pm_root.to_string_lossy().to_string();
    let pm_name = package_manager.to_string();

    let mut app = app::App::new(
        scripts,
        workspace_packages,
        root.nearest_pkg,
        root.monorepo_root,
        &project_dir,
        project_name,
        project_path,
        pm_name,
        package_manager,
    );
    let initial_favorites = app.favorites.clone();
    let outcome = ratatui::run(|terminal| run_tui(terminal, &mut app));
    let action = finalize_tui_session(&project_dir, &initial_favorites, &app, outcome)?;

    // Execute the script only after ratatui has restored the terminal.
    if let app::Action::RunScript {
        script_name,
        cwd,
        env_files,
        args,
    } = action
    {
        let exit_code = if env_files.is_empty() && args.is_empty() {
            // Fast path: no configuration
            core::runner::run_script(package_manager, &script_name, &cwd)
        } else {
            // Load and merge env files
            let env_vars = core::env_files::load_env_files(&env_files).unwrap_or_default();
            core::runner::run_script_with_config(
                package_manager,
                &script_name,
                &cwd,
                env_vars,
                &args,
            )
        };

        process::exit(exit_code);
    }

    Ok(())
}

fn run_tui(terminal: &mut ratatui::DefaultTerminal, app: &mut app::App) -> Result<app::Action> {
    loop {
        terminal.draw(|frame| app.render(frame))?;

        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            // Skip release/repeat events on some terminals
            if key.kind != crossterm::event::KeyEventKind::Press {
                continue;
            }
            let result = app.handle_key(key);
            match result {
                app::Action::Quit => break Ok(app::Action::Quit),
                app::Action::RunScript { .. } => break Ok(result),
                app::Action::Continue => {}
            }
        }
    }
}

fn finalize_tui_session(
    project_dir: &std::path::Path,
    initial_favorites: &std::collections::HashSet<String>,
    app: &app::App,
    outcome: Result<app::Action>,
) -> Result<app::Action> {
    if &app.favorites != initial_favorites {
        store::favorites::save_favorites(project_dir, &app.favorites);
    }

    if matches!(&outcome, Ok(app::Action::RunScript { .. })) {
        store::recents::save_recents(project_dir, &app.recents);
    }

    outcome
}

fn handle_reset(
    project_dir: &std::path::Path,
    reset_all: bool,
    reset_favorites: bool,
    reset_recents: bool,
    reset_configs: bool,
) -> Result<()> {
    let favorites_path = project_dir.join("favorites.json");
    let recents_path = project_dir.join("recents.json");
    let script_configs_path = project_dir.join("script_configs.json");
    let args_history_path = project_dir.join("args_history.json");

    let clear_favorites = reset_all || reset_favorites;
    let clear_recents = reset_all || reset_recents;
    let clear_configs = reset_all || reset_configs;

    let mut cleared = Vec::new();

    if clear_favorites {
        if favorites_path.exists() {
            std::fs::remove_file(&favorites_path).context("Failed to remove favorites.json")?;
            cleared.push("favorites");
        } else {
            cleared.push("favorites (already empty)");
        }
    }

    if clear_recents {
        if recents_path.exists() {
            std::fs::remove_file(&recents_path).context("Failed to remove recents.json")?;
            cleared.push("recents");
        } else {
            cleared.push("recents (already empty)");
        }
    }

    if clear_configs {
        if script_configs_path.exists() {
            std::fs::remove_file(&script_configs_path)
                .context("Failed to remove script_configs.json")?;
            cleared.push("script configs");
        } else {
            cleared.push("script configs (already empty)");
        }

        if args_history_path.exists() {
            std::fs::remove_file(&args_history_path)
                .context("Failed to remove args_history.json")?;
            cleared.push("args history");
        } else {
            cleared.push("args history (already empty)");
        }
    }

    if cleared.is_empty() {
        println!("Nothing to reset.");
    } else {
        println!("Reset complete: {}", cleared.join(", "));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nr::core::package_manager::PackageManager;
    use nr::store::recents::RecentEntry;
    use std::collections::HashSet;
    use std::path::Path;
    use tempfile::TempDir;

    fn test_app(project_dir: &Path) -> app::App {
        let mut scripts = indexmap::IndexMap::new();
        scripts.insert("dev".to_string(), "vite".to_string());

        app::App::new(
            scripts,
            Vec::new(),
            project_dir.to_path_buf(),
            None,
            project_dir,
            "test-project".to_string(),
            project_dir.display().to_string(),
            "npm".to_string(),
            PackageManager::Npm,
        )
    }

    #[test]
    fn changed_favorites_are_persisted_on_quit() {
        let temp_dir = TempDir::new().unwrap();
        let mut app = test_app(temp_dir.path());
        let initial_favorites = app.favorites.clone();
        let expected = HashSet::from(["root:dev".to_string()]);
        app.favorites = expected.clone();

        let _ = finalize_tui_session(
            temp_dir.path(),
            &initial_favorites,
            &app,
            Ok(app::Action::Quit),
        );

        assert_eq!(store::favorites::load_favorites(temp_dir.path()), expected);
    }

    #[test]
    fn favorites_and_recents_are_persisted_before_run_script_is_returned() {
        let temp_dir = TempDir::new().unwrap();
        let mut app = test_app(temp_dir.path());
        let initial_favorites = app.favorites.clone();
        let expected_favorites = HashSet::from(["root:dev".to_string()]);
        let expected_recents = vec![RecentEntry {
            key: "root:dev".to_string(),
            last_run: 1_000,
            count: 2,
        }];
        app.favorites = expected_favorites.clone();
        app.recents = expected_recents.clone();

        let outcome = finalize_tui_session(
            temp_dir.path(),
            &initial_favorites,
            &app,
            Ok(app::Action::RunScript {
                script_name: "dev".to_string(),
                cwd: temp_dir.path().to_path_buf(),
                env_files: Vec::new(),
                args: String::new(),
            }),
        );

        assert!(matches!(outcome, Ok(app::Action::RunScript { .. })));
        assert_eq!(
            (
                store::favorites::load_favorites(temp_dir.path()),
                store::recents::load_recents(temp_dir.path()),
            ),
            (expected_favorites, expected_recents)
        );
    }

    #[test]
    fn unchanged_favorites_do_not_overwrite_a_newer_disk_value_on_quit() {
        let temp_dir = TempDir::new().unwrap();
        let initial_disk_value = HashSet::from(["root:dev".to_string()]);
        store::favorites::save_favorites(temp_dir.path(), &initial_disk_value);
        let app = test_app(temp_dir.path());
        let initial_favorites = app.favorites.clone();
        let newer_disk_value = HashSet::from(["root:test".to_string()]);
        store::favorites::save_favorites(temp_dir.path(), &newer_disk_value);

        let _ = finalize_tui_session(
            temp_dir.path(),
            &initial_favorites,
            &app,
            Ok(app::Action::Quit),
        );

        assert_eq!(
            store::favorites::load_favorites(temp_dir.path()),
            newer_disk_value
        );
    }

    #[test]
    fn changed_favorites_are_persisted_when_original_tui_error_is_returned() {
        let temp_dir = TempDir::new().unwrap();
        let mut app = test_app(temp_dir.path());
        let initial_favorites = app.favorites.clone();
        let expected_favorites = HashSet::from(["root:dev".to_string()]);
        app.favorites = expected_favorites.clone();

        let outcome = finalize_tui_session(
            temp_dir.path(),
            &initial_favorites,
            &app,
            Err(anyhow::anyhow!("original TUI error")),
        );
        let returned_error = match outcome {
            Ok(_) => panic!("expected the original TUI error"),
            Err(error) => error,
        };

        assert_eq!(
            (
                store::favorites::load_favorites(temp_dir.path()),
                returned_error.to_string(),
            ),
            (expected_favorites, "original TUI error".to_string())
        );
    }
}
