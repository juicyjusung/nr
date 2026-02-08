use crate::core::workspaces::WorkspacePackage;
use crate::fuzzy::fuzzy_filter;
use crate::sort::{SortableScript, sort_scripts};
use crate::store::favorites;
use crate::store::recents::{self, RecentEntry};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use indexmap::IndexMap;
use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

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

pub enum Action {
    Continue,
    RunScript { script_name: String, cwd: PathBuf },
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
    pub project_id: String,

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
}

impl App {
    pub fn new(
        raw_scripts: IndexMap<String, String>,
        workspace_packages: Vec<WorkspacePackage>,
        nearest_pkg: PathBuf,
        monorepo_root: Option<PathBuf>,
        config_dir: &std::path::Path,
        project_id: String,
    ) -> Self {
        let has_workspaces = !workspace_packages.is_empty();

        // Convert IndexMap to Vec<SortableScript>
        let scripts: Vec<SortableScript> = raw_scripts
            .iter()
            .map(|(name, command)| SortableScript {
                key: format!("{}:root:{}", project_id, name),
                name: name.clone(),
                command: command.clone(),
            })
            .collect();

        // Load persisted state
        let favorites_data = favorites::load_favorites(config_dir);
        let recents_data = recents::load_recents(config_dir);

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
            nearest_pkg,
            monorepo_root,

            favorites: favorites_data,
            recents: recents_data,
            project_id,

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
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => self.handle_esc(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Enter => self.handle_enter(),
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
                Constraint::Length(2), // tabs
                Constraint::Length(1), // search input
                Constraint::Min(1),    // main content
                Constraint::Length(1), // status bar
            ])
            .split(area)
        } else {
            Layout::vertical([
                Constraint::Length(0), // no tabs
                Constraint::Length(1), // search input
                Constraint::Min(1),    // main content
                Constraint::Length(1), // status bar
            ])
            .split(area)
        };

        // Track actual visible height for scroll calculations
        self.visible_height = chunks[2].height as usize;

        // Tabs (only if workspaces exist)
        if self.has_workspaces {
            let tab_labels = vec!["Scripts", "Packages"];
            let active = match self.active_tab {
                Tab::Scripts => 0,
                Tab::Packages => 1,
            };
            crate::ui::tabs::render_tabs(frame, chunks[0], &tab_labels, active);
        }

        // Search input
        let current_query = self.current_query();
        crate::ui::search_input::render_search_input(frame, chunks[1], current_query);

        // Main content
        match self.active_tab {
            Tab::Scripts => {
                crate::ui::script_list::render_script_list(
                    frame,
                    chunks[2],
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
                        chunks[2],
                        &self.workspace_packages,
                        &self.pkg_filtered_indices,
                        self.pkg_selected_index,
                        self.pkg_scroll_offset,
                    );
                }
                PackageMode::SelectingScript { .. } => {
                    crate::ui::script_list::render_script_list(
                        frame,
                        chunks[2],
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
        crate::ui::status_bar::render_status_bar(frame, chunks[3]);
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

                        Action::RunScript { script_name, cwd }
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
                key: format!("{}:{}:{}", self.project_id, pkg_name, name),
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
        ensure_scroll(&mut self.scroll_offset, self.selected_index, self.visible_height);
    }

    fn ensure_visible_packages(&mut self) {
        ensure_scroll(&mut self.pkg_scroll_offset, self.pkg_selected_index, self.visible_height);
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
