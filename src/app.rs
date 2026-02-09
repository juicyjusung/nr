use crate::core::env_files::{EnvFile, EnvFileList, scan_env_files};
use crate::core::workspaces::WorkspacePackage;
use crate::fuzzy::fuzzy_filter;
use crate::sort::{SortableScript, sort_scripts};
use crate::store::args_history::{self, ArgsHistory};
use crate::store::favorites;
use crate::store::recents::{self, RecentEntry};
use crate::store::script_configs::{self, ScriptConfig, ScriptConfigs};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use indexmap::IndexMap;
use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::*;
use std::collections::HashSet;
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

pub struct App {
    // Navigation
    pub active_tab: Tab,
    pub package_mode: PackageMode,
    pub has_workspaces: bool,

    // Data
    pub scripts: Vec<SortableScript>,
    pub workspace_packages: Vec<WorkspacePackage>,
    pub nearest_pkg: PathBuf,
    pub monorepo_root: Option<PathBuf>,

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
    pub pkg_script_sortable: Vec<SortableScript>,

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
        let has_workspaces = !workspace_packages.is_empty();

        // Convert IndexMap to Vec<SortableScript>
        let scripts: Vec<SortableScript> = raw_scripts
            .iter()
            .map(|(name, command)| SortableScript {
                key: format!("root:{}", name),
                name: name.clone(),
                command: command.clone(),
            })
            .collect();

        // Load persisted state from project-scoped directory
        let favorites_data = favorites::load_favorites(project_dir);
        let recents_data = recents::load_recents(project_dir);
        let script_configs_data =
            script_configs::load_script_configs(project_dir).unwrap_or_default();
        let global_env_data =
            crate::store::global_env::load_global_env_config(project_dir).unwrap_or_default();
        let args_history_data = args_history::load_args_history(project_dir).unwrap_or_default();

        // Initial sort/filter
        let filtered_indices = sort_scripts(&scripts, &favorites_data, &recents_data, "");

        // Initial package filter (all packages, original order)
        let pkg_filtered_indices: Vec<usize> = (0..workspace_packages.len()).collect();

        App {
            active_tab: Tab::Scripts,
            package_mode: PackageMode::SelectingPackage,
            has_workspaces,

            scripts,
            workspace_packages,
            nearest_pkg: nearest_pkg.clone(),
            monorepo_root: monorepo_root.clone(),

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
            pkg_script_sortable: Vec::new(),

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
                        &self.pkg_script_sortable,
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
        match self.active_tab {
            Tab::Scripts => {
                if let Some(&script_idx) = self.filtered_indices.get(self.selected_index) {
                    let script = &self.scripts[script_idx];
                    let script_name = script.name.clone();
                    let key = script.key.clone();

                    // Record execution
                    recents::record_execution(&mut self.recents, &key);

                    Action::RunScript {
                        script_name,
                        cwd: self.nearest_pkg.clone(),
                        env_files: vec![],
                        args: String::new(),
                    }
                } else {
                    Action::Continue
                }
            }
            Tab::Packages => match self.package_mode {
                PackageMode::SelectingPackage => {
                    if let Some(&pkg_idx) = self.pkg_filtered_indices.get(self.pkg_selected_index) {
                        // Enter package script selection mode
                        self.enter_package_scripts(pkg_idx);
                    }
                    Action::Continue
                }
                PackageMode::SelectingScript { package_index } => {
                    if let Some(&script_idx) = self
                        .pkg_script_filtered_indices
                        .get(self.pkg_script_selected_index)
                    {
                        let script = &self.pkg_script_sortable[script_idx];
                        let script_name = script.name.clone();
                        let key = script.key.clone();

                        // Record execution
                        recents::record_execution(&mut self.recents, &key);

                        // cwd is the monorepo_root joined with the package's relative_path
                        let pkg = &self.workspace_packages[package_index];
                        let cwd = self
                            .monorepo_root
                            .as_ref()
                            .map(|r| r.join(&pkg.relative_path))
                            .unwrap_or_else(|| self.nearest_pkg.clone());

                        Action::RunScript {
                            script_name,
                            cwd,
                            env_files: vec![],
                            args: String::new(),
                        }
                    } else {
                        Action::Continue
                    }
                }
            },
        }
    }

    fn enter_package_scripts(&mut self, pkg_idx: usize) {
        let pkg = &self.workspace_packages[pkg_idx];
        let pkg_name = &pkg.name;

        // Convert package scripts to SortableScript
        self.pkg_script_sortable = pkg
            .scripts
            .iter()
            .map(|(name, command)| SortableScript {
                key: format!("{}:{}", pkg_name, name),
                name: name.clone(),
                command: command.clone(),
            })
            .collect();

        self.package_mode = PackageMode::SelectingScript {
            package_index: pkg_idx,
        };
        self.pkg_script_query.clear();
        self.pkg_script_selected_index = 0;
        self.pkg_script_scroll_offset = 0;

        // Initial filter: all scripts sorted
        self.pkg_script_filtered_indices = sort_scripts(
            &self.pkg_script_sortable,
            &self.favorites,
            &self.recents,
            "",
        );
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
        match self.active_tab {
            Tab::Scripts => {
                if let Some(&script_idx) = self.filtered_indices.get(self.selected_index) {
                    let key = self.scripts[script_idx].key.clone();
                    favorites::toggle_favorite(&mut self.favorites, &key);
                    self.update_filtered();
                }
            }
            Tab::Packages => {
                if let PackageMode::SelectingScript { .. } = self.package_mode {
                    if let Some(&script_idx) = self
                        .pkg_script_filtered_indices
                        .get(self.pkg_script_selected_index)
                    {
                        let key = self.pkg_script_sortable[script_idx].key.clone();
                        favorites::toggle_favorite(&mut self.favorites, &key);
                        self.update_pkg_script_filtered();
                    }
                }
            }
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
            sort_scripts(&self.scripts, &self.favorites, &self.recents, &self.query);
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
        self.pkg_script_filtered_indices = sort_scripts(
            &self.pkg_script_sortable,
            &self.favorites,
            &self.recents,
            &self.pkg_script_query,
        );
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
        // Get current script key
        let script_key = self.get_current_script_key();

        // Restore script-specific args (if exists)
        if let Some(config) = self.script_configs.get(&script_key) {
            self.execution_config.args = config.args.clone();
        } else {
            self.execution_config = ExecutionConfig::default();
        }

        // Scan .env files
        let cwd = self.get_current_cwd();
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

    fn get_current_script_key(&self) -> String {
        let project_id = crate::store::project_id::project_id(&self.config_dir);

        match self.active_tab {
            Tab::Scripts => {
                if let Some(&script_idx) = self.filtered_indices.get(self.selected_index) {
                    let script = &self.scripts[script_idx];
                    format!("{}:{}", project_id, script.key)
                } else {
                    format!("{}:unknown", project_id)
                }
            }
            Tab::Packages => match self.package_mode {
                PackageMode::SelectingScript { package_index: _ } => {
                    if let Some(&script_idx) = self
                        .pkg_script_filtered_indices
                        .get(self.pkg_script_selected_index)
                    {
                        let script = &self.pkg_script_sortable[script_idx];
                        format!("{}:{}", project_id, script.key)
                    } else {
                        format!("{}:unknown", project_id)
                    }
                }
                _ => format!("{}:unknown", project_id),
            },
        }
    }

    fn get_current_cwd(&self) -> PathBuf {
        match self.active_tab {
            Tab::Scripts => self.nearest_pkg.clone(),
            Tab::Packages => match self.package_mode {
                PackageMode::SelectingScript { package_index } => {
                    let pkg = &self.workspace_packages[package_index];
                    self.monorepo_root
                        .as_ref()
                        .map(|r| r.join(&pkg.relative_path))
                        .unwrap_or_else(|| self.nearest_pkg.clone())
                }
                _ => self.nearest_pkg.clone(),
            },
        }
    }

    fn handle_env_mode(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Esc => {
                // Cancel configuration
                self.mode = AppMode::Normal;
                self.execution_config = ExecutionConfig::default();
                self.env_files_list = None;
                Action::Continue
            }
            KeyCode::Enter => {
                // Proceed to args input
                self.mode = AppMode::ConfigureArgs;
                self.args_input = self.execution_config.args.clone();
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
                self.args_cursor_pos = self.args_input.len();
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
                self.args_cursor_pos = self.args_input.len();
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
                if self.args_cursor_pos < self.args_input.len() {
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
                self.args_cursor_pos = self.args_input.len();
                Action::Continue
            }
            KeyCode::Char(c) => {
                // Insert character at cursor position
                self.args_input.insert(self.args_cursor_pos, c);
                self.args_cursor_pos += 1;
                self.args_history_index = None;
                Action::Continue
            }
            KeyCode::Backspace => {
                // Delete character before cursor
                if self.args_cursor_pos > 0 {
                    self.args_input.remove(self.args_cursor_pos - 1);
                    self.args_cursor_pos -= 1;
                    self.args_history_index = None;
                }
                Action::Continue
            }
            KeyCode::Delete => {
                // Delete character at cursor
                if self.args_cursor_pos < self.args_input.len() {
                    self.args_input.remove(self.args_cursor_pos);
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
                // Execute with configuration
                let script_key = self.get_current_script_key();
                let script_name = self.get_current_script_name();
                let cwd = self.get_current_cwd();

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

                // Record execution in recents
                let execution_key = script_key.split(':').skip(1).collect::<Vec<_>>().join(":");
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
        match self.active_tab {
            Tab::Scripts => {
                if let Some(&script_idx) = self.filtered_indices.get(self.selected_index) {
                    self.scripts[script_idx].name.clone()
                } else {
                    String::new()
                }
            }
            Tab::Packages => match self.package_mode {
                PackageMode::SelectingScript { .. } => {
                    if let Some(&script_idx) = self
                        .pkg_script_filtered_indices
                        .get(self.pkg_script_selected_index)
                    {
                        self.pkg_script_sortable[script_idx].name.clone()
                    } else {
                        String::new()
                    }
                }
                _ => String::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::args_history::ArgsHistory;
    use crate::store::script_configs::ScriptConfigs;

    // Test helper to create SortableScript
    fn script(name: &str, command: &str) -> SortableScript {
        SortableScript {
            key: format!("root:{}", name),
            name: name.to_string(),
            command: command.to_string(),
        }
    }

    // Test builder for App
    struct TestAppBuilder {
        scripts: Vec<SortableScript>,
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

        fn with_scripts(mut self, scripts: Vec<SortableScript>) -> Self {
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
            let filtered_indices = sort_scripts(&self.scripts, &self.favorites, &self.recents, "");
            let pkg_filtered_indices: Vec<usize> = (0..self.workspace_packages.len()).collect();

            App {
                active_tab: Tab::Scripts,
                package_mode: PackageMode::SelectingPackage,
                has_workspaces: self.has_workspaces,
                scripts: self.scripts,
                workspace_packages: self.workspace_packages,
                nearest_pkg: PathBuf::from("/test/project"),
                monorepo_root: None,
                favorites: self.favorites,
                recents: self.recents,
                project_name: "test-project".to_string(),
                project_path: "/test/project".to_string(),
                package_manager_name: "npm".to_string(),
                visible_height: self.visible_height,
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
                pkg_script_sortable: Vec::new(),

                // NEW: Config flow fields (test defaults)
                mode: AppMode::Normal,
                execution_config: ExecutionConfig::default(),
                script_configs: ScriptConfigs::new(),
                global_env_config: crate::store::global_env::GlobalEnvConfig::default(),
                args_history: ArgsHistory::new(),
                config_dir: PathBuf::from("/test/.config/nr"),
                package_manager: crate::core::package_manager::PackageManager::Npm,

                // NEW: Env selection UI state (test defaults)
                env_files_list: None,
                env_selected_index: 0,
                env_scroll_offset: 0,
                env_selected_files: HashSet::new(),

                // NEW: Args input UI state (test defaults)
                args_input: String::new(),
                args_cursor_pos: 0,
                args_history_index: None,
            }
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

        let key = "root:test";
        assert!(!app.favorites.contains(key));

        app.toggle_fav();
        assert!(app.favorites.contains(key));
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
    fn test_update_filtered_with_query_filters_correctly() {
        let mut app = TestAppBuilder::new()
            .with_scripts(vec![
                script("build", "echo build"),
                script("test", "echo test"),
                script("lint", "echo lint"),
            ])
            .build();

        app.query = "te".to_string();
        app.update_filtered();

        // Should only match "test"
        assert_eq!(app.filtered_indices.len(), 1);
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
