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

/// Represents a single tab in the tab bar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: TabId,
    pub path: PathBuf,
    pub title: String,
    pub needs_refresh: bool,
    pub scroll_position: f32,
    pub selection: Option<usize>,
}

impl Tab {
    /// Create a new tab for the given path
    pub fn new(id: TabId, path: PathBuf) -> Self {
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // For root paths, use the path string itself
                path.to_string_lossy().to_string()
            });

        Self {
            id,
            path,
            title,
            needs_refresh: false,
            scroll_position: 0.0,
            selection: None,
        }
    }

    /// Update the tab's path and title
    pub fn set_path(&mut self, path: PathBuf) {
        self.title = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        self.path = path;
    }

    /// Mark the tab as needing refresh
    pub fn mark_needs_refresh(&mut self) {
        self.needs_refresh = true;
    }

    /// Clear the needs_refresh flag
    pub fn clear_needs_refresh(&mut self) {
        self.needs_refresh = false;
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
