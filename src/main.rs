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
    let wants_any_reset = wants_reset || wants_reset_favorites || wants_reset_recents || wants_reset_configs;

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

    // 2. Install panic hook so terminal is restored on panic
    install_panic_hook();

    // 3. Initialize TUI
    let mut terminal = ratatui::init();
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

    // 4. Event loop
    let action = loop {
        terminal.draw(|frame| app.render(frame))?;

        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            // Skip release/repeat events on some terminals
            if key.kind != crossterm::event::KeyEventKind::Press {
                continue;
            }
            let result = app.handle_key(key);
            match result {
                app::Action::Quit => break app::Action::Quit,
                app::Action::RunScript { .. } => break result,
                app::Action::Continue => {}
            }
        }
    };

    // 5. Restore terminal
    ratatui::restore();

    // 6. Execute script (after TUI cleanup)
    if let app::Action::RunScript { script_name, cwd, env_files, args } = action {
        store::favorites::save_favorites(&project_dir, &app.favorites);
        store::recents::save_recents(&project_dir, &app.recents);

        let exit_code = if env_files.is_empty() && args.is_empty() {
            // Fast path: no configuration
            core::runner::run_script(package_manager, &script_name, &cwd)
        } else {
            // Load and merge env files
            let env_vars = core::env_files::load_env_files(&env_files).unwrap_or_default();
            core::runner::run_script_with_config(package_manager, &script_name, &cwd, env_vars, &args)
        };
        
        process::exit(exit_code);
    }

    Ok(())
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
            std::fs::remove_file(&script_configs_path).context("Failed to remove script_configs.json")?;
            cleared.push("script configs");
        } else {
            cleared.push("script configs (already empty)");
        }
        
        if args_history_path.exists() {
            std::fs::remove_file(&args_history_path).context("Failed to remove args_history.json")?;
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

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();
        original_hook(panic_info);
    }));
}
