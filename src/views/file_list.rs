use std::path::PathBuf;
use std::time::SystemTime;

use gpui::{
    actions, anchored, div, prelude::*, px, svg, uniform_list, App, Context, Corner, FocusHandle, 
    Focusable, InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement, Point, 
    Pixels, Render, ScrollStrategy, SharedString, Styled, UniformListScrollHandle, Window, 
    MouseDownEvent,
};

use crate::models::{CloudSyncStatus, FileEntry, IconKey, SortColumn, SortDirection, SortState, theme_colors, file_list as file_list_spacing};
use crate::views::sidebar::{DraggedFolder, DraggedFolderView};

/// Context menu actions that can be triggered on files/folders
#[derive(Clone, Debug, PartialEq)]
pub enum ContextMenuAction {
    Open(PathBuf),
    OpenWith(PathBuf),
    OpenWithApp { file_path: PathBuf, app_path: PathBuf, app_name: String },
    OpenWithOther(PathBuf),
    OpenInNewWindow(PathBuf),
    GetInfo(PathBuf),
    Rename(PathBuf),
    Copy(PathBuf),
    Cut(PathBuf),
    Paste,
    Duplicate(PathBuf),
    MoveToTrash(PathBuf),
    Compress(PathBuf),
    Share(PathBuf),
    CopyPath(PathBuf),
    ShowInFinder(PathBuf),
    QuickLook(PathBuf),
    AddToFavorites(PathBuf),
    NewFolder,
    NewFile,
}

// Define actions for keyboard navigation
actions!(file_list, [
    MoveSelectionUp,
    MoveSelectionDown,
    OpenSelected,
    NavigateToParent,
]);

// Use typography constants for row height (40px as per design spec)
pub const DEFAULT_ROW_HEIGHT: f32 = file_list_spacing::ROW_HEIGHT;
pub const DEFAULT_BUFFER_SIZE: usize = 5;

// RPG styling constants
const ICON_SIZE: f32 = file_list_spacing::ICON_SIZE;
const ICON_GAP: f32 = file_list_spacing::ICON_GAP;
const ROW_PADDING_X: f32 = file_list_spacing::ROW_PADDING_X;
const HEADER_HEIGHT: f32 = file_list_spacing::HEADER_HEIGHT;
const FOOTER_HEIGHT: f32 = file_list_spacing::FOOTER_HEIGHT;

pub struct FileList {
    entries: Vec<FileEntry>,
    filtered_entries: Option<Vec<FilteredEntry>>,
    row_height: f32,
    buffer_size: usize,
    scroll_offset: f32,
    viewport_height: f32,
    highlight_positions: Option<Vec<Vec<usize>>>,
    selected_index: Option<usize>,
    search_query: String,
    sort_state: SortState,
}

/// A filtered entry with its original index and match positions
#[derive(Debug, Clone)]
pub struct FilteredEntry {
    pub original_index: usize,
    pub entry: FileEntry,
    pub match_positions: Vec<usize>,
    pub score: u32,
}

pub struct FileListView {
    file_list: FileList,
    focus_handle: FocusHandle,
    scroll_handle: UniformListScrollHandle,
    pending_navigation: Option<PathBuf>,
    pending_parent_navigation: bool,
    context_menu_position: Option<Point<Pixels>>,
    context_menu_index: Option<usize>,
    pending_context_action: Option<ContextMenuAction>,
    show_open_with_submenu: bool,
}

impl FileListView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            file_list: FileList::new(),
            focus_handle: cx.focus_handle(),
            scroll_handle: UniformListScrollHandle::new(),
            pending_navigation: None,
            pending_parent_navigation: false,
            context_menu_position: None,
            context_menu_index: None,
            pending_context_action: None,
            show_open_with_submenu: false,
        }
    }

    pub fn with_file_list(file_list: FileList, cx: &mut Context<Self>) -> Self {
        Self {
            file_list,
            focus_handle: cx.focus_handle(),
            scroll_handle: UniformListScrollHandle::new(),
            pending_navigation: None,
            pending_parent_navigation: false,
            context_menu_position: None,
            context_menu_index: None,
            pending_context_action: None,
            show_open_with_submenu: false,
        }
    }
    
    pub fn close_context_menu(&mut self) {
        self.context_menu_position = None;
        self.context_menu_index = None;
        self.show_open_with_submenu = false;
    }
    
    pub fn take_pending_context_action(&mut self) -> Option<ContextMenuAction> {
        self.pending_context_action.take()
    }

    pub fn inner(&self) -> &FileList {
        &self.file_list
    }

    pub fn inner_mut(&mut self) -> &mut FileList {
        &mut self.file_list
    }

    pub fn take_pending_navigation(&mut self) -> Option<PathBuf> {
        self.pending_navigation.take()
    }

    /// Check and clear the pending parent navigation flag
    pub fn take_pending_parent_navigation(&mut self) -> bool {
        let result = self.pending_parent_navigation;
        self.pending_parent_navigation = false;
        result
    }

    pub fn select_item(&mut self, index: usize, cx: &mut Context<Self>) {
        self.file_list.selected_index = Some(index);
        cx.notify();
    }

    pub fn open_item(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(entry) = self.file_list.entries.get(index) {
            if entry.is_dir {
                self.pending_navigation = Some(entry.path.clone());
                cx.notify();
            }
        }
    }

    /// Register key bindings for file list navigation
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("up", MoveSelectionUp, Some("FileList")),
            KeyBinding::new("down", MoveSelectionDown, Some("FileList")),
            KeyBinding::new("enter", OpenSelected, Some("FileList")),
            KeyBinding::new("backspace", NavigateToParent, Some("FileList")),
        ]);
    }

    /// Move selection up by one item
    fn handle_move_up(&mut self, _: &MoveSelectionUp, _window: &mut Window, cx: &mut Context<Self>) {
        let item_count = self.file_list.item_count();
        if item_count == 0 {
            return;
        }

        let new_index = match self.file_list.selected_index {
            Some(current) if current > 0 => current - 1,
            Some(_) => 0,
            None => 0,
        };

        self.file_list.selected_index = Some(new_index);
        self.scroll_to_index(new_index);
        cx.notify();
    }

    /// Move selection down by one item
    fn handle_move_down(&mut self, _: &MoveSelectionDown, _window: &mut Window, cx: &mut Context<Self>) {
        let item_count = self.file_list.item_count();
        if item_count == 0 {
            return;
        }

        let max_index = item_count.saturating_sub(1);
        let new_index = match self.file_list.selected_index {
            Some(current) if current < max_index => current + 1,
            Some(current) => current,
            None => 0,
        };

        self.file_list.selected_index = Some(new_index);
        self.scroll_to_index(new_index);
        cx.notify();
    }

    /// Open the selected item (navigate into directory)
    fn handle_open_selected(&mut self, _: &OpenSelected, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.file_list.selected_index {
            let entry = if let Some(filtered) = &self.file_list.filtered_entries {
                filtered.get(index).map(|f| &f.entry)
            } else {
                self.file_list.entries.get(index)
            };

            if let Some(entry) = entry {
                if entry.is_dir {
                    self.pending_navigation = Some(entry.path.clone());
                    cx.notify();
                }
            }
        }
    }

    /// Navigate to parent directory
    fn handle_navigate_to_parent(&mut self, _: &NavigateToParent, _window: &mut Window, cx: &mut Context<Self>) {
        // Signal to workspace to navigate to parent directory
        self.pending_parent_navigation = true;
        cx.notify();
    }

    /// Scroll to ensure the given index is visible
    fn scroll_to_index(&self, index: usize) {
        self.scroll_handle.scroll_to_item(index, ScrollStrategy::Center);
    }

    /// Move selection up by one item (public API for testing)
    pub fn move_selection_up(&mut self) {
        let item_count = self.file_list.item_count();
        if item_count == 0 {
            return;
        }

        let new_index = match self.file_list.selected_index {
            Some(current) if current > 0 => current - 1,
            Some(_) => 0,
            None => 0,
        };

        self.file_list.selected_index = Some(new_index);
    }

    /// Move selection down by one item (public API for testing)
    pub fn move_selection_down(&mut self) {
        let item_count = self.file_list.item_count();
        if item_count == 0 {
            return;
        }

        let max_index = item_count.saturating_sub(1);
        let new_index = match self.file_list.selected_index {
            Some(current) if current < max_index => current + 1,
            Some(current) => current,
            None => 0,
        };

        self.file_list.selected_index = Some(new_index);
    }
}

impl Focusable for FileListView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let total_items = self.file_list.item_count();
        let row_height = self.file_list.row_height();
        let selected_index = self.file_list.selected_index;
        let context_menu_pos = self.context_menu_position;
        let context_menu_idx = self.context_menu_index;

        let colors = theme_colors();
        
        // Background colors from theme
        let bg_darker = colors.bg_void;
        let bg_dark = colors.bg_primary;
        let border_color = colors.border_default;
        let border_subtle = colors.border_subtle;
        let text_gray = colors.text_secondary;
        let text_light = colors.text_primary;
        let hover_bg = colors.bg_hover;
        let selected_bg = colors.bg_selected;
        let folder_color = colors.folder_color;
        let folder_open_color = colors.folder_open_color;
        let _file_color = colors.text_muted;
        let menu_bg = colors.bg_tertiary;
        
        // Accent colors for glow effects and selection
        let _accent_glow = colors.accent_glow;
        let accent_primary = colors.accent_primary;

        div()
            .id("file-list")
            .key_context("FileList")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_move_up))
            .on_action(cx.listener(Self::handle_move_down))
            .on_action(cx.listener(Self::handle_open_selected))
            .on_action(cx.listener(Self::handle_navigate_to_parent))
            .size_full()
            .bg(bg_darker)
            .flex()
            .flex_col()
            .relative()
            .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, window, cx| {
                view.close_context_menu();
                window.focus(&view.focus_handle);
                cx.notify();
            }))
            .child({
                let sort_column = self.file_list.sort_state.column;
                let sort_direction = self.file_list.sort_state.direction;
                let entity = cx.entity().clone();
                let entity_date = entity.clone();
                let entity_type = entity.clone();
                let entity_size = entity.clone();
                
                // Header row with RPG styling
                div()
                    .flex()
                    .h(px(HEADER_HEIGHT))
                    .bg(bg_dark)
                    .border_b_1()
                    .border_color(border_color)
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(text_gray)
                    .child(
                        div()
                            .id("header-name")
                            .flex_1()
                            .px_4()
                            .flex()
                            .items_center()
                            .gap_1()
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg).text_color(text_light))
                            .when(sort_column == SortColumn::Name, |s| s.text_color(text_light))
                            .on_click(move |_event, _window, cx| {
                                entity.update(cx, |view, cx| {
                                    view.file_list.toggle_sort_column(SortColumn::Name);
                                    cx.notify();
                                });
                            })
                            .child("NAME")
                            .child(render_sort_indicator(SortColumn::Name, sort_column, sort_direction, text_gray, text_light)),
                    )
                    .child(
                        div()
                            .id("header-date")
                            .w(px(120.0))
                            .px_4()
                            .flex()
                            .items_center()
                            .gap_1()
                            .border_l_1()
                            .border_color(border_subtle)
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg).text_color(text_light))
                            .when(sort_column == SortColumn::Date, |s| s.text_color(text_light))
                            .on_click(move |_event, _window, cx| {
                                entity_date.update(cx, |view, cx| {
                                    view.file_list.toggle_sort_column(SortColumn::Date);
                                    cx.notify();
                                });
                            })
                            .child("DATE")
                            .child(render_sort_indicator(SortColumn::Date, sort_column, sort_direction, text_gray, text_light)),
                    )
                    .child(
                        div()
                            .id("header-type")
                            .w(px(100.0))
                            .px_4()
                            .flex()
                            .items_center()
                            .gap_1()
                            .border_l_1()
                            .border_color(border_subtle)
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg).text_color(text_light))
                            .when(sort_column == SortColumn::Type, |s| s.text_color(text_light))
                            .on_click(move |_event, _window, cx| {
                                entity_type.update(cx, |view, cx| {
                                    view.file_list.toggle_sort_column(SortColumn::Type);
                                    cx.notify();
                                });
                            })
                            .child("TYPE")
                            .child(render_sort_indicator(SortColumn::Type, sort_column, sort_direction, text_gray, text_light)),
                    )
                    .child(
                        div()
                            .id("header-size")
                            .w(px(80.0))
                            .px_4()
                            .flex()
                            .items_center()
                            .gap_1()
                            .border_l_1()
                            .border_color(border_subtle)
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg).text_color(text_light))
                            .when(sort_column == SortColumn::Size, |s| s.text_color(text_light))
                            .on_click(move |_event, _window, cx| {
                                entity_size.update(cx, |view, cx| {
                                    view.file_list.toggle_sort_column(SortColumn::Size);
                                    cx.notify();
                                });
                            })
                            .child("SIZE")
                            .child(render_sort_indicator(SortColumn::Size, sort_column, sort_direction, text_gray, text_light)),
                    )
            })
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when(total_items == 0, |this| {
                        this.flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .text_color(text_gray)
                                    .child(
                                        svg()
                                            .path("assets/icons/folder-open.svg")
                                            .size(px(48.0))
                                            .text_color(border_subtle)
                                            .mb_4(),
                                    )
                                    .child("Folder is empty"),
                            )
                    })
                    .when(total_items > 0, |this| {
                        let entity = cx.entity().clone();
                        this.child(
                            uniform_list(
                                "file-list-items",
                                total_items,
                                cx.processor(move |view, range, _window, _cx| {
                                    let mut items = Vec::new();
                                    for ix in range {
                                        let (entry, match_positions) = if let Some(filtered) = view.file_list.get_filtered_entry(ix) {
                                            (filtered.entry.clone(), Some(filtered.match_positions.clone()))
                                        } else if let Some(entry) = view.file_list.entries.get(ix) {
                                            (entry.clone(), None)
                                        } else {
                                            continue;
                                        };
                                        
                                        let is_selected = selected_index == Some(ix);
                                        let is_dir = entry.is_dir;
                                        let name = entry.name.clone();
                                        let size = format_size(entry.size, entry.is_dir);
                                        let date = format_date(entry.modified);
                                        let file_type = if is_dir {
                                            "Folder".to_string()
                                        } else {
                                            get_file_type(&name)
                                        };
                                        let icon_name = get_file_icon(&name, is_dir);
                                        let icon_color = if is_dir { 
                                            if is_selected { folder_open_color } else { folder_color }
                                        } else { 
                                            get_file_icon_color(&name) 
                                        };
                                        let sync_status = entry.sync_status;
                                        let entry_path = entry.path.clone();
                                        let entity = entity.clone();
                                        let entity_for_ctx = entity.clone();

                                        // RPG-styled file row with 40px height, hover glow, themed selection
                                        items.push(
                                                div()
                                                    .id(SharedString::from(format!("file-{}", ix)))
                                                    .h(px(row_height))
                                                    .w_full()
                                                    .flex()
                                                    .items_center()
                                                    .text_sm()
                                                    .cursor_pointer()
                                                    .border_b_1()
                                                    .border_color(border_subtle)
                                                    // Selected state with themed accent color
                                                    .when(is_selected, |s| s
                                                        .bg(selected_bg)
                                                        .border_l_2()
                                                        .border_color(accent_primary)
                                                    )
                                                    // Hover state with subtle glow effect
                                                    .when(!is_selected, |s| s.hover(|h| h
                                                        .bg(hover_bg)
                                                    ))
                                                    .on_click({
                                                        let entry_path = entry_path.clone();
                                                        let entity = entity.clone();
                                                        move |event, _window, cx| {
                                                            entity.update(cx, |view, cx| {
                                                                view.close_context_menu();
                                                                if event.click_count() == 2 && is_dir {
                                                                    view.pending_navigation = Some(entry_path.clone());
                                                                } else {
                                                                    view.file_list.selected_index = Some(ix);
                                                                }
                                                                cx.notify();
                                                            });
                                                        }
                                                    })
                                                    .on_mouse_down(MouseButton::Right, {
                                                        let entity = entity_for_ctx.clone();
                                                        move |event: &MouseDownEvent, _window, cx| {
                                                            entity.update(cx, |view, cx| {
                                                                view.file_list.selected_index = Some(ix);
                                                                view.context_menu_position = Some(event.position);
                                                                view.context_menu_index = Some(ix);
                                                                cx.notify();
                                                            });
                                                        }
                                                    })
                                                    .when(is_dir, |d| {
                                                        let drag_path = entry_path.clone();
                                                        let drag_name = name.clone();
                                                        d.on_drag(DraggedFolder { path: drag_path, name: drag_name }, |folder, _, _, cx| {
                                                            cx.new(|_| DraggedFolderView { name: folder.name.clone() })
                                                        })
                                                    })
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .px(px(ROW_PADDING_X))
                                                            .flex()
                                                            .items_center()
                                                            .overflow_hidden()
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap(px(ICON_GAP))
                                                                    .child(
                                                                        svg()
                                                                            .path(SharedString::from(format!("assets/icons/{}.svg", icon_name)))
                                                                            .size(px(ICON_SIZE))
                                                                            .text_color(icon_color)
                                                                            .flex_shrink_0(),
                                                                    )
                                                                    .child(
                                                                        render_highlighted_name(
                                                                            &name,
                                                                            match_positions.as_ref(),
                                                                            is_selected,
                                                                            text_light,
                                                                            accent_primary,
                                                                        ),
                                                                    )
                                                                    // Cloud sync status indicator
                                                                    .when(sync_status.icon_name().is_some(), |s| {
                                                                        let icon = sync_status.icon_name().unwrap_or("check");
                                                                        let color = sync_status.color().unwrap_or(0x8b949e);
                                                                        s.child(
                                                                            div()
                                                                                .ml_2()
                                                                                .flex()
                                                                                .items_center()
                                                                                .child(
                                                                                    svg()
                                                                                        .path(SharedString::from(format!("assets/icons/{}.svg", icon)))
                                                                                        .size(px(12.0))
                                                                                        .text_color(gpui::rgb(color))
                                                                                )
                                                                        )
                                                                    }),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(120.0))
                                                            .px_4()
                                                            .text_xs()
                                                            .text_color(text_gray)
                                                            .truncate()
                                                            .child(date),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(100.0))
                                                            .px_4()
                                                            .text_xs()
                                                            .text_color(text_gray)
                                                            .truncate()
                                                            .child(file_type),
                                                    )
                                                    .child(
                                                        div()
                                                            .w(px(80.0))
                                                            .px_4()
                                                            .text_xs()
                                                            .text_color(text_gray)
                                                            .font_family("Mono")
                                                            .truncate()
                                                            .child(size),
                                                    ),
                                            );
                                    }
                                    items
                                }),
                            )
                            .size_full()
                            .track_scroll(self.scroll_handle.clone()),
                        )
                    }),
            )
            // Footer/status bar with RPG styling
            .child(
                div()
                    .h(px(FOOTER_HEIGHT))
                    .bg(bg_dark)
                    .border_t_1()
                    .border_color(border_color)
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(ROW_PADDING_X))
                    .text_xs()
                    .text_color(text_gray)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                svg()
                                    .path("assets/icons/files.svg")
                                    .size(px(12.0))
                                    .text_color(text_gray),
                            )
                            .child(format!("{} items", total_items)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child("UTF-8")
                            .child(
                                div()
                                    .w(px(1.0))
                                    .h(px(12.0))
                                    .bg(border_subtle),
                            )
                            .child("List View"),
                    ),
            )
            .when_some(context_menu_pos, |this, pos| {
                let entity = cx.entity().clone();
                let selected_entry = context_menu_idx.and_then(|idx| self.file_list.entries.get(idx).cloned());
                let is_dir = selected_entry.as_ref().map(|e| e.is_dir).unwrap_or(false);
                
                this.child(
                    anchored()
                        .snap_to_window_with_margin(px(8.0))
                        .anchor(Corner::TopLeft)
                        .position(pos)
                        .child(
                            div()
                                .id("file-list-context-menu")
                                .occlude()
                                .w(px(220.0))
                                .bg(menu_bg)
                                .border_1()
                                .border_color(border_color)
                                .rounded_lg()
                                .shadow_lg()
                                .py_1()
                                .on_mouse_down_out(cx.listener(|view, _, _, cx| {
                                    view.close_context_menu();
                                    cx.notify();
                                }))
                                .child(render_context_menu_item("folder-open", "Open", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::Open(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_open_with_submenu(
                                    selected_entry.clone(),
                                    self.show_open_with_submenu,
                                    text_light,
                                    hover_bg,
                                    menu_bg,
                                    border_color,
                                    entity.clone(),
                                    cx,
                                ))
                                .when(is_dir, |this| {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    this.child(render_context_menu_item("app-window", "Open in New Window", text_light, hover_bg, {
                                        move |_window, cx| {
                                            if let Some(ref e) = entry {
                                                entity.update(cx, |view, cx| {
                                                    view.pending_context_action = Some(ContextMenuAction::OpenInNewWindow(e.path.clone()));
                                                    view.close_context_menu();
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    }))
                                })
                                .child(render_context_menu_divider(border_subtle))
                                .child(render_context_menu_item("eye", "Quick Look", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::QuickLook(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_context_menu_item("info", "Get Info", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::GetInfo(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_context_menu_divider(border_subtle))
                                .child(render_context_menu_item("pen", "Rename", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::Rename(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_context_menu_item("copy", "Copy", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::Copy(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_context_menu_item("scissors", "Cut", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::Cut(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_context_menu_item("clipboard-paste", "Paste", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    move |_window, cx| {
                                        entity.update(cx, |view, cx| {
                                            view.pending_context_action = Some(ContextMenuAction::Paste);
                                            view.close_context_menu();
                                            cx.notify();
                                        });
                                    }
                                }))
                                .child(render_context_menu_item("files", "Duplicate", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::Duplicate(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_context_menu_divider(border_subtle))
                                .child(render_context_menu_item("archive", "Compress", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::Compress(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_context_menu_item("share-2", "Share...", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::Share(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_context_menu_divider(border_subtle))
                                .child(render_context_menu_item("link", "Copy Path", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::CopyPath(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_context_menu_item("folder-search", "Show in Finder", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::ShowInFinder(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_context_menu_item("star", "Add to Favorites", text_light, hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::AddToFavorites(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                }))
                                .child(render_context_menu_divider(border_subtle))
                                .child(render_context_menu_item("trash-2", "Move to Trash", gpui::rgb(0xf85149), hover_bg, {
                                    let entity = entity.clone();
                                    let entry = selected_entry.clone();
                                    move |_window, cx| {
                                        if let Some(ref e) = entry {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::MoveToTrash(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    }
                                })),
                        )
                )
            })
    }
}

fn render_sort_indicator(
    column: SortColumn,
    active_column: SortColumn,
    direction: SortDirection,
    inactive_color: gpui::Rgba,
    active_color: gpui::Rgba,
) -> impl IntoElement {
    let is_active = column == active_column;
    let icon_name = if is_active {
        match direction {
            SortDirection::Ascending => "chevron-up",
            SortDirection::Descending => "chevron-down",
        }
    } else {
        "arrow-down-up"
    };
    
    svg()
        .path(SharedString::from(format!("assets/icons/{}.svg", icon_name)))
        .size(px(12.0))
        .text_color(if is_active { active_color } else { inactive_color })
        .when(!is_active, |s| s.opacity(0.5))
}

fn render_highlighted_name(
    name: &str,
    match_positions: Option<&Vec<usize>>,
    is_selected: bool,
    text_light: gpui::Rgba,
    accent_color: gpui::Rgba,
) -> impl IntoElement {
    // Use theme accent color for search highlights
    let highlight_color = accent_color;
    let text_color = if is_selected { gpui::rgb(0xffffff) } else { text_light };
    let font_weight = if is_selected { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL };

    match match_positions {
        Some(positions) if !positions.is_empty() => {
            // Render with highlights
            let chars: Vec<char> = name.chars().collect();
            let mut elements: Vec<gpui::AnyElement> = Vec::new();
            let mut current_segment = String::new();
            let mut in_highlight = false;

            for (i, ch) in chars.iter().enumerate() {
                let should_highlight = positions.contains(&i);
                
                if should_highlight != in_highlight {
                    // Flush current segment
                    if !current_segment.is_empty() {
                        if in_highlight {
                            elements.push(
                                div()
                                    .text_color(highlight_color)
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(current_segment.clone())
                                    .into_any_element()
                            );
                        } else {
                            elements.push(
                                div()
                                    .text_color(text_color)
                                    .font_weight(font_weight)
                                    .child(current_segment.clone())
                                    .into_any_element()
                            );
                        }
                        current_segment.clear();
                    }
                    in_highlight = should_highlight;
                }
                current_segment.push(*ch);
            }

            // Flush remaining segment
            if !current_segment.is_empty() {
                if in_highlight {
                    elements.push(
                        div()
                            .text_color(highlight_color)
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(current_segment)
                            .into_any_element()
                    );
                } else {
                    elements.push(
                        div()
                            .text_color(text_color)
                            .font_weight(font_weight)
                            .child(current_segment)
                            .into_any_element()
                    );
                }
            }

            div()
                .flex()
                .truncate()
                .children(elements)
                .into_any_element()
        }
        _ => {
            // No highlights, render normally
            div()
                .text_color(text_color)
                .font_weight(font_weight)
                .truncate()
                .child(name.to_string())
                .into_any_element()
        }
    }
}

fn render_context_menu_item<F>(
    icon_name: &'static str, 
    label: &'static str, 
    text_color: gpui::Rgba, 
    hover_bg: gpui::Rgba,
    on_click: F,
) -> impl IntoElement 
where
    F: Fn(&mut Window, &mut App) + 'static,
{
    div()
        .id(SharedString::from(format!("ctx-{}", label)))
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .py_1p5()
        .mx_1()
        .rounded_md()
        .cursor_pointer()
        .text_sm()
        .text_color(text_color)
        .hover(|s| s.bg(hover_bg))
        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
            on_click(window, cx);
        })
        .child(
            svg()
                .path(SharedString::from(format!("assets/icons/{}.svg", icon_name)))
                .size(px(14.0))
                .text_color(text_color),
        )
        .child(label)
}

fn render_context_menu_divider(color: gpui::Rgba) -> impl IntoElement {
    div()
        .h(px(1.0))
        .mx_2()
        .my_1()
        .bg(color)
}

fn render_open_with_submenu(
    selected_entry: Option<FileEntry>,
    show_submenu: bool,
    text_color: gpui::Rgba,
    hover_bg: gpui::Rgba,
    menu_bg: gpui::Rgba,
    border_color: gpui::Rgba,
    entity: gpui::Entity<FileListView>,
    _cx: &mut Context<FileListView>,
) -> impl IntoElement {
    let apps = selected_entry.as_ref()
        .map(|e| crate::models::get_apps_for_file(&e.path))
        .unwrap_or_default();
    
    let has_apps = !apps.is_empty();
    let entry_for_other = selected_entry.clone();
    let entity_for_show = entity.clone();
    let entity_for_hide = entity.clone();
    
    div()
        .id("open-with-menu-wrapper")
        .flex()
        .child(
            div()
                .id("open-with-menu")
                .flex_1()
                .on_mouse_move(move |_event, _window, cx| {
                    entity_for_show.update(cx, |view, cx| {
                        if !view.show_open_with_submenu {
                            view.show_open_with_submenu = true;
                            cx.notify();
                        }
                    });
                })
                .child(
                    div()
                        .id("open-with-trigger")
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .px_3()
                        .py_1p5()
                        .mx_1()
                        .rounded_md()
                        .cursor_pointer()
                        .text_sm()
                        .text_color(text_color)
                        .hover(|s| s.bg(hover_bg))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    svg()
                                        .path("assets/icons/external-link.svg")
                                        .size(px(14.0))
                                        .text_color(text_color),
                                )
                                .child("Open With")
                        )
                        .child(
                            svg()
                                .path("assets/icons/chevron-right.svg")
                                .size(px(12.0))
                                .text_color(text_color),
                        )
                )
        )
        .when(show_submenu, move |this| {
            this.child(
                div()
                    .id("open-with-submenu")
                    .on_hover(move |is_hovered, _window, cx| {
                        if !*is_hovered {
                            entity_for_hide.update(cx, |view, cx| {
                                view.show_open_with_submenu = false;
                                cx.notify();
                            });
                        }
                    })
                    .child(
                        div()
                            .w(px(220.0))
                            .bg(menu_bg)
                            .border_1()
                            .border_color(border_color)
                            .rounded_lg()
                            .shadow_lg()
                            .py_1()
                            .when(has_apps, |submenu| {
                                let mut submenu = submenu;
                                for app in apps.iter().take(10) {
                                    let app_name = app.name.clone();
                                    let app_path = app.path.clone();
                                    let file_path = selected_entry.as_ref().map(|e| e.path.clone());
                                    let entity = entity.clone();
                                    
                                    submenu = submenu.child(
                                        div()
                                            .id(SharedString::from(format!("app-{}", app_name)))
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .px_3()
                                            .py_1p5()
                                            .mx_1()
                                            .rounded_md()
                                            .cursor_pointer()
                                            .text_sm()
                                            .text_color(text_color)
                                            .hover(|s| s.bg(hover_bg))
                                            .on_mouse_down(MouseButton::Left, {
                                                let app_name = app_name.clone();
                                                let app_path = app_path.clone();
                                                move |_event, _window, cx| {
                                                    if let Some(ref fp) = file_path {
                                                        entity.update(cx, |view, cx| {
                                                            view.pending_context_action = Some(ContextMenuAction::OpenWithApp {
                                                                file_path: fp.clone(),
                                                                app_path: app_path.clone(),
                                                                app_name: app_name.clone(),
                                                            });
                                                            view.close_context_menu();
                                                            cx.notify();
                                                        });
                                                    }
                                                }
                                            })
                                            .child(
                                                svg()
                                                    .path("assets/icons/app-window.svg")
                                                    .size(px(16.0))
                                                    .text_color(text_color)
                                            )
                                            .child(app_name)
                                    );
                                }
                                submenu
                            })
                            .when(has_apps, |submenu| {
                                submenu.child(render_context_menu_divider(border_color))
                            })
                            .child({
                                let entity = entity.clone();
                                div()
                                    .id("open-with-other")
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py_1p5()
                                    .mx_1()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_sm()
                                    .text_color(text_color)
                                    .hover(|s| s.bg(hover_bg))
                                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                        if let Some(ref e) = entry_for_other {
                                            entity.update(cx, |view, cx| {
                                                view.pending_context_action = Some(ContextMenuAction::OpenWithOther(e.path.clone()));
                                                view.close_context_menu();
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .child(
                                        div()
                                            .size(px(18.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                svg()
                                                    .path("assets/icons/more-horizontal.svg")
                                                    .size(px(16.0))
                                                    .text_color(text_color),
                                            )
                                    )
                                    .child("Other...")
                            })
                    )
            )
        })
}

fn get_file_type(name: &str) -> String {
    if let Some(ext) = name.rsplit('.').next() {
        if ext != name {
            return ext.to_uppercase();
        }
    }
    "File".to_string()
}

pub fn get_file_icon(name: &str, is_dir: bool) -> &'static str {
    if is_dir {
        if name.starts_with('.') {
            return "folder-cog";
        }
        return match name.to_lowercase().as_str() {
            "src" | "source" => "folder-code",
            "node_modules" | "vendor" | "packages" => "folder-archive",
            "test" | "tests" | "__tests__" | "spec" => "folder-check",
            "docs" | "documentation" => "folder-open",
            ".git" => "folder-git",
            "build" | "dist" | "target" | "out" => "folder-output",
            "assets" | "images" | "img" | "icons" => "folder-heart",
            "config" | "configs" | ".config" => "folder-cog",
            _ => "folder",
        };
    }
    
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext.to_lowercase().as_str() {
        "rs" => "file-code",
        "ts" | "tsx" => "file-code",
        "js" | "jsx" | "mjs" => "file-code",
        "py" => "file-code",
        "go" => "file-code",
        "java" | "kt" | "scala" => "file-code",
        "c" | "cpp" | "h" | "hpp" => "file-code",
        "html" | "htm" => "file-code",
        "css" | "scss" | "sass" | "less" => "file-code",
        "json" => "file-json",
        "yaml" | "yml" => "file-cog",
        "toml" => "file-cog",
        "xml" => "file-code",
        "md" | "markdown" => "file-text",
        "txt" => "file-text",
        "pdf" => "file-text",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" => "file-image",
        "mp3" | "wav" | "ogg" | "flac" => "file-audio",
        "mp4" | "mov" | "avi" | "mkv" | "webm" => "file-video-camera",
        "zip" | "tar" | "gz" | "rar" | "7z" => "file-archive",
        "lock" => "file-lock",
        "env" => "file-key",
        "log" => "file-text",
        "sh" | "bash" | "zsh" => "file-terminal",
        _ => "file",
    }
}

pub fn get_file_icon_color(name: &str) -> gpui::Rgba {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext.to_lowercase().as_str() {
        "rs" => gpui::rgb(0xdea584),
        "ts" | "tsx" => gpui::rgb(0x3178c6),
        "js" | "jsx" | "mjs" => gpui::rgb(0xf7df1e),
        "py" => gpui::rgb(0x3776ab),
        "go" => gpui::rgb(0x00add8),
        "java" => gpui::rgb(0xb07219),
        "json" => gpui::rgb(0xf5a623),
        "yaml" | "yml" => gpui::rgb(0xcb171e),
        "toml" => gpui::rgb(0x9c4221),
        "md" | "markdown" => gpui::rgb(0x519aba),
        "html" | "htm" => gpui::rgb(0xe34c26),
        "css" | "scss" | "sass" => gpui::rgb(0x563d7c),
        "png" | "jpg" | "jpeg" | "gif" | "svg" => gpui::rgb(0xa855f7),
        "zip" | "tar" | "gz" => gpui::rgb(0xf59e0b),
        _ => gpui::rgb(0x8b949e),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct RenderedEntry {
    pub name: String,
    pub formatted_size: String,
    pub formatted_date: String,
    pub icon_key: IconKey,
    pub is_dir: bool,
    pub highlight_positions: Vec<usize>,
}

impl FileList {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            filtered_entries: None,
            row_height: DEFAULT_ROW_HEIGHT,
            buffer_size: DEFAULT_BUFFER_SIZE,
            scroll_offset: 0.0,
            viewport_height: 0.0,
            highlight_positions: None,
            selected_index: None,
            search_query: String::new(),
            sort_state: SortState::new(),
        }
    }

    pub fn with_config(row_height: f32, buffer_size: usize) -> Self {
        Self {
            entries: Vec::new(),
            filtered_entries: None,
            row_height: row_height.max(1.0),
            buffer_size,
            scroll_offset: 0.0,
            viewport_height: 0.0,
            highlight_positions: None,
            selected_index: None,
            search_query: String::new(),
            sort_state: SortState::new(),
        }
    }

    pub fn item_count(&self) -> usize {
        if let Some(filtered) = &self.filtered_entries {
            filtered.len()
        } else {
            self.entries.len()
        }
    }

    pub fn is_filtered(&self) -> bool {
        self.filtered_entries.is_some()
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn row_height(&self) -> f32 {
        self.row_height
    }

    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    pub fn viewport_height(&self) -> f32 {
        self.viewport_height
    }

    pub fn set_entries(&mut self, entries: Vec<FileEntry>) {
        self.entries = entries;
        self.sort_state.sort_entries(&mut self.entries);
        self.filtered_entries = None;
        self.highlight_positions = None;
        self.selected_index = None;
        self.search_query.clear();
    }

    pub fn sort_state(&self) -> &SortState {
        &self.sort_state
    }

    pub fn sort_state_mut(&mut self) -> &mut SortState {
        &mut self.sort_state
    }

    pub fn toggle_sort_column(&mut self, column: SortColumn) {
        self.sort_state.toggle_column(column);
        self.sort_state.sort_entries(&mut self.entries);
    }

    pub fn apply_sort(&mut self) {
        self.sort_state.sort_entries(&mut self.entries);
    }

    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    /// Returns the currently selected index
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Sets the selected index
    pub fn set_selected_index(&mut self, index: Option<usize>) {
        self.selected_index = index;
    }

    /// Returns the currently visible entries (filtered if search is active)
    pub fn visible_entries(&self) -> Vec<&FileEntry> {
        if let Some(filtered) = &self.filtered_entries {
            filtered.iter().map(|f| &f.entry).collect()
        } else {
            self.entries.iter().collect()
        }
    }

    /// Get entry at display index (accounts for filtering)
    pub fn get_display_entry(&self, display_index: usize) -> Option<&FileEntry> {
        if let Some(filtered) = &self.filtered_entries {
            filtered.get(display_index).map(|f| &f.entry)
        } else {
            self.entries.get(display_index)
        }
    }

    /// Get filtered entry with match positions at display index
    pub fn get_filtered_entry(&self, display_index: usize) -> Option<&FilteredEntry> {
        self.filtered_entries.as_ref()?.get(display_index)
    }

    /// Apply search filter using nucleo fuzzy matching results
    pub fn apply_search_filter(&mut self, query: &str, matches: Vec<(usize, Vec<usize>, u32)>) {
        self.search_query = query.to_string();
        
        if query.is_empty() {
            self.clear_search_filter();
            return;
        }

        let filtered: Vec<FilteredEntry> = matches
            .into_iter()
            .filter_map(|(original_index, positions, score)| {
                self.entries.get(original_index).map(|entry| FilteredEntry {
                    original_index,
                    entry: entry.clone(),
                    match_positions: positions,
                    score,
                })
            })
            .collect();

        self.filtered_entries = Some(filtered);
        self.selected_index = None;
        self.scroll_offset = 0.0;
    }

    /// Clear search filter and show all entries
    pub fn clear_search_filter(&mut self) {
        self.filtered_entries = None;
        self.search_query.clear();
        self.selected_index = None;
    }

    /// Get match positions for a display index (for highlighting)
    pub fn get_match_positions(&self, display_index: usize) -> Option<&[usize]> {
        self.filtered_entries
            .as_ref()?
            .get(display_index)
            .map(|f| f.match_positions.as_slice())
    }

    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.scroll_offset = offset.max(0.0);
    }

    pub fn set_viewport_height(&mut self, height: f32) {
        self.viewport_height = height.max(0.0);
    }

    pub fn set_highlight_positions(&mut self, positions: Option<Vec<Vec<usize>>>) {
        self.highlight_positions = positions;
    }

    pub fn calculate_visible_range(&self) -> VisibleRange {
        let total_items = self.entries.len();

        if total_items == 0 || self.viewport_height <= 0.0 || self.row_height <= 0.0 {
            return VisibleRange { start: 0, end: 0 };
        }

        let start_raw = (self.scroll_offset / self.row_height).floor() as usize;
        let start = start_raw.saturating_sub(self.buffer_size);

        let visible_rows = (self.viewport_height / self.row_height).ceil() as usize;
        let end_raw = start_raw + visible_rows + self.buffer_size;
        let end = end_raw.min(total_items);

        VisibleRange { start, end }
    }

    pub fn max_rendered_items(&self) -> usize {
        if self.viewport_height <= 0.0 || self.row_height <= 0.0 {
            return 0;
        }

        let visible_rows = (self.viewport_height / self.row_height).ceil() as usize;
        visible_rows + (self.buffer_size * 2)
    }

    pub fn render_item(&self, index: usize) -> Option<RenderedEntry> {
        let entry = self.entries.get(index)?;

        let highlight_positions = self
            .highlight_positions
            .as_ref()
            .and_then(|positions| positions.get(index))
            .cloned()
            .unwrap_or_default();

        Some(RenderedEntry {
            name: entry.name.clone(),
            formatted_size: format_size(entry.size, entry.is_dir),
            formatted_date: format_date(entry.modified),
            icon_key: entry.icon_key.clone(),
            is_dir: entry.is_dir,
            highlight_positions,
        })
    }

    pub fn render_visible_items(&self) -> Vec<(usize, RenderedEntry)> {
        let range = self.calculate_visible_range();

        (range.start..range.end)
            .filter_map(|i| self.render_item(i).map(|entry| (i, entry)))
            .collect()
    }
}

impl Default for FileList {
    fn default() -> Self {
        Self::new()
    }
}

impl VisibleRange {
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    pub fn contains(&self, index: usize) -> bool {
        index >= self.start && index < self.end
    }
}

impl RenderedEntry {
    pub fn is_highlighted(&self, char_index: usize) -> bool {
        self.highlight_positions.contains(&char_index)
    }

    pub fn name_with_highlights(&self) -> Vec<(char, bool)> {
        self.name
            .chars()
            .enumerate()
            .map(|(i, c)| (c, self.highlight_positions.contains(&i)))
            .collect()
    }
}

pub fn format_size(size: u64, is_dir: bool) -> String {
    if is_dir {
        return "--".to_string();
    }

    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size < KB {
        format!("{} B", size)
    } else if size < MB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else if size < GB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size < TB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else {
        format!("{:.1} TB", size as f64 / TB as f64)
    }
}

pub fn format_date(time: SystemTime) -> String {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => {
            let secs = duration.as_secs();
            let days = secs / 86400;
            let years = 1970 + (days / 365);
            let remaining_days = days % 365;
            let month = (remaining_days / 30) + 1;
            let day = (remaining_days % 30) + 1;

            format!("{:04}-{:02}-{:02}", years, month.min(12), day.min(31))
        }
        Err(_) => "Unknown".to_string(),
    }
}

#[cfg(test)]
#[path = "file_list_tests.rs"]
mod tests;
