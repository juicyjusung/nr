use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::time::Instant;

use nr::core::package_json::PackageJsonError;
use nr::core::task_catalog::{CatalogError, TaskCatalog, TaskQuery, TaskUsageRecord};
use tempfile::TempDir;

const NOW_MS: u64 = 1_800_000_000_000;

fn write_package_json(directory: &std::path::Path, contents: &str) {
    fs::create_dir_all(directory).unwrap();
    fs::write(directory.join("package.json"), contents).unwrap();
}

#[test]
fn standalone_catalog_discovers_and_resolves_nearest_tasks() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name": "solo",
            "scripts": {
                "test": "vitest",
                "dev": "vite"
            }
        }"#,
    );
    let launch_directory = project.path().join("src/deep");
    fs::create_dir_all(&launch_directory).unwrap();

    let catalog = TaskCatalog::discover(&launch_directory).unwrap().0;
    let favorites = HashSet::new();
    let handles = catalog.query(TaskQuery::new("", &favorites, &[], NOW_MS));
    let actual: Vec<_> = handles
        .into_iter()
        .map(|handle| {
            let task = catalog.resolve(handle).unwrap();
            (
                task.scope_label.as_str(),
                task.package_name.as_str(),
                task.relative_path.as_str(),
                task.script_name.as_str(),
                task.command.as_str(),
                task.cwd.as_path(),
            )
        })
        .collect();

    assert_eq!(
        actual,
        vec![
            ("root", "solo", ".", "dev", "vite", project.path()),
            ("root", "solo", ".", "test", "vitest", project.path()),
        ]
    );
}

#[test]
fn workspace_only_root_discovers_runnable_member() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name": "repo",
            "private": true,
            "workspaces": ["packages/*"],
            "scripts": {}
        }"#,
    );
    let package = project.path().join("packages/app");
    write_package_json(
        &package,
        r#"{
            "name": "@acme/app",
            "scripts": {"dev": "vite"}
        }"#,
    );

    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::new();
    let handles = catalog.query(TaskQuery::new("", &favorites, &[], NOW_MS));
    let task = catalog.resolve(handles[0]).unwrap();

    assert_eq!(
        (
            handles.len(),
            task.scope_label.as_str(),
            task.package_name.as_str(),
            task.relative_path.as_str(),
            task.script_name.as_str(),
            task.command.as_str(),
            task.cwd.as_path(),
        ),
        (
            1,
            "@acme/app",
            "@acme/app",
            "packages/app",
            "dev",
            "vite",
            package.as_path(),
        )
    );
}

#[test]
fn pnpm_negative_pattern_excludes_member() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","private":true,"scripts":{}}"#,
    );
    fs::write(
        project.path().join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n  - '!packages/private-*'\n",
    )
    .unwrap();
    write_package_json(
        &project.path().join("packages/public"),
        r#"{"name":"public","scripts":{"build":"echo public"}}"#,
    );
    write_package_json(
        &project.path().join("packages/private-secret"),
        r#"{"name":"private","scripts":{"build":"echo private"}}"#,
    );

    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::new();
    let handles = catalog.query(TaskQuery::new("", &favorites, &[], NOW_MS));
    let actual: Vec<_> = handles
        .into_iter()
        .map(|handle| catalog.resolve(handle).unwrap().relative_path.as_str())
        .collect();

    assert_eq!(actual, vec!["packages/public"]);
}

#[test]
fn query_matches_script_name_fuzzily() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name": "solo",
            "scripts": {
                "build": "vite build",
                "test": "vitest",
                "dev": "vite"
            }
        }"#,
    );

    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::new();
    let handles = catalog.query(TaskQuery::new("tst", &favorites, &[], NOW_MS));
    let actual: Vec<_> = handles
        .into_iter()
        .map(|handle| catalog.resolve(handle).unwrap().script_name.as_str())
        .collect();

    assert_eq!(actual, vec!["test"]);
}

#[test]
fn query_matches_package_name() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","workspaces":["packages/*"],"scripts":{}}"#,
    );
    write_package_json(
        &project.path().join("packages/web"),
        r#"{"name":"@acme/alpha","scripts":{"serve":"vite"}}"#,
    );

    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::new();
    let handles = catalog.query(TaskQuery::new("alpha", &favorites, &[], NOW_MS));
    let actual: Vec<_> = handles
        .into_iter()
        .map(|handle| catalog.resolve(handle).unwrap().script_name.as_str())
        .collect();

    assert_eq!(actual, vec!["serve"]);
}

#[test]
fn query_matches_workspace_relative_path() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","workspaces":["tools/*"],"scripts":{}}"#,
    );
    write_package_json(
        &project.path().join("tools/build-path"),
        r#"{"name":"@acme/gamma","scripts":{"serve":"vite"}}"#,
    );

    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::new();
    let handles = catalog.query(TaskQuery::new("build-path", &favorites, &[], NOW_MS));
    let actual: Vec<_> = handles
        .into_iter()
        .map(|handle| catalog.resolve(handle).unwrap().script_name.as_str())
        .collect();

    assert_eq!(actual, vec!["serve"]);
}

#[test]
fn query_matches_command_text() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name": "solo",
            "scripts": {
                "verify": "vitest --coverage",
                "serve": "vite"
            }
        }"#,
    );

    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::new();
    let handles = catalog.query(TaskQuery::new("coverage", &favorites, &[], NOW_MS));
    let actual: Vec<_> = handles
        .into_iter()
        .map(|handle| catalog.resolve(handle).unwrap().script_name.as_str())
        .collect();

    assert_eq!(actual, vec!["verify"]);
}

#[test]
fn favorite_breaks_equal_query_relevance_and_accepts_unique_legacy_key() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","workspaces":["packages/*"],"scripts":{}}"#,
    );
    write_package_json(
        &project.path().join("packages/a"),
        r#"{"name":"a","scripts":{"test":"vitest"}}"#,
    );
    write_package_json(
        &project.path().join("packages/b"),
        r#"{"name":"b","scripts":{"test":"vitest"}}"#,
    );

    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::from(["b:test".to_string()]);
    let handles = catalog.query(TaskQuery::new("test", &favorites, &[], NOW_MS));
    let actual: Vec<_> = handles
        .into_iter()
        .map(|handle| catalog.resolve(handle).unwrap().package_name.as_str())
        .collect();

    assert_eq!(actual, vec!["b", "a"]);
}

#[test]
fn frecency_breaks_equal_query_relevance() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","workspaces":["packages/*"],"scripts":{}}"#,
    );
    write_package_json(
        &project.path().join("packages/a"),
        r#"{"name":"a","scripts":{"test":"vitest"}}"#,
    );
    write_package_json(
        &project.path().join("packages/b"),
        r#"{"name":"b","scripts":{"test":"vitest"}}"#,
    );

    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::new();
    let recents = vec![TaskUsageRecord::new("b:test", NOW_MS, 4)];
    let handles = catalog.query(TaskQuery::new("test", &favorites, &recents, NOW_MS));
    let actual: Vec<_> = handles
        .into_iter()
        .map(|handle| catalog.resolve(handle).unwrap().package_name.as_str())
        .collect();

    assert_eq!(actual, vec!["b", "a"]);
}

#[test]
fn empty_query_keeps_favorites_then_frecency_then_alphabetical_order() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name": "solo",
            "scripts": {
                "build": "vite build",
                "dev": "vite",
                "test": "vitest"
            }
        }"#,
    );

    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::from(["root:test".to_string()]);
    let recents = vec![TaskUsageRecord::new("root:dev", NOW_MS, 3)];
    let handles = catalog.query(TaskQuery::new("", &favorites, &recents, NOW_MS));
    let actual: Vec<_> = handles
        .into_iter()
        .map(|handle| catalog.resolve(handle).unwrap().script_name.as_str())
        .collect();

    assert_eq!(actual, vec!["test", "dev", "build"]);
}

#[test]
fn root_and_nested_launch_discover_the_same_declared_universe() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name":"repo",
            "workspaces":["packages/*"],
            "scripts":{"lint":"eslint ."}
        }"#,
    );
    let app = project.path().join("packages/app");
    write_package_json(&app, r#"{"name":"app","scripts":{"dev":"vite"}}"#);
    write_package_json(
        &project.path().join("packages/lib"),
        r#"{"name":"lib","scripts":{"test":"vitest"}}"#,
    );
    let nested = app.join("src/deep");
    fs::create_dir_all(&nested).unwrap();

    let root_catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let nested_catalog = TaskCatalog::discover(&nested).unwrap().0;
    let favorites = HashSet::new();
    let resolve_universe = |catalog: &TaskCatalog| {
        catalog
            .query(TaskQuery::new("", &favorites, &[], NOW_MS))
            .into_iter()
            .map(|handle| {
                let task = catalog.resolve(handle).unwrap();
                (
                    task.identity.clone(),
                    task.relative_path.clone(),
                    task.script_name.clone(),
                    task.cwd.clone(),
                )
            })
            .collect::<BTreeSet<_>>()
    };

    assert_eq!(
        resolve_universe(&root_catalog),
        resolve_universe(&nested_catalog)
    );
}

#[test]
fn overlapping_workspace_patterns_emit_each_task_once() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name":"repo",
            "workspaces":["packages/*","packages/app"],
            "scripts":{}
        }"#,
    );
    write_package_json(
        &project.path().join("packages/app"),
        r#"{"name":"app","scripts":{"dev":"vite"}}"#,
    );

    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::new();
    let handles = catalog.query(TaskQuery::new("", &favorites, &[], NOW_MS));

    assert_eq!(handles.len(), 1);
}

#[test]
fn mixed_catalog_skips_empty_members_and_resolves_every_task_deterministically() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name":"repo",
            "workspaces":["packages/*"],
            "scripts":{"build":"echo root"}
        }"#,
    );
    write_package_json(
        &project.path().join("packages/a"),
        r#"{"name":"a","scripts":{"test":"vitest a"}}"#,
    );
    write_package_json(
        &project.path().join("packages/b"),
        r#"{"name":"b","scripts":{"test":"vitest b"}}"#,
    );
    write_package_json(
        &project.path().join("packages/empty"),
        r#"{"name":"empty","scripts":{}}"#,
    );

    let (catalog, _) = TaskCatalog::discover(project.path()).unwrap();
    let favorites = HashSet::new();
    let handles = catalog.query(TaskQuery::new("", &favorites, &[], NOW_MS));
    let unique_handles: HashSet<_> = handles.iter().copied().collect();
    let actual: Vec<_> = handles
        .iter()
        .map(|handle| {
            let task = catalog.resolve(*handle).unwrap();
            (task.relative_path.as_str(), task.script_name.as_str())
        })
        .collect();

    assert_eq!(handles.len(), unique_handles.len());
    assert_eq!(
        actual,
        vec![
            (".", "build"),
            ("packages/a", "test"),
            ("packages/b", "test")
        ]
    );
}

#[test]
fn nested_undeclared_launch_package_joins_the_declared_project_universe() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name":"repo",
            "workspaces":["packages/*"],
            "scripts":{"lint":"eslint ."}
        }"#,
    );
    write_package_json(
        &project.path().join("packages/declared"),
        r#"{"name":"declared","scripts":{"test":"vitest"}}"#,
    );
    let current = project.path().join("tools/local");
    write_package_json(&current, r#"{"name":"local","scripts":{"dev":"vite"}}"#);
    let launch_directory = current.join("src/deep");
    fs::create_dir_all(&launch_directory).unwrap();

    let (catalog, _) = TaskCatalog::discover(&launch_directory).unwrap();
    let favorites = HashSet::new();
    let actual: BTreeSet<_> = catalog
        .query(TaskQuery::new("", &favorites, &[], NOW_MS))
        .into_iter()
        .map(|handle| {
            let task = catalog.resolve(handle).unwrap();
            (task.relative_path.as_str(), task.script_name.as_str())
        })
        .collect();

    assert_eq!(
        actual,
        BTreeSet::from([
            (".", "lint"),
            ("packages/declared", "test"),
            ("tools/local", "dev"),
        ])
    );
}

#[test]
fn empty_project_returns_project_wide_no_tasks_error() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"repo","workspaces":["packages/*"],"scripts":{}}"#,
    );
    write_package_json(
        &project.path().join("packages/empty"),
        r#"{"name":"empty","scripts":{}}"#,
    );

    let error = TaskCatalog::discover(project.path()).err().unwrap();

    assert!(matches!(error, CatalogError::NoTasks { .. }));
    assert!(
        error
            .to_string()
            .contains("project root or declared workspaces")
    );
}

#[test]
fn query_uses_the_discovered_filesystem_snapshot() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{"name":"solo","scripts":{"dev":"vite"}}"#,
    );
    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    write_package_json(
        project.path(),
        r#"{"name":"solo","scripts":{"dev":"webpack"}}"#,
    );
    let favorites = HashSet::new();

    let old = catalog.query(TaskQuery::new("vite", &favorites, &[], NOW_MS));
    let new = catalog.query(TaskQuery::new("webpack", &favorites, &[], NOW_MS));

    assert_eq!((old.len(), new.len()), (1, 0));
}

#[test]
fn script_name_relevance_beats_favorite_command_match() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name":"solo",
            "scripts":{
                "build":"echo build",
                "dev":"build-tool"
            }
        }"#,
    );
    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::from(["root:dev".to_string()]);
    let handles = catalog.query(TaskQuery::new("build", &favorites, &[], NOW_MS));
    let first = catalog.resolve(handles[0]).unwrap();

    assert_eq!(first.script_name, "build");
}

#[test]
fn unicode_package_and_script_are_searchable() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name":"도구",
            "scripts":{"검사":"echo 확인"}
        }"#,
    );
    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::new();
    let handles = catalog.query(TaskQuery::new("검사", &favorites, &[], NOW_MS));
    let task = catalog.resolve(handles[0]).unwrap();

    assert_eq!(
        (task.package_name.as_str(), task.script_name.as_str()),
        ("도구", "검사")
    );
}

#[test]
fn duplicate_package_and_script_names_have_distinct_labels_and_targets() {
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
    let catalog = TaskCatalog::discover(project.path()).unwrap().0;
    let favorites = HashSet::new();
    let handles = catalog.query(TaskQuery::new("test", &favorites, &[], NOW_MS));
    let actual: BTreeSet<_> = handles
        .into_iter()
        .map(|handle| {
            let task = catalog.resolve(handle).unwrap();
            (task.scope_label.as_str(), task.cwd.as_path())
        })
        .collect();

    assert_eq!(
        actual,
        BTreeSet::from([
            ("duplicate (packages/a)", package_a.as_path()),
            ("duplicate (packages/b)", package_b.as_path()),
        ])
    );
}

#[test]
fn workspace_named_root_does_not_collide_with_the_reserved_root_scope() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name":"repo",
            "workspaces":["packages/*"],
            "scripts":{"test":"vitest root"}
        }"#,
    );
    let workspace = project.path().join("packages/app");
    write_package_json(
        &workspace,
        r#"{"name":"root","scripts":{"test":"vitest workspace"}}"#,
    );

    let (catalog, _) = TaskCatalog::discover(project.path()).unwrap();
    let favorites = HashSet::new();
    let actual: BTreeSet<_> = catalog
        .query(TaskQuery::new("test", &favorites, &[], NOW_MS))
        .into_iter()
        .map(|handle| {
            let task = catalog.resolve(handle).unwrap();
            (task.scope_label.as_str(), task.cwd.as_path())
        })
        .collect();

    assert_eq!(
        actual,
        BTreeSet::from([
            ("root", project.path()),
            ("root (packages/app)", workspace.as_path()),
        ])
    );
}

#[test]
fn invalid_nearest_manifest_is_a_typed_discovery_error() {
    let project = TempDir::new().unwrap();
    write_package_json(project.path(), "not json");

    let error = TaskCatalog::discover(project.path()).err().unwrap();

    match error {
        CatalogError::RequiredManifest { path, source } => {
            assert_eq!(path, project.path().join("package.json"));
            assert!(matches!(source, PackageJsonError::Parse { .. }));
        }
        other => panic!("expected required manifest error, got {other:?}"),
    }
}

#[test]
fn invalid_monorepo_root_manifest_is_fatal_from_a_nested_package() {
    let project = TempDir::new().unwrap();
    write_package_json(project.path(), "not json");
    fs::write(
        project.path().join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    let package = project.path().join("packages/app");
    write_package_json(&package, r#"{"name":"app","scripts":{"dev":"vite"}}"#);

    let error = TaskCatalog::discover(&package).err().unwrap();

    match error {
        CatalogError::RequiredManifest { path, source } => {
            assert_eq!(path, project.path().join("package.json"));
            assert!(matches!(source, PackageJsonError::Parse { .. }));
        }
        other => panic!("expected required manifest error, got {other:?}"),
    }
}

#[test]
fn invalid_optional_workspace_manifest_is_ignored() {
    let project = TempDir::new().unwrap();
    write_package_json(
        project.path(),
        r#"{
            "name":"repo",
            "workspaces":["packages/*"],
            "scripts":{"lint":"eslint ."}
        }"#,
    );
    write_package_json(&project.path().join("packages/broken"), "not json");
    write_package_json(
        &project.path().join("packages/valid"),
        r#"{"name":"valid","scripts":{"test":"vitest"}}"#,
    );

    let (catalog, _) = TaskCatalog::discover(project.path()).unwrap();
    let favorites = HashSet::new();
    let tasks: BTreeSet<_> = catalog
        .query(TaskQuery::new("", &favorites, &[], NOW_MS))
        .into_iter()
        .map(|handle| {
            let task = catalog.resolve(handle).unwrap();
            (task.relative_path.as_str(), task.script_name.as_str())
        })
        .collect();

    assert_eq!(
        tasks,
        BTreeSet::from([(".", "lint"), ("packages/valid", "test")])
    );
}

#[test]
fn large_catalog_query_workload_records_elapsed_time_and_keeps_exact_results_first() {
    const TASK_COUNT: usize = 5_000;
    const QUERY_COUNT: usize = 24;

    let project = TempDir::new().unwrap();
    let scripts: serde_json::Map<_, _> = (0..TASK_COUNT)
        .map(|index| {
            (
                format!("task-{index:05}"),
                serde_json::Value::String(format!("runner --job job-{index:05}")),
            )
        })
        .collect();
    write_package_json(
        project.path(),
        &serde_json::json!({
            "name": "large-project",
            "scripts": scripts,
        })
        .to_string(),
    );
    let (catalog, _) = TaskCatalog::discover(project.path()).unwrap();
    let favorites = HashSet::new();
    let queries: Vec<_> = (0..QUERY_COUNT)
        .map(|index| format!("task-{:05}", index * 197 % TASK_COUNT))
        .collect();

    let started = Instant::now();
    let exact_results_stay_first = queries.iter().all(|query| {
        let handles = catalog.query(TaskQuery::new(query, &favorites, &[], NOW_MS));
        handles
            .first()
            .and_then(|handle| catalog.resolve(*handle))
            .is_some_and(|task| task.script_name == *query)
    });
    let elapsed = started.elapsed();

    eprintln!("queried {TASK_COUNT} catalog tasks {QUERY_COUNT} times in {elapsed:?}");
    assert!(exact_results_stay_first);
}
