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
    
    /// Application theme
    pub theme: Theme,
    
    /// Whether to show file extensions
    pub show_extensions: bool,
    
    /// Whether to show file sizes
    pub show_sizes: bool,
    
    /// Whether to show modification dates
    pub show_dates: bool,
    
    /// Default view mode
    pub view_mode: ViewMode,
}

/// Application theme options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
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

    /// Returns the current theme.
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// Sets the theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Returns the current view mode.
    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Sets the view mode.
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
    }
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            show_hidden_files: false,
            sort_key: SortKey::Name,
            sort_order: SortOrder::Ascending,
            theme: Theme::Dark,
            show_extensions: true,
            show_sizes: true,
            show_dates: true,
            view_mode: ViewMode::Details,
        }
    }
}

impl Global for GlobalSettings {}

impl Default for Theme {
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

    #[test]
    fn test_default_settings() {
        let settings = GlobalSettings::default();
        assert!(!settings.show_hidden_files);
        assert_eq!(settings.sort_key, SortKey::Name);
        assert_eq!(settings.sort_order, SortOrder::Ascending);
        assert_eq!(settings.theme, Theme::Dark);
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
}
