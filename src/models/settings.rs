use gpui::Global;
use serde::{Deserialize, Serialize};

use crate::io::{SortKey, SortOrder};

/// Global application settings for user preferences.
/// 
/// This struct is registered as GPUI global state and provides
/// application-wide configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSettings {
    /// Whether to show hidden files (files starting with '.')
    pub show_hidden_files: bool,
    
    /// Current sort key for file listing
    pub sort_key: SortKey,
    
    /// Current sort order (ascending/descending)
    pub sort_order: SortOrder,
    
    /// Application theme mode (light/dark/system)
    pub theme_mode: AppThemeMode,
    
    /// Whether to show file extensions
    pub show_extensions: bool,
    
    /// Whether to show file sizes
    pub show_sizes: bool,
    
    /// Whether to show modification dates
    pub show_dates: bool,
    
    /// Default view mode
    pub view_mode: ViewMode,
    
    /// Grid view configuration
    pub grid_config: GridConfig,
}

/// Application theme mode (light/dark/system)
/// Note: This is separate from the RPG Theme system which provides full theming
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppThemeMode {
    Light,
    Dark,
    System,
}

/// View mode for file listing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    List,
    Grid,
    Details,
}

/// Configuration for grid view layout
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GridConfig {
    /// Icon size in pixels (default: 64.0)
    pub icon_size: f32,
    /// Total item width including padding (default: 120.0)
    pub item_width: f32,
    /// Total item height including name (default: 100.0)
    pub item_height: f32,
    /// Gap between items (default: 16.0)
    pub gap: f32,
    /// Minimum columns to display
    pub min_columns: usize,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            icon_size: 64.0,
            item_width: 120.0,
            item_height: 100.0,
            gap: 16.0,
            min_columns: 2,
        }
    }
}

impl GridConfig {
    /// Creates a new GridConfig with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate the number of columns that fit in the given viewport width
    pub fn columns_for_width(&self, viewport_width: f32) -> usize {
        if viewport_width <= 0.0 || self.item_width <= 0.0 {
            return self.min_columns;
        }
        
        // Account for gap between items: total_width = n * item_width + (n-1) * gap
        // Solving for n: n = (viewport_width + gap) / (item_width + gap)
        let effective_item_width = self.item_width + self.gap;
        let columns = ((viewport_width + self.gap) / effective_item_width).floor() as usize;
        
        columns.max(self.min_columns)
    }

    /// Calculate the number of rows needed for the given item count and viewport width
    pub fn rows_for_items(&self, item_count: usize, viewport_width: f32) -> usize {
        let columns = self.columns_for_width(viewport_width);
        if columns == 0 {
            return 0;
        }
        (item_count + columns - 1) / columns
    }

    /// Get the position (column, row) for an item at the given index
    pub fn position_for_index(&self, index: usize, viewport_width: f32) -> (usize, usize) {
        let columns = self.columns_for_width(viewport_width);
        if columns == 0 {
            return (0, 0);
        }
        let col = index % columns;
        let row = index / columns;
        (col, row)
    }

    /// Get the index for an item at the given position
    pub fn index_for_position(&self, col: usize, row: usize, viewport_width: f32) -> usize {
        let columns = self.columns_for_width(viewport_width);
        row * columns + col
    }

    /// Calculate the total content height for the given item count
    pub fn content_height(&self, item_count: usize, viewport_width: f32) -> f32 {
        let rows = self.rows_for_items(item_count, viewport_width);
        if rows == 0 {
            return 0.0;
        }
        rows as f32 * self.item_height + (rows.saturating_sub(1)) as f32 * self.gap
    }
}

impl GlobalSettings {
    /// Creates new GlobalSettings with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether hidden files should be shown.
    pub fn show_hidden(&self) -> bool {
        self.show_hidden_files
    }

    /// Sets whether to show hidden files.
    pub fn set_show_hidden(&mut self, show: bool) {
        self.show_hidden_files = show;
    }

    /// Toggles the show hidden files setting.
    pub fn toggle_show_hidden(&mut self) {
        self.show_hidden_files = !self.show_hidden_files;
    }

    /// Returns the current sort key.
    pub fn sort_key(&self) -> SortKey {
        self.sort_key
    }

    /// Sets the sort key.
    pub fn set_sort_key(&mut self, key: SortKey) {
        self.sort_key = key;
    }

    /// Returns the current sort order.
    pub fn sort_order(&self) -> SortOrder {
        self.sort_order
    }

    /// Sets the sort order.
    pub fn set_sort_order(&mut self, order: SortOrder) {
        self.sort_order = order;
    }

    /// Toggles the sort order between ascending and descending.
    pub fn toggle_sort_order(&mut self) {
        self.sort_order = match self.sort_order {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        };
    }

    /// Returns the current theme mode.
    pub fn theme_mode(&self) -> AppThemeMode {
        self.theme_mode
    }

    /// Sets the theme mode.
    pub fn set_theme_mode(&mut self, mode: AppThemeMode) {
        self.theme_mode = mode;
    }

    /// Returns the current view mode.
    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Sets the view mode.
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
    }

    /// Toggle between List and Grid view modes
    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::List | ViewMode::Details => ViewMode::Grid,
            ViewMode::Grid => ViewMode::List,
        };
    }

    /// Returns the grid configuration.
    pub fn grid_config(&self) -> &GridConfig {
        &self.grid_config
    }

    /// Returns mutable reference to grid configuration.
    pub fn grid_config_mut(&mut self) -> &mut GridConfig {
        &mut self.grid_config
    }

    /// Sets the grid configuration.
    pub fn set_grid_config(&mut self, config: GridConfig) {
        self.grid_config = config;
    }
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            show_hidden_files: false,
            sort_key: SortKey::Name,
            sort_order: SortOrder::Ascending,
            theme_mode: AppThemeMode::Dark,
            show_extensions: true,
            show_sizes: true,
            show_dates: true,
            view_mode: ViewMode::Details,
            grid_config: GridConfig::default(),
        }
    }
}

impl Global for GlobalSettings {}

impl Default for AppThemeMode {
    fn default() -> Self {
        Self::Dark
    }
}

impl Default for ViewMode {
    fn default() -> Self {
        Self::Details
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_default_settings() {
        let settings = GlobalSettings::default();
        assert!(!settings.show_hidden_files);
        assert_eq!(settings.sort_key, SortKey::Name);
        assert_eq!(settings.sort_order, SortOrder::Ascending);
        assert_eq!(settings.theme_mode, AppThemeMode::Dark);
        assert_eq!(settings.view_mode, ViewMode::Details);
    }

    #[test]
    fn test_toggle_hidden() {
        let mut settings = GlobalSettings::default();
        assert!(!settings.show_hidden());
        
        settings.toggle_show_hidden();
        assert!(settings.show_hidden());
        
        settings.toggle_show_hidden();
        assert!(!settings.show_hidden());
    }

    #[test]
    fn test_toggle_sort_order() {
        let mut settings = GlobalSettings::default();
        assert_eq!(settings.sort_order(), SortOrder::Ascending);
        
        settings.toggle_sort_order();
        assert_eq!(settings.sort_order(), SortOrder::Descending);
        
        settings.toggle_sort_order();
        assert_eq!(settings.sort_order(), SortOrder::Ascending);
    }

    #[test]
    fn test_toggle_view_mode() {
        let mut settings = GlobalSettings::default();
        assert_eq!(settings.view_mode(), ViewMode::Details);
        
        settings.toggle_view_mode();
        assert_eq!(settings.view_mode(), ViewMode::Grid);
        
        settings.toggle_view_mode();
        assert_eq!(settings.view_mode(), ViewMode::List);
    }

    #[test]
    fn test_grid_config_default() {
        let config = GridConfig::default();
        assert_eq!(config.icon_size, 64.0);
        assert_eq!(config.item_width, 120.0);
        assert_eq!(config.item_height, 100.0);
        assert_eq!(config.gap, 16.0);
        assert_eq!(config.min_columns, 2);
    }

    #[test]
    fn test_grid_columns_calculation() {
        let config = GridConfig::default();
        
        // With 120px items and 16px gap, effective width is 136px
        // 400px viewport: (400 + 16) / 136 = 3.05 -> 3 columns
        assert_eq!(config.columns_for_width(400.0), 3);
        
        // 800px viewport: (800 + 16) / 136 = 6.0 -> 6 columns
        assert_eq!(config.columns_for_width(800.0), 6);
        
        // Very small viewport should return min_columns
        assert_eq!(config.columns_for_width(50.0), 2);
        
        // Zero or negative viewport should return min_columns
        assert_eq!(config.columns_for_width(0.0), 2);
        assert_eq!(config.columns_for_width(-100.0), 2);
    }

    #[test]
    fn test_grid_rows_calculation() {
        let config = GridConfig::default();
        
        // 10 items with 3 columns = 4 rows (3+3+3+1)
        assert_eq!(config.rows_for_items(10, 400.0), 4);
        
        // 6 items with 3 columns = 2 rows
        assert_eq!(config.rows_for_items(6, 400.0), 2);
        
        // 0 items = 0 rows
        assert_eq!(config.rows_for_items(0, 400.0), 0);
    }

    #[test]
    fn test_grid_position_for_index() {
        let config = GridConfig::default();
        
        // With 3 columns (400px viewport)
        assert_eq!(config.position_for_index(0, 400.0), (0, 0));
        assert_eq!(config.position_for_index(1, 400.0), (1, 0));
        assert_eq!(config.position_for_index(2, 400.0), (2, 0));
        assert_eq!(config.position_for_index(3, 400.0), (0, 1));
        assert_eq!(config.position_for_index(5, 400.0), (2, 1));
    }

    #[test]
    fn test_grid_content_height() {
        let config = GridConfig::default();
        
        // 10 items with 3 columns = 4 rows
        // Height = 4 * 100 + 3 * 16 = 448
        assert_eq!(config.content_height(10, 400.0), 448.0);
        
        // 0 items = 0 height
        assert_eq!(config.content_height(0, 400.0), 0.0);
    }

    fn arb_grid_config() -> impl Strategy<Value = GridConfig> {
        (
            16.0f32..128.0,   // icon_size
            60.0f32..200.0,   // item_width
            80.0f32..200.0,   // item_height
            4.0f32..32.0,     // gap
            1usize..5,        // min_columns (keep small to ensure valid configs)
        )
            .prop_map(|(icon_size, item_width, item_height, gap, min_columns)| {
                GridConfig {
                    icon_size,
                    item_width,
                    item_height,
                    gap,
                    min_columns,
                }
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: ui-enhancements, Property 17: Grid Columns Calculation**
        /// **Validates: Requirements 5.3**
        ///
        /// *For any* GridConfig and viewport width large enough to fit min_columns,
        /// the calculated number of columns SHALL be at least min_columns and 
        /// SHALL maximize the number of columns that fit within the viewport.
        #[test]
        fn prop_grid_columns_calculation(
            config in arb_grid_config(),
            viewport_width in 400.0f32..2000.0,
        ) {
            // Only test when viewport can fit at least min_columns
            let min_required_width = config.min_columns as f32 * config.item_width 
                + (config.min_columns.saturating_sub(1)) as f32 * config.gap;
            
            prop_assume!(viewport_width >= min_required_width);
            
            let columns = config.columns_for_width(viewport_width);
            
            // Property 1: Columns should be at least min_columns
            prop_assert!(
                columns >= config.min_columns,
                "Columns {} should be >= min_columns {}",
                columns, config.min_columns
            );
            
            // Property 2: The calculated columns should fit within viewport
            let total_width = columns as f32 * config.item_width 
                + (columns.saturating_sub(1)) as f32 * config.gap;
            
            prop_assert!(
                total_width <= viewport_width + 1.0,
                "Total width {} should fit in viewport {}",
                total_width, viewport_width
            );
            
            // Property 3: Adding one more column should exceed viewport (greedy fit)
            let extra_column_width = (columns + 1) as f32 * config.item_width 
                + columns as f32 * config.gap;
            prop_assert!(
                extra_column_width > viewport_width,
                "Adding one more column ({} width) should exceed viewport {}",
                extra_column_width, viewport_width
            );
        }

        /// **Feature: ui-enhancements, Property 17b: Grid Position Index Round-Trip**
        /// **Validates: Requirements 5.3**
        ///
        /// *For any* valid index, converting to position and back to index SHALL return
        /// the original index.
        #[test]
        fn prop_grid_position_index_round_trip(
            config in arb_grid_config(),
            viewport_width in 100.0f32..2000.0,
            index in 0usize..1000,
        ) {
            let (col, row) = config.position_for_index(index, viewport_width);
            let recovered_index = config.index_for_position(col, row, viewport_width);
            
            prop_assert_eq!(
                index, recovered_index,
                "Index {} -> position ({}, {}) -> index {} should round-trip",
                index, col, row, recovered_index
            );
        }

        /// **Feature: ui-enhancements, Property 17c: Grid Rows Consistency**
        /// **Validates: Requirements 5.3**
        ///
        /// *For any* item count and viewport width, the number of rows times columns
        /// SHALL be >= item_count (all items fit).
        #[test]
        fn prop_grid_rows_consistency(
            config in arb_grid_config(),
            viewport_width in 100.0f32..2000.0,
            item_count in 0usize..500,
        ) {
            let columns = config.columns_for_width(viewport_width);
            let rows = config.rows_for_items(item_count, viewport_width);
            
            // All items should fit in the grid
            let capacity = rows * columns;
            prop_assert!(
                capacity >= item_count,
                "Grid capacity {} ({}x{}) should fit {} items",
                capacity, rows, columns, item_count
            );
            
            // But we shouldn't have more than one extra row
            if item_count > 0 && rows > 0 {
                let min_rows_needed = (item_count + columns - 1) / columns;
                prop_assert_eq!(
                    rows, min_rows_needed,
                    "Rows {} should equal minimum needed {}",
                    rows, min_rows_needed
                );
            }
        }
    }
}
