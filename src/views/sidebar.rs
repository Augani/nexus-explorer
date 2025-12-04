use std::path::PathBuf;

use gpui::{
    div, prelude::*, px, svg, App, Context, DragMoveEvent, ExternalPaths, FocusHandle,
    Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement, Render, SharedString,
    Styled, Window,
};

use crate::models::{
    Bookmark, BookmarkId, BookmarkManager, CloudStorageManager, Device, DeviceMonitor,
    DeviceType, Favorite, Favorites, NetworkLocationId, NetworkSidebarState, NetworkStorageManager,
    WslDistribution, theme_colors, sidebar as sidebar_spacing,
    SmartFolder, SmartFolderId, SmartFolderManager, SearchQuery,
};

#[derive(Clone)]
pub struct SidebarItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub depth: usize,
    pub is_expanded: bool,
    pub children: Vec<SidebarItem>,
}

impl SidebarItem {
    pub fn new(name: String, path: PathBuf, is_dir: bool, depth: usize) -> Self {
        Self {
            name,
            path,
            is_dir,
            depth,
            is_expanded: false,
            children: Vec::new(),
        }
    }
}

/// Data transferred during drag operations
#[derive(Clone)]
pub struct DraggedFolder {
    pub path: PathBuf,
    pub name: String,
}

/// View for rendering dragged folder
pub struct DraggedFolderView {
    pub name: String,
}

impl Render for DraggedFolderView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        div()
            .px_2()
            .py_1()
            .bg(theme.bg_hover)
            .rounded_md()
            .text_sm()
            .text_color(theme.text_primary)
            .child(self.name.clone())
    }
}

/// Actions that can be triggered from the Tools section
#[derive(Clone, Debug, PartialEq)]
pub enum ToolAction {
    NewFile,
    NewFolder,
    CopyPath,
    Refresh,
    OpenTerminalHere,
    ToggleHiddenFiles,
    SetAsDefault,
    Copy,
    Move,
    Paste,
    Delete,
}

pub struct Sidebar {
    favorites: Favorites,
    bookmarks: BookmarkManager,
    smart_folders: SmartFolderManager,
    workspace_root: Option<SidebarItem>,
    selected_path: Option<PathBuf>,
    is_drop_target: bool,
    is_tools_expanded: bool,
    is_bookmarks_expanded: bool,
    is_network_expanded: bool,
    is_devices_expanded: bool,
    is_smart_folders_expanded: bool,
    show_hidden_files: bool,
    current_directory: Option<PathBuf>,
    network_manager: NetworkStorageManager,
    cloud_manager: CloudStorageManager,
    device_monitor: DeviceMonitor,
}

impl Sidebar {
    pub fn new() -> Self {
        let favorites = Favorites::load().unwrap_or_else(|_| {
            let mut favs = Favorites::new();
            // Add default favorites
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            let _ = favs.add(home.clone());
            let _ = favs.add(home.join("Desktop"));
            let _ = favs.add(home.join("Documents"));
            let _ = favs.add(home.join("Downloads"));
            favs
        });

        let bookmarks = BookmarkManager::load().unwrap_or_else(|_| BookmarkManager::new());

        let network_manager = NetworkStorageManager::load();

        // Detect cloud storage providers
        let mut cloud_manager = CloudStorageManager::new();
        cloud_manager.detect_all_providers();

        let mut device_monitor = DeviceMonitor::new();
        device_monitor.start_monitoring();

        let smart_folders = SmartFolderManager::load().unwrap_or_else(|_| SmartFolderManager::new());

        Self {
            favorites,
            bookmarks,
            smart_folders,
            workspace_root: None,
            selected_path: None,
            is_drop_target: false,
            is_tools_expanded: true,
            is_bookmarks_expanded: true,
            is_network_expanded: true,
            is_devices_expanded: true,
            is_smart_folders_expanded: true,
            show_hidden_files: false,
            current_directory: None,
            network_manager,
            cloud_manager,
            device_monitor,
        }
    }

    pub fn is_tools_expanded(&self) -> bool {
        self.is_tools_expanded
    }

    pub fn toggle_tools_expanded(&mut self) {
        self.is_tools_expanded = !self.is_tools_expanded;
    }

    pub fn is_bookmarks_expanded(&self) -> bool {
        self.is_bookmarks_expanded
    }

    pub fn toggle_bookmarks_expanded(&mut self) {
        self.is_bookmarks_expanded = !self.is_bookmarks_expanded;
    }

    pub fn bookmarks(&self) -> &BookmarkManager {
        &self.bookmarks
    }

    pub fn bookmarks_mut(&mut self) -> &mut BookmarkManager {
        &mut self.bookmarks
    }

    pub fn add_bookmark(&mut self, path: PathBuf) -> Result<BookmarkId, crate::models::BookmarkError> {
        let result = self.bookmarks.add(path);
        if result.is_ok() {
            let _ = self.bookmarks.save();
        }
        result
    }

    pub fn remove_bookmark(&mut self, id: BookmarkId) -> Result<Bookmark, crate::models::BookmarkError> {
        let result = self.bookmarks.remove(id);
        if result.is_ok() {
            let _ = self.bookmarks.save();
        }
        result
    }

    pub fn rename_bookmark(&mut self, id: BookmarkId, name: String) -> Result<(), crate::models::BookmarkError> {
        let result = self.bookmarks.rename(id, name);
        if result.is_ok() {
            let _ = self.bookmarks.save();
        }
        result
    }

    pub fn show_hidden_files(&self) -> bool {
        self.show_hidden_files
    }

    pub fn set_show_hidden_files(&mut self, show: bool) {
        self.show_hidden_files = show;
    }

    pub fn toggle_hidden_files(&mut self) {
        self.show_hidden_files = !self.show_hidden_files;
    }

    pub fn current_directory(&self) -> Option<&PathBuf> {
        self.current_directory.as_ref()
    }

    pub fn set_current_directory(&mut self, path: PathBuf) {
        self.current_directory = Some(path);
    }

    pub fn set_workspace_root(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Root")
            .to_string();
        self.workspace_root = Some(SidebarItem::new(name, path, true, 0));
    }

    pub fn set_selected_path(&mut self, path: PathBuf) {
        self.selected_path = Some(path);
    }

    pub fn favorites(&self) -> &Favorites {
        &self.favorites
    }

    pub fn favorites_mut(&mut self) -> &mut Favorites {
        &mut self.favorites
    }

    pub fn add_favorite(&mut self, path: PathBuf) -> Result<(), crate::models::FavoritesError> {
        let result = self.favorites.add(path);
        if result.is_ok() {
            let _ = self.favorites.save();
        }
        result
    }

    pub fn remove_favorite(&mut self, index: usize) -> Result<Favorite, crate::models::FavoritesError> {
        let result = self.favorites.remove(index);
        if result.is_ok() {
            let _ = self.favorites.save();
        }
        result
    }

    pub fn reorder_favorites(&mut self, from: usize, to: usize) -> Result<(), crate::models::FavoritesError> {
        let result = self.favorites.reorder(from, to);
        if result.is_ok() {
            let _ = self.favorites.save();
        }
        result
    }

    pub fn set_drop_target(&mut self, is_target: bool) {
        self.is_drop_target = is_target;
    }

    pub fn is_network_expanded(&self) -> bool {
        self.is_network_expanded
    }

    pub fn toggle_network_expanded(&mut self) {
        self.is_network_expanded = !self.is_network_expanded;
    }

    pub fn network_manager(&self) -> &NetworkStorageManager {
        &self.network_manager
    }

    pub fn network_manager_mut(&mut self) -> &mut NetworkStorageManager {
        &mut self.network_manager
    }

    pub fn cloud_manager(&self) -> &CloudStorageManager {
        &self.cloud_manager
    }

    pub fn cloud_manager_mut(&mut self) -> &mut CloudStorageManager {
        &mut self.cloud_manager
    }

    pub fn refresh_cloud_providers(&mut self) {
        self.cloud_manager.detect_all_providers();
    }

    pub fn get_network_sidebar_state(&self) -> NetworkSidebarState {
        NetworkSidebarState::from_managers(&self.network_manager, &self.cloud_manager)
    }

    pub fn is_devices_expanded(&self) -> bool {
        self.is_devices_expanded
    }

    pub fn toggle_devices_expanded(&mut self) {
        self.is_devices_expanded = !self.is_devices_expanded;
    }

    pub fn device_monitor(&self) -> &DeviceMonitor {
        &self.device_monitor
    }

    pub fn device_monitor_mut(&mut self) -> &mut DeviceMonitor {
        &mut self.device_monitor
    }

    pub fn refresh_devices(&mut self) {
        self.device_monitor.enumerate_devices();
        self.device_monitor.refresh_space_info();
    }

    pub fn devices(&self) -> &[Device] {
        self.device_monitor.devices()
    }

    pub fn wsl_distributions(&self) -> &[WslDistribution] {
        self.device_monitor.wsl_distributions()
    }

    pub fn is_smart_folders_expanded(&self) -> bool {
        self.is_smart_folders_expanded
    }

    pub fn toggle_smart_folders_expanded(&mut self) {
        self.is_smart_folders_expanded = !self.is_smart_folders_expanded;
    }

    pub fn smart_folders(&self) -> &SmartFolderManager {
        &self.smart_folders
    }

    pub fn smart_folders_mut(&mut self) -> &mut SmartFolderManager {
        &mut self.smart_folders
    }

    pub fn create_smart_folder(&mut self, name: String, query: SearchQuery) -> Result<SmartFolderId, crate::models::SmartFolderError> {
        let result = self.smart_folders.create(name, query);
        if result.is_ok() {
            let _ = self.smart_folders.save();
        }
        result
    }

    pub fn delete_smart_folder(&mut self, id: SmartFolderId) -> Result<SmartFolder, crate::models::SmartFolderError> {
        let result = self.smart_folders.delete(id);
        if result.is_ok() {
            let _ = self.smart_folders.save();
        }
        result
    }

    pub fn update_smart_folder(&mut self, id: SmartFolderId, query: SearchQuery) -> Result<(), crate::models::SmartFolderError> {
        let result = self.smart_folders.update(id, query);
        if result.is_ok() {
            let _ = self.smart_folders.save();
        }
        result
    }
}

pub struct SidebarView {
    sidebar: Sidebar,
    focus_handle: FocusHandle,
    dragging_favorite_index: Option<usize>,
    drop_target_index: Option<usize>,
    pending_navigation: Option<PathBuf>,
    pending_action: Option<ToolAction>,
    selected_file_count: usize,
    has_clipboard: bool,
    context_menu_bookmark_id: Option<BookmarkId>,
    show_network_dialog: bool,
    show_smart_folder_dialog: bool,
    editing_smart_folder: Option<SmartFolderId>,
    pending_smart_folder_click: Option<SmartFolderId>,
}

impl SidebarView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            sidebar: Sidebar::new(),
            focus_handle: cx.focus_handle(),
            dragging_favorite_index: None,
            drop_target_index: None,
            pending_navigation: None,
            pending_action: None,
            selected_file_count: 0,
            has_clipboard: false,
            context_menu_bookmark_id: None,
            show_network_dialog: false,
            show_smart_folder_dialog: false,
            editing_smart_folder: None,
            pending_smart_folder_click: None,
        }
    }

    fn get_trash_path() -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir()
                .map(|h| h.join(".Trash"))
                .unwrap_or_else(|| PathBuf::from("/.Trash"))
        }
        #[cfg(target_os = "linux")]
        {
            dirs::data_local_dir()
                .map(|d| d.join("Trash/files"))
                .unwrap_or_else(|| {
                    dirs::home_dir()
                        .map(|h| h.join(".local/share/Trash/files"))
                        .unwrap_or_else(|| PathBuf::from("/tmp"))
                })
        }
        #[cfg(target_os = "windows")]
        {
            PathBuf::from("C:\\$Recycle.Bin")
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            PathBuf::from("/tmp")
        }
    }

    /// Set whether there's content in the clipboard for paste
    pub fn set_has_clipboard(&mut self, has_clipboard: bool) {
        self.has_clipboard = has_clipboard;
    }

    /// Check if there's content in the clipboard
    pub fn has_clipboard(&self) -> bool {
        self.has_clipboard
    }

    /// Add a bookmark for the current directory
    pub fn add_bookmark_for_current(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.sidebar.current_directory.clone() {
            let _ = self.sidebar.add_bookmark(path);
            cx.notify();
        }
    }

    /// Handle bookmark click for navigation
    fn handle_bookmark_click(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar.selected_path = Some(path.clone());
        self.pending_navigation = Some(path);
        cx.notify();
    }

    /// Handle bookmark removal
    fn handle_bookmark_remove(&mut self, id: BookmarkId, cx: &mut Context<Self>) {
        let _ = self.sidebar.remove_bookmark(id);
        cx.notify();
    }

    /// Toggle bookmarks section
    fn toggle_bookmarks_section(&mut self, cx: &mut Context<Self>) {
        self.sidebar.toggle_bookmarks_expanded();
        cx.notify();
    }

    /// Take the pending navigation path (if any)
    pub fn take_pending_navigation(&mut self) -> Option<PathBuf> {
        self.pending_navigation.take()
    }

    /// Take the pending tool action (if any)
    pub fn take_pending_action(&mut self) -> Option<ToolAction> {
        self.pending_action.take()
    }

    /// Set the current directory for tools context
    pub fn set_current_directory(&mut self, path: PathBuf) {
        self.sidebar.set_current_directory(path);
    }

    /// Set the number of selected files (for enabling batch operations)
    pub fn set_selected_file_count(&mut self, count: usize) {
        self.selected_file_count = count;
    }

    /// Get whether hidden files should be shown
    pub fn show_hidden_files(&self) -> bool {
        self.sidebar.show_hidden_files()
    }

    /// Toggle hidden files visibility
    pub fn toggle_hidden_files(&mut self, cx: &mut Context<Self>) {
        self.sidebar.toggle_hidden_files();
        self.pending_action = Some(ToolAction::ToggleHiddenFiles);
        cx.notify();
    }

    /// Toggle default browser setting
    pub fn toggle_default_browser(&mut self, cx: &mut Context<Self>) {
        let is_default = crate::models::is_default_file_browser();
        if is_default {
            let _ = crate::models::restore_default_file_browser();
        } else {
            let _ = crate::models::set_as_default_file_browser();
        }
        self.pending_action = Some(ToolAction::SetAsDefault);
        cx.notify();
    }

    /// Toggle network section
    fn toggle_network_section(&mut self, cx: &mut Context<Self>) {
        self.sidebar.toggle_network_expanded();
        cx.notify();
    }

    /// Handle cloud location click for navigation
    fn handle_cloud_click(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar.selected_path = Some(path.clone());
        self.pending_navigation = Some(path);
        cx.notify();
    }

    /// Handle network location click
    fn handle_network_click(&mut self, id: NetworkLocationId, _window: &mut Window, cx: &mut Context<Self>) {
        // Connect to the network location if not connected
        let _ = self.sidebar.network_manager_mut().connect(id);
        
        // If connected and has mount point, navigate to it
        let mount_point = self.sidebar.network_manager()
            .get_location(id)
            .and_then(|loc| loc.mount_point.clone());
        
        if let Some(path) = mount_point {
            self.sidebar.selected_path = Some(path.clone());
            self.pending_navigation = Some(path);
        }
        cx.notify();
    }

    /// Show the network connection dialog
    pub fn show_network_dialog(&mut self, cx: &mut Context<Self>) {
        self.show_network_dialog = true;
        cx.notify();
    }

    /// Hide the network connection dialog
    pub fn hide_network_dialog(&mut self, cx: &mut Context<Self>) {
        self.show_network_dialog = false;
        cx.notify();
    }

    /// Check if network dialog is visible
    pub fn is_network_dialog_visible(&self) -> bool {
        self.show_network_dialog
    }

    /// Toggle smart folders section
    fn toggle_smart_folders_section(&mut self, cx: &mut Context<Self>) {
        self.sidebar.toggle_smart_folders_expanded();
        cx.notify();
    }

    /// Show the smart folder dialog for creating a new smart folder
    pub fn show_smart_folder_dialog(&mut self, cx: &mut Context<Self>) {
        self.show_smart_folder_dialog = true;
        self.editing_smart_folder = None;
        cx.notify();
    }

    /// Show the smart folder dialog for editing an existing smart folder
    pub fn edit_smart_folder(&mut self, id: SmartFolderId, cx: &mut Context<Self>) {
        self.show_smart_folder_dialog = true;
        self.editing_smart_folder = Some(id);
        cx.notify();
    }

    /// Hide the smart folder dialog
    pub fn hide_smart_folder_dialog(&mut self, cx: &mut Context<Self>) {
        self.show_smart_folder_dialog = false;
        self.editing_smart_folder = None;
        cx.notify();
    }

    /// Check if smart folder dialog is visible
    pub fn is_smart_folder_dialog_visible(&self) -> bool {
        self.show_smart_folder_dialog
    }

    /// Get the smart folder being edited (if any)
    pub fn editing_smart_folder(&self) -> Option<&SmartFolder> {
        self.editing_smart_folder.and_then(|id| self.sidebar.smart_folders.get(id))
    }

    /// Handle smart folder click for executing the query
    fn handle_smart_folder_click(&mut self, id: SmartFolderId, _window: &mut Window, cx: &mut Context<Self>) {
        self.pending_smart_folder_click = Some(id);
        cx.notify();
    }

    /// Take the pending smart folder click (if any)
    pub fn take_pending_smart_folder_click(&mut self) -> Option<SmartFolderId> {
        self.pending_smart_folder_click.take()
    }

    /// Handle smart folder removal
    fn handle_smart_folder_remove(&mut self, id: SmartFolderId, cx: &mut Context<Self>) {
        let _ = self.sidebar.delete_smart_folder(id);
        cx.notify();
    }

    /// Create a new smart folder
    pub fn create_smart_folder(&mut self, name: String, query: SearchQuery, cx: &mut Context<Self>) -> Result<SmartFolderId, crate::models::SmartFolderError> {
        let result = self.sidebar.create_smart_folder(name, query);
        cx.notify();
        result
    }

    /// Update an existing smart folder
    pub fn update_smart_folder(&mut self, id: SmartFolderId, query: SearchQuery, cx: &mut Context<Self>) -> Result<(), crate::models::SmartFolderError> {
        let result = self.sidebar.update_smart_folder(id, query);
        cx.notify();
        result
    }

    /// Get all smart folders
    pub fn smart_folders(&self) -> &[SmartFolder] {
        self.sidebar.smart_folders.folders()
    }

    /// Refresh cloud storage providers
    pub fn refresh_cloud_providers(&mut self, cx: &mut Context<Self>) {
        self.sidebar.refresh_cloud_providers();
        cx.notify();
    }

    /// Get network sidebar state for display
    pub fn network_state(&self) -> NetworkSidebarState {
        self.sidebar.get_network_sidebar_state()
    }

    /// Toggle devices section
    fn toggle_devices_section(&mut self, cx: &mut Context<Self>) {
        self.sidebar.toggle_devices_expanded();
        cx.notify();
    }

    /// Handle device click for navigation
    fn handle_device_click(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar.selected_path = Some(path.clone());
        self.pending_navigation = Some(path);
        cx.notify();
    }

    /// Refresh devices list
    pub fn refresh_devices(&mut self, cx: &mut Context<Self>) {
        self.sidebar.refresh_devices();
        cx.notify();
    }

    /// Get all devices
    pub fn devices(&self) -> &[Device] {
        self.sidebar.devices()
    }

    /// Get WSL distributions
    pub fn wsl_distributions(&self) -> &[WslDistribution] {
        self.sidebar.wsl_distributions()
    }

    fn handle_tool_action(&mut self, action: ToolAction, _window: &mut Window, cx: &mut Context<Self>) {
        match &action {
            ToolAction::CopyPath => {
                // Copy current directory path to clipboard
                if let Some(path) = self.sidebar.current_directory() {
                    let path_str = path.to_string_lossy().to_string();
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(path_str));
                }
            }
            ToolAction::ToggleHiddenFiles => {
                self.sidebar.toggle_hidden_files();
            }
            ToolAction::SetAsDefault => {
            }
            _ => {
            }
        }
        self.pending_action = Some(action);
        cx.notify();
    }

    fn toggle_tools_section(&mut self, cx: &mut Context<Self>) {
        self.sidebar.toggle_tools_expanded();
        cx.notify();
    }

    pub fn set_workspace_root(&mut self, path: PathBuf) {
        self.sidebar.set_workspace_root(path);
    }

    pub fn sidebar(&self) -> &Sidebar {
        &self.sidebar
    }

    pub fn sidebar_mut(&mut self) -> &mut Sidebar {
        &mut self.sidebar
    }

    fn handle_favorite_click(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar.selected_path = Some(path.clone());
        self.pending_navigation = Some(path);
        cx.notify();
    }

    fn handle_favorite_remove(&mut self, index: usize, cx: &mut Context<Self>) {
        let _ = self.sidebar.remove_favorite(index);
        cx.notify();
    }

    fn handle_drop(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Only add directories as favorites
        if path.is_dir() {
            let _ = self.sidebar.add_favorite(path);
        }
        self.sidebar.set_drop_target(false);
        cx.notify();
    }

    fn handle_reorder_drop(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        let _ = self.sidebar.reorder_favorites(from, to);
        self.dragging_favorite_index = None;
        self.drop_target_index = None;
        cx.notify();
    }

    fn get_icon_for_favorite(&self, index: usize, path: &PathBuf) -> &'static str {
        if let Some(home) = dirs::home_dir() {
            if path == &home {
                return "house";
            }
            if path == &home.join("Desktop") {
                return "monitor";
            }
            if path == &home.join("Documents") {
                return "file-text";
            }
            if path == &home.join("Downloads") {
                return "cloud";
            }
        }
        
        // Default folder icon
        match index % 4 {
            0 => "folder",
            1 => "folder-open",
            2 => "folder-heart",
            _ => "folder-check",
        }
    }
}

impl Focusable for SidebarView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SidebarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Use theme colors for RPG styling
        let theme = theme_colors();
        let bg_dark = theme.bg_secondary;
        let text_gray = theme.text_secondary;
        let text_light = theme.text_primary;
        let hover_bg = theme.bg_hover;
        let selected_bg = theme.bg_selected;
        let label_color = theme.text_muted;
        let icon_blue = theme.accent_primary;
        let drop_zone_bg = gpui::Rgba { 
            r: theme.accent_primary.r, 
            g: theme.accent_primary.g, 
            b: theme.accent_primary.b, 
            a: 0.2 
        };
        let drop_zone_border = theme.accent_primary;
        let warning_color = theme.warning;
        let success_color = gpui::rgb(0x3fb950);

        let selected_path = self.sidebar.selected_path.clone();
        let favorites = self.sidebar.favorites.items().to_vec();
        let is_drop_target = self.sidebar.is_drop_target;
        let is_full = self.sidebar.favorites.is_full();
        let dragging_index = self.dragging_favorite_index;
        let drop_target_index = self.drop_target_index;
        let is_tools_expanded = self.sidebar.is_tools_expanded();
        let show_hidden = self.sidebar.show_hidden_files();
        let has_selection = self.selected_file_count > 0;
        let has_clipboard = self.has_clipboard;

        // Use typography spacing constants
        let section_gap = px(sidebar_spacing::SECTION_GAP);
        let item_padding_x = px(sidebar_spacing::ITEM_PADDING_X);
        let icon_size = px(sidebar_spacing::ICON_SIZE);
        let icon_gap = px(sidebar_spacing::ICON_GAP);

        div()
            .id("sidebar-content")
            .size_full()
            .bg(bg_dark)
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .child(
                div()
                    .p_3()
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .min_h_full()
                    .child(
                        div()
                            .mb(section_gap)
                            .child(
                                div()
                                    .id("tools-header")
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(label_color)
                                    .mb_2()
                                    .px(item_padding_x)
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                                        view.toggle_tools_section(cx);
                                    }))
                                    .child("TOOLS")
                                    .child(
                                        svg()
                                            .path(if is_tools_expanded { 
                                                "assets/icons/chevron-down.svg" 
                                            } else { 
                                                "assets/icons/chevron-right.svg" 
                                            })
                                            .size(px(12.0))
                                            .text_color(label_color),
                                    ),
                            )
                            .when(is_tools_expanded, |s| {
                                s.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_0p5()
                                        .p_1()
                                        // New File button
                                        .child(self.render_tool_button(
                                            "new-file",
                                            "file-plus",
                                            "New File",
                                            ToolAction::NewFile,
                                            true,
                                            text_gray,
                                            text_light,
                                            hover_bg,
                                            icon_blue,
                                            cx,
                                        ))
                                        // New Folder button
                                        .child(self.render_tool_button(
                                            "new-folder",
                                            "folder-plus",
                                            "New Folder",
                                            ToolAction::NewFolder,
                                            true,
                                            text_gray,
                                            text_light,
                                            hover_bg,
                                            icon_blue,
                                            cx,
                                        ))
                                        .child(
                                            div()
                                                .h(px(1.0))
                                                .bg(gpui::rgb(0x21262d))
                                                .my_1()
                                        )
                                        // Copy button (batch operation)
                                        .child(self.render_tool_button(
                                            "copy-files",
                                            "copy",
                                            "Copy",
                                            ToolAction::Copy,
                                            has_selection,
                                            text_gray,
                                            text_light,
                                            hover_bg,
                                            icon_blue,
                                            cx,
                                        ))
                                        // Move button (batch operation)
                                        .child(self.render_tool_button(
                                            "move-files",
                                            "files",
                                            "Move",
                                            ToolAction::Move,
                                            has_selection,
                                            text_gray,
                                            text_light,
                                            hover_bg,
                                            icon_blue,
                                            cx,
                                        ))
                                        // Paste button (enabled when clipboard has content)
                                        .child(self.render_tool_button(
                                            "paste-files",
                                            "clipboard-check",
                                            "Paste",
                                            ToolAction::Paste,
                                            has_clipboard,
                                            text_gray,
                                            text_light,
                                            hover_bg,
                                            gpui::rgb(0x3fb950),
                                            cx,
                                        ))
                                        // Delete button (batch operation)
                                        .child(self.render_tool_button(
                                            "delete-files",
                                            "trash-2",
                                            "Delete",
                                            ToolAction::Delete,
                                            has_selection,
                                            text_gray,
                                            text_light,
                                            hover_bg,
                                            gpui::rgb(0xf85149),
                                            cx,
                                        ))
                                        .child(
                                            div()
                                                .h(px(1.0))
                                                .bg(gpui::rgb(0x21262d))
                                                .my_1()
                                        )
                                        // Open Terminal Here
                                        .child(self.render_tool_button(
                                            "terminal-here",
                                            "terminal",
                                            "Open Terminal Here",
                                            ToolAction::OpenTerminalHere,
                                            true,
                                            text_gray,
                                            text_light,
                                            hover_bg,
                                            icon_blue,
                                            cx,
                                        ))
                                        .child(self.render_tool_button(
                                            "copy-path",
                                            "clipboard-paste",
                                            "Copy Path",
                                            ToolAction::CopyPath,
                                            true,
                                            text_gray,
                                            text_light,
                                            hover_bg,
                                            icon_blue,
                                            cx,
                                        ))
                                        .child(self.render_tool_button(
                                            "refresh",
                                            "refresh-cw",
                                            "Refresh",
                                            ToolAction::Refresh,
                                            true,
                                            text_gray,
                                            text_light,
                                            hover_bg,
                                            icon_blue,
                                            cx,
                                        ))
                                        .child(
                                            div()
                                                .h(px(1.0))
                                                .bg(gpui::rgb(0x21262d))
                                                .my_1()
                                        )
                                        // Show Hidden Files toggle
                                        .child(
                                            div()
                                                .id("toggle-hidden")
                                                .flex()
                                                .items_center()
                                                .gap_3()
                                                .px_2()
                                                .py_1p5()
                                                .rounded_md()
                                                .cursor_pointer()
                                                .text_sm()
                                                .text_color(text_gray)
                                                .hover(|h| h.bg(hover_bg).text_color(text_light))
                                                .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                                                    view.toggle_hidden_files(cx);
                                                }))
                                                .child(
                                                    svg()
                                                        .path(if show_hidden { 
                                                            "assets/icons/eye.svg" 
                                                        } else { 
                                                            "assets/icons/eye-off.svg" 
                                                        })
                                                        .size(px(14.0))
                                                        .text_color(if show_hidden { success_color } else { icon_blue }),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .child(if show_hidden { "Hide Hidden Files" } else { "Show Hidden Files" })
                                                )
                                                .when(show_hidden, |s| {
                                                    s.child(
                                                        div()
                                                            .w(px(6.0))
                                                            .h(px(6.0))
                                                            .rounded_full()
                                                            .bg(success_color)
                                                    )
                                                })
                                        )
                                        .child({
                                            let is_default = crate::models::is_default_file_browser();
                                            div()
                                                .id("set-as-default")
                                                .flex()
                                                .items_center()
                                                .gap_3()
                                                .px_2()
                                                .py_1p5()
                                                .rounded_md()
                                                .cursor_pointer()
                                                .text_sm()
                                                .text_color(text_gray)
                                                .hover(|h| h.bg(hover_bg).text_color(text_light))
                                                .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                                                    view.toggle_default_browser(cx);
                                                }))
                                                .child(
                                                    svg()
                                                        .path("assets/icons/layout-grid.svg")
                                                        .size(px(14.0))
                                                        .text_color(if is_default { success_color } else { icon_blue }),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .child(if is_default { "Default Browser ✓" } else { "Set as Default Browser" })
                                                )
                                                .when(is_default, |s| {
                                                    s.child(
                                                        div()
                                                            .w(px(6.0))
                                                            .h(px(6.0))
                                                            .rounded_full()
                                                            .bg(success_color)
                                                    )
                                                })
                                        })
                                )
                            })
                    )
                    // Devices Section
                    .child(self.render_devices_section(
                        label_color,
                        text_gray,
                        text_light,
                        hover_bg,
                        selected_bg,
                        icon_blue,
                        warning_color,
                        cx,
                    ))
                    // Network & Cloud Section
                    .child(self.render_network_section(
                        label_color,
                        text_gray,
                        text_light,
                        hover_bg,
                        selected_bg,
                        icon_blue,
                        cx,
                    ))
                    // Smart Folders Section
                    .child(self.render_smart_folders_section(
                        label_color,
                        text_gray,
                        text_light,
                        hover_bg,
                        selected_bg,
                        icon_blue,
                        cx,
                    ))
                    // Bookmarks Section (above Favorites per Requirements 20.2)
                    .child(self.render_bookmarks_section(
                        label_color,
                        text_gray,
                        text_light,
                        hover_bg,
                        selected_bg,
                        icon_blue,
                        warning_color,
                        cx,
                    ))
                    // Favorites Section
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(label_color)
                            .mb_2()
                            .px_2()
                            .child("FAVORITES"),
                    )
                    .child(
                        div()
                            .id("favorites-drop-zone")
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .mb_6()
                            .p_1()
                            .rounded_md()
                            .when(is_drop_target && !is_full, |s| {
                                s.bg(drop_zone_bg)
                                    .border_2()
                                    .border_color(drop_zone_border)
                            })
                            .on_drag_move(cx.listener(|view, _event: &DragMoveEvent<DraggedFolder>, _window, cx| {
                                if !view.sidebar.favorites.is_full() {
                                    view.sidebar.set_drop_target(true);
                                    cx.notify();
                                }
                            }))
                            .on_drop(cx.listener(|view, paths: &ExternalPaths, _window, cx| {
                                for path in paths.paths() {
                                    if path.is_dir() {
                                        view.handle_drop(path.clone(), cx);
                                    }
                                }
                            }))
                            .on_drop(cx.listener(|view, dragged: &DraggedFolder, _window, cx| {
                                view.handle_drop(dragged.path.clone(), cx);
                            }))
                            .children(
                                favorites.into_iter().enumerate().map(|(i, favorite)| {
                                    let is_selected = selected_path.as_ref() == Some(&favorite.path);
                                    let path_clone = favorite.path.clone();
                                    let path_for_drag = favorite.path.clone();
                                    let name_for_drag = favorite.name.clone();
                                    let icon_name = self.get_icon_for_favorite(i, &favorite.path);
                                    let is_valid = favorite.is_valid;
                                    let is_being_dragged = dragging_index == Some(i);
                                    let is_drop_target_here = drop_target_index == Some(i);

                                    div()
                                        .id(SharedString::from(format!("fav-{}", i)))
                                        .flex()
                                        .items_center()
                                        .gap_3()
                                        .px_2()
                                        .py_1p5()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .text_sm()
                                        .when(is_being_dragged, |s| s.opacity(0.5))
                                        .when(is_drop_target_here, |s| {
                                            s.border_t_2().border_color(drop_zone_border)
                                        })
                                        .when(is_selected, |s| {
                                            s.bg(selected_bg).text_color(text_light)
                                        })
                                        .when(!is_selected && is_valid, |s| {
                                            s.text_color(text_gray)
                                                .hover(|h| h.bg(hover_bg).text_color(text_light))
                                        })
                                        .when(!is_valid, |s| {
                                            s.text_color(warning_color).opacity(0.7)
                                        })
                                        .on_mouse_down(MouseButton::Left, cx.listener(move |view, _event, window, cx| {
                                            view.handle_favorite_click(path_clone.clone(), window, cx);
                                        }))
                                        .on_mouse_down(MouseButton::Right, cx.listener(move |view, _event, _window, cx| {
                                            view.handle_favorite_remove(i, cx);
                                        }))
                                        .on_drag(DraggedFolder {
                                            path: path_for_drag,
                                            name: name_for_drag,
                                        }, |dragged: &DraggedFolder, _position, _window, cx| {
                                            let name = dragged.name.clone();
                                            cx.new(|_| DraggedFolderView { name })
                                        })
                                        .on_drag_move(cx.listener(move |view, _event: &DragMoveEvent<DraggedFolder>, _window, cx| {
                                            view.drop_target_index = Some(i);
                                            cx.notify();
                                        }))
                                        .on_drop(cx.listener(move |view, dragged: &DraggedFolder, _window, cx| {
                                            // Find the index of the dragged item
                                            if let Some(from_idx) = view.sidebar.favorites.find_index(&dragged.path) {
                                                if from_idx != i {
                                                    view.handle_reorder_drop(from_idx, i, cx);
                                                }
                                            } else {
                                                // New item being dropped
                                                view.handle_drop(dragged.path.clone(), cx);
                                            }
                                        }))
                                        .child(
                                            svg()
                                                .path(SharedString::from(format!("assets/icons/{}.svg", icon_name)))
                                                .size(px(14.0))
                                                .text_color(if !is_valid { 
                                                    warning_color 
                                                } else if is_selected { 
                                                    text_light 
                                                } else { 
                                                    icon_blue 
                                                }),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .overflow_hidden()
                                                .child(favorite.name.clone())
                                        )
                                        .when(!is_valid, |s| {
                                            s.child(
                                                svg()
                                                    .path("assets/icons/triangle-alert.svg")
                                                    .size(px(12.0))
                                                    .text_color(warning_color)
                                            )
                                        })
                                }),
                            )
                            .when(is_drop_target && !is_full, |s| {
                                s.child(
                                    div()
                                        .px_2()
                                        .py_1p5()
                                        .text_sm()
                                        .text_color(icon_blue)
                                        .text_center()
                                        .child("Drop folder here to add")
                                )
                            }),
                    )
                    .child({
                        let trash_path = Self::get_trash_path();
                        let is_trash_selected = selected_path.as_ref() == Some(&trash_path);
                        div()
                            .id("trash-item")
                            .px_2()
                            .py_1p5()
                            .mx_1()
                            .rounded_md()
                            .text_sm()
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(is_trash_selected, |s| s.bg(theme.bg_hover).text_color(text_light))
                            .when(!is_trash_selected, |s| s.text_color(text_gray))
                            .hover(|h| h.bg(theme.bg_hover))
                            .on_mouse_down(MouseButton::Left, {
                                let path = trash_path.clone();
                                cx.listener(move |view, _event, _window, cx| {
                                    view.sidebar.selected_path = Some(path.clone());
                                    view.pending_navigation = Some(path.clone());
                                    cx.notify();
                                })
                            })
                            .child(
                                svg()
                                    .path("assets/icons/trash-2.svg")
                                    .size(px(14.0))
                                    .text_color(if is_trash_selected { text_light } else { text_gray }),
                            )
                            .child("Trash")
                    }),
            )
    }
}

impl SidebarView {
    fn render_tool_button(
        &self,
        id: &'static str,
        icon: &'static str,
        label: &'static str,
        action: ToolAction,
        enabled: bool,
        text_gray: gpui::Rgba,
        text_light: gpui::Rgba,
        hover_bg: gpui::Rgba,
        icon_color: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let action_clone = action.clone();
        let base = div()
            .id(SharedString::from(id))
            .flex()
            .items_center()
            .gap_3()
            .px_2()
            .py_1p5()
            .rounded_md()
            .text_sm()
            .child(
                svg()
                    .path(SharedString::from(format!("assets/icons/{}.svg", icon)))
                    .size(px(14.0))
                    .text_color(if enabled { icon_color } else { text_gray }),
            )
            .child(
                div()
                    .flex_1()
                    .child(label)
            );

        if enabled {
            base.cursor_pointer()
                .text_color(text_gray)
                .hover(|h| h.bg(hover_bg).text_color(text_light))
                .on_mouse_down(MouseButton::Left, cx.listener(move |view, _event, window, cx| {
                    view.handle_tool_action(action_clone.clone(), window, cx);
                }))
        } else {
            base.opacity(0.4)
                .cursor_not_allowed()
                .text_color(text_gray)
        }
    }

    fn render_smart_folders_section(
        &self,
        label_color: gpui::Rgba,
        text_gray: gpui::Rgba,
        text_light: gpui::Rgba,
        hover_bg: gpui::Rgba,
        _selected_bg: gpui::Rgba,
        icon_blue: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_expanded = self.sidebar.is_smart_folders_expanded();
        let smart_folders = self.sidebar.smart_folders.folders().to_vec();

        div()
            .mb_4()
            .child(
                div()
                    .id("smart-folders-header")
                    .text_xs()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(label_color)
                    .mb_2()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                        view.toggle_smart_folders_section(cx);
                    }))
                    .child("SMART FOLDERS")
                    .child(
                        svg()
                            .path(if is_expanded {
                                "assets/icons/chevron-down.svg"
                            } else {
                                "assets/icons/chevron-right.svg"
                            })
                            .size(px(12.0))
                            .text_color(label_color),
                    ),
            )
            .when(is_expanded, |s| {
                s.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .p_1()
                        .when(smart_folders.is_empty(), |s| {
                            s.child(
                                div()
                                    .px_2()
                                    .py_1p5()
                                    .text_sm()
                                    .text_color(text_gray)
                                    .opacity(0.7)
                                    .child("No smart folders yet")
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(text_gray)
                                            .opacity(0.5)
                                            .mt_1()
                                            .child("Create saved searches")
                                    )
                            )
                        })
                        .children(
                            smart_folders.into_iter().map(|folder| {
                                let folder_id = folder.id;
                                let display_name = folder.name.clone();
                                let description = folder.query.description();

                                div()
                                    .id(SharedString::from(format!("smart-folder-{}", folder.id.0)))
                                    .flex()
                                    .flex_col()
                                    .px_2()
                                    .py_1p5()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_color(text_gray)
                                    .hover(|h| h.bg(hover_bg).text_color(text_light))
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |view, _event, window, cx| {
                                        view.handle_smart_folder_click(folder_id, window, cx);
                                    }))
                                    .on_mouse_down(MouseButton::Right, cx.listener(move |view, _event, _window, cx| {
                                        view.handle_smart_folder_remove(folder_id, cx);
                                    }))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_3()
                                            .child(
                                                svg()
                                                    .path("assets/icons/sparkles.svg")
                                                    .size(px(14.0))
                                                    .text_color(icon_blue),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .overflow_hidden()
                                                    .text_sm()
                                                    .child(display_name)
                                            )
                                    )
                                    .child(
                                        div()
                                            .pl(px(26.0))
                                            .text_xs()
                                            .text_color(text_gray)
                                            .opacity(0.6)
                                            .overflow_hidden()
                                            .child(description)
                                    )
                            }),
                        )
                        .child(
                            div()
                                .id("create-smart-folder-btn")
                                .flex()
                                .items_center()
                                .gap_3()
                                .px_2()
                                .py_1p5()
                                .mt_1()
                                .rounded_md()
                                .cursor_pointer()
                                .text_sm()
                                .text_color(text_gray)
                                .hover(|h| h.bg(hover_bg).text_color(text_light))
                                .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                                    view.show_smart_folder_dialog(cx);
                                }))
                                .child(
                                    svg()
                                        .path("assets/icons/folder-plus.svg")
                                        .size(px(14.0))
                                        .text_color(icon_blue),
                                )
                                .child("New Smart Folder...")
                        )
                )
            })
    }

    fn render_bookmarks_section(
        &self,
        label_color: gpui::Rgba,
        text_gray: gpui::Rgba,
        text_light: gpui::Rgba,
        hover_bg: gpui::Rgba,
        selected_bg: gpui::Rgba,
        icon_blue: gpui::Rgba,
        warning_color: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_expanded = self.sidebar.is_bookmarks_expanded();
        let bookmarks = self.sidebar.bookmarks.bookmarks().to_vec();
        let selected_path = self.sidebar.selected_path.clone();

        div()
            .mb_4()
            .child(
                div()
                    .id("bookmarks-header")
                    .text_xs()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(label_color)
                    .mb_2()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                        view.toggle_bookmarks_section(cx);
                    }))
                    .child("BOOKMARKS")
                    .child(
                        svg()
                            .path(if is_expanded {
                                "assets/icons/chevron-down.svg"
                            } else {
                                "assets/icons/chevron-right.svg"
                            })
                            .size(px(12.0))
                            .text_color(label_color),
                    ),
            )
            .when(is_expanded, |s| {
                s.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .p_1()
                        .when(bookmarks.is_empty(), |s| {
                            s.child(
                                div()
                                    .px_2()
                                    .py_1p5()
                                    .text_sm()
                                    .text_color(text_gray)
                                    .opacity(0.7)
                                    .child("No bookmarks yet")
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(text_gray)
                                            .opacity(0.5)
                                            .mt_1()
                                            .child("Press ⌘D to bookmark current folder")
                                    )
                            )
                        })
                        .children(
                            bookmarks.into_iter().map(|bookmark| {
                                let is_selected = selected_path.as_ref() == Some(&bookmark.path);
                                let path_clone = bookmark.path.clone();
                                let is_valid = bookmark.is_valid;
                                let bookmark_id = bookmark.id;
                                let shortcut_display = bookmark.shortcut.as_ref().map(|s| s.display());

                                div()
                                    .id(SharedString::from(format!("bookmark-{}", bookmark.id.0)))
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .px_2()
                                    .py_1p5()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_sm()
                                    .when(is_selected, |s| {
                                        s.bg(selected_bg).text_color(text_light)
                                    })
                                    .when(!is_selected && is_valid, |s| {
                                        s.text_color(text_gray)
                                            .hover(|h| h.bg(hover_bg).text_color(text_light))
                                    })
                                    .when(!is_valid, |s| {
                                        s.text_color(warning_color).opacity(0.7)
                                    })
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |view, _event, window, cx| {
                                        view.handle_bookmark_click(path_clone.clone(), window, cx);
                                    }))
                                    .on_mouse_down(MouseButton::Right, cx.listener(move |view, _event, _window, cx| {
                                        view.handle_bookmark_remove(bookmark_id, cx);
                                    }))
                                    .child(
                                        svg()
                                            .path("assets/icons/folder-heart.svg")
                                            .size(px(14.0))
                                            .text_color(if !is_valid {
                                                warning_color
                                            } else if is_selected {
                                                text_light
                                            } else {
                                                icon_blue
                                            }),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .child(bookmark.name.clone())
                                    )
                                    .when(shortcut_display.is_some(), |s| {
                                        s.child(
                                            div()
                                                .text_xs()
                                                .text_color(text_gray)
                                                .opacity(0.6)
                                                .child(shortcut_display.unwrap_or_default())
                                        )
                                    })
                                    .when(!is_valid, |s| {
                                        s.child(
                                            svg()
                                                .path("assets/icons/triangle-alert.svg")
                                                .size(px(12.0))
                                                .text_color(warning_color)
                                        )
                                    })
                            }),
                        )
                )
            })
    }

    fn render_network_section(
        &self,
        label_color: gpui::Rgba,
        text_gray: gpui::Rgba,
        text_light: gpui::Rgba,
        hover_bg: gpui::Rgba,
        selected_bg: gpui::Rgba,
        icon_blue: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_expanded = self.sidebar.is_network_expanded();
        let network_state = self.network_state();
        let selected_path = self.sidebar.selected_path.clone();

        div()
            .mb_4()
            .child(
                div()
                    .id("network-header")
                    .text_xs()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(label_color)
                    .mb_2()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                        view.toggle_network_section(cx);
                    }))
                    .child("NETWORK & CLOUD")
                    .child(
                        svg()
                            .path(if is_expanded {
                                "assets/icons/chevron-down.svg"
                            } else {
                                "assets/icons/chevron-right.svg"
                            })
                            .size(px(12.0))
                            .text_color(label_color),
                    ),
            )
            .when(is_expanded, |s| {
                s.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .p_1()
                        // Cloud Storage Locations
                        .children(
                            network_state.cloud_locations.iter().map(|cloud| {
                                let is_selected = selected_path.as_ref() == Some(&cloud.path);
                                let path_clone = cloud.path.clone();
                                let is_available = cloud.is_available;
                                let icon_name = cloud.provider.icon_name();
                                let display_name = cloud.name.clone();

                                div()
                                    .id(SharedString::from(format!("cloud-{}", display_name)))
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .px_2()
                                    .py_1p5()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_sm()
                                    .when(is_selected, |s| {
                                        s.bg(selected_bg).text_color(text_light)
                                    })
                                    .when(!is_selected && is_available, |s| {
                                        s.text_color(text_gray)
                                            .hover(|h| h.bg(hover_bg).text_color(text_light))
                                    })
                                    .when(!is_available, |s| {
                                        s.text_color(text_gray).opacity(0.5)
                                    })
                                    .when(is_available, |s| {
                                        s.on_mouse_down(MouseButton::Left, cx.listener(move |view, _event, window, cx| {
                                            view.handle_cloud_click(path_clone.clone(), window, cx);
                                        }))
                                    })
                                    .child(
                                        svg()
                                            .path(SharedString::from(format!("assets/icons/{}.svg", icon_name)))
                                            .size(px(14.0))
                                            .text_color(if is_selected { text_light } else { icon_blue }),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .child(display_name)
                                    )
                                    .when(is_available, |s| {
                                        s.child(
                                            div()
                                                .w(px(6.0))
                                                .h(px(6.0))
                                                .rounded_full()
                                                .bg(gpui::rgb(0x3fb950))
                                        )
                                    })
                            }).collect::<Vec<_>>()
                        )
                        // Network Locations
                        .children(
                            network_state.network_locations.iter().map(|network| {
                                let is_connected = network.is_connected;
                                let display_name = network.name.clone();
                                let protocol_icon = network.protocol.icon_name();
                                let latency = network.latency_ms;
                                let network_id = network.id;

                                div()
                                    .id(SharedString::from(format!("network-{}", network.id.0)))
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .px_2()
                                    .py_1p5()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(text_gray)
                                    .hover(|h| h.bg(hover_bg).text_color(text_light))
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |view, _event, window, cx| {
                                        view.handle_network_click(network_id, window, cx);
                                    }))
                                    .child(
                                        svg()
                                            .path(SharedString::from(format!("assets/icons/{}.svg", protocol_icon)))
                                            .size(px(14.0))
                                            .text_color(icon_blue),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .child(display_name)
                                    )
                                    .when(is_connected, |s| {
                                        s.child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .w(px(6.0))
                                                        .h(px(6.0))
                                                        .rounded_full()
                                                        .bg(gpui::rgb(0x3fb950))
                                                )
                                                .when(latency.is_some(), |s| {
                                                    s.child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(text_gray)
                                                            .opacity(0.6)
                                                            .child(format!("{}ms", latency.unwrap_or(0)))
                                                    )
                                                })
                                        )
                                    })
                            }).collect::<Vec<_>>()
                        )
                        // Connect to Server button
                        .child(
                            div()
                                .id("connect-server-btn")
                                .flex()
                                .items_center()
                                .gap_3()
                                .px_2()
                                .py_1p5()
                                .rounded_md()
                                .cursor_pointer()
                                .text_sm()
                                .text_color(text_gray)
                                .hover(|h| h.bg(hover_bg).text_color(text_light))
                                .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                                    view.show_network_dialog(cx);
                                }))
                                .child(
                                    svg()
                                        .path("assets/icons/folder-plus.svg")
                                        .size(px(14.0))
                                        .text_color(icon_blue),
                                )
                                .child("Connect to Server...")
                        )
                        // Empty state when no locations
                        .when(network_state.cloud_locations.is_empty() && network_state.network_locations.is_empty(), |s| {
                            s.child(
                                div()
                                    .px_2()
                                    .py_1p5()
                                    .text_sm()
                                    .text_color(text_gray)
                                    .opacity(0.7)
                                    .child("No cloud storage detected")
                            )
                        })
                )
            })
    }

    fn render_devices_section(
        &self,
        label_color: gpui::Rgba,
        text_gray: gpui::Rgba,
        text_light: gpui::Rgba,
        hover_bg: gpui::Rgba,
        selected_bg: gpui::Rgba,
        icon_blue: gpui::Rgba,
        warning_color: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_expanded = self.sidebar.is_devices_expanded();
        let devices = self.sidebar.devices().to_vec();
        let wsl_distros = self.sidebar.wsl_distributions().to_vec();
        let selected_path = self.sidebar.selected_path.clone();

        div()
            .mb_4()
            .child(
                div()
                    .id("devices-header")
                    .text_xs()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(label_color)
                    .mb_2()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                        view.toggle_devices_section(cx);
                    }))
                    .child("DEVICES")
                    .child(
                        svg()
                            .path(if is_expanded {
                                "assets/icons/chevron-down.svg"
                            } else {
                                "assets/icons/chevron-right.svg"
                            })
                            .size(px(12.0))
                            .text_color(label_color),
                    ),
            )
            .when(is_expanded, |s| {
                s.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .p_1()
                        .children(
                            devices.iter().map(|device| {
                                let is_selected = selected_path.as_ref() == Some(&device.path);
                                let path_clone = device.path.clone();
                                let icon_name = device.device_type.icon_name();
                                let display_name = device.name.clone();
                                let is_read_only = device.is_read_only;
                                let is_removable = device.is_removable;
                                let is_wsl = matches!(device.device_type, DeviceType::WslDistribution);
                                
                                // Format space info
                                let space_info = if device.total_space > 0 {
                                    let used_gb = device.used_space() as f64 / 1_073_741_824.0;
                                    let total_gb = device.total_space as f64 / 1_073_741_824.0;
                                    Some(format!("{:.1} GB / {:.1} GB", used_gb, total_gb))
                                } else {
                                    None
                                };

                                div()
                                    .id(SharedString::from(format!("device-{}", device.id.0)))
                                    .flex()
                                    .flex_col()
                                    .px_2()
                                    .py_1p5()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .when(is_selected, |s| {
                                        s.bg(selected_bg)
                                    })
                                    .when(!is_selected, |s| {
                                        s.hover(|h| h.bg(hover_bg))
                                    })
                                    .on_mouse_down(MouseButton::Left, cx.listener(move |view, _event, window, cx| {
                                        view.handle_device_click(path_clone.clone(), window, cx);
                                    }))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_3()
                                            .child(
                                                svg()
                                                    .path(SharedString::from(format!("assets/icons/{}.svg", icon_name)))
                                                    .size(px(14.0))
                                                    .text_color(if is_selected { text_light } else { icon_blue }),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .overflow_hidden()
                                                    .text_sm()
                                                    .text_color(if is_selected { text_light } else { text_gray })
                                                    .child(display_name)
                                            )
                                            // Read-only lock icon
                                            .when(is_read_only, |s| {
                                                s.child(
                                                    svg()
                                                        .path("assets/icons/file-lock.svg")
                                                        .size(px(12.0))
                                                        .text_color(warning_color)
                                                )
                                            })
                                            // Removable indicator
                                            .when(is_removable && !is_wsl, |s| {
                                                s.child(
                                                    svg()
                                                        .path("assets/icons/hard-drive.svg")
                                                        .size(px(10.0))
                                                        .text_color(text_gray)
                                                        .opacity(0.5)
                                                )
                                            })
                                    )
                                    // Space info on second line
                                    .when(space_info.is_some(), |s| {
                                        s.child(
                                            div()
                                                .pl(px(26.0))
                                                .text_xs()
                                                .text_color(text_gray)
                                                .opacity(0.7)
                                                .child(space_info.unwrap_or_default())
                                        )
                                    })
                            }).collect::<Vec<_>>()
                        )
                        // Render WSL distributions (Windows only)
                        .when(!wsl_distros.is_empty(), |s| {
                            s.child(
                                div()
                                    .h(px(1.0))
                                    .bg(gpui::rgb(0x21262d))
                                    .my_2()
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(label_color)
                                    .opacity(0.7)
                                    .px_2()
                                    .mb_1()
                                    .child("WSL Distributions")
                            )
                            .children(
                                wsl_distros.iter().map(|distro| {
                                    let is_selected = selected_path.as_ref() == Some(&distro.path);
                                    let path_clone = distro.path.clone();
                                    let display_name = distro.name.clone();
                                    let is_running = distro.is_running;
                                    let version = distro.version;

                                    div()
                                        .id(SharedString::from(format!("wsl-{}", display_name)))
                                        .flex()
                                        .items_center()
                                        .gap_3()
                                        .px_2()
                                        .py_1p5()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .text_sm()
                                        .when(is_selected, |s| {
                                            s.bg(selected_bg).text_color(text_light)
                                        })
                                        .when(!is_selected, |s| {
                                            s.text_color(text_gray)
                                                .hover(|h| h.bg(hover_bg).text_color(text_light))
                                        })
                                        .on_mouse_down(MouseButton::Left, cx.listener(move |view, _event, window, cx| {
                                            view.handle_device_click(path_clone.clone(), window, cx);
                                        }))
                                        .child(
                                            svg()
                                                .path("assets/icons/terminal.svg")
                                                .size(px(14.0))
                                                .text_color(if is_selected { text_light } else { icon_blue }),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .overflow_hidden()
                                                .child(display_name)
                                        )
                                        // Running status indicator
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .w(px(6.0))
                                                        .h(px(6.0))
                                                        .rounded_full()
                                                        .bg(if is_running { 
                                                            gpui::rgb(0x3fb950)
                                                        } else { 
                                                            gpui::rgb(0x6e7681)
                                                        })
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(text_gray)
                                                        .opacity(0.6)
                                                        .child(format!("WSL{}", version))
                                                )
                                        )
                                }).collect::<Vec<_>>()
                            )
                        })
                        // Empty state when no devices
                        .when(devices.is_empty() && wsl_distros.is_empty(), |s| {
                            s.child(
                                div()
                                    .px_2()
                                    .py_1p5()
                                    .text_sm()
                                    .text_color(text_gray)
                                    .opacity(0.7)
                                    .child("No devices detected")
                            )
                        })
                )
            })
    }

}
