//! Integration tests for App keyboard interaction scenarios

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use indexmap::IndexMap;
use nr::{Action, App};
use std::path::PathBuf;

// Helper functions for creating key events
fn key_char(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty())
}

fn key_enter() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
}

fn key_esc() -> KeyEvent {
    KeyEvent::new(KeyCode::Esc, KeyModifiers::empty())
}

fn key_up() -> KeyEvent {
    KeyEvent::new(KeyCode::Up, KeyModifiers::empty())
}

fn key_down() -> KeyEvent {
    KeyEvent::new(KeyCode::Down, KeyModifiers::empty())
}

fn key_space() -> KeyEvent {
    KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty())
}

fn key_backspace() -> KeyEvent {
    KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty())
}

fn key_ctrl_c() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
}

// Helper to create a test app
fn create_test_app() -> App {
    let mut scripts = IndexMap::new();
    scripts.insert("test".to_string(), "echo test".to_string());
    scripts.insert("build".to_string(), "echo build".to_string());
    scripts.insert("lint".to_string(), "echo lint".to_string());
    scripts.insert("dev".to_string(), "echo dev".to_string());

    App::new(
        scripts,
        vec![],
        PathBuf::from("/test/project"),
        None,
        &PathBuf::from("/tmp/test"),
        "test-project".to_string(),
        "/test/project".to_string(),
        "npm".to_string(),
        nr::core::package_manager::PackageManager::Npm,
    )
}

#[test]
fn test_complete_workflow_search_filter_and_select() {
    let mut app = create_test_app();

    // Initial state: 4 scripts visible
    assert_eq!(app.filtered_indices.len(), 4);
    assert_eq!(app.selected_index, 0);

    // Type 'te' to filter
    app.handle_key(key_char('t'));
    app.handle_key(key_char('e'));
    assert_eq!(app.query, "te");

    // Should filter to only "test"
    assert_eq!(app.filtered_indices.len(), 1);
    assert_eq!(app.selected_index, 0);

    // Press Enter to run
    let action = app.handle_key(key_enter());
    assert!(matches!(action, Action::RunScript { .. }));

    if let Action::RunScript { script_name, .. } = action {
        assert_eq!(script_name, "test");
    }
}

#[test]
fn test_navigation_with_arrow_keys() {
    let mut app = create_test_app();

    // Start at index 0
    assert_eq!(app.selected_index, 0);

    // Move down twice
    app.handle_key(key_down());
    assert_eq!(app.selected_index, 1);
    app.handle_key(key_down());
    assert_eq!(app.selected_index, 2);

    // Move up once
    app.handle_key(key_up());
    assert_eq!(app.selected_index, 1);
}

#[test]
fn test_favorite_toggle_and_sorting() {
    let mut app = create_test_app();

    // Initially no favorites
    assert_eq!(app.favorites.len(), 0);

    // Move to second item and toggle favorite
    app.handle_key(key_down());
    assert_eq!(app.selected_index, 1);

    app.handle_key(key_space());

    // Should have one favorite now
    assert_eq!(app.favorites.len(), 1);

    // Favorites should appear first in the filtered list
    // The favorited item should now be at index 0
    let first_script_key = &app.scripts[app.filtered_indices[0]].key;
    assert!(app.favorites.contains(first_script_key));
}

#[test]
fn test_backspace_clears_search() {
    let mut app = create_test_app();

    // Type a query
    app.handle_key(key_char('t'));
    app.handle_key(key_char('e'));
    app.handle_key(key_char('s'));
    assert_eq!(app.query, "tes");

    // Backspace to remove characters
    app.handle_key(key_backspace());
    assert_eq!(app.query, "te");
    app.handle_key(key_backspace());
    assert_eq!(app.query, "t");
    app.handle_key(key_backspace());
    assert_eq!(app.query, "");

    // Should show all scripts again
    assert_eq!(app.filtered_indices.len(), 4);
}

#[test]
fn test_ctrl_c_quits() {
    let mut app = create_test_app();

    let action = app.handle_key(key_ctrl_c());
    assert!(matches!(action, Action::Quit));
}

#[test]
fn test_esc_quits_from_scripts_tab() {
    let mut app = create_test_app();

    let action = app.handle_key(key_esc());
    assert!(matches!(action, Action::Quit));
}

#[test]
fn test_search_resets_selection_to_zero() {
    let mut app = create_test_app();

    // Move to third item
    app.handle_key(key_down());
    app.handle_key(key_down());
    assert_eq!(app.selected_index, 2);

    // Type a character (triggers filter update)
    app.handle_key(key_char('b'));

    // Selection should reset to 0
    assert_eq!(app.selected_index, 0);
}

#[test]
fn test_wrapping_navigation() {
    let mut app = create_test_app();

    // Start at first item
    assert_eq!(app.selected_index, 0);

    // Move up (should wrap to last)
    app.handle_key(key_up());
    assert_eq!(app.selected_index, app.filtered_indices.len() - 1);

    // Move down (should wrap to first)
    app.handle_key(key_down());
    assert_eq!(app.selected_index, 0);
}
