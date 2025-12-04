use gpui::{
    div, prelude::*, px, svg, App, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Window,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::models::theme_colors;

/// Content types that can be previewed
#[derive(Debug, Clone, PartialEq)]
pub enum PreviewContent {
    Text {
        content: String,
        language: Option<String>,
        line_count: usize,
    },
    Image {
        path: PathBuf,
        dimensions: Option<(u32, u32)>,
        format: String,
    },
    HexDump {
        bytes: Vec<u8>,
        total_size: u64,
    },
    Directory {
        item_count: usize,
        total_size: u64,
        subdir_count: usize,
        file_count: usize,
    },
    Error {
        message: String,
    },
    Loading,
    None,
}

/// File metadata for preview display
#[derive(Debug, Clone, PartialEq)]
pub struct FileMetadata {
    pub name: String,
    pub size: u64,
    pub file_type: String,
    pub modified: Option<SystemTime>,
    pub permissions: String,
    pub is_dir: bool,
}

impl Default for FileMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            size: 0,
            file_type: String::from("Unknown"),
            modified: None,
            permissions: String::new(),
            is_dir: false,
        }
    }
}

impl FileMetadata {
    pub fn from_path(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };
        let modified = metadata.modified().ok();

        let file_type = if is_dir {
            "Directory".to_string()
        } else {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_uppercase())
                .unwrap_or_else(|| "File".to_string())
        };

        let permissions = format_permissions(&metadata);

        Some(Self {
            name,
            size,
            file_type,
            modified,
            permissions,
            is_dir,
        })
    }

    pub fn has_all_fields(&self) -> bool {
        !self.name.is_empty() && !self.file_type.is_empty()
    }
}

/// Format file permissions (Unix-style)
fn format_permissions(metadata: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        let user = format_permission_triple((mode >> 6) & 0o7);
        let group = format_permission_triple((mode >> 3) & 0o7);
        let other = format_permission_triple(mode & 0o7);
        format!("{}{}{}", user, group, other)
    }
    #[cfg(not(unix))]
    {
        if metadata.permissions().readonly() {
            "Read-only".to_string()
        } else {
            "Read/Write".to_string()
        }
    }
}

#[cfg(unix)]
fn format_permission_triple(bits: u32) -> String {
    let r = if bits & 0o4 != 0 { 'r' } else { '-' };
    let w = if bits & 0o2 != 0 { 'w' } else { '-' };
    let x = if bits & 0o1 != 0 { 'x' } else { '-' };
    format!("{}{}{}", r, w, x)
}

/// Preview model holding current preview state
#[derive(Debug, Clone)]
pub struct Preview {
    content: PreviewContent,
    metadata: Option<FileMetadata>,
    current_path: Option<PathBuf>,
    scroll_offset: f32,
}

impl Default for Preview {
    fn default() -> Self {
        Self::new()
    }
}

impl Preview {
    pub fn new() -> Self {
        Self {
            content: PreviewContent::None,
            metadata: None,
            current_path: None,
            scroll_offset: 0.0,
        }
    }

    pub fn content(&self) -> &PreviewContent {
        &self.content
    }

    pub fn metadata(&self) -> Option<&FileMetadata> {
        self.metadata.as_ref()
    }

    pub fn current_path(&self) -> Option<&PathBuf> {
        self.current_path.as_ref()
    }

    pub fn clear(&mut self) {
        self.content = PreviewContent::None;
        self.metadata = None;
        self.current_path = None;
        self.scroll_offset = 0.0;
    }

    pub fn load_file(&mut self, path: &Path) {
        self.current_path = Some(path.to_path_buf());
        self.scroll_offset = 0.0;

        self.metadata = FileMetadata::from_path(path);

        if path.is_dir() {
            self.load_directory_content(path);
        } else {
            self.load_file_content(path);
        }
    }

    fn load_directory_content(&mut self, path: &Path) {
        match calculate_directory_stats(path) {
            Ok((item_count, total_size, subdir_count, file_count)) => {
                self.content = PreviewContent::Directory {
                    item_count,
                    total_size,
                    subdir_count,
                    file_count,
                };
            }
            Err(e) => {
                self.content = PreviewContent::Error {
                    message: e.to_string(),
                };
            }
        }
    }

    fn load_file_content(&mut self, path: &Path) {
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

        // Default to hex dump for binary files
        self.load_hex_dump(path);
    }

    fn load_text_content(&mut self, path: &Path, extension: Option<String>) {
        match fs::read_to_string(path) {
            Ok(content) => {
                let line_count = content.lines().count();
                let language = extension.and_then(|ext| detect_language(&ext));

                // Limit content size for preview
                let preview_content = if content.len() > 50000 {
                    format!(
                        "{}...\n\n[Content truncated - file too large]",
                        &content[..50000]
                    )
                } else {
                    content
                };

                self.content = PreviewContent::Text {
                    content: preview_content,
                    language,
                    line_count,
                };
            }
            Err(_) => {
                // If we can't read as text, try hex dump
                self.load_hex_dump(path);
            }
        }
    }

    fn load_image_content(&mut self, path: &Path, extension: Option<&str>) {
        let format = extension
            .map(|e| e.to_uppercase())
            .unwrap_or_else(|| "Image".to_string());

        let dimensions = get_image_dimensions(path);

        self.content = PreviewContent::Image {
            path: path.to_path_buf(),
            dimensions,
            format,
        };
    }

    fn load_hex_dump(&mut self, path: &Path) {
        let total_size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        match fs::read(path) {
            Ok(data) => {
                // Read first 256 bytes for hex dump
                let bytes: Vec<u8> = data.into_iter().take(256).collect();
                self.content = PreviewContent::HexDump { bytes, total_size };
            }
            Err(e) => {
                self.content = PreviewContent::Error {
                    message: format!("Cannot read file: {}", e),
                };
            }
        }
    }
}

/// Calculate directory statistics
pub fn calculate_directory_stats(path: &Path) -> std::io::Result<(usize, u64, usize, usize)> {
    let mut item_count = 0;
    let mut total_size = 0u64;
    let mut subdir_count = 0;
    let mut file_count = 0;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        item_count += 1;

        if metadata.is_dir() {
            subdir_count += 1;
        } else {
            file_count += 1;
            total_size += metadata.len();
        }
    }

    Ok((item_count, total_size, subdir_count, file_count))
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
            "txt"
                | "md"
                | "rs"
                | "js"
                | "ts"
                | "jsx"
                | "tsx"
                | "py"
                | "rb"
                | "go"
                | "java"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "css"
                | "scss"
                | "sass"
                | "less"
                | "html"
                | "htm"
                | "xml"
                | "json"
                | "yaml"
                | "yml"
                | "toml"
                | "ini"
                | "cfg"
                | "conf"
                | "sh"
                | "bash"
                | "zsh"
                | "fish"
                | "ps1"
                | "bat"
                | "cmd"
                | "sql"
                | "graphql"
                | "vue"
                | "svelte"
                | "astro"
                | "php"
                | "swift"
                | "kt"
                | "kts"
                | "scala"
                | "clj"
                | "cljs"
                | "ex"
                | "exs"
                | "erl"
                | "hrl"
                | "hs"
                | "ml"
                | "mli"
                | "fs"
                | "fsx"
                | "r"
                | "R"
                | "jl"
                | "lua"
                | "vim"
                | "el"
                | "lisp"
                | "scm"
                | "rkt"
                | "pl"
                | "pm"
                | "t"
                | "awk"
                | "sed"
                | "makefile"
                | "cmake"
                | "dockerfile"
                | "gitignore"
                | "gitattributes"
                | "editorconfig"
                | "env"
                | "lock"
                | "log"
                | "csv"
                | "tsv"
        )
    )
}

/// Check if file is likely text by reading first bytes
fn is_likely_text_file(path: &Path) -> bool {
    if let Ok(data) = fs::read(path) {
        let sample: Vec<u8> = data.into_iter().take(512).collect();
        !sample.contains(&0)
            && sample
                .iter()
                .filter(|&&b| b < 32 && b != 9 && b != 10 && b != 13)
                .count()
                < sample.len() / 10
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
    // Try to read image dimensions using the image crate if available
    let _ = path;
    None
}

/// Format file size for display
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// Format date for display
pub fn format_date(time: SystemTime) -> String {
    use std::time::UNIX_EPOCH;

    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    // Simple date formatting
    let days_since_epoch = secs / 86400;
    let years = 1970 + (days_since_epoch / 365);
    let remaining_days = days_since_epoch % 365;
    let month = (remaining_days / 30) + 1;
    let day = (remaining_days % 30) + 1;

    format!("{:04}-{:02}-{:02}", years, month.min(12), day.min(31))
}

/// Format hex dump for display
pub fn format_hex_dump(bytes: &[u8]) -> Vec<(String, String, String)> {
    let mut lines = Vec::new();

    for (i, chunk) in bytes.chunks(16).enumerate() {
        let offset = format!("{:08X}", i * 16);

        let hex: String = chunk
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        // Pad hex to 48 chars (16 bytes * 3 chars each - 1 for last space)
        let hex_padded = format!("{:<47}", hex);

        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();

        lines.push((offset, hex_padded, ascii));
    }

    lines
}

/// Preview view component
pub struct PreviewView {
    preview: Preview,
    focus_handle: FocusHandle,
}

impl PreviewView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            preview: Preview::new(),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn preview(&self) -> &Preview {
        &self.preview
    }

    pub fn preview_mut(&mut self) -> &mut Preview {
        &mut self.preview
    }

    pub fn load_file(&mut self, path: &Path) {
        self.preview.load_file(path);
    }

    pub fn clear(&mut self) {
        self.preview.clear();
    }
}

impl Focusable for PreviewView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PreviewView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        let bg_dark = theme.bg_secondary;
        let bg_header = theme.bg_tertiary;
        let border_color = theme.border_default;
        let text_gray = theme.text_muted;
        let text_light = theme.text_primary;
        let accent = theme.accent_primary;

        div()
            .id("preview-content")
            .size_full()
            .bg(bg_dark)
            .flex()
            .flex_col()
            // Header toolbar - matches main toolbar height (52px)
            .child(
                div()
                    .h(px(52.0))
                    .bg(bg_dark)
                    .border_b_1()
                    .border_color(border_color)
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(text_gray)
                            .child("PREVIEW"),
                    )
                    .child(
                        div().flex().gap_2().child(
                            div()
                                .w(px(24.0))
                                .h(px(24.0))
                                .rounded_md()
                                .bg(theme.bg_hover)
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .child(div().text_xs().text_color(text_gray).child("×")),
                        ),
                    ),
            )
            // Metadata header
            .child(self.render_metadata_header(
                bg_header,
                border_color,
                text_light,
                text_gray,
                accent,
            ))
            .child(self.render_content(bg_dark, text_light, text_gray, accent))
            // Bottom info bar
            .child(self.render_info_bar(bg_dark, border_color, text_gray))
    }
}

impl PreviewView {
    fn render_metadata_header(
        &self,
        bg_header: gpui::Rgba,
        border_color: gpui::Rgba,
        text_light: gpui::Rgba,
        text_gray: gpui::Rgba,
        accent: gpui::Rgba,
    ) -> impl IntoElement {
        let metadata = self.preview.metadata();

        div()
            .bg(bg_header)
            .border_b_1()
            .border_color(border_color)
            .p_4()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_3()
                    .mb_3()
                    .child(
                        div()
                            .p_2()
                            .bg(gpui::rgb(0x21262d))
                            .rounded_lg()
                            .child(self.render_file_icon(text_gray)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(text_light)
                                    .truncate()
                                    .child(
                                        metadata
                                            .map(|m| m.name.clone())
                                            .unwrap_or_else(|| "No file selected".to_string()),
                                    ),
                            )
                            .child(
                                div().text_xs().text_color(text_gray).mt_0p5().child(
                                    metadata
                                        .map(|m| {
                                            if m.is_dir {
                                                m.file_type.clone()
                                            } else {
                                                format!("{} • {}", format_size(m.size), m.file_type)
                                            }
                                        })
                                        .unwrap_or_default(),
                                ),
                            ),
                    ),
            )
            .child(self.render_metadata_details(text_gray, accent))
    }

    fn render_file_icon(&self, text_gray: gpui::Rgba) -> impl IntoElement {
        let is_dir = self.preview.metadata().map(|m| m.is_dir).unwrap_or(false);

        let icon_path = if is_dir {
            "assets/icons/folder.svg"
        } else {
            "assets/icons/file.svg"
        };

        svg().path(icon_path).size(px(32.0)).text_color(text_gray)
    }

    fn render_metadata_details(
        &self,
        text_gray: gpui::Rgba,
        accent: gpui::Rgba,
    ) -> impl IntoElement {
        let metadata = self.preview.metadata();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_xs()
            .text_color(text_gray)
            .when_some(metadata, |this, meta| {
                this.child(
                    div()
                        .flex()
                        .gap_4()
                        .child(
                            div().flex().gap_1().child("Size:").child(
                                div()
                                    .text_color(gpui::rgb(0xc9d1d9))
                                    .child(format_size(meta.size)),
                            ),
                        )
                        .when_some(meta.modified, |this, modified| {
                            this.child(
                                div().flex().gap_1().child("Modified:").child(
                                    div()
                                        .text_color(gpui::rgb(0xc9d1d9))
                                        .child(format_date(modified)),
                                ),
                            )
                        }),
                )
                .when(!meta.permissions.is_empty(), |this| {
                    this.child(
                        div().flex().gap_1().child("Permissions:").child(
                            div()
                                .text_color(gpui::rgb(0xc9d1d9))
                                .child(meta.permissions.clone()),
                        ),
                    )
                })
            })
            .when(
                matches!(self.preview.content(), PreviewContent::Text { .. }),
                |this| {
                    this.child(
                        div().mt_2().child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .gap_2()
                                .py_1p5()
                                .px_3()
                                .bg(accent)
                                .rounded_md()
                                .text_xs()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(gpui::white())
                                .cursor_pointer()
                                .child("Explain Code"),
                        ),
                    )
                },
            )
    }

    fn render_content(
        &self,
        bg_dark: gpui::Rgba,
        text_light: gpui::Rgba,
        text_gray: gpui::Rgba,
        accent: gpui::Rgba,
    ) -> impl IntoElement {
        let content_element: gpui::Div = match self.preview.content() {
            PreviewContent::None => self.render_empty_state(text_gray),
            PreviewContent::Loading => self.render_loading_state(text_gray),
            PreviewContent::Error { message } => self.render_error_state(message, text_gray),
            PreviewContent::Text {
                content,
                language,
                line_count,
            } => self.render_text_content(
                content,
                language.as_deref(),
                *line_count,
                text_light,
                text_gray,
            ),
            PreviewContent::Image {
                path,
                dimensions,
                format,
            } => self.render_image_content(path, dimensions.as_ref(), format, text_gray),
            PreviewContent::HexDump { bytes, total_size } => {
                self.render_hex_dump(bytes, *total_size, text_light, text_gray)
            }
            PreviewContent::Directory {
                item_count,
                total_size,
                subdir_count,
                file_count,
            } => self.render_directory_stats(
                *item_count,
                *total_size,
                *subdir_count,
                *file_count,
                text_light,
                text_gray,
                accent,
            ),
        };

        div()
            .flex_1()
            .overflow_hidden()
            .bg(bg_dark)
            .p_4()
            .child(content_element)
    }

    fn render_empty_state(&self, text_gray: gpui::Rgba) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .h_full()
            .text_color(text_gray)
            .child(
                svg()
                    .path("assets/icons/file.svg")
                    .size(px(48.0))
                    .text_color(text_gray)
                    .mb_4(),
            )
            .child(div().text_sm().child("Select a file to preview"))
    }

    fn render_loading_state(&self, text_gray: gpui::Rgba) -> gpui::Div {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h_full()
            .text_color(text_gray)
            .child("Loading...")
    }

    fn render_error_state(&self, message: &str, text_gray: gpui::Rgba) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .h_full()
            .text_color(gpui::rgb(0xf85149))
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child("Error"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(text_gray)
                    .mt_2()
                    .child(message.to_string()),
            )
    }

    fn render_text_content(
        &self,
        content: &str,
        language: Option<&str>,
        line_count: usize,
        text_light: gpui::Rgba,
        text_gray: gpui::Rgba,
    ) -> gpui::Div {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let line_number_width = line_count.to_string().len().max(3);
        let lang_str = language.map(|s| s.to_string());

        div()
            .flex()
            .flex_col()
            .font_family("JetBrains Mono")
            .text_xs()
            .when_some(lang_str, |this, lang| {
                this.child(
                    div()
                        .text_color(text_gray)
                        .mb_2()
                        .child(format!("Language: {}", lang)),
                )
            })
            .child(
                div()
                    .flex()
                    .child(
                        // Line numbers column
                        div()
                            .flex()
                            .flex_col()
                            .text_color(text_gray)
                            .pr_3()
                            .border_r_1()
                            .border_color(gpui::rgb(0x30363d))
                            .mr_3()
                            .children((0..lines.len()).map(|i| {
                                div()
                                    .text_right()
                                    .min_w(px((line_number_width * 8) as f32))
                                    .child(format!("{}", i + 1))
                            })),
                    )
                    .child(div().flex().flex_col().text_color(text_light).children(
                        lines.into_iter().map(|line| {
                            div().whitespace_nowrap().child(if line.is_empty() {
                                " ".to_string()
                            } else {
                                line
                            })
                        }),
                    )),
            )
    }

    fn render_image_content(
        &self,
        _path: &Path,
        dimensions: Option<&(u32, u32)>,
        format: &str,
        text_gray: gpui::Rgba,
    ) -> gpui::Div {
        let dims = dimensions.copied();
        let format_str = format.to_string();

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_4()
            .child(
                div()
                    .w(px(200.0))
                    .h(px(200.0))
                    .bg(gpui::rgb(0x21262d))
                    .rounded_lg()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        svg()
                            .path("assets/icons/file-image.svg")
                            .size(px(64.0))
                            .text_color(text_gray),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(text_gray)
                    .text_center()
                    .child(format!("Format: {}", format_str))
                    .when_some(dims, |this, (w, h)| {
                        this.child(div().mt_1().child(format!("Dimensions: {}×{}", w, h)))
                    }),
            )
    }

    fn render_hex_dump(
        &self,
        bytes: &[u8],
        total_size: u64,
        text_light: gpui::Rgba,
        text_gray: gpui::Rgba,
    ) -> gpui::Div {
        let hex_lines = format_hex_dump(bytes);
        let bytes_len = bytes.len();

        div()
            .flex()
            .flex_col()
            .font_family("JetBrains Mono")
            .text_xs()
            .child(div().text_color(text_gray).mb_3().child(format!(
                "Showing first {} of {} bytes",
                bytes_len,
                format_size(total_size)
            )))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .children(hex_lines.into_iter().map(|(offset, hex, ascii)| {
                        div()
                            .flex()
                            .gap_3()
                            .child(div().text_color(text_gray).min_w(px(72.0)).child(offset))
                            .child(div().text_color(text_light).min_w(px(380.0)).child(hex))
                            .child(div().text_color(gpui::rgb(0x7ee787)).child(ascii))
                    })),
            )
    }

    fn render_directory_stats(
        &self,
        item_count: usize,
        total_size: u64,
        subdir_count: usize,
        file_count: usize,
        text_light: gpui::Rgba,
        text_gray: gpui::Rgba,
        accent: gpui::Rgba,
    ) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        svg()
                            .path("assets/icons/folder-open.svg")
                            .size(px(48.0))
                            .text_color(accent),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(text_light)
                            .child(format!("{} items", item_count)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .text_sm()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(div().text_color(text_gray).child("Folders"))
                            .child(
                                div()
                                    .text_color(text_light)
                                    .child(format!("{}", subdir_count)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(div().text_color(text_gray).child("Files"))
                            .child(
                                div()
                                    .text_color(text_light)
                                    .child(format!("{}", file_count)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(div().text_color(text_gray).child("Total Size"))
                            .child(div().text_color(text_light).child(format_size(total_size))),
                    ),
            )
    }

    fn render_info_bar(
        &self,
        bg_dark: gpui::Rgba,
        border_color: gpui::Rgba,
        text_gray: gpui::Rgba,
    ) -> impl IntoElement {
        let metadata = self.preview.metadata();
        let file_type = metadata.map(|m| m.file_type.clone()).unwrap_or_default();
        let modified = metadata
            .and_then(|m| m.modified)
            .map(format_date)
            .unwrap_or_default();

        div()
            .h(px(28.0))
            .bg(bg_dark)
            .border_t_1()
            .border_color(border_color)
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .text_xs()
            .text_color(text_gray)
            .child("UTF-8")
            .child(modified)
            .child(file_type)
    }
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod tests;
