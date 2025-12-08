use crate::models::ViewMode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub usize);

impl TabId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

/
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

/
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

    /
    pub fn navigate_to(&mut self, path: PathBuf) {
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

/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
    tabs: Vec<Tab>,
    active_index: usize,
    next_id: usize,
}

impl TabState {
    /
    pub fn new(initial_path: PathBuf) -> Self {
        let initial_tab = Tab::new(TabId::new(0), initial_path);
        Self {
            tabs: vec![initial_tab],
            active_index: 0,
            next_id: 1,
        }
    }

    /
    pub fn open_tab(&mut self, path: PathBuf) -> TabId {
        let id = TabId::new(self.next_id);
        self.next_id += 1;

        let tab = Tab::new(id, path);
        self.tabs.push(tab);

        self.active_index = self.tabs.len() - 1;

        id
    }

    /
    /
    pub fn close_tab(&mut self, id: TabId) -> bool {
        if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
            if self.tabs.len() == 1 {
                let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
                self.tabs[0].set_path(home);
                return true;
            }

            self.tabs.remove(index);

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

    /
    pub fn switch_to(&mut self, id: TabId) -> bool {
        if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
            self.active_index = index;
            true
        } else {
            false
        }
    }

    /
    pub fn switch_to_index(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_index = index;
            true
        } else {
            false
        }
    }

    /
    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_index]
    }

    /
    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_index]
    }

    /
    pub fn active_tab_id(&self) -> TabId {
        self.tabs[self.active_index].id
    }

    /
    pub fn active_index(&self) -> usize {
        self.active_index
    }

    /
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /
    pub fn get_tab(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    /
    pub fn get_tab_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    /
    pub fn update_active_path(&mut self, path: PathBuf) {
        self.tabs[self.active_index].set_path(path);
    }

    /
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = (self.active_index + 1) % self.tabs.len();
        }
    }

    /
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = if self.active_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_index - 1
            };
        }
    }

    /
    pub fn close_active_tab(&mut self) -> bool {
        let id = self.active_tab_id();
        self.close_tab(id)
    }

    /
    pub fn navigate_active_to(&mut self, path: PathBuf) {
        self.tabs[self.active_index].navigate_to(path);
    }

    /
    pub fn go_back(&mut self) -> Option<PathBuf> {
        self.tabs[self.active_index].go_back()
    }

    /
    pub fn go_forward(&mut self) -> Option<PathBuf> {
        self.tabs[self.active_index].go_forward()
    }

    /
    pub fn can_go_back(&self) -> bool {
        self.tabs[self.active_index].can_go_back()
    }

    /
    pub fn can_go_forward(&self) -> bool {
        self.tabs[self.active_index].can_go_forward()
    }

    /
    pub fn duplicate_active_tab(&mut self) -> TabId {
        let current_path = self.tabs[self.active_index].path.clone();
        self.open_tab(current_path)
    }

    /
    pub fn move_tab(&mut self, from_index: usize, to_index: usize) {
        if from_index < self.tabs.len() && to_index < self.tabs.len() && from_index != to_index {
            let tab = self.tabs.remove(from_index);
            self.tabs.insert(to_index, tab);

            if self.active_index == from_index {
                self.active_index = to_index;
            } else if from_index < self.active_index && to_index >= self.active_index {
                self.active_index -= 1;
            } else if from_index > self.active_index && to_index <= self.active_index {
                self.active_index += 1;
            }
        }
    }

    /
    pub fn close_other_tabs(&mut self) {
        let active_tab = self.tabs.remove(self.active_index);
        self.tabs.clear();
        self.tabs.push(active_tab);
        self.active_index = 0;
    }

    /
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
