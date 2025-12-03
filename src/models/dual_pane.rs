use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use super::{FileEntry, SortState};

/// Identifies which pane is active in dual pane mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PaneSide {
    #[default]
    Left,
    Right,
}

impl PaneSide {
    /// Returns the opposite pane side
    pub fn opposite(&self) -> Self {
        match self {
            PaneSide::Left => PaneSide::Right,
            PaneSide::Right => PaneSide::Left,
        }
    }
}

/// State for a single pane in dual pane mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneState {
    pub path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selection: Vec<usize>,
    pub scroll_offset: f32,
    pub sort_state: SortState,
}

impl PaneState {
    /// Creates a new pane state with the given path
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            entries: Vec::new(),
            selection: Vec::new(),
            scroll_offset: 0.0,
            sort_state: SortState::default(),
        }
    }

    /// Sets the entries for this pane
    pub fn set_entries(&mut self, entries: Vec<FileEntry>) {
        self.entries = entries;
        // Clear selection when entries change
        self.selection.clear();
    }

    /// Navigates to a new path
    pub fn navigate_to(&mut self, path: PathBuf) {
        self.path = path;
        self.entries.clear();
        self.selection.clear();
        self.scroll_offset = 0.0;
    }

    /// Returns the currently selected entries
    pub fn selected_entries(&self) -> Vec<&FileEntry> {
        self.selection
            .iter()
            .filter_map(|&idx| self.entries.get(idx))
            .collect()
    }

    /// Returns paths of selected entries
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.selected_entries()
            .iter()
            .map(|e| e.path.clone())
            .collect()
    }

    /// Selects a single item by index
    pub fn select(&mut self, index: usize) {
        self.selection.clear();
        if index < self.entries.len() {
            self.selection.push(index);
        }
    }

    /// Toggles selection of an item (for multi-select)
    pub fn toggle_selection(&mut self, index: usize) {
        if index >= self.entries.len() {
            return;
        }
        
        if let Some(pos) = self.selection.iter().position(|&i| i == index) {
            self.selection.remove(pos);
        } else {
            self.selection.push(index);
        }
    }

    /// Clears all selections
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// Returns true if the given index is selected
    pub fn is_selected(&self, index: usize) -> bool {
        self.selection.contains(&index)
    }

    /// Returns the first selected index, if any
    pub fn first_selected(&self) -> Option<usize> {
        self.selection.first().copied()
    }
}

impl Default for PaneState {
    fn default() -> Self {
        Self::new(PathBuf::from("/"))
    }
}

/// Dual pane mode state management
/// 
/// Manages two independent file list panes that can be displayed side by side.
/// Each pane maintains its own path, entries, selection, and sort state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualPane {
    left: PaneState,
    right: PaneState,
    active: PaneSide,
    is_enabled: bool,
}

impl DualPane {
    /// Creates a new dual pane with both panes at the given path
    pub fn new(initial_path: PathBuf) -> Self {
        Self {
            left: PaneState::new(initial_path.clone()),
            right: PaneState::new(initial_path),
            active: PaneSide::Left,
            is_enabled: false,
        }
    }

    /// Creates a new dual pane with different paths for each pane
    pub fn with_paths(left_path: PathBuf, right_path: PathBuf) -> Self {
        Self {
            left: PaneState::new(left_path),
            right: PaneState::new(right_path),
            active: PaneSide::Left,
            is_enabled: false,
        }
    }

    /// Enables dual pane mode
    pub fn enable(&mut self) {
        self.is_enabled = true;
    }

    /// Disables dual pane mode
    pub fn disable(&mut self) {
        self.is_enabled = false;
    }

    /// Toggles dual pane mode on/off
    pub fn toggle(&mut self) {
        self.is_enabled = !self.is_enabled;
    }

    /// Returns true if dual pane mode is enabled
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    /// Switches the active pane to the other side
    pub fn switch_active(&mut self) {
        self.active = self.active.opposite();
    }

    /// Sets the active pane
    pub fn set_active(&mut self, side: PaneSide) {
        self.active = side;
    }

    /// Returns which pane is currently active
    pub fn active_side(&self) -> PaneSide {
        self.active
    }

    /// Returns a reference to the active pane
    pub fn active_pane(&self) -> &PaneState {
        match self.active {
            PaneSide::Left => &self.left,
            PaneSide::Right => &self.right,
        }
    }

    /// Returns a mutable reference to the active pane
    pub fn active_pane_mut(&mut self) -> &mut PaneState {
        match self.active {
            PaneSide::Left => &mut self.left,
            PaneSide::Right => &mut self.right,
        }
    }

    /// Returns a reference to the inactive pane
    pub fn inactive_pane(&self) -> &PaneState {
        match self.active {
            PaneSide::Left => &self.right,
            PaneSide::Right => &self.left,
        }
    }

    /// Returns a mutable reference to the inactive pane
    pub fn inactive_pane_mut(&mut self) -> &mut PaneState {
        match self.active {
            PaneSide::Left => &mut self.right,
            PaneSide::Right => &mut self.left,
        }
    }

    /// Returns a reference to the left pane
    pub fn left_pane(&self) -> &PaneState {
        &self.left
    }

    /// Returns a mutable reference to the left pane
    pub fn left_pane_mut(&mut self) -> &mut PaneState {
        &mut self.left
    }

    /// Returns a reference to the right pane
    pub fn right_pane(&self) -> &PaneState {
        &self.right
    }

    /// Returns a mutable reference to the right pane
    pub fn right_pane_mut(&mut self) -> &mut PaneState {
        &mut self.right
    }

    /// Returns a reference to the pane on the given side
    pub fn pane(&self, side: PaneSide) -> &PaneState {
        match side {
            PaneSide::Left => &self.left,
            PaneSide::Right => &self.right,
        }
    }

    /// Returns a mutable reference to the pane on the given side
    pub fn pane_mut(&mut self, side: PaneSide) -> &mut PaneState {
        match side {
            PaneSide::Left => &mut self.left,
            PaneSide::Right => &mut self.right,
        }
    }

    /// Returns paths of selected files in the active pane for copy operation
    pub fn copy_to_other(&self) -> Vec<PathBuf> {
        self.active_pane().selected_paths()
    }

    /// Returns paths of selected files in the active pane for move operation
    pub fn move_to_other(&self) -> Vec<PathBuf> {
        self.active_pane().selected_paths()
    }

    /// Returns the destination path (inactive pane's current path)
    pub fn destination_path(&self) -> &PathBuf {
        &self.inactive_pane().path
    }

    /// Synchronizes the inactive pane to the same path as the active pane
    pub fn sync_panes(&mut self) {
        let active_path = self.active_pane().path.clone();
        self.inactive_pane_mut().navigate_to(active_path);
    }
}

impl Default for DualPane {
    fn default() -> Self {
        Self::new(PathBuf::from("/"))
    }
}
