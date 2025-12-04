use std::path::PathBuf;

use gpui::{
    actions, div, prelude::*, px, svg, App, Context, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement, Pixels, Point, Render,
    SharedString, Styled, Window, MouseDownEvent,
};

use crate::models::{Column, ColumnView, FileEntry};
use super::file_list::{get_file_icon, get_file_icon_color};

// Define actions for keyboard navigation
actions!(column_view, [
    ColumnNavigateUp,
    ColumnNavigateDown,
    ColumnNavigateLeft,
    ColumnNavigateRight,
    ColumnOpenSelected,
]);

/// Actions for the column view (public structs for external use)
pub struct NavigateToPath(pub PathBuf);
pub struct SelectColumnEntry { pub column: usize, pub entry: usize }
pub struct NavigateUp;
pub struct NavigateDown;
pub struct NavigateLeft;
pub struct NavigateRight;

/// Component wrapper for ColumnView with GPUI integration
pub struct ColumnViewComponent {
    column_view: ColumnView,
    focus_handle: FocusHandle,
    pending_navigation: Option<PathBuf>,
    context_menu_position: Option<Point<Pixels>>,
    context_menu_column: Option<usize>,
    context_menu_entry: Option<usize>,
}

impl ColumnViewComponent {
    pub fn new(root: PathBuf, cx: &mut Context<Self>) -> Self {
        Self {
            column_view: ColumnView::new(root),
            focus_handle: cx.focus_handle(),
            pending_navigation: None,
            context_menu_position: None,
            context_menu_column: None,
            context_menu_entry: None,
        }
    }

    pub fn with_column_view(column_view: ColumnView, cx: &mut Context<Self>) -> Self {
        Self {
            column_view,
            focus_handle: cx.focus_handle(),
            pending_navigation: None,
            context_menu_position: None,
            context_menu_column: None,
            context_menu_entry: None,
        }
    }

    pub fn inner(&self) -> &ColumnView {
        &self.column_view
    }

    pub fn inner_mut(&mut self) -> &mut ColumnView {
        &mut self.column_view
    }

    pub fn take_pending_navigation(&mut self) -> Option<PathBuf> {
        self.pending_navigation.take()
    }

    pub fn close_context_menu(&mut self) {
        self.context_menu_position = None;
        self.context_menu_column = None;
        self.context_menu_entry = None;
    }

    /// Sets entries for a specific column
    pub fn set_column_entries(&mut self, column_index: usize, entries: Vec<FileEntry>, cx: &mut Context<Self>) {
        self.column_view.set_column_entries(column_index, entries);
        cx.notify();
    }

    /// Handles selection of an entry in a column
    pub fn select_entry(&mut self, column_index: usize, entry_index: usize, cx: &mut Context<Self>) {
        self.column_view.select(column_index, entry_index);
        cx.notify();
    }

    /// Opens the selected entry (navigates into directory)
    pub fn open_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(entry) = self.column_view.selected_entry() {
            if entry.is_dir {
                self.pending_navigation = Some(entry.path.clone());
                cx.notify();
            }
        }
    }

    /// Handles keyboard navigation
    pub fn handle_key_up(&mut self, cx: &mut Context<Self>) {
        self.column_view.navigate_up();
        cx.notify();
    }

    pub fn handle_key_down(&mut self, cx: &mut Context<Self>) {
        self.column_view.navigate_down();
        cx.notify();
    }

    pub fn handle_key_left(&mut self, cx: &mut Context<Self>) {
        self.column_view.navigate_left();
        cx.notify();
    }

    pub fn handle_key_right(&mut self, cx: &mut Context<Self>) {
        if self.column_view.navigate_right() {
            cx.notify();
        } else {
            // If can't navigate right, try to open the selected directory
            self.open_selected(cx);
        }
    }

    /// Sets the root path and resets the view
    pub fn set_root(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.column_view.set_root(path);
        cx.notify();
    }

    /// Register key bindings for column view navigation
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("up", ColumnNavigateUp, Some("ColumnView")),
            KeyBinding::new("down", ColumnNavigateDown, Some("ColumnView")),
            KeyBinding::new("left", ColumnNavigateLeft, Some("ColumnView")),
            KeyBinding::new("right", ColumnNavigateRight, Some("ColumnView")),
            KeyBinding::new("enter", ColumnOpenSelected, Some("ColumnView")),
        ]);
    }

    /// Handle up arrow key - move selection up in current column
    fn handle_navigate_up(&mut self, _: &ColumnNavigateUp, _window: &mut Window, cx: &mut Context<Self>) {
        self.column_view.navigate_up();
        cx.notify();
    }

    /// Handle down arrow key - move selection down in current column
    fn handle_navigate_down(&mut self, _: &ColumnNavigateDown, _window: &mut Window, cx: &mut Context<Self>) {
        self.column_view.navigate_down();
        cx.notify();
    }

    /// Handle left arrow key - move to parent column
    fn handle_navigate_left(&mut self, _: &ColumnNavigateLeft, _window: &mut Window, cx: &mut Context<Self>) {
        self.column_view.navigate_left();
        cx.notify();
    }

    /// Handle right arrow key - move into selected directory
    fn handle_navigate_right(&mut self, _: &ColumnNavigateRight, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.column_view.navigate_right() {
            // If can't navigate right (no child column), try to open the selected directory
            if let Some(entry) = self.column_view.selected_entry() {
                if entry.is_dir {
                    self.pending_navigation = Some(entry.path.clone());
                }
            }
        }
        cx.notify();
    }

    /// Handle enter key - open selected item
    fn handle_open_selected(&mut self, _: &ColumnOpenSelected, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.column_view.selected_entry() {
            if entry.is_dir {
                self.pending_navigation = Some(entry.path.clone());
                cx.notify();
            }
        }
    }
}

impl Focusable for ColumnViewComponent {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ColumnViewComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let columns = self.column_view.columns().to_vec();
        let column_width = self.column_view.column_width();
        let context_menu_pos = self.context_menu_position;

        let bg_darker = gpui::rgb(0x010409);
        let bg_dark = gpui::rgb(0x0d1117);
        let border_color = gpui::rgb(0x30363d);
        let border_subtle = gpui::rgb(0x21262d);
        let text_gray = gpui::rgb(0x8b949e);
        let text_light = gpui::rgb(0xc9d1d9);
        let hover_bg = gpui::rgb(0x161b22);
        let selected_bg = gpui::rgb(0x1f3a5f);
        let folder_color = gpui::rgb(0x54aeff);
        let folder_open_color = gpui::rgb(0x79c0ff);
        let menu_bg = gpui::rgb(0x161b22);

        let entity = cx.entity().clone();

        div()
            .id("column-view")
            .key_context("ColumnView")
            .size_full()
            .bg(bg_darker)
            .flex()
            .flex_col()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_navigate_up))
            .on_action(cx.listener(Self::handle_navigate_down))
            .on_action(cx.listener(Self::handle_navigate_left))
            .on_action(cx.listener(Self::handle_navigate_right))
            .on_action(cx.listener(Self::handle_open_selected))
            .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                view.close_context_menu();
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .h(px(36.0))
                    .bg(bg_dark)
                    .border_b_1()
                    .border_color(border_color)
                    .px_4()
                    .items_center()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(text_gray)
                    .child(format!("{} columns", columns.len()))
            )
            // Columns container with horizontal scroll
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .h_full()
                            .children(
                                columns.iter().enumerate().map(|(col_idx, column)| {
                                    render_column(
                                        col_idx,
                                        column,
                                        column_width,
                                        entity.clone(),
                                        bg_dark,
                                        border_color,
                                        text_gray,
                                        text_light,
                                        hover_bg,
                                        selected_bg,
                                        folder_color,
                                        folder_open_color,
                                    )
                                })
                            )
                    )
            )
            .child(
                div()
                    .h(px(28.0))
                    .bg(bg_dark)
                    .border_t_1()
                    .border_color(border_color)
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .text_xs()
                    .text_color(text_gray)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                svg()
                                    .path("assets/icons/columns-3.svg")
                                    .size(px(12.0))
                                    .text_color(text_gray),
                            )
                            .child("Column View"),
                    )
            )
            .when_some(context_menu_pos, |this, pos| {
                let entity = cx.entity().clone();
                let selected_entry = self.context_menu_column
                    .and_then(|col| self.context_menu_entry.map(|entry| (col, entry)))
                    .and_then(|(col, entry)| {
                        self.column_view.column(col)
                            .and_then(|c| c.entries.get(entry).cloned())
                    });
                
                this.child(
                    div()
                        .absolute()
                        .left(pos.x)
                        .top(pos.y)
                        .w(px(200.0))
                        .bg(menu_bg)
                        .border_1()
                        .border_color(border_color)
                        .rounded_lg()
                        .shadow_lg()
                        .py_1()
                        .child(render_context_menu_item("folder-open", "Open", text_light, hover_bg, {
                            let entity = entity.clone();
                            let entry = selected_entry.clone();
                            move |_window, cx| {
                                if let Some(ref e) = entry {
                                    if e.is_dir {
                                        entity.update(cx, |view, cx| {
                                            view.pending_navigation = Some(e.path.clone());
                                            view.close_context_menu();
                                            cx.notify();
                                        });
                                    }
                                }
                            }
                        }))
                        .child(render_context_menu_divider(border_subtle))
                        .child(render_context_menu_item("copy", "Copy", text_light, hover_bg, {
                            let entity = entity.clone();
                            move |_window, cx| {
                                entity.update(cx, |view, cx| {
                                    view.close_context_menu();
                                    cx.notify();
                                });
                            }
                        }))
                        .child(render_context_menu_item("clipboard-paste", "Paste", text_light, hover_bg, {
                            let entity = entity.clone();
                            move |_window, cx| {
                                entity.update(cx, |view, cx| {
                                    view.close_context_menu();
                                    cx.notify();
                                });
                            }
                        }))
                        .child(render_context_menu_item("pen", "Rename", text_light, hover_bg, {
                            let entity = entity.clone();
                            move |_window, cx| {
                                entity.update(cx, |view, cx| {
                                    view.close_context_menu();
                                    cx.notify();
                                });
                            }
                        }))
                        .child(render_context_menu_divider(border_subtle))
                        .child(render_context_menu_item("trash-2", "Delete", gpui::rgb(0xf85149), hover_bg, {
                            let entity = entity.clone();
                            move |_window, cx| {
                                entity.update(cx, |view, cx| {
                                    view.close_context_menu();
                                    cx.notify();
                                });
                            }
                        })),
                )
            })
    }
}

fn render_column(
    col_idx: usize,
    column: &Column,
    column_width: f32,
    entity: gpui::Entity<ColumnViewComponent>,
    bg_dark: gpui::Rgba,
    border_color: gpui::Rgba,
    text_gray: gpui::Rgba,
    text_light: gpui::Rgba,
    hover_bg: gpui::Rgba,
    selected_bg: gpui::Rgba,
    folder_color: gpui::Rgba,
    folder_open_color: gpui::Rgba,
) -> impl IntoElement {
    let entries = column.entries.clone();
    let selected_index = column.selected_index;
    let path = column.path.clone();
    let column_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("/")
        .to_string();

    div()
        .id(SharedString::from(format!("column-{}", col_idx)))
        .w(px(column_width))
        .h_full()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(border_color)
        .bg(bg_dark)
        .child(
            div()
                .h(px(28.0))
                .px_3()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(border_color)
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(text_gray)
                .truncate()
                .child(column_name)
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .when(entries.is_empty(), |this| {
                    this.flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .text_color(text_gray)
                        .child("Empty")
                })
                .when(!entries.is_empty(), |this| {
                    this.children(
                        entries.iter().enumerate().map(|(entry_idx, entry)| {
                            let is_selected = selected_index == Some(entry_idx);
                            let is_dir = entry.is_dir;
                            let name = entry.name.clone();
                            let icon_name = get_file_icon(&name, is_dir);
                            let icon_color = if is_dir {
                                if is_selected { folder_open_color } else { folder_color }
                            } else {
                                get_file_icon_color(&name)
                            };
                            let entity = entity.clone();
                            let entity_for_ctx = entity.clone();
                            let entry_path = entry.path.clone();

                            div()
                                .id(SharedString::from(format!("col-{}-entry-{}", col_idx, entry_idx)))
                                .h(px(28.0))
                                .px_3()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .when(is_selected, |s| s.bg(selected_bg))
                                .when(!is_selected, |s| s.hover(|h| h.bg(hover_bg)))
                                .on_click({
                                    let entry_path = entry_path.clone();
                                    let entity = entity.clone();
                                    move |event, _window, cx| {
                                        entity.update(cx, |view, cx| {
                                            view.close_context_menu();
                                            if event.click_count() == 2 && is_dir {
                                                view.pending_navigation = Some(entry_path.clone());
                                            } else {
                                                view.column_view.select(col_idx, entry_idx);
                                            }
                                            cx.notify();
                                        });
                                    }
                                })
                                .on_mouse_down(MouseButton::Right, {
                                    let entity = entity_for_ctx.clone();
                                    move |event: &MouseDownEvent, _window, cx| {
                                        entity.update(cx, |view, cx| {
                                            view.column_view.select(col_idx, entry_idx);
                                            view.context_menu_position = Some(event.position);
                                            view.context_menu_column = Some(col_idx);
                                            view.context_menu_entry = Some(entry_idx);
                                            cx.notify();
                                        });
                                    }
                                })
                                .child(
                                    svg()
                                        .path(SharedString::from(format!("assets/icons/{}.svg", icon_name)))
                                        .size(px(16.0))
                                        .text_color(icon_color)
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .text_xs()
                                        .text_color(if is_selected { gpui::rgb(0xffffff) } else { text_light })
                                        .truncate()
                                        .child(name)
                                )
                                // Directory indicator
                                .when(is_dir, |this| {
                                    this.child(
                                        svg()
                                            .path("assets/icons/chevron-right.svg")
                                            .size(px(12.0))
                                            .text_color(text_gray)
                                    )
                                })
                        })
                    )
                })
        )
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
        .id(SharedString::from(format!("col-ctx-{}", label)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn create_test_entry(name: &str, is_dir: bool) -> FileEntry {
        FileEntry::new(
            name.to_string(),
            PathBuf::from(format!("/test/{}", name)),
            is_dir,
            1024,
            SystemTime::now(),
        )
    }

    #[test]
    fn test_column_view_component_creation() {
        let column_view = ColumnView::new(PathBuf::from("/test"));
        assert_eq!(column_view.column_count(), 1);
    }
}
