use std::path::PathBuf;

use gpui::{
    div, prelude::*, px, svg, App, Context, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Point, Pixels, Render, 
    SharedString, Styled, Window, MouseDownEvent,
};

use crate::models::{FileEntry, GridConfig};
use super::file_list::{get_file_icon, get_file_icon_color};

pub struct GridView {
    entries: Vec<FileEntry>,
    config: GridConfig,
    selected_index: Option<usize>,
    viewport_width: f32,
}

pub struct GridViewComponent {
    grid_view: GridView,
    focus_handle: FocusHandle,
    pending_navigation: Option<PathBuf>,
    context_menu_position: Option<Point<Pixels>>,
    context_menu_index: Option<usize>,
}

impl GridView {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            config: GridConfig::default(),
            selected_index: None,
            viewport_width: 800.0,
        }
    }

    pub fn with_config(config: GridConfig) -> Self {
        Self {
            entries: Vec::new(),
            config,
            selected_index: None,
            viewport_width: 800.0,
        }
    }

    pub fn set_entries(&mut self, entries: Vec<FileEntry>) {
        self.entries = entries;
        self.selected_index = None;
    }

    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    pub fn item_count(&self) -> usize {
        self.entries.len()
    }

    pub fn config(&self) -> &GridConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: GridConfig) {
        self.config = config;
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn set_selected_index(&mut self, index: Option<usize>) {
        self.selected_index = index;
    }

    pub fn set_viewport_width(&mut self, width: f32) {
        self.viewport_width = width;
    }

    pub fn viewport_width(&self) -> f32 {
        self.viewport_width
    }

    pub fn columns(&self) -> usize {
        self.config.columns_for_width(self.viewport_width)
    }

    pub fn rows(&self) -> usize {
        self.config.rows_for_items(self.entries.len(), self.viewport_width)
    }

    pub fn content_height(&self) -> f32 {
        self.config.content_height(self.entries.len(), self.viewport_width)
    }
}

impl Default for GridView {
    fn default() -> Self {
        Self::new()
    }
}

impl GridViewComponent {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            grid_view: GridView::new(),
            focus_handle: cx.focus_handle(),
            pending_navigation: None,
            context_menu_position: None,
            context_menu_index: None,
        }
    }

    pub fn with_grid_view(grid_view: GridView, cx: &mut Context<Self>) -> Self {
        Self {
            grid_view,
            focus_handle: cx.focus_handle(),
            pending_navigation: None,
            context_menu_position: None,
            context_menu_index: None,
        }
    }

    pub fn inner(&self) -> &GridView {
        &self.grid_view
    }

    pub fn inner_mut(&mut self) -> &mut GridView {
        &mut self.grid_view
    }

    pub fn take_pending_navigation(&mut self) -> Option<PathBuf> {
        self.pending_navigation.take()
    }

    pub fn close_context_menu(&mut self) {
        self.context_menu_position = None;
        self.context_menu_index = None;
    }

    pub fn select_item(&mut self, index: usize, cx: &mut Context<Self>) {
        self.grid_view.selected_index = Some(index);
        cx.notify();
    }

    pub fn open_item(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(entry) = self.grid_view.entries.get(index) {
            if entry.is_dir {
                self.pending_navigation = Some(entry.path.clone());
                cx.notify();
            }
        }
    }
}

impl Focusable for GridViewComponent {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GridViewComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let total_items = self.grid_view.item_count();
        let config = self.grid_view.config;
        let selected_index = self.grid_view.selected_index;
        let context_menu_pos = self.context_menu_position;
        let _context_menu_idx = self.context_menu_index;

        // Colors
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

        div()
            .id("grid-view")
            .size_full()
            .bg(bg_darker)
            .flex()
            .flex_col()
            .relative()
            .on_mouse_down(MouseButton::Left, cx.listener(|view, _event, _window, cx| {
                view.close_context_menu();
                cx.notify();
            }))
            // Header
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
                    .child(format!("{} items", total_items))
            )
            // Grid content
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_4()
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
                        let entries = self.grid_view.entries.clone();
                        
                        this.child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap(px(config.gap))
                                .children(
                                    entries.iter().enumerate().map(|(ix, entry)| {
                                        let is_selected = selected_index == Some(ix);
                                        let is_dir = entry.is_dir;
                                        let name = entry.name.clone();
                                        let icon_name = get_file_icon(&name, is_dir);
                                        let icon_color = if is_dir { 
                                            if is_selected { folder_open_color } else { folder_color }
                                        } else { 
                                            get_file_icon_color(&name) 
                                        };
                                        let entry_path = entry.path.clone();
                                        let entity = entity.clone();
                                        let entity_for_ctx = entity.clone();

                                        div()
                                            .id(SharedString::from(format!("grid-item-{}", ix)))
                                            .w(px(config.item_width))
                                            .h(px(config.item_height))
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .justify_center()
                                            .gap_2()
                                            .p_2()
                                            .rounded_lg()
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
                                                            view.grid_view.selected_index = Some(ix);
                                                        }
                                                        cx.notify();
                                                    });
                                                }
                                            })
                                            .on_mouse_down(MouseButton::Right, {
                                                let entity = entity_for_ctx.clone();
                                                move |event: &MouseDownEvent, _window, cx| {
                                                    entity.update(cx, |view, cx| {
                                                        view.grid_view.selected_index = Some(ix);
                                                        view.context_menu_position = Some(event.position);
                                                        view.context_menu_index = Some(ix);
                                                        cx.notify();
                                                    });
                                                }
                                            })
                                            // Icon
                                            .child(
                                                svg()
                                                    .path(SharedString::from(format!("assets/icons/{}.svg", icon_name)))
                                                    .size(px(config.icon_size))
                                                    .text_color(icon_color)
                                            )
                                            // File name
                                            .child(
                                                div()
                                                    .w_full()
                                                    .text_center()
                                                    .text_xs()
                                                    .text_color(if is_selected { gpui::rgb(0xffffff) } else { text_light })
                                                    .truncate()
                                                    .child(truncate_name(&name, 14))
                                            )
                                    })
                                )
                        )
                    })
            )
            // Footer
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
                                    .path("assets/icons/grid-2x2.svg")
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
                            .child("Grid View"),
                    ),
            )
            // Context Menu
            .when_some(context_menu_pos, |this, pos| {
                let entity = cx.entity().clone();
                let selected_entry = self.context_menu_index.and_then(|idx| self.grid_view.entries.get(idx).cloned());
                
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

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}...", &name[..max_len.saturating_sub(3)])
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
        .id(SharedString::from(format!("grid-ctx-{}", label)))
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
    fn test_grid_view_new() {
        let grid = GridView::new();
        assert_eq!(grid.item_count(), 0);
        assert!(grid.selected_index().is_none());
    }

    #[test]
    fn test_grid_view_set_entries() {
        let mut grid = GridView::new();
        let entries = vec![
            create_test_entry("file1.txt", false),
            create_test_entry("folder1", true),
        ];
        
        grid.set_entries(entries);
        assert_eq!(grid.item_count(), 2);
    }

    #[test]
    fn test_grid_view_selection() {
        let mut grid = GridView::new();
        let entries = vec![
            create_test_entry("file1.txt", false),
            create_test_entry("file2.txt", false),
        ];
        
        grid.set_entries(entries);
        assert!(grid.selected_index().is_none());
        
        grid.set_selected_index(Some(1));
        assert_eq!(grid.selected_index(), Some(1));
        
        grid.set_selected_index(None);
        assert!(grid.selected_index().is_none());
    }

    #[test]
    fn test_grid_view_columns_and_rows() {
        let mut grid = GridView::new();
        let entries: Vec<FileEntry> = (0..10)
            .map(|i| create_test_entry(&format!("file{}.txt", i), false))
            .collect();
        
        grid.set_entries(entries);
        grid.set_viewport_width(400.0);
        
        let columns = grid.columns();
        let rows = grid.rows();
        
        assert!(columns >= grid.config().min_columns);
        assert!(rows * columns >= 10);
    }

    #[test]
    fn test_truncate_name() {
        assert_eq!(truncate_name("short.txt", 14), "short.txt");
        assert_eq!(truncate_name("very_long_filename.txt", 14), "very_long_f...");
        assert_eq!(truncate_name("a", 14), "a");
    }
}
