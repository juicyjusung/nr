use crate::core::env_files::{EnvFile, EnvFileList, scan_env_files};
use crate::core::task_catalog::{
    CatalogContext, CatalogTask, TaskCatalog, TaskHandle, TaskQuery, TaskUsageRecord,
};
use crate::core::workspaces::WorkspacePackage;
use crate::fuzzy::fuzzy_filter;
use crate::store::args_history::{self, ArgsHistory};
use crate::store::favorites;
use crate::store::recents::{self, RecentEntry};
use crate::store::script_configs::{self, ScriptConfig, ScriptConfigs};
use crate::ui::script_list::TaskListItem;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use indexmap::IndexMap;
use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Scripts,
    Packages,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageMode {
    SelectingPackage,
    SelectingScript { package_index: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    ConfigureEnv,
    ConfigureArgs,
    ConfirmExecution,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionConfig {
    pub args: String,
}

pub enum Action {
    Continue,
    RunScript {
        script_name: String,
        cwd: PathBuf,
        env_files: Vec<PathBuf>,
        args: String,
    },
    Quit,
}

fn task_list_item(task: &CatalogTask) -> TaskListItem {
    TaskListItem {
        key: task.persistence_key().to_string(),
        scope_label: task.scope_label.clone(),
        name: task.script_name.clone(),
        command: task.command.clone(),
    }
}

struct PackageGroup {
    name: String,
    scripts: IndexMap<String, String>,
    task_handles: Vec<TaskHandle>,
}

impl PackageGroup {
    fn new(name: String) -> Self {
        Self {
            name,
            scripts: IndexMap::new(),
            task_handles: Vec::new(),
        }
    }
}

fn query_catalog(
    catalog: &TaskCatalog,
    text: &str,
    favorites: &HashSet<String>,
    recents: &[RecentEntry],
) -> Vec<TaskHandle> {
    let usage: Vec<_> = recents
        .iter()
        .map(|entry| TaskUsageRecord::new(&entry.key, entry.last_run, entry.count))
        .collect();
    catalog.query(TaskQuery::new(text, favorites, &usage, recents::now_ms()))
}

pub struct App {
    // Navigation
    pub active_tab: Tab,
    pub package_mode: PackageMode,
    pub has_workspaces: bool,

    // Data
    pub scripts: Vec<TaskListItem>,
    pub workspace_packages: Vec<WorkspacePackage>,
    pub nearest_pkg: PathBuf,
    pub monorepo_root: Option<PathBuf>,
    catalog: TaskCatalog,
    task_handles: Vec<TaskHandle>,
    task_indices: HashMap<TaskHandle, usize>,
    package_task_handles: Vec<Vec<TaskHandle>>,

    // State
    pub favorites: HashSet<String>,
    pub recents: Vec<RecentEntry>,

    // Header info
    pub project_name: String,
    pub project_path: String,
    pub package_manager_name: String,

    // Layout
    visible_height: usize,

    // Scripts tab UI state
    pub query: String,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub filtered_indices: Vec<usize>,

    // Package tab UI state
    pub pkg_query: String,
    pub pkg_selected_index: usize,
    pub pkg_scroll_offset: usize,
    pub pkg_filtered_indices: Vec<usize>,

    // Package script selection UI state (when inside a package)
    pub pkg_script_query: String,
    pub pkg_script_selected_index: usize,
    pub pkg_script_scroll_offset: usize,
    pub pkg_script_filtered_indices: Vec<usize>,
    pub pkg_script_items: Vec<TaskListItem>,
    pkg_script_handles: Vec<TaskHandle>,

    // NEW: Configuration flow state
    pub mode: AppMode,
    pub execution_config: ExecutionConfig,
    pub script_configs: ScriptConfigs,
    pub global_env_config: crate::store::global_env::GlobalEnvConfig,
    pub args_history: ArgsHistory,
    pub config_dir: PathBuf,
    pub package_manager: crate::core::package_manager::PackageManager,

    // NEW: Env selection UI state
    pub env_files_list: Option<EnvFileList>,
    pub env_selected_index: usize,
    pub env_scroll_offset: usize,
    pub env_selected_files: HashSet<PathBuf>,

    // NEW: Args input UI state
    pub args_input: String,
    pub args_cursor_pos: usize, // NEW: cursor position in args_input
    pub args_history_index: Option<usize>,
    pending_task: Option<TaskHandle>,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        raw_scripts: IndexMap<String, String>,
        workspace_packages: Vec<WorkspacePackage>,
        nearest_pkg: PathBuf,
        monorepo_root: Option<PathBuf>,
        project_dir: &std::path::Path,
        project_name: String,
        project_path: String,
        package_manager_name: String,
        package_manager: crate::core::package_manager::PackageManager,
    ) -> Self {
        let (catalog, context) = TaskCatalog::from_legacy_parts(
            raw_scripts,
            workspace_packages,
            nearest_pkg,
            monorepo_root,
            project_name.clone(),
        );
        Self::build(
            catalog,
            context,
            project_dir,
            project_name,
            project_path,
            package_manager_name,
            package_manager,
        )
    }

    pub fn from_catalog(
        discovery: (TaskCatalog, CatalogContext),
        project_dir: &std::path::Path,
        package_manager_name: String,
        package_manager: crate::core::package_manager::PackageManager,
    ) -> Self {
        let (catalog, context) = discovery;
        let project_name = context.project_name.clone();
        let project_path = context.project_root.display().to_string();
        Self::build(
            catalog,
            context,
            project_dir,
            project_name,
            project_path,
            package_manager_name,
            package_manager,
        )
    }

    fn build(
        catalog: TaskCatalog,
        context: CatalogContext,
        project_dir: &std::path::Path,
        project_name: String,
        project_path: String,
        package_manager_name: String,
        package_manager: crate::core::package_manager::PackageManager,
    ) -> Self {
        let nearest_pkg = context.nearest_package;
        let monorepo_root = context.monorepo_root;

        // Load persisted state from project-scoped directory
        let mut favorites_data = favorites::load_favorites(project_dir);
        let recents_data = recents::load_recents(project_dir);
        let script_configs_data =
            script_configs::load_script_configs(project_dir).unwrap_or_default();
        let global_env_data =
            crate::store::global_env::load_global_env_config(project_dir).unwrap_or_default();
        let args_history_data = args_history::load_args_history(project_dir).unwrap_or_default();

        let initial_handles = query_catalog(&catalog, "", &favorites_data, &recents_data);
        for handle in &initial_handles {
            let Some(task) = catalog.resolve(*handle) else {
                continue;
            };
            if task
                .legacy_keys()
                .iter()
                .any(|key| favorites_data.contains(key))
            {
                favorites_data.insert(task.persistence_key().to_string());
            }
        }
        let task_handles = query_catalog(&catalog, "", &favorites_data, &recents_data);
        let scripts: Vec<_> = task_handles
            .iter()
            .filter_map(|handle| catalog.resolve(*handle).map(task_list_item))
            .collect();
        let task_indices: HashMap<_, _> = task_handles
            .iter()
            .copied()
            .enumerate()
            .map(|(index, handle)| (handle, index))
            .collect();
        let filtered_indices: Vec<_> = (0..task_handles.len()).collect();

        let mut package_groups = BTreeMap::<String, PackageGroup>::new();
        for handle in &task_handles {
            let Some(task) = catalog.resolve(*handle) else {
                continue;
            };
            if task.relative_path == "." {
                continue;
            }
            let group = package_groups
                .entry(task.relative_path.clone())
                .or_insert_with(|| PackageGroup::new(task.package_name.clone()));
            group
                .scripts
                .insert(task.script_name.clone(), task.command.clone());
            group.task_handles.push(*handle);
        }
        let mut workspace_packages = Vec::with_capacity(package_groups.len());
        let mut package_task_handles = Vec::with_capacity(package_groups.len());
        for (relative_path, group) in package_groups {
            workspace_packages.push(WorkspacePackage {
                name: group.name,
                relative_path,
                scripts: group.scripts,
            });
            package_task_handles.push(group.task_handles);
        }
        let has_workspaces = !workspace_packages.is_empty();

        // Initial package filter (all packages, original order)
        let pkg_filtered_indices: Vec<usize> = (0..workspace_packages.len()).collect();

        App {
            active_tab: Tab::Scripts,
            package_mode: PackageMode::SelectingPackage,
            has_workspaces,

            scripts,
            workspace_packages,
            nearest_pkg,
            monorepo_root,
            catalog,
            task_handles,
            task_indices,
            package_task_handles,

            favorites: favorites_data,
            recents: recents_data,

            project_name,
            project_path,
            package_manager_name,

            visible_height: 20,

            query: String::new(),
            selected_index: 0,
            scroll_offset: 0,
            filtered_indices,

            pkg_query: String::new(),
            pkg_selected_index: 0,
            pkg_scroll_offset: 0,
            pkg_filtered_indices,

            pkg_script_query: String::new(),
            pkg_script_selected_index: 0,
            pkg_script_scroll_offset: 0,
            pkg_script_filtered_indices: Vec::new(),
            pkg_script_items: Vec::new(),
            pkg_script_handles: Vec::new(),

            // NEW: Configuration flow
            mode: AppMode::Normal,
            execution_config: ExecutionConfig::default(),
            script_configs: script_configs_data,
            global_env_config: global_env_data,
            args_history: args_history_data,
            config_dir: project_dir.to_path_buf(),
            package_manager,

            // NEW: Env selection UI state
            env_files_list: None,
            env_selected_index: 0,
            env_scroll_offset: 0,
            env_selected_files: HashSet::new(),

            // NEW: Args input UI state
            args_input: String::new(),
            args_cursor_pos: 0,
            args_history_index: None,
            pending_task: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        // Route to mode-specific handler
        match self.mode {
            AppMode::Normal => self.handle_normal_mode(key),
            AppMode::ConfigureEnv => self.handle_env_mode(key),
            AppMode::ConfigureArgs => self.handle_args_mode(key),
            AppMode::ConfirmExecution => self.handle_confirm_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => self.handle_esc(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Enter => self.handle_enter(),
            // Use Tab key for configure flow
            KeyCode::Tab => {
                self.start_configure_flow();
                Action::Continue
            }
            KeyCode::Up => {
                self.move_selection(-1);
                Action::Continue
            }
            KeyCode::Down => {
                self.move_selection(1);
                Action::Continue
            }
            KeyCode::Left => {
                self.switch_tab(-1);
                Action::Continue
            }
            KeyCode::Right => {
                self.switch_tab(1);
                Action::Continue
            }
            KeyCode::Char(' ') => {
                self.toggle_fav();
                Action::Continue
            }
            KeyCode::Char(c) => {
                self.type_char(c);
                Action::Continue
            }
            KeyCode::Backspace => {
                self.delete_char();
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Build layout constraints depending on whether we show the tab bar
        let chunks = if self.has_workspaces {
            Layout::vertical([
                Constraint::Length(1), // header bar
                Constraint::Length(2), // tabs
                Constraint::Length(1), // search input
                Constraint::Min(1),    // main content
                Constraint::Length(1), // status bar
            ])
            .split(area)
        } else {
            Layout::vertical([
                Constraint::Length(1), // header bar
                Constraint::Length(0), // no tabs
                Constraint::Length(1), // search input
                Constraint::Min(1),    // main content
                Constraint::Length(1), // status bar
            ])
            .split(area)
        };

        // Track actual visible height for scroll calculations
        self.visible_height = chunks[3].height as usize;

        // Header bar
        crate::ui::header_bar::render_header_bar(
            frame,
            chunks[0],
            &self.project_name,
            &self.project_path,
            &self.package_manager_name,
        );

        // Tabs (only if workspaces exist)
        if self.has_workspaces {
            let tab_labels = vec!["Scripts", "Packages"];
            let active = match self.active_tab {
                Tab::Scripts => 0,
                Tab::Packages => 1,
            };
            crate::ui::tabs::render_tabs(frame, chunks[1], &tab_labels, active);
        }

        // Search input
        let current_query = self.current_query();
        crate::ui::search_input::render_search_input(frame, chunks[2], current_query);

        // Main content
        match self.active_tab {
            Tab::Scripts => {
                crate::ui::script_list::render_script_list(
                    frame,
                    chunks[3],
                    &self.scripts,
                    &self.filtered_indices,
                    self.selected_index,
                    self.scroll_offset,
                    &self.favorites,
                );
            }
            Tab::Packages => match self.package_mode {
                PackageMode::SelectingPackage => {
                    crate::ui::package_list::render_package_list(
                        frame,
                        chunks[3],
                        &self.workspace_packages,
                        &self.pkg_filtered_indices,
                        self.pkg_selected_index,
                        self.pkg_scroll_offset,
                    );
                }
                PackageMode::SelectingScript { .. } => {
                    crate::ui::script_list::render_script_list(
                        frame,
                        chunks[3],
                        &self.pkg_script_items,
                        &self.pkg_script_filtered_indices,
                        self.pkg_script_selected_index,
                        self.pkg_script_scroll_offset,
                        &self.favorites,
                    );
                }
            },
        }

        // Status bar
        crate::ui::status_bar::render_status_bar(frame, chunks[4]);

        // NEW: Render modal overlays based on mode
        match self.mode {
            AppMode::ConfigureEnv => {
                if let Some(ref env_list) = self.env_files_list {
                    crate::ui::env_selector::render_env_selector(
                        frame,
                        area,
                        env_list,
                        self.env_selected_index,
                        self.env_scroll_offset,
                        &self.env_selected_files,
                    );
                }
            }
            AppMode::ConfigureArgs => {
                crate::ui::args_input::render_args_input(
                    frame,
                    area,
                    &self.args_input,
                    self.args_cursor_pos,
                    &self.args_history.entries,
                    self.args_history_index,
                );
            }
            AppMode::ConfirmExecution => {
                let env_file_names: Vec<String> = if let Some(ref env_list) = self.env_files_list {
                    env_list
                        .all_files()
                        .filter(|f| self.env_selected_files.contains(&f.path))
                        .map(|f| f.display_name.clone())
                        .collect()
                } else {
                    vec![]
                };

                let script_name = self.get_current_script_name();
                let cwd = self.get_current_cwd();

                crate::ui::execution_confirm::render_execution_confirm(
                    frame,
                    area,
                    self.package_manager,
                    &script_name,
                    &env_file_names,
                    &self.execution_config.args,
                    &cwd,
                );
            }
            AppMode::Normal => {
                // No overlay
            }
        }
    }

    // -- Private helpers --

    fn current_query(&self) -> &str {
        match self.active_tab {
            Tab::Scripts => &self.query,
            Tab::Packages => match self.package_mode {
                PackageMode::SelectingPackage => &self.pkg_query,
                PackageMode::SelectingScript { .. } => &self.pkg_script_query,
            },
        }
    }

    fn handle_esc(&mut self) -> Action {
        match self.active_tab {
            Tab::Scripts => Action::Quit,
            Tab::Packages => match self.package_mode {
                PackageMode::SelectingPackage => Action::Quit,
                PackageMode::SelectingScript { .. } => {
                    // Go back to package list
                    self.package_mode = PackageMode::SelectingPackage;
                    self.pkg_script_query.clear();
                    self.pkg_script_selected_index = 0;
                    self.pkg_script_scroll_offset = 0;
                    Action::Continue
                }
            },
        }
    }

    fn handle_enter(&mut self) -> Action {
        if self.active_tab == Tab::Packages
            && matches!(self.package_mode, PackageMode::SelectingPackage)
        {
            if let Some(&package_index) = self.pkg_filtered_indices.get(self.pkg_selected_index) {
                self.enter_package_scripts(package_index);
            }
            return Action::Continue;
        }

        let Some(handle) = self.selected_task_handle() else {
            return Action::Continue;
        };
        let Some(task) = self.catalog.resolve(handle) else {
            return Action::Continue;
        };
        let script_name = task.script_name.clone();
        let cwd = task.cwd.clone();
        let key = task.persistence_key().to_string();
        recents::record_execution(&mut self.recents, &key);

        Action::RunScript {
            script_name,
            cwd,
            env_files: Vec::new(),
            args: String::new(),
        }
    }

    fn enter_package_scripts(&mut self, pkg_idx: usize) {
        self.pkg_script_handles = self
            .package_task_handles
            .get(pkg_idx)
            .cloned()
            .unwrap_or_default();
        self.pkg_script_items = self
            .pkg_script_handles
            .iter()
            .filter_map(|handle| self.catalog.resolve(*handle).map(task_list_item))
            .collect();

        self.package_mode = PackageMode::SelectingScript {
            package_index: pkg_idx,
        };
        self.pkg_script_query.clear();
        self.pkg_script_selected_index = 0;
        self.pkg_script_scroll_offset = 0;

        self.update_pkg_script_filtered();
    }

    fn move_selection(&mut self, delta: i32) {
        match self.active_tab {
            Tab::Scripts => {
                let len = self.filtered_indices.len();
                if len == 0 {
                    return;
                }
                self.selected_index = wrap_index(self.selected_index, delta, len);
                self.ensure_visible_scripts();
            }
            Tab::Packages => match self.package_mode {
                PackageMode::SelectingPackage => {
                    let len = self.pkg_filtered_indices.len();
                    if len == 0 {
                        return;
                    }
                    self.pkg_selected_index = wrap_index(self.pkg_selected_index, delta, len);
                    self.ensure_visible_packages();
                }
                PackageMode::SelectingScript { .. } => {
                    let len = self.pkg_script_filtered_indices.len();
                    if len == 0 {
                        return;
                    }
                    self.pkg_script_selected_index =
                        wrap_index(self.pkg_script_selected_index, delta, len);
                    self.ensure_visible_pkg_scripts();
                }
            },
        }
    }

    fn switch_tab(&mut self, delta: i32) {
        if !self.has_workspaces {
            return;
        }
        match (self.active_tab, delta) {
            (Tab::Scripts, 1) => {
                self.active_tab = Tab::Packages;
            }
            (Tab::Packages, -1) => {
                // Reset package mode when switching away
                self.package_mode = PackageMode::SelectingPackage;
                self.pkg_script_query.clear();
                self.active_tab = Tab::Scripts;
            }
            _ => {}
        }
    }

    fn toggle_fav(&mut self) {
        let Some(handle) = self.selected_task_handle() else {
            return;
        };
        let Some(task) = self.catalog.resolve(handle) else {
            return;
        };
        let canonical = task.persistence_key().to_string();
        let legacy = task.legacy_keys().to_vec();
        let is_favorite = self.favorites.contains(&canonical)
            || legacy.iter().any(|key| self.favorites.contains(key));
        if is_favorite {
            self.favorites.remove(&canonical);
            for key in legacy {
                self.favorites.remove(&key);
            }
        } else {
            self.favorites.insert(canonical);
        }

        match self.active_tab {
            Tab::Scripts => self.update_filtered(),
            Tab::Packages => self.update_pkg_script_filtered(),
        }
    }

    fn type_char(&mut self, c: char) {
        match self.active_tab {
            Tab::Scripts => {
                self.query.push(c);
                self.update_filtered();
            }
            Tab::Packages => match self.package_mode {
                PackageMode::SelectingPackage => {
                    self.pkg_query.push(c);
                    self.update_pkg_filtered();
                }
                PackageMode::SelectingScript { .. } => {
                    self.pkg_script_query.push(c);
                    self.update_pkg_script_filtered();
                }
            },
        }
    }

    fn delete_char(&mut self) {
        match self.active_tab {
            Tab::Scripts => {
                self.query.pop();
                self.update_filtered();
            }
            Tab::Packages => match self.package_mode {
                PackageMode::SelectingPackage => {
                    self.pkg_query.pop();
                    self.update_pkg_filtered();
                }
                PackageMode::SelectingScript { .. } => {
                    self.pkg_script_query.pop();
                    self.update_pkg_script_filtered();
                }
            },
        }
    }

    fn update_filtered(&mut self) {
        self.filtered_indices =
            query_catalog(&self.catalog, &self.query, &self.favorites, &self.recents)
                .into_iter()
                .filter_map(|handle| self.task_indices.get(&handle).copied())
                .collect();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    fn update_pkg_filtered(&mut self) {
        self.pkg_filtered_indices =
            fuzzy_filter(&self.workspace_packages, &self.pkg_query, |p| &p.name);
        self.pkg_selected_index = 0;
        self.pkg_scroll_offset = 0;
    }

    fn update_pkg_script_filtered(&mut self) {
        self.pkg_script_filtered_indices = query_catalog(
            &self.catalog,
            &self.pkg_script_query,
            &self.favorites,
            &self.recents,
        )
        .into_iter()
        .filter_map(|handle| {
            self.pkg_script_handles
                .iter()
                .position(|candidate| *candidate == handle)
        })
        .collect();
        self.pkg_script_selected_index = 0;
        self.pkg_script_scroll_offset = 0;
    }

    fn ensure_visible_scripts(&mut self) {
        ensure_scroll(
            &mut self.scroll_offset,
            self.selected_index,
            self.visible_height,
        );
    }

    fn ensure_visible_packages(&mut self) {
        ensure_scroll(
            &mut self.pkg_scroll_offset,
            self.pkg_selected_index,
            self.visible_height,
        );
    }

    fn ensure_visible_pkg_scripts(&mut self) {
        ensure_scroll(
            &mut self.pkg_script_scroll_offset,
            self.pkg_script_selected_index,
            self.visible_height,
        );
    }

    fn selected_task_handle(&self) -> Option<TaskHandle> {
        match self.active_tab {
            Tab::Scripts => self
                .filtered_indices
                .get(self.selected_index)
                .and_then(|index| self.task_handles.get(*index))
                .copied(),
            Tab::Packages => match self.package_mode {
                PackageMode::SelectingPackage => None,
                PackageMode::SelectingScript { .. } => self
                    .pkg_script_filtered_indices
                    .get(self.pkg_script_selected_index)
                    .and_then(|index| self.pkg_script_handles.get(*index))
                    .copied(),
            },
        }
    }

    fn current_task_handle(&self) -> Option<TaskHandle> {
        self.pending_task.or_else(|| self.selected_task_handle())
    }
}

/// Wrap index with delta, cycling around `len`.
fn wrap_index(current: usize, delta: i32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let new = current as i32 + delta;
    if new < 0 {
        len - 1
    } else if new >= len as i32 {
        0
    } else {
        new as usize
    }
}

fn char_index_to_byte_index(input: &str, char_index: usize) -> usize {
    input
        .char_indices()
        .nth(char_index)
        .map_or(input.len(), |(byte_index, _)| byte_index)
}

/// Adjust scroll_offset so that `selected` stays visible within the given height.
fn ensure_scroll(scroll_offset: &mut usize, selected: usize, visible_height: usize) {
    if selected < *scroll_offset {
        *scroll_offset = selected;
    }
    let height = visible_height.max(1);
    if selected >= *scroll_offset + height {
        *scroll_offset = selected.saturating_sub(height - 1);
    }
}

impl App {
    // NEW: Configuration flow methods

    fn start_configure_flow(&mut self) {
        let Some(handle) = self.selected_task_handle() else {
            return;
        };
        let Some(task) = self.catalog.resolve(handle) else {
            return;
        };
        let cwd = task.cwd.clone();
        let canonical_config_key = self.config_key(task.persistence_key());
        let legacy_config_keys: Vec<_> = task
            .legacy_keys()
            .iter()
            .map(|key| self.config_key(key))
            .collect();
        self.pending_task = Some(handle);

        // Restore script-specific args (if exists)
        let restored_config = self.script_configs.get(&canonical_config_key).or_else(|| {
            legacy_config_keys
                .iter()
                .filter_map(|key| self.script_configs.get(key))
                .max_by_key(|config| config.last_used)
        });
        if let Some(config) = restored_config {
            self.execution_config.args = config.args.clone();
        } else {
            self.execution_config = ExecutionConfig::default();
        }

        // Scan .env files
        self.env_files_list = Some(scan_env_files(&cwd, &self.monorepo_root));

        // Pre-select globally last used env files
        self.env_selected_files = if let Some(ref env_list) = self.env_files_list {
            env_list
                .all_files()
                .filter(|f| {
                    self.global_env_config
                        .last_env_files
                        .contains(&f.display_name)
                })
                .map(|f| f.path.clone())
                .collect()
        } else {
            HashSet::new()
        };
        self.env_selected_index = 0;
        self.env_scroll_offset = 0;

        // Enter env selection mode
        self.mode = AppMode::ConfigureEnv;
    }

    fn config_key(&self, task_key: &str) -> String {
        let legacy_project_id = crate::store::project_id::project_id(&self.config_dir);
        format!("{legacy_project_id}:{task_key}")
    }

    fn get_current_cwd(&self) -> PathBuf {
        self.current_task_handle()
            .and_then(|handle| self.catalog.resolve(handle))
            .map(|task| task.cwd.clone())
            .unwrap_or_else(|| self.nearest_pkg.clone())
    }

    fn handle_env_mode(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Esc => {
                // Cancel configuration
                self.mode = AppMode::Normal;
                self.execution_config = ExecutionConfig::default();
                self.env_files_list = None;
                self.pending_task = None;
                Action::Continue
            }
            KeyCode::Enter => {
                // Proceed to args input
                self.mode = AppMode::ConfigureArgs;
                self.args_input = self.execution_config.args.clone();
                self.args_cursor_pos = self.args_input.chars().count();
                self.args_history_index = None;
                Action::Continue
            }
            KeyCode::Up => {
                if let Some(ref env_list) = self.env_files_list {
                    let total_files = env_list.package_files.len() + env_list.root_files.len();
                    if total_files > 0 && self.env_selected_index > 0 {
                        self.env_selected_index -= 1;
                    }
                }
                Action::Continue
            }
            KeyCode::Down => {
                if let Some(ref env_list) = self.env_files_list {
                    let total_files = env_list.package_files.len() + env_list.root_files.len();
                    if self.env_selected_index + 1 < total_files {
                        self.env_selected_index += 1;
                    }
                }
                Action::Continue
            }
            KeyCode::Char(' ') => {
                // Toggle selection
                if let Some(ref env_list) = self.env_files_list {
                    let all_files: Vec<&EnvFile> = env_list.all_files().collect();
                    if let Some(file) = all_files.get(self.env_selected_index) {
                        if self.env_selected_files.contains(&file.path) {
                            self.env_selected_files.remove(&file.path);
                        } else {
                            self.env_selected_files.insert(file.path.clone());
                        }
                    }
                }
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    fn handle_args_mode(&mut self, key: KeyEvent) -> Action {
        self.args_cursor_pos = self.args_cursor_pos.min(self.args_input.chars().count());

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Esc => {
                // Go back to env selection
                self.mode = AppMode::ConfigureEnv;
                Action::Continue
            }
            KeyCode::Enter => {
                // Save input and proceed to confirmation
                self.execution_config.args = self.args_input.clone();
                self.mode = AppMode::ConfirmExecution;
                Action::Continue
            }
            KeyCode::Up => {
                // Navigate history (up = move to older/higher index)
                if let Some(idx) = self.args_history_index {
                    if idx == 0 {
                        self.args_history_index = None;
                        self.args_input = self.execution_config.args.clone();
                    } else {
                        let new_idx = idx - 1;
                        self.args_input = self.args_history.entries[new_idx].clone();
                        self.args_history_index = Some(new_idx);
                    }
                }
                self.args_cursor_pos = self.args_input.chars().count();
                Action::Continue
            }
            KeyCode::Down => {
                // Navigate history (down = move to newer/lower index)
                let history_len = self.args_history.entries.len();
                if history_len > 0 {
                    let new_index = match self.args_history_index {
                        Some(idx) if idx + 1 < history_len => Some(idx + 1),
                        None => Some(0),
                        _ => self.args_history_index,
                    };
                    if let Some(idx) = new_index {
                        self.args_input = self.args_history.entries[idx].clone();
                        self.args_history_index = Some(idx);
                    }
                }
                self.args_cursor_pos = self.args_input.chars().count();
                Action::Continue
            }
            KeyCode::Left => {
                // Move cursor left
                if self.args_cursor_pos > 0 {
                    self.args_cursor_pos -= 1;
                }
                Action::Continue
            }
            KeyCode::Right => {
                // Move cursor right
                if self.args_cursor_pos < self.args_input.chars().count() {
                    self.args_cursor_pos += 1;
                }
                Action::Continue
            }
            KeyCode::Home => {
                // Move cursor to start
                self.args_cursor_pos = 0;
                Action::Continue
            }
            KeyCode::End => {
                // Move cursor to end
                self.args_cursor_pos = self.args_input.chars().count();
                Action::Continue
            }
            KeyCode::Char(c) => {
                // Insert character at cursor position
                let byte_index = char_index_to_byte_index(&self.args_input, self.args_cursor_pos);
                self.args_input.insert(byte_index, c);
                self.args_cursor_pos += 1;
                self.args_history_index = None;
                Action::Continue
            }
            KeyCode::Backspace => {
                // Delete character before cursor
                if self.args_cursor_pos > 0 {
                    let byte_index =
                        char_index_to_byte_index(&self.args_input, self.args_cursor_pos - 1);
                    self.args_input.remove(byte_index);
                    self.args_cursor_pos -= 1;
                    self.args_history_index = None;
                }
                Action::Continue
            }
            KeyCode::Delete => {
                // Delete character at cursor
                if self.args_cursor_pos < self.args_input.chars().count() {
                    let byte_index =
                        char_index_to_byte_index(&self.args_input, self.args_cursor_pos);
                    self.args_input.remove(byte_index);
                    self.args_history_index = None;
                }
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    fn handle_confirm_mode(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Esc => {
                // Go back to args input
                self.mode = AppMode::ConfigureArgs;
                Action::Continue
            }
            KeyCode::Enter => {
                let Some(handle) = self.pending_task else {
                    self.mode = AppMode::Normal;
                    return Action::Continue;
                };
                let Some(task) = self.catalog.resolve(handle) else {
                    self.mode = AppMode::Normal;
                    self.pending_task = None;
                    return Action::Continue;
                };
                let script_name = task.script_name.clone();
                let cwd = task.cwd.clone();
                let execution_key = task.persistence_key().to_string();
                let script_key = self.config_key(&execution_key);

                // Save script-specific args
                self.script_configs.insert(
                    script_key.clone(),
                    ScriptConfig {
                        args: self.execution_config.args.clone(),
                        last_used: SystemTime::now(),
                    },
                );
                let _ = script_configs::save_script_configs(&self.config_dir, &self.script_configs);

                // Save globally last used env files
                if let Some(ref env_list) = self.env_files_list {
                    self.global_env_config.last_env_files = env_list
                        .all_files()
                        .filter(|f| self.env_selected_files.contains(&f.path))
                        .map(|f| f.display_name.clone())
                        .collect();
                    let _ = crate::store::global_env::save_global_env_config(
                        &self.config_dir,
                        &self.global_env_config,
                    );
                }

                // Save args to history
                if !self.execution_config.args.is_empty() {
                    self.args_history
                        .add_entry(self.execution_config.args.clone());
                    let _ = args_history::save_args_history(&self.config_dir, &self.args_history);
                }

                recents::record_execution(&mut self.recents, &execution_key);

                // Build env file paths in merge order (root → package, so package overrides root)
                let env_file_paths: Vec<PathBuf> = if let Some(ref env_list) = self.env_files_list {
                    env_list
                        .all_files_merge_order()
                        .filter(|f| self.env_selected_files.contains(&f.path))
                        .map(|f| f.path.clone())
                        .collect()
                } else {
                    vec![]
                };

                // Reset mode
                self.mode = AppMode::Normal;
                self.pending_task = None;

                Action::RunScript {
                    script_name,
                    cwd,
                    env_files: env_file_paths,
                    args: self.execution_config.args.clone(),
                }
            }
            _ => Action::Continue,
        }
    }

    fn get_current_script_name(&self) -> String {
        self.current_task_handle()
            .and_then(|handle| self.catalog.resolve(handle))
            .map(|task| task.script_name.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test helper to create a task list item.
    fn script(name: &str, command: &str) -> TaskListItem {
        TaskListItem {
            key: format!("root:{}", name),
            scope_label: "root".to_string(),
            name: name.to_string(),
            command: command.to_string(),
        }
    }

    // Test builder for App
    struct TestAppBuilder {
        scripts: Vec<TaskListItem>,
        workspace_packages: Vec<WorkspacePackage>,
        favorites: HashSet<String>,
        recents: Vec<RecentEntry>,
        visible_height: usize,
        has_workspaces: bool,
    }

    impl TestAppBuilder {
        fn new() -> Self {
            Self {
                scripts: vec![],
                workspace_packages: vec![],
                favorites: HashSet::new(),
                recents: vec![],
                visible_height: 20,
                has_workspaces: false,
            }
        }

        fn with_scripts(mut self, scripts: Vec<TaskListItem>) -> Self {
            self.scripts = scripts;
            self
        }

        fn with_favorite(mut self, key: &str) -> Self {
            self.favorites.insert(key.to_string());
            self
        }

        fn with_workspaces(mut self, packages: Vec<WorkspacePackage>) -> Self {
            self.has_workspaces = !packages.is_empty();
            self.workspace_packages = packages;
            self
        }

        fn build(self) -> App {
            let raw_scripts = self
                .scripts
                .into_iter()
                .map(|script| (script.name, script.command))
                .collect();
            let mut app = App::new(
                raw_scripts,
                self.workspace_packages,
                PathBuf::from("/test/project"),
                None,
                std::path::Path::new("/test/.config/nr"),
                "test-project".to_string(),
                "/test/project".to_string(),
                "npm".to_string(),
                crate::core::package_manager::PackageManager::Npm,
            );
            app.favorites = self.favorites;
            app.recents = self.recents;
            app.visible_height = self.visible_height;
            app.has_workspaces = self.has_workspaces;
            app.update_filtered();
            app
        }
    }

    // --- move_selection tests ---

    #[test]
    fn test_move_selection_down_increments_index() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![
                script("test", "echo test"),
                script("build", "echo build"),
                script("lint", "echo lint"),
            ])
            .build();

        assert_eq!(app.selected_index, 0);
        app.move_selection(1);
        assert_eq!(app.selected_index, 1);
        app.move_selection(1);
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn test_move_selection_up_decrements_index() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![
                script("test", "echo test"),
                script("build", "echo build"),
                script("lint", "echo lint"),
            ])
            .build();

        app.selected_index = 2;
        app.move_selection(-1);
        assert_eq!(app.selected_index, 1);
        app.move_selection(-1);
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_move_selection_wraps_at_bottom() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![
                script("test", "echo test"),
                script("build", "echo build"),
            ])
            .build();

        app.selected_index = 1; // last item
        app.move_selection(1); // should wrap to 0
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_move_selection_wraps_at_top() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![
                script("test", "echo test"),
                script("build", "echo build"),
            ])
            .build();

        assert_eq!(app.selected_index, 0);
        app.move_selection(-1); // should wrap to last
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn test_move_selection_handles_empty_list() {
        let mut app = TestAppBuilder::new().build();
        assert_eq!(app.selected_index, 0);
        app.move_selection(1);
        assert_eq!(app.selected_index, 0); // no change
    }

    // --- toggle_fav tests ---

    #[test]
    fn test_toggle_fav_adds_to_favorites() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("test", "echo test")])
            .build();

        let key = app.scripts[app.filtered_indices[0]].key.clone();
        assert!(!app.favorites.contains(&key));

        app.toggle_fav();
        assert!(app.favorites.contains(&key));
    }

    #[test]
    fn test_toggle_fav_removes_from_favorites() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("test", "echo test")])
            .with_favorite("root:test")
            .build();

        assert!(app.favorites.contains("root:test"));
        app.toggle_fav();
        assert!(!app.favorites.contains("root:test"));
    }

    #[test]
    fn test_toggle_fav_updates_filtered_indices() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("aaa", "echo aaa"), script("zzz", "echo zzz")])
            .build();

        // Initially alphabetical order: aaa, zzz
        assert_eq!(app.filtered_indices, vec![0, 1]);

        // Toggle favorite on zzz (index 1)
        app.selected_index = 1;
        app.toggle_fav();

        // Now favorites come first: zzz, aaa
        assert_eq!(app.filtered_indices, vec![1, 0]);
    }

    // --- switch_tab tests ---

    #[test]
    fn test_switch_tab_changes_to_packages() {
        let pkg = WorkspacePackage {
            name: "pkg1".to_string(),
            relative_path: "packages/pkg1".to_string(),
            scripts: IndexMap::new(),
        };

        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("test", "echo test")])
            .with_workspaces(vec![pkg])
            .build();

        assert_eq!(app.active_tab, Tab::Scripts);
        app.switch_tab(1);
        assert_eq!(app.active_tab, Tab::Packages);
    }

    #[test]
    fn test_switch_tab_changes_to_scripts() {
        let pkg = WorkspacePackage {
            name: "pkg1".to_string(),
            relative_path: "packages/pkg1".to_string(),
            scripts: IndexMap::new(),
        };

        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("test", "echo test")])
            .with_workspaces(vec![pkg])
            .build();

        app.active_tab = Tab::Packages;
        app.switch_tab(-1);
        assert_eq!(app.active_tab, Tab::Scripts);
    }

    #[test]
    fn test_switch_tab_does_nothing_without_workspaces() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("test", "echo test")])
            .build();

        assert_eq!(app.active_tab, Tab::Scripts);
        app.switch_tab(1);
        assert_eq!(app.active_tab, Tab::Scripts); // no change
    }

    // --- type_char / delete_char tests ---

    #[test]
    fn test_type_char_updates_query() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("test", "echo test")])
            .build();

        assert_eq!(app.query, "");
        app.type_char('t');
        assert_eq!(app.query, "t");
        app.type_char('e');
        assert_eq!(app.query, "te");
    }

    #[test]
    fn test_delete_char_removes_last_char() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("test", "echo test")])
            .build();

        app.query = "test".to_string();
        app.delete_char();
        assert_eq!(app.query, "tes");
        app.delete_char();
        assert_eq!(app.query, "te");
    }

    #[test]
    fn test_delete_char_on_empty_query() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("test", "echo test")])
            .build();

        assert_eq!(app.query, "");
        app.delete_char();
        assert_eq!(app.query, ""); // no panic, no change
    }

    // --- update_filtered tests ---

    #[test]
    fn test_update_filtered_with_empty_query() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![
                script("build", "echo build"),
                script("test", "echo test"),
            ])
            .build();

        app.query = "".to_string();
        app.update_filtered();

        // Should return all scripts in alphabetical order
        assert_eq!(app.filtered_indices.len(), 2);
    }

    #[test]
    fn test_update_filtered_prioritizes_script_name_match() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![
                script("build", "echo build"),
                script("test", "echo test"),
                script("lint", "echo lint"),
            ])
            .build();

        app.query = "te".to_string();
        app.update_filtered();

        assert_eq!(app.scripts[app.filtered_indices[0]].name, "test");
    }

    #[test]
    fn test_update_filtered_resets_selection() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![
                script("build", "echo build"),
                script("test", "echo test"),
            ])
            .build();

        app.selected_index = 1;
        app.scroll_offset = 5;

        app.update_filtered();

        assert_eq!(app.selected_index, 0);
        assert_eq!(app.scroll_offset, 0);
    }

    // --- handle_esc tests ---

    #[test]
    fn test_handle_esc_on_scripts_tab_quits() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("test", "echo test")])
            .build();

        let action = app.handle_esc();
        assert!(matches!(action, Action::Quit));
    }

    #[test]
    fn test_handle_esc_in_package_mode_goes_back() {
        let pkg = WorkspacePackage {
            name: "pkg1".to_string(),
            relative_path: "packages/pkg1".to_string(),
            scripts: {
                let mut map = IndexMap::new();
                map.insert("test".to_string(), "echo test".to_string());
                map
            },
        };

        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("test", "echo test")])
            .with_workspaces(vec![pkg])
            .build();

        // Enter package script mode
        app.active_tab = Tab::Packages;
        app.enter_package_scripts(0);
        assert!(matches!(
            app.package_mode,
            PackageMode::SelectingScript { .. }
        ));

        // Esc should go back to package list
        let action = app.handle_esc();
        assert!(matches!(action, Action::Continue));
        assert_eq!(app.package_mode, PackageMode::SelectingPackage);
    }

    // --- handle_enter tests ---

    #[test]
    fn test_handle_enter_returns_run_action() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![script("test", "echo test")])
            .build();

        let action = app.handle_enter();
        assert!(matches!(action, Action::RunScript { .. }));

        if let Action::RunScript { script_name, .. } = action {
            assert_eq!(script_name, "test");
        }
    }

    #[test]
    fn test_handle_enter_on_empty_list_returns_continue() {
        let mut app = TestAppBuilder::new().build();

        let action = app.handle_enter();
        assert!(matches!(action, Action::Continue));
    }

    // --- ensure_scroll tests ---

    #[test]
    fn test_ensure_scroll_adjusts_when_selected_below_offset() {
        let mut offset = 5;
        ensure_scroll(&mut offset, 3, 10);
        assert_eq!(offset, 3);
    }

    #[test]
    fn test_ensure_scroll_adjusts_when_selected_above_visible() {
        let mut offset = 0;
        ensure_scroll(&mut offset, 15, 10);
        assert_eq!(offset, 6); // 15 - 10 + 1
    }

    #[test]
    fn test_ensure_scroll_no_change_when_in_view() {
        let mut offset = 5;
        ensure_scroll(&mut offset, 10, 10);
        assert_eq!(offset, 5); // 10 is within [5, 15)
    }

    // --- wrap_index tests ---

    #[test]
    fn test_wrap_index_normal_increment() {
        assert_eq!(wrap_index(0, 1, 5), 1);
        assert_eq!(wrap_index(2, 1, 5), 3);
    }

    #[test]
    fn test_wrap_index_wraps_at_end() {
        assert_eq!(wrap_index(4, 1, 5), 0);
    }

    #[test]
    fn test_wrap_index_wraps_at_start() {
        assert_eq!(wrap_index(0, -1, 5), 4);
    }

    #[test]
    fn test_wrap_index_handles_zero_length() {
        assert_eq!(wrap_index(0, 1, 0), 0);
        assert_eq!(wrap_index(5, -1, 0), 0);
    }
}
