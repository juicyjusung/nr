use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nr::app::{Action, App};
use nr::core::package_manager::PackageManager;
use nr::core::task_catalog::{TaskCatalog, TaskQuery};
use nr::store::favorites::{load_favorites, save_favorites};
use nr::store::project_id::project_id;
use nr::store::recents::{RecentEntry, load_recents, now_ms, save_recents};
use nr::store::script_configs::{ScriptConfig, load_script_configs, save_script_configs};
use tempfile::TempDir;

fn write_package_json(directory: &std::path::Path, contents: &str) {
    fs::create_dir_all(directory).unwrap();
    fs::write(directory.join("package.json"), contents).unwrap();
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn canonical_task_key(catalog: &TaskCatalog, relative_path: &str, script_name: &str) -> String {
    let favorites = HashSet::new();
    catalog
        .query(TaskQuery::new("", &favorites, &[], now_ms()))
        .into_iter()
        .find_map(|handle| {
            let task = catalog.resolve(handle)?;
            (task.relative_path == relative_path && task.script_name == script_name)
                .then(|| task.persistence_key().to_string())
        })
        .unwrap()
}

fn script_config_key(config_dir: &Path, task_key: &str) -> String {
    format!("{}:{task_key}", project_id(config_dir))
}

fn type_query(app: &mut App, query: &str) {
    for character in query.chars() {
        app.handle_key(key(KeyCode::Char(character)));
    }
}

fn restored_args_for_query(project: &Path, config: &Path, query: &str) -> String {
    let mut app = App::from_catalog(
        TaskCatalog::discover(project).unwrap(),
        config,
        "npm".to_string(),
        PackageManager::Npm,
    );
    type_query(&mut app, query);
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Enter));
    app.args_input
}

#[test]
fn unique_legacy_favorite_and_recent_fixtures_rank_the_same_workspace_and_root_tasks() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name":"repo",
            "workspaces":["packages/*"],
            "scripts":{
                "alpha":"echo alpha",
                "root-check":"echo root"
            }
        }"#,
    );
    write_package_json(
        &project.path().join("packages/app"),
        r#"{"name":"app","scripts":{"workspace-check":"echo workspace"}}"#,
    );
    let discovery = TaskCatalog::discover(project.path()).unwrap();
    let alpha_key = canonical_task_key(&discovery.0, ".", "alpha");
    let root_key = canonical_task_key(&discovery.0, ".", "root-check");
    let workspace_key = canonical_task_key(&discovery.0, "packages/app", "workspace-check");
    let config = TempDir::new().unwrap();
    let legacy_workspace_favorite = "app:workspace-check".to_string();
    let legacy_root_recent = "root:root-check".to_string();
    let favorites = HashSet::from([legacy_workspace_favorite.clone()]);
    let recents = vec![RecentEntry {
        key: legacy_root_recent.clone(),
        last_run: now_ms(),
        count: 12,
    }];
    save_favorites(config.path(), &favorites).unwrap();
    save_recents(config.path(), &recents).unwrap();

    let app = App::from_catalog(
        discovery,
        config.path(),
        "npm".to_string(),
        PackageManager::Npm,
    );
    let actual: Vec<_> = app
        .scripts
        .iter()
        .map(|script| (script.name.as_str(), script.key.as_str()))
        .collect();

    assert_eq!(
        (
            actual,
            app.favorites.contains(&legacy_workspace_favorite),
            app.favorites.contains(&workspace_key),
            app.recents
                .iter()
                .any(|entry| entry.key == legacy_root_recent),
        ),
        (
            vec![
                ("workspace-check", workspace_key.as_str()),
                ("root-check", root_key.as_str()),
                ("alpha", alpha_key.as_str()),
            ],
            true,
            true,
            true,
        )
    );
}

#[test]
fn unique_legacy_script_config_fixtures_restore_for_the_same_root_and_workspace_tasks() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name":"repo",
            "workspaces":["packages/*"],
            "scripts":{"root-check":"echo root"}
        }"#,
    );
    write_package_json(
        &project.path().join("packages/app"),
        r#"{"name":"app","scripts":{"workspace-check":"echo workspace"}}"#,
    );
    let config = TempDir::new().unwrap();
    let configs = HashMap::from([
        (
            script_config_key(config.path(), "root:root-check"),
            ScriptConfig {
                args: "--root-fixture".to_string(),
                last_used: SystemTime::now(),
            },
        ),
        (
            script_config_key(config.path(), "app:workspace-check"),
            ScriptConfig {
                args: "--workspace-fixture".to_string(),
                last_used: SystemTime::now(),
            },
        ),
    ]);
    save_script_configs(config.path(), &configs).unwrap();

    let actual = (
        restored_args_for_query(project.path(), config.path(), "root-check"),
        restored_args_for_query(project.path(), config.path(), "workspace-check"),
    );

    assert_eq!(
        actual,
        (
            "--root-fixture".to_string(),
            "--workspace-fixture".to_string()
        )
    );
}

#[test]
fn workspace_task_restores_unique_legacy_package_config() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","workspaces":["packages/*"],"scripts":{}}"#,
    );
    write_package_json(
        &project.path().join("packages/app"),
        r#"{"name":"@acme/app","scripts":{"dev":"vite"}}"#,
    );
    let config = TempDir::new().unwrap();
    let legacy_key = format!("{}:@acme/app:dev", project_id(config.path()));
    let configs = HashMap::from([(
        legacy_key,
        ScriptConfig {
            args: "--host 0.0.0.0".to_string(),
            last_used: SystemTime::now(),
        },
    )]);
    save_script_configs(config.path(), &configs).unwrap();
    let mut app = App::from_catalog(
        TaskCatalog::discover(project.path()).unwrap(),
        config.path(),
        "npm".to_string(),
        PackageManager::Npm,
    );

    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.args_input, "--host 0.0.0.0");
}

#[test]
fn ambiguous_legacy_state_is_preserved_without_applying_to_colliding_tasks() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name":"repo",
            "workspaces":["packages/*"],
            "scripts":{
                "alpha":"echo alpha",
                "dev":"rootonlytoken"
            }
        }"#,
    );
    write_package_json(
        &project.path().join("packages/app"),
        r#"{"name":"app","scripts":{"dev":"workspaceonlytoken"}}"#,
    );
    let config = TempDir::new().unwrap();
    let ambiguous_legacy_key = "root:dev".to_string();
    let ambiguous_config_key = script_config_key(config.path(), &ambiguous_legacy_key);
    save_favorites(
        config.path(),
        &HashSet::from([ambiguous_legacy_key.clone()]),
    )
    .unwrap();
    save_recents(
        config.path(),
        &[RecentEntry {
            key: ambiguous_legacy_key.clone(),
            last_run: now_ms(),
            count: 100,
        }],
    )
    .unwrap();
    let configs = HashMap::from([(
        ambiguous_config_key.clone(),
        ScriptConfig {
            args: "--legacy".to_string(),
            last_used: SystemTime::now(),
        },
    )]);
    save_script_configs(config.path(), &configs).unwrap();
    let mut app = App::from_catalog(
        TaskCatalog::discover(project.path()).unwrap(),
        config.path(),
        "npm".to_string(),
        PackageManager::Npm,
    );
    let initial_order: Vec<_> = app
        .scripts
        .iter()
        .map(|script| (script.name.as_str(), script.command.as_str()))
        .collect();

    assert_eq!(
        initial_order,
        vec![
            ("alpha", "echo alpha"),
            ("dev", "rootonlytoken"),
            ("dev", "workspaceonlytoken"),
        ]
    );

    type_query(&mut app, "rootonlytoken");
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Enter));
    assert!(app.args_input.is_empty());
    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Enter));
    save_favorites(config.path(), &app.favorites).unwrap();
    save_recents(config.path(), &app.recents).unwrap();

    let saved_favorites = load_favorites(config.path());
    let saved_recents = load_recents(config.path());
    let saved_configs = load_script_configs(config.path()).unwrap();
    let workspace_args =
        restored_args_for_query(project.path(), config.path(), "workspaceonlytoken");

    assert_eq!(
        (
            saved_favorites.contains(&ambiguous_legacy_key),
            saved_recents
                .iter()
                .any(|entry| entry.key == ambiguous_legacy_key),
            saved_configs
                .get(&ambiguous_config_key)
                .map(|config| config.args.as_str()),
            saved_configs.len(),
            workspace_args,
        ),
        (true, true, Some("--legacy"), 2, String::new())
    );
}

#[test]
fn configured_workspace_run_uses_the_selected_package_directory() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","workspaces":["packages/*"],"scripts":{}}"#,
    );
    let package = project.path().join("packages/app");
    write_package_json(&package, r#"{"name":"app","scripts":{"dev":"vite"}}"#);
    let config = TempDir::new().unwrap();
    let mut app = App::from_catalog(
        TaskCatalog::discover(project.path()).unwrap(),
        config.path(),
        "npm".to_string(),
        PackageManager::Npm,
    );

    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Char('-')));
    app.handle_key(key(KeyCode::Char('w')));
    app.handle_key(key(KeyCode::Enter));
    let action = app.handle_key(key(KeyCode::Enter));

    assert!(matches!(
        action,
        Action::RunScript { script_name, cwd, args, .. }
            if script_name == "dev" && cwd == package && args == "-w"
    ));
}
