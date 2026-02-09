use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use indexmap::IndexMap;
use nr::app::{Action, App, AppMode};
use nr::core::package_manager::PackageManager;
use std::collections::HashSet;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test app with minimal setup
fn create_test_app(project_dir: &std::path::Path) -> App {
    let mut scripts = IndexMap::new();
    scripts.insert("test".to_string(), "echo test".to_string());
    scripts.insert("build".to_string(), "echo build".to_string());

    App::new(
        scripts,
        vec![],
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
fn test_configure_flow_starts_with_tab_key() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create env files
    fs::write(project_dir.join(".env"), "VAR=test").unwrap();

    let mut app = create_test_app(project_dir);

    // Initially in Normal mode
    assert_eq!(app.mode, AppMode::Normal);

    // Press Tab to start configure flow
    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    let action = app.handle_key(key);

    // Should stay in app (Continue action) and switch to ConfigureEnv mode
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.mode, AppMode::ConfigureEnv);

    // Should have scanned env files
    assert!(app.env_files_list.is_some());
}

#[test]
fn test_configure_flow_restores_global_env() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    // Create env files
    fs::write(project_dir.join(".env"), "VAR=test").unwrap();
    fs::write(project_dir.join(".env.local"), "LOCAL=test").unwrap();

    let mut app = create_test_app(project_dir);

    // Simulate previous global env selection
    app.global_env_config.last_env_files = vec![".env".to_string(), ".env.local".to_string()];

    // Start configure flow with Tab
    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    app.handle_key(key);

    // Should have pre-selected the global env files
    assert_eq!(app.env_selected_files.len(), 2);

    // Verify correct files are selected
    let selected_names: HashSet<String> = app
        .env_files_list
        .as_ref()
        .unwrap()
        .all_files()
        .filter(|f| app.env_selected_files.contains(&f.path))
        .map(|f| f.display_name.clone())
        .collect();

    assert!(selected_names.contains(".env"));
    assert!(selected_names.contains(".env.local"));
}

#[test]
fn test_configure_flow_restores_script_specific_args() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(project_dir.join(".env"), "VAR=test").unwrap();

    let mut app = create_test_app(project_dir);

    // First execution: set args
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)); // Start flow
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)); // Skip env
    // Type some args
    app.handle_key(KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)); // To confirm
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)); // Execute

    // Reset to normal mode after "execution"
    app.mode = AppMode::Normal;

    // Second execution: should restore args
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)); // To args mode

    // Args should be restored in args_input after entering args mode
    // (when mode switches to ConfigureArgs, args from execution_config should be in args_input)
    assert_eq!(app.args_input, "-w");
}

#[test]
fn test_configure_flow_env_to_args_transition() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(project_dir.join(".env"), "VAR=test").unwrap();

    let mut app = create_test_app(project_dir);

    // Start at env selection
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::ConfigureEnv);

    // Press Enter to proceed to args
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(action, Action::Continue));
    assert_eq!(app.mode, AppMode::ConfigureArgs);
}

#[test]
fn test_configure_flow_args_to_confirm_transition() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(project_dir.join(".env"), "VAR=test").unwrap();

    let mut app = create_test_app(project_dir);

    // Navigate to args mode
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::ConfigureArgs);

    // Press Enter to proceed to confirmation
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(matches!(action, Action::Continue));
    assert_eq!(app.mode, AppMode::ConfirmExecution);
}

#[test]
fn test_configure_flow_esc_navigation() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(project_dir.join(".env"), "VAR=test").unwrap();

    let mut app = create_test_app(project_dir);

    // Go to confirmation
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::ConfirmExecution);

    // Esc should go back to args
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::ConfigureArgs);

    // Esc should go back to env
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::ConfigureEnv);

    // Esc should cancel and go back to normal
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn test_configure_flow_saves_global_env_on_execution() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(project_dir.join(".env"), "VAR=test").unwrap();
    fs::write(project_dir.join(".env.local"), "LOCAL=test").unwrap();

    let mut app = create_test_app(project_dir);

    // Start configure flow
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

    // Select .env file (first one)
    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

    // Proceed to args
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    // Type args
    app.handle_key(KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));

    // Proceed to confirmation
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    // Execute
    let action = app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    // Should return RunScript action
    assert!(matches!(action, Action::RunScript { .. }));

    // Global env should be saved (at least one file selected)
    assert!(!app.global_env_config.last_env_files.is_empty());
}

#[test]
fn test_configure_flow_saves_script_args_on_execution() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(project_dir.join(".env"), "VAR=test").unwrap();

    let mut app = create_test_app(project_dir);

    let initial_config_count = app.script_configs.len();

    // Start configure flow and go through it
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)); // Skip env selection

    // Type args
    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)); // To confirmation
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)); // Execute

    // Script config should be saved (count should increase)
    assert!(app.script_configs.len() > initial_config_count);
}

#[test]
fn test_configure_flow_env_selection_toggle() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(project_dir.join(".env"), "VAR=test").unwrap();

    let mut app = create_test_app(project_dir);

    // Start configure flow
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

    let initial_count = app.env_selected_files.len();

    // Toggle selection with Space
    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

    let after_toggle = app.env_selected_files.len();

    // Count should change
    assert_ne!(initial_count, after_toggle);
}

#[test]
fn test_configure_flow_args_input_editing() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(project_dir.join(".env"), "VAR=test").unwrap();

    let mut app = create_test_app(project_dir);

    // Navigate to args mode
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    // Type some args
    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));

    assert_eq!(app.args_input, "test");

    // Backspace should delete
    app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    assert_eq!(app.args_input, "tes");

    // Cursor should be at end
    assert_eq!(app.args_cursor_pos, 3);
}

#[test]
fn test_configure_flow_args_cursor_movement() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(project_dir.join(".env"), "VAR=test").unwrap();

    let mut app = create_test_app(project_dir);

    // Navigate to args mode
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    // Type "test"
    for c in "test".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }

    assert_eq!(app.args_cursor_pos, 4);

    // Move cursor left
    app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    assert_eq!(app.args_cursor_pos, 3);

    // Move cursor to start
    app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    assert_eq!(app.args_cursor_pos, 0);

    // Move cursor to end
    app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    assert_eq!(app.args_cursor_pos, 4);
}

#[test]
fn test_multiple_scripts_share_global_env_in_app() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path();

    fs::write(project_dir.join(".env"), "VAR=test").unwrap();

    let mut app = create_test_app(project_dir);

    // Configure first script (test)
    app.selected_index = 0;
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)); // Select .env
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE)); // Args "1"
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)); // Execute

    let global_env_after_first = app.global_env_config.last_env_files.clone();

    // Switch to second script (build)
    app.selected_index = 1;
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

    // Global env should be pre-selected
    let pre_selected_count = app.env_selected_files.len();
    assert_eq!(global_env_after_first.len(), pre_selected_count);
}

#[test]
fn test_configure_flow_with_monorepo() {
    let temp_dir = TempDir::new().unwrap();
    let root_dir = temp_dir.path();
    let package_dir = root_dir.join("packages").join("web");
    fs::create_dir_all(&package_dir).unwrap();

    // Create package.json files
    fs::write(
        root_dir.join("package.json"),
        r#"{"name":"root","workspaces":["packages/*"]}"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("package.json"),
        r#"{"name":"web","scripts":{"dev":"vite"}}"#,
    )
    .unwrap();

    // Create env files
    fs::write(root_dir.join(".env"), "ROOT=yes").unwrap();
    fs::write(package_dir.join(".env"), "PKG=yes").unwrap();

    let mut scripts = IndexMap::new();
    scripts.insert("dev".to_string(), "vite".to_string());

    let app = App::new(
        scripts,
        vec![],
        package_dir.clone(),
        Some(root_dir.to_path_buf()),
        &package_dir,
        "web".to_string(),
        package_dir.display().to_string(),
        "npm".to_string(),
        PackageManager::Npm,
    );

    // Should be able to start configure flow
    assert_eq!(app.mode, AppMode::Normal);
}
