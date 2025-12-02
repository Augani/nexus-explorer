use std::time::SystemTime;

use gpui::{
    div, prelude::*, px, App, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled, Window,
};

use crate::models::{FileEntry, IconKey};

/// Default row height in pixels for file list items
pub const DEFAULT_ROW_HEIGHT: f32 = 24.0;

/// Default buffer size for virtualization (extra rows above/below viewport)
pub const DEFAULT_BUFFER_SIZE: usize = 5;

/// Virtualized file list view that renders only visible items.
pub struct FileList {
    entries: Vec<FileEntry>,
    row_height: f32,
    buffer_size: usize,
    scroll_offset: f32,
    viewport_height: f32,
    highlight_positions: Option<Vec<Vec<usize>>>,
}

/// View wrapper for FileList to integrate with GPUI
pub struct FileListView {
    file_list: FileList,
    focus_handle: FocusHandle,
}

impl FileListView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            file_list: FileList::new(),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn with_file_list(file_list: FileList, cx: &mut Context<Self>) -> Self {
        Self {
            file_list,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn inner(&self) -> &FileList {
        &self.file_list
    }

    pub fn inner_mut(&mut self) -> &mut FileList {
        &mut self.file_list
    }
}

impl Focusable for FileListView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileListView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let visible_items = self.file_list.render_visible_items();
        let row_height = self.file_list.row_height();
        let bg_darker = gpui::rgb(0x010409);
        let border_color = gpui::rgb(0x30363d);
        let text_gray = gpui::rgb(0x8b949e);
        let hover_bg = gpui::rgb(0x161b22);

        div()
            .id("file-list")
            .size_full()
            .bg(bg_darker)
            .flex()
            .flex_col()
            // List Header
            .child(
                div()
                    .flex()
                    .h(px(32.0))
                    .bg(gpui::rgb(0x0d1117))
                    .border_b_1()
                    .border_color(border_color)
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(text_gray)
                    .child(
                        div().flex_1().px_4().flex().items_center().child("NAME")
                    )
                    .child(
                        div().w(px(120.0)).px_4().flex().items_center().border_l_1().border_color(gpui::rgb(0x21262d)).child("DATE ADDED")
                    )
                    .child(
                        div().w(px(100.0)).px_4().flex().items_center().border_l_1().border_color(gpui::rgb(0x21262d)).child("TYPE")
                    )
                    .child(
                        div().w(px(80.0)).px_4().flex().items_center().border_l_1().border_color(gpui::rgb(0x21262d)).child("SIZE")
                    )
            )
            // List Rows
            .child(
                div()
                    .flex_1()
                    // .overflow_y(gpui::Overflow::Scroll)
                    .children(visible_items.into_iter().map(move |(index, entry)| {
                        div()
                            .id(SharedString::from(format!("file-entry-{}", index)))
                            .h(px(row_height))
                            .w_full()
                            .flex()
                            .items_center()
                            .text_sm()
                            .cursor_pointer()
                            .hover(|style| style.bg(hover_bg))
                            .child(
                                div().flex_1().px_4().flex().items_center().truncate().child(
                                    div().flex().items_center().child(
                                        div().mr_3().w(px(16.0)).h(px(16.0)).bg(gpui::rgb(0x6e7681)) // Placeholder icon
                                    ).child(
                                        div().text_color(gpui::rgb(0xc9d1d9)).child(entry.name.clone())
                                    )
                                )
                            )
                            .child(
                                div().w(px(120.0)).px_4().text_xs().text_color(text_gray).truncate().child(entry.formatted_date.clone())
                            )
                            .child(
                                div().w(px(100.0)).px_4().text_xs().text_color(text_gray).truncate().child(if entry.is_dir { "Folder" } else { "File" })
                            )
                            .child(
                                div().w(px(80.0)).px_4().text_xs().text_color(text_gray).font_family("Mono").truncate().child(entry.formatted_size.clone())
                            )
                    }))
            )
    }
}

/// Visible range of items to render
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleRange {
    pub start: usize,
    pub end: usize,
}

/// Rendered representation of a file entry row
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
    /// Creates a new FileList with default settings
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            row_height: DEFAULT_ROW_HEIGHT,
            buffer_size: DEFAULT_BUFFER_SIZE,
            scroll_offset: 0.0,
            viewport_height: 0.0,
            highlight_positions: None,
        }
    }

    /// Creates a new FileList with custom row height and buffer size
    pub fn with_config(row_height: f32, buffer_size: usize) -> Self {
        Self {
            entries: Vec::new(),
            row_height: row_height.max(1.0),
            buffer_size,
            scroll_offset: 0.0,
            viewport_height: 0.0,
            highlight_positions: None,
        }
    }


    /// Returns the total number of items in the list
    pub fn item_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the row height in pixels
    pub fn row_height(&self) -> f32 {
        self.row_height
    }

    /// Returns the buffer size for virtualization
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Returns the current scroll offset
    pub fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    /// Returns the viewport height
    pub fn viewport_height(&self) -> f32 {
        self.viewport_height
    }

    /// Sets the entries to display
    pub fn set_entries(&mut self, entries: Vec<FileEntry>) {
        self.entries = entries;
        self.highlight_positions = None;
    }

    /// Returns a reference to the entries
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    /// Updates the scroll offset
    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.scroll_offset = offset.max(0.0);
    }

    /// Updates the viewport height
    pub fn set_viewport_height(&mut self, height: f32) {
        self.viewport_height = height.max(0.0);
    }

    /// Sets highlight positions for search result highlighting
    pub fn set_highlight_positions(&mut self, positions: Option<Vec<Vec<usize>>>) {
        self.highlight_positions = positions;
    }

    /// Calculates the visible range of items based on viewport and scroll position.
    /// 
    /// The range includes a buffer above and below the visible area for smooth scrolling.
    /// Formula: start = floor(scroll_offset / row_height)
    ///          end = min(start + ceil(viewport_height / row_height) + buffer, total_items)
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

    /// Returns the maximum number of items that can be rendered at once.
    /// This is bounded by viewport height and buffer size, regardless of total items.
    pub fn max_rendered_items(&self) -> usize {
        if self.viewport_height <= 0.0 || self.row_height <= 0.0 {
            return 0;
        }
        
        let visible_rows = (self.viewport_height / self.row_height).ceil() as usize;
        visible_rows + (self.buffer_size * 2)
    }

    /// Renders a single item at the given index.
    /// Returns None if the index is out of bounds.
    pub fn render_item(&self, index: usize) -> Option<RenderedEntry> {
        let entry = self.entries.get(index)?;
        
        let highlight_positions = self.highlight_positions
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

    /// Returns rendered entries for the visible range only.
    /// This is the core virtualization method - only visible items are rendered.
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
    /// Returns the number of items in the range
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns true if the range is empty
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Returns true if the given index is within the range
    pub fn contains(&self, index: usize) -> bool {
        index >= self.start && index < self.end
    }
}

impl RenderedEntry {
    /// Returns true if the character at the given position should be highlighted
    pub fn is_highlighted(&self, char_index: usize) -> bool {
        self.highlight_positions.contains(&char_index)
    }

    /// Returns the name with highlight markers for display.
    /// Characters at highlight positions are wrapped in markers.
    pub fn name_with_highlights(&self) -> Vec<(char, bool)> {
        self.name
            .chars()
            .enumerate()
            .map(|(i, c)| (c, self.highlight_positions.contains(&i)))
            .collect()
    }
}

/// Formats a file size in human-readable format
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

/// Formats a SystemTime as a human-readable date string
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
