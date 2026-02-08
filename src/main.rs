use anyhow::{Context, Result};
use std::process;

mod app;
mod core;
mod fuzzy;
mod sort;
mod store;
mod ui;

fn main() -> Result<()> {
    // 0. Handle CLI arguments
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("nr {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("nr — TUI-based npm script runner with fuzzy search");
        println!();
        println!("USAGE: nr");
        println!();
        println!("Run in a directory containing package.json to interactively");
        println!("browse and execute npm scripts.");
        println!();
        println!("OPTIONS:");
        println!("  -h, --help     Print this help message");
        println!("  -V, --version  Print version");
        return Ok(());
    }

    // 1. Core discovery (before TUI)
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let root = core::project_root::find_project_root(&cwd)?;

    let pm_root = root.monorepo_root.as_ref().unwrap_or(&root.nearest_pkg);
    let package_manager = core::package_manager::detect_package_manager(pm_root);
    let scripts = core::scripts::load_scripts(&root.nearest_pkg);

    if scripts.is_empty() {
        anyhow::bail!(
            "No scripts found in {}/package.json",
            root.nearest_pkg.display()
        );
    }

    let workspace_packages = root
        .monorepo_root
        .as_ref()
        .map(|r| core::workspaces::scan_workspaces(r))
        .unwrap_or_default();

    let config_dir = store::config_path::ensure_config_dir();
    let proj_id = store::project_id::project_id(pm_root);

    // 2. Install panic hook so terminal is restored on panic
    install_panic_hook();

    // 3. Initialize TUI
    let mut terminal = ratatui::init();
    let mut app = app::App::new(
        scripts,
        workspace_packages,
        root.nearest_pkg,
        root.monorepo_root,
        &config_dir,
        proj_id,
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
    if let app::Action::RunScript { script_name, cwd } = action {
        store::favorites::save_favorites(&config_dir, &app.favorites);
        store::recents::save_recents(&config_dir, &app.recents);

        let exit_code = core::runner::run_script(package_manager, &script_name, &cwd);
        process::exit(exit_code);
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
