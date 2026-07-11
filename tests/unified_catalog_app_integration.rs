use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nr::app::{Action, App};
use nr::core::package_manager::PackageManager;
use nr::core::task_catalog::TaskCatalog;
use tempfile::TempDir;

fn write_package_json(directory: &std::path::Path, contents: &str) {
    fs::create_dir_all(directory).unwrap();
    fs::write(directory.join("package.json"), contents).unwrap();
}

#[test]
fn scripts_palette_runs_workspace_task_from_workspace_only_root() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","workspaces":["packages/*"],"scripts":{}}"#,
    );
    let package = project.path().join("packages/app");
    write_package_json(&package, r#"{"name":"@acme/app","scripts":{"dev":"vite"}}"#);
    let config = TempDir::new().unwrap();
    let catalog = TaskCatalog::discover(project.path()).unwrap();
    let mut app = App::from_catalog(
        catalog,
        config.path(),
        "npm".to_string(),
        PackageManager::Npm,
    );

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(
        action,
        Action::RunScript {
            script_name,
            cwd,
            env_files,
            args,
        } if script_name == "dev"
            && cwd == package
            && env_files.is_empty()
            && args.is_empty()
    ));
}

#[test]
fn packages_drill_down_resolves_the_same_task_as_scripts_palette() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","workspaces":["packages/*"],"scripts":{}}"#,
    );
    let package = project.path().join("packages/app");
    write_package_json(&package, r#"{"name":"@acme/app","scripts":{"dev":"vite"}}"#);
    let scripts_config = TempDir::new().unwrap();
    let packages_config = TempDir::new().unwrap();
    let mut scripts_app = App::from_catalog(
        TaskCatalog::discover(project.path()).unwrap(),
        scripts_config.path(),
        "npm".to_string(),
        PackageManager::Npm,
    );
    let mut packages_app = App::from_catalog(
        TaskCatalog::discover(project.path()).unwrap(),
        packages_config.path(),
        "npm".to_string(),
        PackageManager::Npm,
    );

    let scripts_action = scripts_app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    packages_app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    packages_app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let packages_action =
        packages_app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let target = |action| match action {
        Action::RunScript {
            script_name, cwd, ..
        } => (script_name, cwd),
        _ => panic!("expected a script execution"),
    };
    assert_eq!(target(scripts_action), target(packages_action));
}

#[test]
fn scripts_palette_searches_workspace_package_name() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","workspaces":["packages/*"],"scripts":{}}"#,
    );
    let package = project.path().join("packages/app");
    write_package_json(&package, r#"{"name":"@acme/app","scripts":{"dev":"vite"}}"#);
    let config = TempDir::new().unwrap();
    let mut app = App::from_catalog(
        TaskCatalog::discover(project.path()).unwrap(),
        config.path(),
        "npm".to_string(),
        PackageManager::Npm,
    );
    for character in "acme".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
    }

    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(
        action,
        Action::RunScript { script_name, cwd, .. }
            if script_name == "dev" && cwd == package
    ));
}

#[test]
fn duplicate_script_selection_runs_the_exact_selected_package() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","workspaces":["packages/*"],"scripts":{}}"#,
    );
    let package_a = project.path().join("packages/a");
    let package_b = project.path().join("packages/b");
    write_package_json(
        &package_a,
        r#"{"name":"duplicate","scripts":{"test":"vitest a"}}"#,
    );
    write_package_json(
        &package_b,
        r#"{"name":"duplicate","scripts":{"test":"vitest b"}}"#,
    );
    let config = TempDir::new().unwrap();
    let mut app = App::from_catalog(
        TaskCatalog::discover(project.path()).unwrap(),
        config.path(),
        "npm".to_string(),
        PackageManager::Npm,
    );

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(
        action,
        Action::RunScript { script_name, cwd, .. }
            if script_name == "test" && cwd == package_b
    ));
}
