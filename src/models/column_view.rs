use std::ops::Range;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use super::FileEntry;

/// A single column in the Miller Columns view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: Option<usize>,
}

impl Column {
    /// Creates a new column for the given path
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            entries: Vec::new(),
            selected_index: None,
        }
    }

    /// Creates a column with entries
    pub fn with_entries(path: PathBuf, entries: Vec<FileEntry>) -> Self {
        Self {
            path,
            entries,
            selected_index: None,
        }
    }

    /// Returns the currently selected entry, if any
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.selected_index.and_then(|idx| self.entries.get(idx))
    }

    /// Selects an entry by index
    pub fn select(&mut self, index: usize) {
        if index < self.entries.len() {
            self.selected_index = Some(index);
        }
    }

    /// Clears the selection
    pub fn clear_selection(&mut self) {
        self.selected_index = None;
    }

    /// Returns true if the column has entries
    pub fn has_entries(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Returns the number of entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new(PathBuf::new())
    }
}

/// Miller Columns view state management
/// 
/// Displays directories as cascading columns where selecting a directory
/// in one column shows its contents in the next column to the right.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnView {
    columns: Vec<Column>,
    scroll_offset: f32,
    column_width: f32,
    root_path: PathBuf,
}

impl ColumnView {
    /// Default column width in pixels
    pub const DEFAULT_COLUMN_WIDTH: f32 = 220.0;
    
    /// Minimum column width
    pub const MIN_COLUMN_WIDTH: f32 = 150.0;
    
    /// Maximum column width
    pub const MAX_COLUMN_WIDTH: f32 = 400.0;

    /// Creates a new column view starting at the given root path
    pub fn new(root: PathBuf) -> Self {
        let root_column = Column::new(root.clone());
        Self {
            columns: vec![root_column],
            scroll_offset: 0.0,
            column_width: Self::DEFAULT_COLUMN_WIDTH,
            root_path: root,
        }
    }

    /// Creates a column view with a custom column width
    pub fn with_column_width(root: PathBuf, width: f32) -> Self {
        let clamped_width = width.clamp(Self::MIN_COLUMN_WIDTH, Self::MAX_COLUMN_WIDTH);
        let root_column = Column::new(root.clone());
        Self {
            columns: vec![root_column],
            scroll_offset: 0.0,
            column_width: clamped_width,
            root_path: root,
        }
    }

    /// Returns a reference to all columns
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// Returns the number of columns
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Returns the current scroll offset
    pub fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    /// Sets the scroll offset
    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.scroll_offset = offset.max(0.0);
    }

    /// Returns the column width
    pub fn column_width(&self) -> f32 {
        self.column_width
    }

    /// Sets the column width
    pub fn set_column_width(&mut self, width: f32) {
        self.column_width = width.clamp(Self::MIN_COLUMN_WIDTH, Self::MAX_COLUMN_WIDTH);
    }

    /// Returns the root path
    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }

    /// Returns the range of visible columns for the given viewport width
    pub fn visible_columns(&self, viewport_width: f32) -> Range<usize> {
        if self.columns.is_empty() || viewport_width <= 0.0 {
            return 0..0;
        }

        let columns_per_viewport = (viewport_width / self.column_width).ceil() as usize;
        let start_column = (self.scroll_offset / self.column_width).floor() as usize;
        let end_column = (start_column + columns_per_viewport + 1).min(self.columns.len());
        
        start_column.min(self.columns.len())..end_column
    }

    /// Returns a reference to a column by index
    pub fn column(&self, index: usize) -> Option<&Column> {
        self.columns.get(index)
    }

    /// Returns a mutable reference to a column by index
    pub fn column_mut(&mut self, index: usize) -> Option<&mut Column> {
        self.columns.get_mut(index)
    }

    /// Returns the last column
    pub fn last_column(&self) -> Option<&Column> {
        self.columns.last()
    }

    /// Returns a mutable reference to the last column
    pub fn last_column_mut(&mut self) -> Option<&mut Column> {
        self.columns.last_mut()
    }

    /// Returns the index of the last column
    pub fn last_column_index(&self) -> Option<usize> {
        if self.columns.is_empty() {
            None
        } else {
            Some(self.columns.len() - 1)
        }
    }

    /// Sets entries for a column at the given index
    pub fn set_column_entries(&mut self, column_index: usize, entries: Vec<FileEntry>) {
        if let Some(column) = self.columns.get_mut(column_index) {
            column.entries = entries;
        }
    }

    /// Selects an entry in a column and updates the view accordingly
    /// 
    /// When a directory is selected, a new column is added (or existing columns
    /// to the right are replaced). When a file is selected, columns to the right
    /// are removed.
    pub fn select(&mut self, column_index: usize, entry_index: usize) {
        if column_index >= self.columns.len() {
            return;
        }
        
        let column = &self.columns[column_index];
        if entry_index >= column.entries.len() {
            return;
        }

        self.columns[column_index].selected_index = Some(entry_index);

        let selected_entry = &self.columns[column_index].entries[entry_index];
        let is_dir = selected_entry.is_dir;
        let entry_path = selected_entry.path.clone();

        // Remove all columns to the right of the selected column
        self.columns.truncate(column_index + 1);

        // If a directory is selected, add a new column for it
        if is_dir {
            let new_column = Column::new(entry_path);
            self.columns.push(new_column);
        }
    }

    /// Navigates right into the selected directory
    /// 
    /// If the current selection is a directory, moves focus to the next column.
    /// Returns true if navigation occurred.
    pub fn navigate_right(&mut self) -> bool {
        // Find the rightmost column with a selection
        let active_column_idx = self.find_active_column_index();
        
        if let Some(col_idx) = active_column_idx {
            let column = &self.columns[col_idx];
            
            if let Some(entry) = column.selected_entry() {
                if entry.is_dir {
                    // If there's a column to the right, select its first entry
                    let next_col_idx = col_idx + 1;
                    if next_col_idx < self.columns.len() {
                        let next_column = &self.columns[next_col_idx];
                        if !next_column.entries.is_empty() {
                            self.columns[next_col_idx].selected_index = Some(0);
                            self.ensure_column_visible(next_col_idx);
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }

    /// Navigates left to the parent column
    /// 
    /// Moves focus to the column to the left of the current active column.
    /// Returns true if navigation occurred.
    pub fn navigate_left(&mut self) -> bool {
        let active_column_idx = self.find_active_column_index();
        
        if let Some(col_idx) = active_column_idx {
            if col_idx > 0 {
                // Clear selection in current column and move to parent
                self.columns[col_idx].clear_selection();
                self.ensure_column_visible(col_idx - 1);
                return true;
            }
        }
        
        false
    }

    /// Navigates up within the current column
    /// 
    /// Moves selection to the previous entry in the active column.
    /// Returns true if navigation occurred.
    pub fn navigate_up(&mut self) -> bool {
        let active_column_idx = self.find_active_column_index();
        
        if let Some(col_idx) = active_column_idx {
            let column = &mut self.columns[col_idx];
            if let Some(current_idx) = column.selected_index {
                if current_idx > 0 {
                    let new_idx = current_idx - 1;
                    self.select(col_idx, new_idx);
                    return true;
                }
            } else if !column.entries.is_empty() {
                // No selection, select last item
                let last_idx = column.entries.len() - 1;
                self.select(col_idx, last_idx);
                return true;
            }
        }
        
        false
    }

    /// Navigates down within the current column
    /// 
    /// Moves selection to the next entry in the active column.
    /// Returns true if navigation occurred.
    pub fn navigate_down(&mut self) -> bool {
        let active_column_idx = self.find_active_column_index();
        
        if let Some(col_idx) = active_column_idx {
            let column = &self.columns[col_idx];
            let entry_count = column.entries.len();
            
            if entry_count == 0 {
                return false;
            }
            
            if let Some(current_idx) = column.selected_index {
                if current_idx + 1 < entry_count {
                    let new_idx = current_idx + 1;
                    self.select(col_idx, new_idx);
                    return true;
                }
            } else {
                // No selection, select first item
                self.select(col_idx, 0);
                return true;
            }
        }
        
        false
    }

    /// Finds the index of the rightmost column with a selection
    fn find_active_column_index(&self) -> Option<usize> {
        // Find the rightmost column that has a selection
        for (idx, column) in self.columns.iter().enumerate().rev() {
            if column.selected_index.is_some() {
                return Some(idx);
            }
        }
        
        for (idx, column) in self.columns.iter().enumerate().rev() {
            if !column.entries.is_empty() {
                return Some(idx);
            }
        }
        
        if !self.columns.is_empty() {
            Some(0)
        } else {
            None
        }
    }

    /// Ensures a column is visible by adjusting scroll offset
    fn ensure_column_visible(&mut self, column_index: usize) {
        let column_start = column_index as f32 * self.column_width;
        let column_end = column_start + self.column_width;
        
        // Scroll left if column is before visible area
        if column_start < self.scroll_offset {
            self.scroll_offset = column_start;
        }
        
        // Note: We can't scroll right without knowing viewport width
    }

    /// Ensures a column is visible within the given viewport width
    pub fn ensure_column_visible_in_viewport(&mut self, column_index: usize, viewport_width: f32) {
        let column_start = column_index as f32 * self.column_width;
        let column_end = column_start + self.column_width;
        
        // Scroll left if column is before visible area
        if column_start < self.scroll_offset {
            self.scroll_offset = column_start;
        }
        // Scroll right if column is after visible area
        else if column_end > self.scroll_offset + viewport_width {
            self.scroll_offset = column_end - viewport_width;
        }
    }

    /// Resets the view to show only the root column
    pub fn reset(&mut self) {
        self.columns.truncate(1);
        if let Some(column) = self.columns.first_mut() {
            column.clear_selection();
        }
        self.scroll_offset = 0.0;
    }

    /// Changes the root path and resets the view
    pub fn set_root(&mut self, path: PathBuf) {
        self.root_path = path.clone();
        self.columns.clear();
        self.columns.push(Column::new(path));
        self.scroll_offset = 0.0;
    }

    /// Returns the currently selected entry across all columns
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.find_active_column_index()
            .and_then(|idx| self.columns.get(idx))
            .and_then(|col| col.selected_entry())
    }

    /// Returns the path of the currently selected entry
    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.selected_entry().map(|e| &e.path)
    }

    /// Returns the total width needed to display all columns
    pub fn total_width(&self) -> f32 {
        self.columns.len() as f32 * self.column_width
    }

    /// Reconstructs the full path from root to the deepest selected directory
    pub fn current_path(&self) -> PathBuf {
        // Find the deepest column with content
        for column in self.columns.iter().rev() {
            if !column.path.as_os_str().is_empty() {
                return column.path.clone();
            }
        }
        self.root_path.clone()
    }

    /// Returns the path hierarchy from root to current selection
    pub fn path_hierarchy(&self) -> Vec<&PathBuf> {
        self.columns.iter().map(|c| &c.path).collect()
    }
}

impl Default for ColumnView {
    fn default() -> Self {
        Self::new(PathBuf::from("/"))
    }
}
