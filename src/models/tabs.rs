use crate::models::ViewMode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique identifier for a tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub usize);

impl TabId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

/// Per-tab state that persists when switching between tabs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabViewState {
    pub scroll_position: f32,
    pub selection: Option<usize>,
    pub view_mode: ViewMode,
    pub show_hidden_files: bool,
}

impl Default for TabViewState {
    fn default() -> Self {
        Self {
            scroll_position: 0.0,
            selection: None,
            view_mode: ViewMode::List,
            show_hidden_files: false,
        }
    }
}

/// Represents a single tab in the tab bar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: TabId,
    pub path: PathBuf,
    pub title: String,
    pub needs_refresh: bool,
    pub history: Vec<PathBuf>,
    pub history_index: usize,
    pub view_state: TabViewState,
    pub is_loading: bool,
    pub pinned: bool,
}

impl Tab {
    pub fn new(id: TabId, path: PathBuf) -> Self {
        let title = Self::title_from_path(&path);
        Self {
            id,
            path: path.clone(),
            title,
            needs_refresh: false,
            history: vec![path],
            history_index: 0,
            view_state: TabViewState::default(),
            is_loading: false,
            pinned: false,
        }
    }

    fn title_from_path(path: &PathBuf) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.title = Self::title_from_path(&path);
        self.path = path;
    }

    /// Navigate to a new path, adding to history
    pub fn navigate_to(&mut self, path: PathBuf) {
        // Truncate forward history if we're not at the end
        if self.history_index < self.history.len().saturating_sub(1) {
            self.history.truncate(self.history_index + 1);
        }

        self.history.push(path.clone());
        self.history_index = self.history.len() - 1;
        self.set_path(path);
        self.view_state.scroll_position = 0.0;
        self.view_state.selection = None;
    }

    pub fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_index < self.history.len().saturating_sub(1)
    }

    pub fn go_back(&mut self) -> Option<PathBuf> {
        if self.can_go_back() {
            self.history_index -= 1;
            let path = self.history[self.history_index].clone();
            self.set_path(path.clone());
            Some(path)
        } else {
            None
        }
    }

    pub fn go_forward(&mut self) -> Option<PathBuf> {
        if self.can_go_forward() {
            self.history_index += 1;
            let path = self.history[self.history_index].clone();
            self.set_path(path.clone());
            Some(path)
        } else {
            None
        }
    }

    pub fn mark_needs_refresh(&mut self) {
        self.needs_refresh = true;
    }

    pub fn clear_needs_refresh(&mut self) {
        self.needs_refresh = false;
    }

    pub fn toggle_pinned(&mut self) {
        self.pinned = !self.pinned;
    }
}

/// Manages the state of all open tabs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
    tabs: Vec<Tab>,
    active_index: usize,
    next_id: usize,
}

impl TabState {
    /// Create a new TabState with an initial tab for the given path
    pub fn new(initial_path: PathBuf) -> Self {
        let initial_tab = Tab::new(TabId::new(0), initial_path);
        Self {
            tabs: vec![initial_tab],
            active_index: 0,
            next_id: 1,
        }
    }

    /// Open a new tab for the given path and return its ID
    pub fn open_tab(&mut self, path: PathBuf) -> TabId {
        let id = TabId::new(self.next_id);
        self.next_id += 1;

        let tab = Tab::new(id, path);
        self.tabs.push(tab);

        // Switch to the new tab
        self.active_index = self.tabs.len() - 1;

        id
    }

    /// Close the tab with the given ID
    /// Returns true if the tab was closed, false if it wasn't found
    pub fn close_tab(&mut self, id: TabId) -> bool {
        if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
            // Don't close if it's the last tab - instead open home directory
            if self.tabs.len() == 1 {
                let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
                self.tabs[0].set_path(home);
                return true;
            }

            self.tabs.remove(index);

            // Adjust active index if needed
            if self.active_index >= self.tabs.len() {
                self.active_index = self.tabs.len().saturating_sub(1);
            } else if self.active_index > index {
                self.active_index = self.active_index.saturating_sub(1);
            }

            true
        } else {
            false
        }
    }

    /// Switch to the tab with the given ID
    pub fn switch_to(&mut self, id: TabId) -> bool {
        if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
            self.active_index = index;
            true
        } else {
            false
        }
    }

    /// Switch to the tab at the given index
    pub fn switch_to_index(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_index = index;
            true
        } else {
            false
        }
    }

    /// Get a reference to the active tab
    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_index]
    }

    /// Get a mutable reference to the active tab
    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_index]
    }

    /// Get the active tab's ID
    pub fn active_tab_id(&self) -> TabId {
        self.tabs[self.active_index].id
    }

    /// Get the active tab index
    pub fn active_index(&self) -> usize {
        self.active_index
    }

    /// Get all tabs
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// Get the number of open tabs
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Get a tab by ID
    pub fn get_tab(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    /// Get a mutable reference to a tab by ID
    pub fn get_tab_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    /// Update the path of the active tab
    pub fn update_active_path(&mut self, path: PathBuf) {
        self.tabs[self.active_index].set_path(path);
    }

    /// Switch to the next tab (wraps around)
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = (self.active_index + 1) % self.tabs.len();
        }
    }

    /// Switch to the previous tab (wraps around)
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = if self.active_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_index - 1
            };
        }
    }

    /// Close the active tab
    pub fn close_active_tab(&mut self) -> bool {
        let id = self.active_tab_id();
        self.close_tab(id)
    }

    /// Navigate the active tab to a new path
    pub fn navigate_active_to(&mut self, path: PathBuf) {
        self.tabs[self.active_index].navigate_to(path);
    }

    /// Go back in the active tab's history
    pub fn go_back(&mut self) -> Option<PathBuf> {
        self.tabs[self.active_index].go_back()
    }

    /// Go forward in the active tab's history
    pub fn go_forward(&mut self) -> Option<PathBuf> {
        self.tabs[self.active_index].go_forward()
    }

    /// Check if active tab can go back
    pub fn can_go_back(&self) -> bool {
        self.tabs[self.active_index].can_go_back()
    }

    /// Check if active tab can go forward
    pub fn can_go_forward(&self) -> bool {
        self.tabs[self.active_index].can_go_forward()
    }

    /// Duplicate the active tab
    pub fn duplicate_active_tab(&mut self) -> TabId {
        let current_path = self.tabs[self.active_index].path.clone();
        self.open_tab(current_path)
    }

    /// Move tab to a new position
    pub fn move_tab(&mut self, from_index: usize, to_index: usize) {
        if from_index < self.tabs.len() && to_index < self.tabs.len() && from_index != to_index {
            let tab = self.tabs.remove(from_index);
            self.tabs.insert(to_index, tab);

            // Adjust active index
            if self.active_index == from_index {
                self.active_index = to_index;
            } else if from_index < self.active_index && to_index >= self.active_index {
                self.active_index -= 1;
            } else if from_index > self.active_index && to_index <= self.active_index {
                self.active_index += 1;
            }
        }
    }

    /// Close all tabs except the active one
    pub fn close_other_tabs(&mut self) {
        let active_tab = self.tabs.remove(self.active_index);
        self.tabs.clear();
        self.tabs.push(active_tab);
        self.active_index = 0;
    }

    /// Close tabs to the right of the active tab
    pub fn close_tabs_to_right(&mut self) {
        self.tabs.truncate(self.active_index + 1);
    }
}

impl Default for TabState {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        Self::new(home)
    }
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tests;
