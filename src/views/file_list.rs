use std::path::PathBuf;
use std::time::SystemTime;

use gpui::{
    div, prelude::*, px, svg, uniform_list, App, Context, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Point, Pixels, Render, 
    SharedString, Styled, UniformListScrollHandle, Window, MouseDownEvent, Hsla,
};

use crate::models::{FileEntry, IconKey};

pub const DEFAULT_ROW_HEIGHT: f32 = 36.0;
pub const DEFAULT_BUFFER_SIZE: usize = 5;

pub struct FileList {
    entries: Vec<FileEntry>,
    row_height: f32,
    buffer_size: usize,
    scroll_offset: f32,
    viewport_height: f32,
    highlight_positions: Option<Vec<Vec<usize>>>,
    selected_index: Option<usize>,
}

pub struct FileListView {
    file_list: FileList,
    focus_handle: FocusHandle,
    scroll_handle: UniformListScrollHandle,
    pending_navigation: Option<PathBuf>,
    context_menu_position: Option<Point<Pixels>>,
    context_menu_index: Option<usize>,
}

impl FileListView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            file_list: FileList::new(),
            focus_handle: cx.focus_handle(),
            scroll_handle: UniformListScrollHandle::new(),
            pending_navigation: None,
            context_menu_position: None,
            context_menu_index: None,
        }
    }

    pub fn with_file_list(file_list: FileList, cx: &mut Context<Self>) -> Self {
        Self {
            file_list,
            focus_handle: cx.focus_handle(),
            scroll_handle: UniformListScrollHandle::new(),
            pending_navigation: None,
            context_menu_position: None,
            context_menu_index: None,
        }
    }
    
    pub fn close_context_menu(&mut self) {
        self.context_menu_position = None;
        self.context_menu_index = None;
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
        let file_color = gpui::rgb(0x8b949e);
        let menu_bg = gpui::rgb(0x161b22);

        div()
            .id("file-list")
            .size_full()
            .bg(bg_darker)
            .flex()
            .flex_col()
            .relative()
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
                            .child("NAME")
                            .child(render_sort_icon("arrow-down-up", text_gray)),
                    )
                    .child(
                        div()
                            .w(px(120.0))
                            .px_4()
                            .flex()
                            .items_center()
                            .gap_1()
                            .border_l_1()
                            .border_color(border_subtle)
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg).text_color(text_light))
                            .child("DATE")
                            .child(render_sort_icon("calendar", text_gray)),
                    )
                    .child(
                        div()
                            .w(px(100.0))
                            .px_4()
                            .flex()
                            .items_center()
                            .border_l_1()
                            .border_color(border_subtle)
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg).text_color(text_light))
                            .child("TYPE"),
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .px_4()
                            .flex()
                            .items_center()
                            .border_l_1()
                            .border_color(border_subtle)
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg).text_color(text_light))
                            .child("SIZE"),
                    ),
            )
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
                                        if let Some(entry) = view.file_list.entries.get(ix) {
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
                                            let entry_path = entry.path.clone();
                                            let entity = entity.clone();
                                            let entity_for_ctx = entity.clone();

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
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .px_4()
                                                            .flex()
                                                            .items_center()
                                                            .overflow_hidden()
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_3()
                                                                    .child(
                                                                        svg()
                                                                            .path(SharedString::from(format!("assets/icons/{}.svg", icon_name)))
                                                                            .size(px(16.0))
                                                                            .text_color(icon_color)
                                                                            .flex_shrink_0(),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .text_color(if is_selected { gpui::rgb(0xffffff) } else { text_light })
                                                                            .font_weight(if is_selected { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                                                                            .truncate()
                                                                            .child(name),
                                                                    ),
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
                                    }
                                    items
                                }),
                            )
                            .size_full()
                            .track_scroll(self.scroll_handle.clone()),
                        )
                    }),
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
            // Context Menu
            .when_some(context_menu_pos, |this, pos| {
                let entity = cx.entity().clone();
                let selected_entry = context_menu_idx.and_then(|idx| self.file_list.entries.get(idx).cloned());
                
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

fn render_sort_icon(icon_name: &str, color: gpui::Rgba) -> impl IntoElement {
    svg()
        .path(SharedString::from(format!("assets/icons/{}.svg", icon_name)))
        .size(px(12.0))
        .text_color(color)
        .opacity(0.5)
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

fn get_file_type(name: &str) -> String {
    if let Some(ext) = name.rsplit('.').next() {
        if ext != name {
            return ext.to_uppercase();
        }
    }
    "File".to_string()
}

fn get_file_icon(name: &str, is_dir: bool) -> &'static str {
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

fn get_file_icon_color(name: &str) -> gpui::Rgba {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext.to_lowercase().as_str() {
        "rs" => gpui::rgb(0xdea584),      // Rust orange
        "ts" | "tsx" => gpui::rgb(0x3178c6), // TypeScript blue
        "js" | "jsx" | "mjs" => gpui::rgb(0xf7df1e), // JavaScript yellow
        "py" => gpui::rgb(0x3776ab),      // Python blue
        "go" => gpui::rgb(0x00add8),      // Go cyan
        "java" => gpui::rgb(0xb07219),    // Java brown
        "json" => gpui::rgb(0xf5a623),    // JSON orange
        "yaml" | "yml" => gpui::rgb(0xcb171e), // YAML red
        "toml" => gpui::rgb(0x9c4221),    // TOML brown
        "md" | "markdown" => gpui::rgb(0x519aba), // Markdown blue
        "html" | "htm" => gpui::rgb(0xe34c26), // HTML orange
        "css" | "scss" | "sass" => gpui::rgb(0x563d7c), // CSS purple
        "png" | "jpg" | "jpeg" | "gif" | "svg" => gpui::rgb(0xa855f7), // Image purple
        "zip" | "tar" | "gz" => gpui::rgb(0xf59e0b), // Archive amber
        _ => gpui::rgb(0x8b949e),          // Default gray
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
            row_height: DEFAULT_ROW_HEIGHT,
            buffer_size: DEFAULT_BUFFER_SIZE,
            scroll_offset: 0.0,
            viewport_height: 0.0,
            highlight_positions: None,
            selected_index: None,
        }
    }

    pub fn with_config(row_height: f32, buffer_size: usize) -> Self {
        Self {
            entries: Vec::new(),
            row_height: row_height.max(1.0),
            buffer_size,
            scroll_offset: 0.0,
            viewport_height: 0.0,
            highlight_positions: None,
            selected_index: None,
        }
    }

    pub fn item_count(&self) -> usize {
        self.entries.len()
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
        self.highlight_positions = None;
        self.selected_index = None;
    }

    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
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
