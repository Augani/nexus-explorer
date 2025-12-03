use std::path::PathBuf;

use gpui::{
    div, prelude::*, px, svg, App, Context, DragMoveEvent, ExternalPaths, FocusHandle,
    Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement, Render, SharedString,
    Styled, Window,
};

use crate::models::{Favorite, Favorites, theme_colors, sidebar as sidebar_spacing};

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
    name: String,
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
    Copy,
    Move,
    Delete,
}

pub struct Sidebar {
    favorites: Favorites,
    workspace_root: Option<SidebarItem>,
    selected_path: Option<PathBuf>,
    is_drop_target: bool,
    is_tools_expanded: bool,
    show_hidden_files: bool,
    current_directory: Option<PathBuf>,
}

impl Sidebar {
    pub fn new() -> Self {
        // Try to load favorites from disk, fall back to defaults
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

        Self {
            favorites,
            workspace_root: None,
            selected_path: None,
            is_drop_target: false,
            is_tools_expanded: true,
            show_hidden_files: false,
            current_directory: None,
        }
    }

    pub fn is_tools_expanded(&self) -> bool {
        self.is_tools_expanded
    }

    pub fn toggle_tools_expanded(&mut self) {
        self.is_tools_expanded = !self.is_tools_expanded;
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
}

pub struct SidebarView {
    sidebar: Sidebar,
    focus_handle: FocusHandle,
    dragging_favorite_index: Option<usize>,
    drop_target_index: Option<usize>,
    pending_navigation: Option<PathBuf>,
    pending_action: Option<ToolAction>,
    selected_file_count: usize,
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
        }
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
            _ => {
                // Other actions are handled by workspace
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
        // Check for common directories
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
            .child(
                div()
                    .p_3()
                    // Tools Section
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
                                        // Divider
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
                                        // Divider
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
                                        // Copy Path
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
                                        // Refresh
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
                                        // Divider
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
                                )
                            })
                    )
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
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(label_color)
                            .mb_2()
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child("WORKSPACE")
                            .child(
                                svg()
                                    .path("assets/icons/chevron-down.svg")
                                    .size(px(12.0))
                                    .text_color(label_color),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .pb_4()
                    .child(self.render_workspace_tree(cx)),
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
        div()
            .id(SharedString::from(id))
            .flex()
            .items_center()
            .gap_3()
            .px_2()
            .py_1p5()
            .rounded_md()
            .text_sm()
            .when(enabled, |s| {
                s.cursor_pointer()
                    .text_color(text_gray)
                    .hover(|h| h.bg(hover_bg).text_color(text_light))
            })
            .when(!enabled, |s| {
                s.opacity(0.4)
                    .cursor_not_allowed()
                    .text_color(text_gray)
            })
            .when(enabled, |s| {
                s.on_mouse_down(MouseButton::Left, cx.listener(move |view, _event, window, cx| {
                    view.handle_tool_action(action.clone(), window, cx);
                }))
            })
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
            )
    }

    fn render_workspace_tree(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let text_gray = gpui::rgb(0x8b949e);
        let hover_bg = gpui::rgb(0x21262d);
        let text_light = gpui::rgb(0xe6edf3);
        let folder_color = gpui::rgb(0x54aeff);

        if let Some(ref root) = self.sidebar.workspace_root {
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .id("workspace-root")
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_1p5()
                        .cursor_pointer()
                        .text_sm()
                        .text_color(text_gray)
                        .hover(|s| s.bg(hover_bg).text_color(text_light))
                        .child(
                            svg()
                                .path("assets/icons/chevron-right.svg")
                                .size(px(14.0))
                                .text_color(gpui::rgb(0x6e7681)),
                        )
                        .child(
                            svg()
                                .path("assets/icons/folder.svg")
                                .size(px(14.0))
                                .text_color(folder_color),
                        )
                        .child(root.name.clone()),
                )
        } else {
            div()
                .px_3()
                .py_2()
                .text_sm()
                .text_color(text_gray)
                .flex()
                .items_center()
                .gap_2()
                .child(
                    svg()
                        .path("assets/icons/folder-x.svg")
                        .size(px(14.0))
                        .text_color(text_gray),
                )
                .child("No workspace open")
        }
    }
}
