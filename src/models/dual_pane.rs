use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{FileEntry, SortState};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PaneSide {
    #[default]
    Left,
    Right,
}

impl PaneSide {

    pub fn opposite(&self) -> Self {
        match self {
            PaneSide::Left => PaneSide::Right,
            PaneSide::Right => PaneSide::Left,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneState {
    pub path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selection: Vec<usize>,
    pub scroll_offset: f32,
    pub sort_state: SortState,
}

impl PaneState {

    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            entries: Vec::new(),
            selection: Vec::new(),
            scroll_offset: 0.0,
            sort_state: SortState::default(),
        }
    }


    pub fn set_entries(&mut self, entries: Vec<FileEntry>) {
        self.entries = entries;
        self.selection.clear();
    }


    pub fn navigate_to(&mut self, path: PathBuf) {
        self.path = path;
        self.entries.clear();
        self.selection.clear();
        self.scroll_offset = 0.0;
    }


    pub fn selected_entries(&self) -> Vec<&FileEntry> {
        self.selection
            .iter()
            .filter_map(|&idx| self.entries.get(idx))
            .collect()
    }


    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.selected_entries()
            .iter()
            .map(|e| e.path.clone())
            .collect()
    }


    pub fn select(&mut self, index: usize) {
        self.selection.clear();
        if index < self.entries.len() {
            self.selection.push(index);
        }
    }


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


    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }


    pub fn is_selected(&self, index: usize) -> bool {
        self.selection.contains(&index)
    }


    pub fn first_selected(&self) -> Option<usize> {
        self.selection.first().copied()
    }
}

impl Default for PaneState {
    fn default() -> Self {
        Self::new(PathBuf::from("/"))
    }
}





#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualPane {
    left: PaneState,
    right: PaneState,
    active: PaneSide,
    is_enabled: bool,
}

impl DualPane {

    pub fn new(initial_path: PathBuf) -> Self {
        Self {
            left: PaneState::new(initial_path.clone()),
            right: PaneState::new(initial_path),
            active: PaneSide::Left,
            is_enabled: false,
        }
    }


    pub fn with_paths(left_path: PathBuf, right_path: PathBuf) -> Self {
        Self {
            left: PaneState::new(left_path),
            right: PaneState::new(right_path),
            active: PaneSide::Left,
            is_enabled: false,
        }
    }


    pub fn enable(&mut self) {
        self.is_enabled = true;
    }


    pub fn disable(&mut self) {
        self.is_enabled = false;
    }


    pub fn toggle(&mut self) {
        self.is_enabled = !self.is_enabled;
    }


    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }


    pub fn switch_active(&mut self) {
        self.active = self.active.opposite();
    }


    pub fn set_active(&mut self, side: PaneSide) {
        self.active = side;
    }


    pub fn active_side(&self) -> PaneSide {
        self.active
    }


    pub fn active_pane(&self) -> &PaneState {
        match self.active {
            PaneSide::Left => &self.left,
            PaneSide::Right => &self.right,
        }
    }


    pub fn active_pane_mut(&mut self) -> &mut PaneState {
        match self.active {
            PaneSide::Left => &mut self.left,
            PaneSide::Right => &mut self.right,
        }
    }


    pub fn inactive_pane(&self) -> &PaneState {
        match self.active {
            PaneSide::Left => &self.right,
            PaneSide::Right => &self.left,
        }
    }


    pub fn inactive_pane_mut(&mut self) -> &mut PaneState {
        match self.active {
            PaneSide::Left => &mut self.right,
            PaneSide::Right => &mut self.left,
        }
    }


    pub fn left_pane(&self) -> &PaneState {
        &self.left
    }


    pub fn left_pane_mut(&mut self) -> &mut PaneState {
        &mut self.left
    }


    pub fn right_pane(&self) -> &PaneState {
        &self.right
    }


    pub fn right_pane_mut(&mut self) -> &mut PaneState {
        &mut self.right
    }


    pub fn pane(&self, side: PaneSide) -> &PaneState {
        match side {
            PaneSide::Left => &self.left,
            PaneSide::Right => &self.right,
        }
    }


    pub fn pane_mut(&mut self, side: PaneSide) -> &mut PaneState {
        match side {
            PaneSide::Left => &mut self.left,
            PaneSide::Right => &mut self.right,
        }
    }


    pub fn copy_to_other(&self) -> Vec<PathBuf> {
        self.active_pane().selected_paths()
    }


    pub fn move_to_other(&self) -> Vec<PathBuf> {
        self.active_pane().selected_paths()
    }


    pub fn destination_path(&self) -> &PathBuf {
        &self.inactive_pane().path
    }


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
