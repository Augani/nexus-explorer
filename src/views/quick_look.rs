use gpui::{
    actions, div, img, prelude::*, px, svg, App, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyBinding, ParentElement, Render, SharedString, Styled, Window,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::models::{theme_colors, FileEntry};
use crate::views::preview::{format_date, format_size};

/// Quick Look content types
#[derive(Debug, Clone, PartialEq)]
pub enum QuickLookContent {
    Image {
        path: PathBuf,
        dimensions: Option<(u32, u32)>,
        format: String,
    },
    Text {
        content: String,
        language: Option<String>,
        line_count: usize,
    },
    Document {
        content: String,
        file_type: String,
    },
    Unsupported {
        file_type: String,
    },
    Loading,
    None,
}

/// Quick Look state model
#[derive(Debug, Clone)]
pub struct QuickLook {
    is_visible: bool,
    current_path: Option<PathBuf>,
    content: QuickLookContent,
    zoom_level: f32,
    file_name: String,
    file_size: u64,
    modified: Option<SystemTime>,
}

impl Default for QuickLook {
    fn default() -> Self {
        Self::new()
    }
}

impl QuickLook {
    pub fn new() -> Self {
        Self {
            is_visible: false,
            current_path: None,
            content: QuickLookContent::None,
            zoom_level: 1.0,
            file_name: String::new(),
            file_size: 0,
            modified: None,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    pub fn current_path(&self) -> Option<&PathBuf> {
        self.current_path.as_ref()
    }

    pub fn content(&self) -> &QuickLookContent {
        &self.content
    }

    pub fn zoom_level(&self) -> f32 {
        self.zoom_level
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn modified(&self) -> Option<SystemTime> {
        self.modified
    }

    /// Show Quick Look for a file
    pub fn show(&mut self, path: PathBuf) {
        self.current_path = Some(path.clone());
        self.is_visible = true;
        self.zoom_level = 1.0;
        self.load_content(&path);
    }

    /// Hide Quick Look
    pub fn hide(&mut self) {
        self.is_visible = false;
    }

    /// Toggle Quick Look visibility
    pub fn toggle(&mut self, path: PathBuf) {
        if self.is_visible && self.current_path.as_ref() == Some(&path) {
            self.hide();
        } else {
            self.show(path);
        }
    }

    /// Navigate to next file in the list
    pub fn next(&mut self, entries: &[FileEntry], current_index: usize) {
        if entries.is_empty() {
            return;
        }
        
        // Find next non-directory file
        let mut next_idx = (current_index + 1) % entries.len();
        let start_idx = next_idx;
        
        loop {
            if !entries[next_idx].is_dir {
                self.show(entries[next_idx].path.clone());
                return;
            }
            next_idx = (next_idx + 1) % entries.len();
            if next_idx == start_idx {
                break;
            }
        }
    }

    /// Navigate to previous file in the list
    pub fn previous(&mut self, entries: &[FileEntry], current_index: usize) {
        if entries.is_empty() {
            return;
        }
        
        // Find previous non-directory file
        let mut prev_idx = if current_index == 0 {
            entries.len() - 1
        } else {
            current_index - 1
        };
        let start_idx = prev_idx;
        
        loop {
            if !entries[prev_idx].is_dir {
                self.show(entries[prev_idx].path.clone());
                return;
            }
            prev_idx = if prev_idx == 0 {
                entries.len() - 1
            } else {
                prev_idx - 1
            };
            if prev_idx == start_idx {
                break;
            }
        }
    }

    /// Zoom in on image
    pub fn zoom_in(&mut self) {
        self.zoom_level = (self.zoom_level * 1.25).min(4.0);
    }

    /// Zoom out on image
    pub fn zoom_out(&mut self) {
        self.zoom_level = (self.zoom_level / 1.25).max(0.25);
    }

    /// Reset zoom to 100%
    pub fn reset_zoom(&mut self) {
        self.zoom_level = 1.0;
    }

    fn load_content(&mut self, path: &Path) {
        if let Ok(metadata) = fs::metadata(path) {
            self.file_size = metadata.len();
            self.modified = metadata.modified().ok();
        }
        
        self.file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());

        if is_image_extension(extension.as_deref()) {
            self.load_image_content(path, extension.as_deref());
            return;
        }

        if is_text_extension(extension.as_deref()) || is_likely_text_file(path) {
            self.load_text_content(path, extension);
            return;
        }

        // Unsupported file type
        self.content = QuickLookContent::Unsupported {
            file_type: extension.unwrap_or_else(|| "Unknown".to_string()),
        };
    }

    fn load_image_content(&mut self, path: &Path, extension: Option<&str>) {
        let format = extension
            .map(|e| e.to_uppercase())
            .unwrap_or_else(|| "Image".to_string());

        let dimensions = get_image_dimensions(path);

        self.content = QuickLookContent::Image {
            path: path.to_path_buf(),
            dimensions,
            format,
        };
    }

    fn load_text_content(&mut self, path: &Path, extension: Option<String>) {
        match fs::read_to_string(path) {
            Ok(content) => {
                let line_count = content.lines().count();
                let language = extension.and_then(|ext| detect_language(&ext));
                
                // Limit content size for preview
                let preview_content = if content.len() > 100000 {
                    format!("{}...\n\n[Content truncated - file too large]", &content[..100000])
                } else {
                    content
                };

                self.content = QuickLookContent::Text {
                    content: preview_content,
                    language,
                    line_count,
                };
            }
            Err(_) => {
                self.content = QuickLookContent::Unsupported {
                    file_type: extension.unwrap_or_else(|| "Unknown".to_string()),
                };
            }
        }
    }
}

/// Check if extension indicates an image file
fn is_image_extension(ext: Option<&str>) -> bool {
    matches!(
        ext,
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "svg" | "tiff" | "tif")
    )
}

/// Check if extension indicates a text file
fn is_text_extension(ext: Option<&str>) -> bool {
    matches!(
        ext,
        Some(
            "txt" | "md" | "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "rb" | "go" | "java"
                | "c" | "cpp" | "h" | "hpp" | "css" | "scss" | "sass" | "less" | "html"
                | "htm" | "xml" | "json" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf"
                | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" | "sql" | "graphql"
                | "vue" | "svelte" | "astro" | "php" | "swift" | "kt" | "kts" | "scala"
                | "clj" | "cljs" | "ex" | "exs" | "erl" | "hrl" | "hs" | "ml" | "mli"
                | "fs" | "fsx" | "r" | "R" | "jl" | "lua" | "vim" | "el" | "lisp" | "scm"
                | "rkt" | "pl" | "pm" | "t" | "awk" | "sed" | "makefile" | "cmake"
                | "dockerfile" | "gitignore" | "gitattributes" | "editorconfig" | "env"
                | "lock" | "log" | "csv" | "tsv"
        )
    )
}

/// Check if file is likely text by reading first bytes
fn is_likely_text_file(path: &Path) -> bool {
    if let Ok(data) = fs::read(path) {
        let sample: Vec<u8> = data.into_iter().take(512).collect();
        !sample.contains(&0) && sample.iter().filter(|&&b| b < 32 && b != 9 && b != 10 && b != 13).count() < sample.len() / 10
    } else {
        false
    }
}

/// Detect programming language from extension
fn detect_language(ext: &str) -> Option<String> {
    let lang = match ext {
        "rs" => "Rust",
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" | "mts" | "cts" => "TypeScript",
        "jsx" => "JSX",
        "tsx" => "TSX",
        "py" | "pyw" => "Python",
        "rb" => "Ruby",
        "go" => "Go",
        "java" => "Java",
        "c" => "C",
        "cpp" | "cc" | "cxx" => "C++",
        "h" | "hpp" | "hxx" => "C/C++ Header",
        "css" => "CSS",
        "scss" => "SCSS",
        "sass" => "Sass",
        "less" => "Less",
        "html" | "htm" => "HTML",
        "xml" => "XML",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "md" | "markdown" => "Markdown",
        "sh" | "bash" => "Shell",
        "zsh" => "Zsh",
        "fish" => "Fish",
        "ps1" => "PowerShell",
        "sql" => "SQL",
        "graphql" | "gql" => "GraphQL",
        "vue" => "Vue",
        "svelte" => "Svelte",
        "php" => "PHP",
        "swift" => "Swift",
        "kt" | "kts" => "Kotlin",
        "scala" => "Scala",
        "clj" | "cljs" => "Clojure",
        "ex" | "exs" => "Elixir",
        "erl" | "hrl" => "Erlang",
        "hs" => "Haskell",
        "ml" | "mli" => "OCaml",
        "fs" | "fsx" => "F#",
        "r" => "R",
        "jl" => "Julia",
        "lua" => "Lua",
        "vim" => "Vim Script",
        "el" | "lisp" => "Lisp",
        "txt" => "Plain Text",
        _ => return None,
    };
    Some(lang.to_string())
}

/// Get image dimensions (basic implementation)
fn get_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    // Try to read image dimensions using the image crate
    if let Ok(reader) = image::ImageReader::open(path) {
        if let Ok(dimensions) = reader.into_dimensions() {
            return Some(dimensions);
        }
    }
    None
}


// Define actions for Quick Look key bindings
actions!(quick_look, [
    ToggleQuickLook,
    CloseQuickLook,
    QuickLookNext,
    QuickLookPrevious,
    QuickLookZoomIn,
    QuickLookZoomOut,
    QuickLookResetZoom,
]);

/// Quick Look view component
pub struct QuickLookView {
    quick_look: QuickLook,
    focus_handle: FocusHandle,
    entries: Vec<FileEntry>,
    current_index: Option<usize>,
}

impl QuickLookView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            quick_look: QuickLook::new(),
            focus_handle: cx.focus_handle(),
            entries: Vec::new(),
            current_index: None,
        }
    }

    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("space", ToggleQuickLook, None),
            KeyBinding::new("escape", CloseQuickLook, None),
            KeyBinding::new("right", QuickLookNext, None),
            KeyBinding::new("left", QuickLookPrevious, None),
            KeyBinding::new("cmd-=", QuickLookZoomIn, None),
            KeyBinding::new("cmd--", QuickLookZoomOut, None),
            KeyBinding::new("cmd-0", QuickLookResetZoom, None),
        ]);
    }


    pub fn is_visible(&self) -> bool {
        self.quick_look.is_visible()
    }

    pub fn show(&mut self, path: PathBuf, entries: Vec<FileEntry>, index: usize) {
        self.entries = entries;
        self.current_index = Some(index);
        self.quick_look.show(path);
    }

    pub fn hide(&mut self) {
        self.quick_look.hide();
    }

    pub fn toggle(&mut self, path: PathBuf, entries: Vec<FileEntry>, index: usize) {
        if self.quick_look.is_visible() && self.quick_look.current_path() == Some(&path) {
            self.hide();
        } else {
            self.show(path, entries, index);
        }
    }

    pub fn next(&mut self) {
        if let Some(idx) = self.current_index {
            let next_idx = (idx + 1) % self.entries.len().max(1);
            if !self.entries.is_empty() && !self.entries[next_idx].is_dir {
                self.current_index = Some(next_idx);
                self.quick_look.show(self.entries[next_idx].path.clone());
            } else {
                self.quick_look.next(&self.entries, idx);
                if let Some(path) = self.quick_look.current_path() {
                    self.current_index = self.entries.iter().position(|e| &e.path == path);
                }
            }
        }
    }

    pub fn previous(&mut self) {
        if let Some(idx) = self.current_index {
            let prev_idx = if idx == 0 { self.entries.len().saturating_sub(1) } else { idx - 1 };
            if !self.entries.is_empty() && !self.entries[prev_idx].is_dir {
                self.current_index = Some(prev_idx);
                self.quick_look.show(self.entries[prev_idx].path.clone());
            } else {
                self.quick_look.previous(&self.entries, idx);
                if let Some(path) = self.quick_look.current_path() {
                    self.current_index = self.entries.iter().position(|e| &e.path == path);
                }
            }
        }
    }


    pub fn zoom_in(&mut self) {
        self.quick_look.zoom_in();
    }

    pub fn zoom_out(&mut self) {
        self.quick_look.zoom_out();
    }

    pub fn reset_zoom(&mut self) {
        self.quick_look.reset_zoom();
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }
}

impl Focusable for QuickLookView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for QuickLookView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.quick_look.is_visible() {
            return div().id("quick-look-hidden");
        }

        let theme = theme_colors();
        let overlay_bg = gpui::rgba(0x000000dd);
        let panel_bg = theme.bg_secondary;
        let border_color = theme.border_default;
        let text_primary = theme.text_primary;
        let text_muted = theme.text_muted;

        div()
            .id("quick-look-overlay")
            .track_focus(&self.focus_handle)
            .absolute()
            .inset_0()
            .bg(overlay_bg)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|view, _event, _window, _cx| {
                view.hide();
            }))
            .on_action(cx.listener(|view, _: &CloseQuickLook, _window, _cx| {
                view.hide();
            }))
            .on_action(cx.listener(|view, _: &QuickLookNext, _window, _cx| {
                view.next();
            }))
            .on_action(cx.listener(|view, _: &QuickLookPrevious, _window, _cx| {
                view.previous();
            }))

            .on_action(cx.listener(|view, _: &QuickLookZoomIn, _window, _cx| {
                view.zoom_in();
            }))
            .on_action(cx.listener(|view, _: &QuickLookZoomOut, _window, _cx| {
                view.zoom_out();
            }))
            .on_action(cx.listener(|view, _: &QuickLookResetZoom, _window, _cx| {
                view.reset_zoom();
            }))
            // Header with file info
            .child(self.render_header(text_primary, text_muted))
            // Main content area
            .child(self.render_content(panel_bg, border_color, text_primary, text_muted))
            // Footer with controls
            .child(self.render_footer(text_muted))
    }
}

impl QuickLookView {
    fn render_header(&self, text_primary: gpui::Rgba, text_muted: gpui::Rgba) -> impl IntoElement {
        let file_name = self.quick_look.file_name().to_string();
        let file_size = format_size(self.quick_look.file_size());
        let modified = self.quick_look.modified()
            .map(format_date)
            .unwrap_or_default();

        div()
            .w_full()
            .px_6()
            .py_4()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(text_primary)
                            .child(file_name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_muted)
                            .child(format!("{} • {}", file_size, modified)),
                    ),
            )
            .child(
                div()
                    .id("close-quick-look")
                    .p_2()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|h| h.bg(gpui::rgba(0xffffff22)))
                    .child(
                        div()
                            .text_xl()
                            .text_color(text_muted)
                            .child("×"),
                    ),
            )
    }


    fn render_content(
        &self,
        panel_bg: gpui::Rgba,
        border_color: gpui::Rgba,
        text_primary: gpui::Rgba,
        text_muted: gpui::Rgba,
    ) -> impl IntoElement {
        let content = match self.quick_look.content() {
            QuickLookContent::Image { path, dimensions, format } => {
                self.render_image_content(path, dimensions.as_ref(), format, text_muted)
            }
            QuickLookContent::Text { content, language, line_count } => {
                self.render_text_content(content, language.as_deref(), *line_count, panel_bg, text_primary, text_muted)
            }
            QuickLookContent::Document { content, file_type } => {
                self.render_document_content(content, file_type, panel_bg, text_primary, text_muted)
            }
            QuickLookContent::Unsupported { file_type } => {
                self.render_unsupported_content(file_type, text_muted)
            }
            QuickLookContent::Loading => {
                self.render_loading_content(text_muted)
            }
            QuickLookContent::None => {
                div()
            }
        };

        div()
            .id("quick-look-content-wrapper")
            .flex_1()
            .w_full()
            .max_w(px(900.0))
            .max_h(px(700.0))
            .mx_6()
            .bg(panel_bg)
            .rounded_lg()
            .border_1()
            .border_color(border_color)
            .overflow_hidden()
            .child(content)
    }

    fn render_image_content(
        &self,
        path: &Path,
        dimensions: Option<&(u32, u32)>,
        format: &str,
        text_muted: gpui::Rgba,
    ) -> gpui::Div {
        let zoom = self.quick_look.zoom_level();
        let path_str = path.to_string_lossy().to_string();
        let dims_str = dimensions
            .map(|(w, h)| format!("{}×{}", w, h))
            .unwrap_or_else(|| "Unknown".to_string());
        let format_str = format.to_string();


        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p_4()
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .child(
                        img(SharedString::from(path_str))
                            .w(px(600.0 * zoom))
                            .h(px(400.0 * zoom))
                            .object_fit(gpui::ObjectFit::Contain),
                    ),
            )
            .child(
                div()
                    .mt_4()
                    .flex()
                    .items_center()
                    .gap_4()
                    .text_xs()
                    .text_color(text_muted)
                    .child(format!("{} • {}", format_str, dims_str))
                    .child(format!("Zoom: {}%", (zoom * 100.0) as i32)),
            )
            .child(self.render_zoom_controls(text_muted))
    }

    fn render_zoom_controls(&self, text_muted: gpui::Rgba) -> impl IntoElement {
        div()
            .mt_2()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .id("zoom-out-btn")
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(gpui::rgba(0xffffff11))
                    .hover(|h| h.bg(gpui::rgba(0xffffff22)))
                    .text_sm()
                    .text_color(text_muted)
                    .child("−"),
            )
            .child(
                div()
                    .id("zoom-reset-btn")
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(gpui::rgba(0xffffff11))
                    .hover(|h| h.bg(gpui::rgba(0xffffff22)))
                    .text_xs()
                    .text_color(text_muted)
                    .child("100%"),
            )
            .child(
                div()
                    .id("zoom-in-btn")
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(gpui::rgba(0xffffff11))
                    .hover(|h| h.bg(gpui::rgba(0xffffff22)))
                    .text_sm()
                    .text_color(text_muted)
                    .child("+"),
            )
    }


    fn render_text_content(
        &self,
        content: &str,
        language: Option<&str>,
        line_count: usize,
        panel_bg: gpui::Rgba,
        text_primary: gpui::Rgba,
        text_muted: gpui::Rgba,
    ) -> gpui::Div {
        let lines: Vec<String> = content.lines().take(500).map(|s| s.to_string()).collect();
        let line_number_width = line_count.to_string().len().max(3);
        let lang_str = language.map(|s| s.to_string());

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(panel_bg)
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(gpui::rgb(0x30363d))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_muted)
                            .when_some(lang_str.clone(), |this, lang| {
                                this.child(format!("{} • {} lines", lang, line_count))
                            })
                            .when(lang_str.is_none(), |this| {
                                this.child(format!("{} lines", line_count))
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_4()
                    .font_family("JetBrains Mono")
                    .text_xs()
                    .child(
                        div()
                            .flex()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .text_color(text_muted)
                                    .pr_3()
                                    .border_r_1()
                                    .border_color(gpui::rgb(0x30363d))
                                    .mr_3()
                                    .children(
                                        (0..lines.len()).map(|i| {
                                            div()
                                                .text_right()
                                                .min_w(px((line_number_width * 8) as f32))
                                                .child(format!("{}", i + 1))
                                        }),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .text_color(text_primary)
                                    .children(
                                        lines.into_iter().map(|line| {
                                            div()
                                                .whitespace_nowrap()
                                                .child(if line.is_empty() { " ".to_string() } else { line })
                                        }),
                                    ),
                            ),
                    ),
            )
    }


    fn render_document_content(
        &self,
        content: &str,
        file_type: &str,
        panel_bg: gpui::Rgba,
        text_primary: gpui::Rgba,
        text_muted: gpui::Rgba,
    ) -> gpui::Div {
        let file_type_str = file_type.to_string();
        let content_str = content.to_string();
        
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(panel_bg)
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(gpui::rgb(0x30363d))
                    .text_xs()
                    .text_color(text_muted)
                    .child(format!("{} Document", file_type_str)),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_4()
                    .text_sm()
                    .text_color(text_primary)
                    .child(content_str),
            )
    }

    fn render_unsupported_content(&self, file_type: &str, text_muted: gpui::Rgba) -> gpui::Div {
        let file_type_str = file_type.to_string();
        
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                svg()
                    .path("assets/icons/file.svg")
                    .size(px(64.0))
                    .text_color(text_muted),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(text_muted)
                    .child(format!("No preview available for {} files", file_type_str)),
            )
    }

    fn render_loading_content(&self, text_muted: gpui::Rgba) -> gpui::Div {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(text_muted)
            .child("Loading...")
    }


    fn render_footer(&self, text_muted: gpui::Rgba) -> impl IntoElement {
        let has_entries = !self.entries.is_empty();
        let current_pos = self.current_index
            .map(|i| format!("{} of {}", i + 1, self.entries.len()))
            .unwrap_or_default();

        div()
            .w_full()
            .px_6()
            .py_3()
            .flex()
            .items_center()
            .justify_between()
            .text_xs()
            .text_color(text_muted)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .when(has_entries, |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child("←")
                                .child("Previous")
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child("Next")
                                .child("→")
                        )
                    }),
            )
            .child(
                div()
                    .when(has_entries, |this| this.child(current_pos)),
            )
            .child(
                div()
                    .child("Press Space or Escape to close"),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn test_quick_look_new() {
        let ql = QuickLook::new();
        assert!(!ql.is_visible());
        assert!(ql.current_path().is_none());
        assert_eq!(ql.zoom_level(), 1.0);
    }

    #[test]
    fn test_quick_look_show_hide() {
        let mut ql = QuickLook::new();
        let path = PathBuf::from("/tmp/test.txt");
        
        ql.show(path.clone());
        assert!(ql.is_visible());
        assert_eq!(ql.current_path(), Some(&path));
        
        ql.hide();
        assert!(!ql.is_visible());
    }

    #[test]
    fn test_quick_look_toggle() {
        let mut ql = QuickLook::new();
        let path = PathBuf::from("/tmp/test.txt");
        
        ql.toggle(path.clone());
        assert!(ql.is_visible());
        
        ql.toggle(path.clone());
        assert!(!ql.is_visible());
    }

    #[test]
    fn test_quick_look_zoom() {
        let mut ql = QuickLook::new();
        
        assert_eq!(ql.zoom_level(), 1.0);
        
        ql.zoom_in();
        assert!(ql.zoom_level() > 1.0);
        
        ql.reset_zoom();
        assert_eq!(ql.zoom_level(), 1.0);
        
        ql.zoom_out();
        assert!(ql.zoom_level() < 1.0);
    }

    #[test]
    fn test_quick_look_zoom_limits() {
        let mut ql = QuickLook::new();
        
        for _ in 0..20 {
            ql.zoom_in();
        }
        assert!(ql.zoom_level() <= 4.0);
        
        // Zoom out to min
        for _ in 0..40 {
            ql.zoom_out();
        }
        assert!(ql.zoom_level() >= 0.25);
    }
}
