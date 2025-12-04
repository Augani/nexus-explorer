use std::path::Path;

use gpui::{
    div, prelude::*, px, svg, App, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, Styled, Window,
};

use crate::models::{FileEntry, ViewMode, theme_colors};

/// State for the status bar display
#[derive(Debug, Clone)]
pub struct StatusBarState {
    pub total_items: usize,
    pub selected_count: usize,
    pub selected_size: u64,
    pub view_mode: ViewMode,
    pub git_branch: Option<String>,
    pub is_loading: bool,
    pub is_terminal_open: bool,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            total_items: 0,
            selected_count: 0,
            selected_size: 0,
            view_mode: ViewMode::List,
            git_branch: None,
            is_loading: false,
            is_terminal_open: false,
        }
    }
}

impl StatusBarState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update state from file list entries and selection
    pub fn update_from_entries(&mut self, entries: &[FileEntry], selected_indices: &[usize]) {
        self.total_items = entries.len();
        self.selected_count = selected_indices.len();
        self.selected_size = selected_indices
            .iter()
            .filter_map(|&idx| entries.get(idx))
            .map(|e| e.size)
            .sum();
    }

    /// Update state from file list with single selection
    pub fn update_from_file_list(&mut self, entries: &[FileEntry], selected_index: Option<usize>) {
        self.total_items = entries.len();
        if let Some(idx) = selected_index {
            self.selected_count = 1;
            self.selected_size = entries.get(idx).map(|e| e.size).unwrap_or(0);
        } else {
            self.selected_count = 0;
            self.selected_size = 0;
        }
    }

    /// Detect git branch from the given directory path
    pub fn detect_git_branch(&mut self, path: &Path) {
        self.git_branch = detect_git_branch(path);
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
    }

    pub fn set_terminal_open(&mut self, is_open: bool) {
        self.is_terminal_open = is_open;
    }

    pub fn set_loading(&mut self, is_loading: bool) {
        self.is_loading = is_loading;
    }
}

/// Detect the current git branch for a directory
pub fn detect_git_branch(path: &Path) -> Option<String> {
    // Walk up the directory tree to find .git
    let mut current = Some(path);
    
    while let Some(dir) = current {
        let git_dir = dir.join(".git");
        
        if git_dir.is_dir() {
            // Found .git directory, read HEAD
            let head_path = git_dir.join("HEAD");
            if let Ok(content) = std::fs::read_to_string(&head_path) {
                let content = content.trim();
                // HEAD format: "ref: refs/heads/branch-name" or a commit hash
                if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
                    return Some(branch.to_string());
                } else if content.len() >= 7 {
                    return Some(content[..7].to_string());
                }
            }
            return None;
        } else if git_dir.is_file() {
            // .git file (worktree) - read the gitdir path
            if let Ok(content) = std::fs::read_to_string(&git_dir) {
                if let Some(gitdir) = content.trim().strip_prefix("gitdir: ") {
                    let head_path = Path::new(gitdir).join("HEAD");
                    if let Ok(head_content) = std::fs::read_to_string(&head_path) {
                        let head_content = head_content.trim();
                        if let Some(branch) = head_content.strip_prefix("ref: refs/heads/") {
                            return Some(branch.to_string());
                        }
                    }
                }
            }
        }
        
        current = dir.parent();
    }
    
    None
}

/// Format file size for display
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size >= TB {
        format!("{:.1} TB", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// Actions that can be triggered from the status bar
#[derive(Debug, Clone, PartialEq)]
pub enum StatusBarAction {
    ToggleTerminal,
    ToggleViewMode,
}

/// Status bar view component
pub struct StatusBarView {
    state: StatusBarState,
    focus_handle: FocusHandle,
    pending_action: Option<StatusBarAction>,
}

impl StatusBarView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            state: StatusBarState::new(),
            focus_handle: cx.focus_handle(),
            pending_action: None,
        }
    }

    pub fn state(&self) -> &StatusBarState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut StatusBarState {
        &mut self.state
    }

    /// Take any pending action (for parent to handle)
    pub fn take_pending_action(&mut self) -> Option<StatusBarAction> {
        self.pending_action.take()
    }

    /// Update the status bar state
    pub fn update_state(&mut self, state: StatusBarState, cx: &mut Context<Self>) {
        self.state = state;
        cx.notify();
    }

    /// Update from file entries
    pub fn update_from_entries(&mut self, entries: &[FileEntry], selected_index: Option<usize>, cx: &mut Context<Self>) {
        self.state.update_from_file_list(entries, selected_index);
        cx.notify();
    }

    /// Set the current directory for git detection
    pub fn set_current_directory(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.state.detect_git_branch(path);
        cx.notify();
    }

    /// Set view mode
    pub fn set_view_mode(&mut self, mode: ViewMode, cx: &mut Context<Self>) {
        self.state.set_view_mode(mode);
        cx.notify();
    }

    /// Set terminal open state
    pub fn set_terminal_open(&mut self, is_open: bool, cx: &mut Context<Self>) {
        self.state.set_terminal_open(is_open);
        cx.notify();
    }
}

impl Focusable for StatusBarView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StatusBarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        let bg_color = theme.bg_secondary;
        let border_color = theme.border_default;
        let text_primary = theme.text_primary;
        let text_muted = theme.text_muted;
        let accent = theme.accent_primary;
        let hover_bg = theme.bg_hover;

        let total_items = self.state.total_items;
        let selected_count = self.state.selected_count;
        let selected_size = self.state.selected_size;
        let view_mode = self.state.view_mode;
        let git_branch = self.state.git_branch.clone();
        let is_terminal_open = self.state.is_terminal_open;

        div()
            .id("status-bar")
            .h(px(28.0))
            .w_full()
            .bg(bg_color)
            .border_t_1()
            .border_color(border_color)
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .text_xs()
            .child(
                // Left section: Item counts
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_color(text_muted)
                            .child(
                                svg()
                                    .path("assets/icons/files.svg")
                                    .size(px(12.0))
                                    .text_color(text_muted),
                            )
                            .child(format!("{} items", total_items))
                    )
                    .when(selected_count > 0, |el| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_color(text_primary)
                                .child(
                                    div()
                                        .h(px(12.0))
                                        .w(px(1.0))
                                        .bg(border_color)
                                        .mx_1(),
                                )
                                .child(format!("{} selected", selected_count))
                                .when(selected_size > 0, |el| {
                                    el.child(
                                        div()
                                            .text_color(text_muted)
                                            .child(format!("({})", format_size(selected_size)))
                                    )
                                })
                        )
                    })
            )
            .child(
                // Right section: Actions and git branch
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    // Git branch (if available)
                    .when_some(git_branch, |el, branch| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_color(text_muted)
                                .child(
                                    svg()
                                        .path("assets/icons/folder-git.svg")
                                        .size(px(12.0))
                                        .text_color(accent),
                                )
                                .child(branch)
                        )
                        .child(
                            div()
                                .h(px(12.0))
                                .w(px(1.0))
                                .bg(border_color)
                                .mx_1(),
                        )
                    })
                    // Terminal toggle
                    .child(
                        div()
                            .id("status-terminal-toggle")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_1p5()
                            .py_0p5()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg))
                            .when(is_terminal_open, |s| s.bg(hover_bg))
                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                                view.pending_action = Some(StatusBarAction::ToggleTerminal);
                                cx.notify();
                            }))
                            .child(
                                svg()
                                    .path("assets/icons/terminal.svg")
                                    .size(px(12.0))
                                    .text_color(if is_terminal_open { accent } else { text_muted }),
                            )
                            .child(
                                div()
                                    .text_color(if is_terminal_open { text_primary } else { text_muted })
                                    .child("Terminal")
                            )
                    )
                    .child(
                        div()
                            .h(px(12.0))
                            .w(px(1.0))
                            .bg(border_color),
                    )
                    // View mode toggle
                    .child(
                        div()
                            .id("status-view-toggle")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_1p5()
                            .py_0p5()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg))
                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                                view.pending_action = Some(StatusBarAction::ToggleViewMode);
                                cx.notify();
                            }))
                            .child(
                                svg()
                                    .path(match view_mode {
                                        ViewMode::List | ViewMode::Details => "assets/icons/list.svg",
                                        ViewMode::Grid => "assets/icons/grid-2x2.svg",
                                    })
                                    .size(px(12.0))
                                    .text_color(text_muted),
                            )
                            .child(
                                div()
                                    .text_color(text_muted)
                                    .child(match view_mode {
                                        ViewMode::List | ViewMode::Details => "List",
                                        ViewMode::Grid => "Grid",
                                    })
                            )
                    )
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use crate::models::{FileType, IconKey};
    use proptest::prelude::*;

    fn create_test_entry(name: &str, is_dir: bool, size: u64) -> FileEntry {
        FileEntry {
            name: name.to_string(),
            path: std::path::PathBuf::from(name),
            is_dir,
            size,
            modified: SystemTime::now(),
            file_type: if is_dir { FileType::Directory } else { FileType::RegularFile },
            icon_key: if is_dir { IconKey::Directory } else { IconKey::GenericFile },
            linux_permissions: None,
            sync_status: crate::models::CloudSyncStatus::None,
        }
    }

    #[test]
    fn test_status_bar_state_default() {
        let state = StatusBarState::default();
        assert_eq!(state.total_items, 0);
        assert_eq!(state.selected_count, 0);
        assert_eq!(state.selected_size, 0);
        assert_eq!(state.view_mode, ViewMode::List);
        assert!(state.git_branch.is_none());
        assert!(!state.is_loading);
        assert!(!state.is_terminal_open);
    }

    #[test]
    fn test_update_from_file_list_no_selection() {
        let mut state = StatusBarState::new();
        let entries = vec![
            create_test_entry("file1.txt", false, 100),
            create_test_entry("file2.txt", false, 200),
            create_test_entry("folder", true, 0),
        ];

        state.update_from_file_list(&entries, None);

        assert_eq!(state.total_items, 3);
        assert_eq!(state.selected_count, 0);
        assert_eq!(state.selected_size, 0);
    }

    #[test]
    fn test_update_from_file_list_with_selection() {
        let mut state = StatusBarState::new();
        let entries = vec![
            create_test_entry("file1.txt", false, 100),
            create_test_entry("file2.txt", false, 200),
            create_test_entry("folder", true, 0),
        ];

        state.update_from_file_list(&entries, Some(1));

        assert_eq!(state.total_items, 3);
        assert_eq!(state.selected_count, 1);
        assert_eq!(state.selected_size, 200);
    }

    #[test]
    fn test_update_from_entries_multiple_selection() {
        let mut state = StatusBarState::new();
        let entries = vec![
            create_test_entry("file1.txt", false, 100),
            create_test_entry("file2.txt", false, 200),
            create_test_entry("file3.txt", false, 300),
        ];

        state.update_from_entries(&entries, &[0, 2]);

        assert_eq!(state.total_items, 3);
        assert_eq!(state.selected_count, 2);
        assert_eq!(state.selected_size, 400);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
        assert_eq!(format_size(1099511627776), "1.0 TB");
    }

    #[test]
    fn test_set_view_mode() {
        let mut state = StatusBarState::new();
        assert_eq!(state.view_mode, ViewMode::List);

        state.set_view_mode(ViewMode::Grid);
        assert_eq!(state.view_mode, ViewMode::Grid);

        state.set_view_mode(ViewMode::Details);
        assert_eq!(state.view_mode, ViewMode::Details);
    }

    #[test]
    fn test_set_terminal_open() {
        let mut state = StatusBarState::new();
        assert!(!state.is_terminal_open);

        state.set_terminal_open(true);
        assert!(state.is_terminal_open);

        state.set_terminal_open(false);
        assert!(!state.is_terminal_open);
    }

    #[test]
    fn test_detect_git_branch_in_git_repo() {
        // Test with current directory (which should be a git repo)
        let current_dir = std::env::current_dir().unwrap();
        let branch = detect_git_branch(&current_dir);
        
        // We expect to find a branch since we're in a git repo
        // The branch name should be non-empty if found
        if let Some(ref b) = branch {
            assert!(!b.is_empty(), "Branch name should not be empty");
        }
        // Note: branch could be None if not in a git repo, which is also valid
    }

    #[test]
    fn test_detect_git_branch_non_git_dir() {
        let temp_dir = std::env::temp_dir();
        let branch = detect_git_branch(&temp_dir);
        
        // Temp dir is unlikely to be a git repo (unless nested in one)
        // This test mainly verifies the function doesn't panic
        let _ = branch;
    }

    /// Generate arbitrary file entries for property testing
    fn arb_file_entry() -> impl Strategy<Value = FileEntry> {
        (
            "[a-zA-Z0-9_]{1,20}",
            prop::bool::ANY,
            0u64..10_000_000_000,
        )
            .prop_map(|(name, is_dir, size)| {
                let actual_size = if is_dir { 0 } else { size };
                create_test_entry(&name, is_dir, actual_size)
            })
    }

    /// Generate a vector of file entries
    fn arb_entries() -> impl Strategy<Value = Vec<FileEntry>> {
        prop::collection::vec(arb_file_entry(), 0..100)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: ui-enhancements, Property 31: Status Bar Item Count**
        /// **Validates: Requirements 10.2**
        ///
        /// *For any* list of file entries, when the status bar state is updated,
        /// the total_items count SHALL equal the number of entries in the list.
        #[test]
        fn prop_status_bar_item_count(entries in arb_entries()) {
            let mut state = StatusBarState::new();
            state.update_from_file_list(&entries, None);
            
            prop_assert_eq!(
                state.total_items, 
                entries.len(),
                "Status bar total_items {} should equal entries count {}",
                state.total_items, entries.len()
            );
        }

        /// **Feature: ui-enhancements, Property 31b: Status Bar Selected Count**
        /// **Validates: Requirements 10.2**
        ///
        /// *For any* list of file entries and valid selection index,
        /// the selected_count SHALL be 1 when an item is selected, 0 otherwise.
        #[test]
        fn prop_status_bar_selected_count(
            entries in arb_entries(),
            selection_offset in 0usize..100,
        ) {
            let mut state = StatusBarState::new();
            
            // Test with no selection
            state.update_from_file_list(&entries, None);
            prop_assert_eq!(state.selected_count, 0, "No selection should have count 0");
            
            // Test with valid selection
            if !entries.is_empty() {
                let valid_index = selection_offset % entries.len();
                state.update_from_file_list(&entries, Some(valid_index));
                prop_assert_eq!(
                    state.selected_count, 1,
                    "Single selection should have count 1"
                );
            }
        }

        /// **Feature: ui-enhancements, Property 31c: Status Bar Multiple Selection Count**
        /// **Validates: Requirements 10.2**
        ///
        /// *For any* list of file entries and selection indices,
        /// the selected_count SHALL equal the number of valid selected indices.
        #[test]
        fn prop_status_bar_multiple_selection_count(
            entries in arb_entries(),
            selection_indices in prop::collection::vec(0usize..100, 0..20),
        ) {
            let mut state = StatusBarState::new();
            
            // Filter to valid indices and deduplicate
            let valid_indices: Vec<usize> = selection_indices
                .into_iter()
                .filter(|&idx| idx < entries.len())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            
            state.update_from_entries(&entries, &valid_indices);
            
            prop_assert_eq!(
                state.selected_count, 
                valid_indices.len(),
                "Selected count {} should equal valid indices count {}",
                state.selected_count, valid_indices.len()
            );
        }

        /// **Feature: ui-enhancements, Property 32: Status Bar Selection Size**
        /// **Validates: Requirements 10.3**
        ///
        /// *For any* list of file entries and selection indices,
        /// the selected_size SHALL equal the sum of sizes of all selected entries.
        #[test]
        fn prop_status_bar_selection_size(
            entries in arb_entries(),
            selection_indices in prop::collection::vec(0usize..100, 0..20),
        ) {
            let mut state = StatusBarState::new();
            
            // Filter to valid indices and deduplicate
            let valid_indices: Vec<usize> = selection_indices
                .into_iter()
                .filter(|&idx| idx < entries.len())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            
            state.update_from_entries(&entries, &valid_indices);
            
            // Calculate expected size
            let expected_size: u64 = valid_indices
                .iter()
                .filter_map(|&idx| entries.get(idx))
                .map(|e| e.size)
                .sum();
            
            prop_assert_eq!(
                state.selected_size, 
                expected_size,
                "Selected size {} should equal sum of selected entry sizes {}",
                state.selected_size, expected_size
            );
        }

        /// **Feature: ui-enhancements, Property 32b: Status Bar Single Selection Size**
        /// **Validates: Requirements 10.3**
        ///
        /// *For any* list of file entries and valid selection index,
        /// the selected_size SHALL equal the size of the selected entry.
        #[test]
        fn prop_status_bar_single_selection_size(
            entries in arb_entries(),
            selection_offset in 0usize..100,
        ) {
            let mut state = StatusBarState::new();
            
            // Test with no selection
            state.update_from_file_list(&entries, None);
            prop_assert_eq!(state.selected_size, 0, "No selection should have size 0");
            
            // Test with valid selection
            if !entries.is_empty() {
                let valid_index = selection_offset % entries.len();
                state.update_from_file_list(&entries, Some(valid_index));
                
                let expected_size = entries[valid_index].size;
                prop_assert_eq!(
                    state.selected_size, 
                    expected_size,
                    "Single selection size {} should equal entry size {}",
                    state.selected_size, expected_size
                );
            }
        }
    }
}
