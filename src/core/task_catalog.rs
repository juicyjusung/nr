use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use indexmap::IndexMap;

use crate::core::package_json::{PackageJson, PackageJsonError};
use crate::core::project_root::{ProjectRootError, find_project_root};
use crate::core::workspaces::{WorkspacePackage, canonical_path, scan_workspaces};
use crate::fuzzy::fuzzy_matches;

/// An opaque reference to a task in one catalog snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskHandle(usize);

/// Stable identity for a task, independent of package display names and ordering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskIdentity {
    relative_path: String,
    script_name: String,
}

/// A runnable script resolved from a [`TaskHandle`].
#[derive(Debug, Clone)]
pub struct CatalogTask {
    pub identity: TaskIdentity,
    pub scope_label: String,
    pub package_name: String,
    pub relative_path: String,
    pub script_name: String,
    pub command: String,
    pub cwd: PathBuf,
    pub(crate) persistence_key: String,
    pub(crate) legacy_keys: Vec<String>,
}

impl CatalogTask {
    pub fn persistence_key(&self) -> &str {
        &self.persistence_key
    }

    pub fn legacy_keys(&self) -> &[String] {
        &self.legacy_keys
    }

    pub fn matches_persistence_key(&self, key: &str) -> bool {
        self.persistence_key == key || self.legacy_keys.iter().any(|legacy| legacy == key)
    }
}

/// Project paths discovered together with the task catalog.
#[derive(Debug, Clone)]
pub struct CatalogContext {
    pub project_root: PathBuf,
    pub nearest_package: PathBuf,
    pub monorepo_root: Option<PathBuf>,
    pub project_name: String,
}

/// Usage metadata consumed while ranking a catalog query.
#[derive(Debug, Clone, Copy)]
pub struct TaskUsageRecord<'a> {
    key: &'a str,
    last_run: u64,
    count: u32,
}

impl<'a> TaskUsageRecord<'a> {
    pub fn new(key: &'a str, last_run: u64, count: u32) -> Self {
        Self {
            key,
            last_run,
            count,
        }
    }
}

/// A query over an already-discovered catalog snapshot.
pub struct TaskQuery<'a> {
    text: &'a str,
    favorites: &'a HashSet<String>,
    recents: &'a [TaskUsageRecord<'a>],
    now_ms: u64,
}

impl<'a> TaskQuery<'a> {
    pub fn new(
        text: &'a str,
        favorites: &'a HashSet<String>,
        recents: &'a [TaskUsageRecord<'a>],
        now_ms: u64,
    ) -> Self {
        Self {
            text,
            favorites,
            recents,
            now_ms,
        }
    }
}

/// Errors returned while discovering a project task universe.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error(transparent)]
    ProjectRoot(#[from] ProjectRootError),
    #[error("Failed to load required package manifest at {path}: {source}")]
    RequiredManifest {
        path: PathBuf,
        #[source]
        source: PackageJsonError,
    },
    #[error(
        "No runnable scripts found in the project root or declared workspaces at {project_root}.\n\n💡 Add a string-valued script to a project package.json and run 'nr' again."
    )]
    NoTasks { project_root: String },
}

/// Project-wide snapshot of runnable package scripts.
pub struct TaskCatalog {
    tasks: Vec<CatalogTask>,
}

impl TaskCatalog {
    /// Discovers the nearest project and builds its task snapshot.
    pub fn discover(start_directory: &Path) -> Result<(Self, CatalogContext), CatalogError> {
        let root = find_project_root(start_directory)?;
        let nearest_package = root.nearest_pkg;
        let project_root = root
            .monorepo_root
            .clone()
            .unwrap_or_else(|| nearest_package.clone());
        let monorepo_root = root.monorepo_root;

        let nearest_manifest = load_required_manifest(&nearest_package)?;
        let project_name = nearest_manifest
            .name
            .clone()
            .unwrap_or_else(|| fallback_package_name(&nearest_package));
        let project_root_manifest =
            if canonical_path(&project_root) == canonical_path(&nearest_package) {
                nearest_manifest.clone()
            } else {
                load_required_manifest(&project_root)?
            };
        let packages = discover_packages(
            &project_root,
            &nearest_package,
            monorepo_root.is_some(),
            nearest_manifest,
            project_root_manifest,
        );
        let mut tasks = build_tasks(packages);
        assign_scope_labels(&mut tasks);
        assign_unique_legacy_keys(&mut tasks);

        if tasks.is_empty() {
            return Err(CatalogError::NoTasks {
                project_root: project_root.display().to_string(),
            });
        }

        Ok((
            Self { tasks },
            CatalogContext {
                project_root,
                nearest_package,
                monorepo_root,
                project_name,
            },
        ))
    }

    /// Returns task handles in display order without touching the filesystem.
    pub fn query(&self, query: TaskQuery<'_>) -> Vec<TaskHandle> {
        let favorite_flags: Vec<_> = self
            .tasks
            .iter()
            .map(|task| task_is_favorite(task, query.favorites))
            .collect();
        let frecency_scores: Vec<_> = self
            .tasks
            .iter()
            .map(|task| task_frecency(task, query.recents, query.now_ms))
            .collect();
        if !query.text.is_empty() {
            let script_scores = fuzzy_scores(&self.tasks, query.text, |task| &task.script_name);
            let package_scores = fuzzy_scores(&self.tasks, query.text, |task| &task.package_name);
            let path_scores = fuzzy_scores(&self.tasks, query.text, |task| &task.relative_path);
            let command_scores = fuzzy_scores(&self.tasks, query.text, |task| &task.command);
            let mut matches: Vec<_> = self
                .tasks
                .iter()
                .enumerate()
                .filter_map(|(index, _)| {
                    script_scores[index]
                        .map(|score| (index, MatchTier::ScriptName, score))
                        .or_else(|| {
                            package_scores[index]
                                .into_iter()
                                .chain(path_scores[index])
                                .max()
                                .map(|score| (index, MatchTier::PackageOrPath, score))
                        })
                        .or_else(|| {
                            command_scores[index].map(|score| (index, MatchTier::Command, score))
                        })
                })
                .collect();
            matches.sort_by(|left, right| {
                let left_favorite = favorite_flags[left.0];
                let right_favorite = favorite_flags[right.0];
                let left_frecency = frecency_scores[left.0];
                let right_frecency = frecency_scores[right.0];
                right
                    .1
                    .cmp(&left.1)
                    .then_with(|| right.2.cmp(&left.2))
                    .then_with(|| right_favorite.cmp(&left_favorite))
                    .then_with(|| {
                        right_frecency
                            .partial_cmp(&left_frecency)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| {
                        self.tasks[left.0]
                            .identity
                            .cmp(&self.tasks[right.0].identity)
                    })
            });
            return matches
                .into_iter()
                .map(|matched| TaskHandle(matched.0))
                .collect();
        }

        let mut handles: Vec<_> = (0..self.tasks.len()).map(TaskHandle).collect();
        handles.sort_by(|left, right| {
            let left_task = &self.tasks[left.0];
            let right_task = &self.tasks[right.0];
            let left_favorite = favorite_flags[left.0];
            let right_favorite = favorite_flags[right.0];
            match (left_favorite, right_favorite) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (true, true) => left_task
                    .script_name
                    .cmp(&right_task.script_name)
                    .then_with(|| left_task.identity.cmp(&right_task.identity)),
                (false, false) => {
                    let left_frecency = frecency_scores[left.0];
                    let right_frecency = frecency_scores[right.0];
                    right_frecency
                        .partial_cmp(&left_frecency)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| left_task.script_name.cmp(&right_task.script_name))
                        .then_with(|| left_task.identity.cmp(&right_task.identity))
                }
            }
        });
        handles
    }

    /// Resolves one handle to its immutable task metadata and execution target.
    pub fn resolve(&self, handle: TaskHandle) -> Option<&CatalogTask> {
        self.tasks.get(handle.0)
    }

    pub(crate) fn from_legacy_parts(
        raw_scripts: IndexMap<String, String>,
        workspace_packages: Vec<WorkspacePackage>,
        nearest_package: PathBuf,
        monorepo_root: Option<PathBuf>,
        project_name: String,
    ) -> (Self, CatalogContext) {
        let project_root = monorepo_root
            .clone()
            .unwrap_or_else(|| nearest_package.clone());
        let mut packages = BTreeMap::<PathBuf, DiscoveredPackage>::new();
        packages.insert(
            canonical_path(&nearest_package),
            DiscoveredPackage {
                name: project_name.clone(),
                relative_path: relative_path(&project_root, &nearest_package),
                cwd: nearest_package.clone(),
                scripts: raw_scripts,
            },
        );
        for workspace in workspace_packages {
            let cwd = project_root.join(&workspace.relative_path);
            packages.insert(
                canonical_path(&cwd),
                DiscoveredPackage {
                    name: workspace.name,
                    relative_path: normalize_relative_path(&workspace.relative_path),
                    cwd,
                    scripts: workspace.scripts,
                },
            );
        }
        let mut tasks = build_tasks(packages.into_values().collect());
        assign_scope_labels(&mut tasks);
        assign_unique_legacy_keys(&mut tasks);

        (
            Self { tasks },
            CatalogContext {
                project_root,
                nearest_package,
                monorepo_root,
                project_name,
            },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchTier {
    Command,
    PackageOrPath,
    ScriptName,
}

fn task_is_favorite(task: &CatalogTask, favorites: &HashSet<String>) -> bool {
    favorites.contains(task.persistence_key())
        || task.legacy_keys().iter().any(|key| favorites.contains(key))
}

fn task_frecency(task: &CatalogTask, recents: &[TaskUsageRecord<'_>], now: u64) -> f64 {
    let (count, last_run) = recents
        .iter()
        .filter(|entry| task.matches_persistence_key(entry.key))
        .fold((0_u32, 0_u64), |(count, last_run), entry| {
            (
                count.saturating_add(entry.count),
                last_run.max(entry.last_run),
            )
        });
    frecency_score(count, last_run, now)
}

pub(crate) fn frecency_score(count: u32, last_run_ms: u64, now_ms: u64) -> f64 {
    let age_in_days = (now_ms.saturating_sub(last_run_ms)) as f64 / (1000.0 * 60.0 * 60.0 * 24.0);
    let frequency_score = ((count + 1) as f64).log2() + 1.0;
    frequency_score * (0.5_f64).powf(age_in_days / 14.0)
}

fn fuzzy_scores<T, F>(items: &[T], query: &str, get_text: F) -> Vec<Option<u32>>
where
    F: Fn(&T) -> &str,
{
    let mut scores = vec![None; items.len()];
    for matched in fuzzy_matches(items, query, get_text) {
        scores[matched.index] = Some(matched.score);
    }
    scores
}

struct DiscoveredPackage {
    name: String,
    relative_path: String,
    cwd: PathBuf,
    scripts: IndexMap<String, String>,
}

fn discover_packages(
    project_root: &Path,
    nearest_package: &Path,
    is_monorepo: bool,
    nearest_manifest: PackageJson,
    project_root_manifest: PackageJson,
) -> Vec<DiscoveredPackage> {
    if !is_monorepo {
        return vec![package_from_manifest(
            project_root,
            nearest_package,
            nearest_manifest,
        )];
    }

    let mut packages = BTreeMap::<PathBuf, DiscoveredPackage>::new();
    let root_package = package_from_manifest(project_root, project_root, project_root_manifest);
    packages.insert(canonical_path(project_root), root_package);

    for workspace in scan_workspaces(project_root) {
        let cwd = project_root.join(&workspace.relative_path);
        packages.insert(
            canonical_path(&cwd),
            DiscoveredPackage {
                name: workspace.name,
                relative_path: normalize_relative_path(&workspace.relative_path),
                cwd,
                scripts: workspace.scripts,
            },
        );
    }

    packages
        .entry(canonical_path(nearest_package))
        .or_insert_with(|| package_from_manifest(project_root, nearest_package, nearest_manifest));

    let mut packages: Vec<_> = packages.into_values().collect();
    packages.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    packages
}

fn package_from_manifest(
    project_root: &Path,
    cwd: &Path,
    manifest: PackageJson,
) -> DiscoveredPackage {
    let fallback_name = fallback_package_name(cwd);
    let scripts = manifest.scripts();
    let name = manifest.name.unwrap_or(fallback_name);
    let relative_path = relative_path(project_root, cwd);

    DiscoveredPackage {
        name,
        relative_path,
        cwd: cwd.to_path_buf(),
        scripts,
    }
}

fn relative_path(project_root: &Path, cwd: &Path) -> String {
    cwd.strip_prefix(project_root)
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| normalize_relative_path(&path.to_string_lossy()))
        .unwrap_or_else(|| ".".to_string())
}

fn build_tasks(packages: Vec<DiscoveredPackage>) -> Vec<CatalogTask> {
    packages
        .into_iter()
        .flat_map(|package| {
            package
                .scripts
                .into_iter()
                .map(move |(script_name, command)| {
                    let identity = TaskIdentity {
                        relative_path: package.relative_path.clone(),
                        script_name: script_name.clone(),
                    };
                    CatalogTask {
                        persistence_key: persistence_key(&identity),
                        legacy_keys: Vec::new(),
                        identity,
                        scope_label: String::new(),
                        package_name: package.name.clone(),
                        relative_path: package.relative_path.clone(),
                        script_name,
                        command,
                        cwd: package.cwd.clone(),
                    }
                })
        })
        .collect()
}

fn assign_scope_labels(tasks: &mut [CatalogTask]) {
    let mut package_paths = HashMap::<String, BTreeSet<String>>::new();
    for task in tasks.iter().filter(|task| task.relative_path != ".") {
        package_paths
            .entry(task.package_name.clone())
            .or_default()
            .insert(task.relative_path.clone());
    }

    for task in tasks.iter_mut() {
        task.scope_label = if task.relative_path == "." {
            "root".to_string()
        } else if task.package_name == "root"
            || package_paths
                .get(task.package_name.as_str())
                .is_some_and(|paths| paths.len() > 1)
        {
            format!("{} ({})", task.package_name, task.relative_path)
        } else {
            task.package_name.clone()
        };
    }
}

fn assign_unique_legacy_keys(tasks: &mut [CatalogTask]) {
    let candidates: Vec<BTreeSet<String>> = tasks
        .iter()
        .map(|task| {
            let mut keys = BTreeSet::from([format!("root:{}", task.script_name)]);
            if task.relative_path != "." {
                keys.insert(format!("{}:{}", task.package_name, task.script_name));
            }
            keys
        })
        .collect();
    let mut owners = HashMap::<String, BTreeSet<usize>>::new();
    for (index, keys) in candidates.iter().enumerate() {
        for key in keys {
            owners.entry(key.clone()).or_default().insert(index);
        }
    }

    for (index, task) in tasks.iter_mut().enumerate() {
        task.legacy_keys = candidates[index]
            .iter()
            .filter(|key| owners.get(*key).is_some_and(|indices| indices.len() == 1))
            .cloned()
            .collect();
    }
}

fn load_required_manifest(dir: &Path) -> Result<PackageJson, CatalogError> {
    PackageJson::load_required(dir).map_err(|source| CatalogError::RequiredManifest {
        path: dir.join("package.json"),
        source,
    })
}

fn normalize_relative_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn fallback_package_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string())
}

fn persistence_key(identity: &TaskIdentity) -> String {
    format!(
        "task:v1:{}:{}:{}{}",
        identity.relative_path.len(),
        identity.script_name.len(),
        identity.relative_path,
        identity.script_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW_MS: u64 = 1_800_000_000_000;

    #[test]
    fn legacy_windows_workspace_path_resolves_with_normalized_identity() {
        let workspace = WorkspacePackage {
            name: "app".to_string(),
            relative_path: r"packages\app".to_string(),
            scripts: IndexMap::from([("dev".to_string(), "vite".to_string())]),
        };
        let (catalog, _) = TaskCatalog::from_legacy_parts(
            IndexMap::new(),
            vec![workspace],
            PathBuf::from("/repo"),
            Some(PathBuf::from("/repo")),
            "repo".to_string(),
        );
        let favorites = HashSet::new();
        let handles = catalog.query(TaskQuery::new("", &favorites, &[], NOW_MS));
        let task = catalog.resolve(handles[0]).unwrap();

        assert_eq!(task.relative_path, "packages/app");
        assert!(task.persistence_key().contains("packages/app"));
    }
}
