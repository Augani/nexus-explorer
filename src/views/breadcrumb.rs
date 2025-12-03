use std::path::{Path, PathBuf};

use gpui::{
    div, prelude::*, px, svg, App, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, SharedString, Styled, Window,
};

use crate::models::{theme_colors, toolbar as toolbar_spacing};

/// Represents a single segment in the breadcrumb path
#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    pub name: String,
    pub path: PathBuf,
    pub is_root: bool,
}

impl PathSegment {
    pub fn new(name: String, path: PathBuf, is_root: bool) -> Self {
        Self { name, path, is_root }
    }
}

/// Breadcrumb component for path navigation
/// Renders clickable path segments with truncation support
pub struct Breadcrumb {
    segments: Vec<PathSegment>,
    max_visible: usize,
    show_ellipsis_menu: bool,
}

impl Breadcrumb {
    /// Create a new Breadcrumb from a path
    pub fn from_path(path: &Path) -> Self {
        let segments = Self::parse_path(path);
        Self {
            segments,
            max_visible: 4,
            show_ellipsis_menu: false,
        }
    }

    /// Parse a path into segments
    fn parse_path(path: &Path) -> Vec<PathSegment> {
        let mut segments = Vec::new();
        let mut current = Some(path);
        let mut paths_collected: Vec<(&Path, String)> = Vec::new();

        while let Some(p) = current {
            let name = if p.parent().is_none() {
                // Root path
                #[cfg(target_os = "windows")]
                {
                    p.to_str().unwrap_or("C:").to_string()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    "/".to_string()
                }
            } else {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string()
            };

            if !name.is_empty() {
                paths_collected.push((p, name));
            }
            current = p.parent();
        }

        // Reverse to get root-to-leaf order
        paths_collected.reverse();

        for (i, (p, name)) in paths_collected.into_iter().enumerate() {
            segments.push(PathSegment::new(
                name,
                p.to_path_buf(),
                i == 0,
            ));
        }

        segments
    }

    /// Get all segments
    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }

    /// Get the number of segments
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Get visible segments (respecting max_visible)
    pub fn visible_segments(&self) -> Vec<&PathSegment> {
        if self.segments.len() <= self.max_visible {
            self.segments.iter().collect()
        } else {
            // Show first segment, ellipsis, then last (max_visible - 2) segments
            let mut visible = Vec::new();
            visible.push(&self.segments[0]);
            
            let start = self.segments.len() - (self.max_visible - 1);
            for seg in &self.segments[start..] {
                visible.push(seg);
            }
            visible
        }
    }

    /// Get hidden segments (for ellipsis dropdown)
    pub fn hidden_segments(&self) -> Vec<&PathSegment> {
        if self.segments.len() <= self.max_visible {
            Vec::new()
        } else {
            let end = self.segments.len() - (self.max_visible - 1);
            self.segments[1..end].iter().collect()
        }
    }

    /// Check if truncation is needed
    pub fn needs_truncation(&self) -> bool {
        self.segments.len() > self.max_visible
    }

    /// Get path for a segment by index
    pub fn path_for_segment(&self, index: usize) -> Option<&Path> {
        self.segments.get(index).map(|s| s.path.as_path())
    }

    /// Set maximum visible segments
    pub fn set_max_visible(&mut self, max: usize) {
        self.max_visible = max.max(2); // At least 2 (root + current)
    }

    /// Toggle ellipsis menu visibility
    pub fn toggle_ellipsis_menu(&mut self) {
        self.show_ellipsis_menu = !self.show_ellipsis_menu;
    }

    /// Check if ellipsis menu is shown
    pub fn is_ellipsis_menu_shown(&self) -> bool {
        self.show_ellipsis_menu
    }

    /// Reconstruct full path from segments up to given index
    pub fn path_to_index(&self, index: usize) -> Option<PathBuf> {
        self.segments.get(index).map(|s| s.path.clone())
    }

    /// Get the full current path
    pub fn current_path(&self) -> Option<&Path> {
        self.segments.last().map(|s| s.path.as_path())
    }
}

/// View wrapper for Breadcrumb with GPUI rendering
pub struct BreadcrumbView {
    breadcrumb: Breadcrumb,
    focus_handle: FocusHandle,
    pending_navigation: Option<PathBuf>,
    on_navigate: Option<Box<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>>,
    context_menu_path: Option<PathBuf>,
}

impl BreadcrumbView {
    pub fn new(path: &Path, cx: &mut Context<Self>) -> Self {
        Self {
            breadcrumb: Breadcrumb::from_path(path),
            focus_handle: cx.focus_handle(),
            pending_navigation: None,
            on_navigate: None,
            context_menu_path: None,
        }
    }

    pub fn set_path(&mut self, path: &Path) {
        self.breadcrumb = Breadcrumb::from_path(path);
    }

    pub fn set_on_navigate<F>(&mut self, callback: F)
    where
        F: Fn(PathBuf, &mut Window, &mut App) + 'static,
    {
        self.on_navigate = Some(Box::new(callback));
    }

    pub fn inner(&self) -> &Breadcrumb {
        &self.breadcrumb
    }

    pub fn inner_mut(&mut self) -> &mut Breadcrumb {
        &mut self.breadcrumb
    }

    pub fn take_pending_navigation(&mut self) -> Option<PathBuf> {
        self.pending_navigation.take()
    }

    fn handle_segment_click(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        self.pending_navigation = Some(path);
        cx.notify();
    }

    fn handle_context_menu(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        self.context_menu_path = Some(path);
        cx.notify();
    }

    fn copy_path_to_clipboard(&mut self, path: &Path, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path_str) = path.to_str() {
            // Copy to clipboard - using cx to write to clipboard
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(path_str.to_string()));
        }
        self.context_menu_path = None;
    }
}

impl Focusable for BreadcrumbView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BreadcrumbView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Get theme colors for RPG styling
        let colors = theme_colors();
        
        let text_gray = colors.text_secondary;
        let text_light = colors.text_primary;
        let hover_color = colors.accent_primary;
        let hover_bg = colors.bg_hover;
        let menu_bg = colors.bg_tertiary;
        let border_color = colors.border_default;
        let accent_secondary = colors.accent_secondary;

        let needs_truncation = self.breadcrumb.needs_truncation();
        let visible_segments = self.breadcrumb.visible_segments();
        let hidden_segments = self.breadcrumb.hidden_segments();
        let show_ellipsis_menu = self.breadcrumb.show_ellipsis_menu;
        
        // Breadcrumb segment padding from toolbar spacing
        let segment_padding = px(toolbar_spacing::BREADCRUMB_PADDING);

        // RPG-styled breadcrumb with themed colors and decorative separators
        div()
            .id("breadcrumb")
            .flex()
            .items_center()
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .children(visible_segments.into_iter().enumerate().map(|(i, segment)| {
                let path = segment.path.clone();
                let path_for_context = segment.path.clone();
                let is_first = i == 0;
                let show_ellipsis = needs_truncation && i == 1;

                div()
                    .flex()
                    .items_center()
                    // Themed separator with accent color
                    .when(!is_first, |s| {
                        s.child(
                            svg()
                                .path("assets/icons/chevron-right.svg")
                                .size(px(14.0))
                                .text_color(accent_secondary)
                                .mx_1(),
                        )
                    })
                    .when(show_ellipsis, |s| {
                        let hidden = hidden_segments.clone();
                        s.child(
                            div()
                                .id("ellipsis-trigger")
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .px(segment_padding)
                                        .py_0p5()
                                        .rounded_sm()
                                        .cursor_pointer()
                                        .text_color(text_gray)
                                        .hover(|h| h.bg(hover_bg).text_color(hover_color))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|view, _event, _window, cx| {
                                                view.breadcrumb.toggle_ellipsis_menu();
                                                cx.notify();
                                            }),
                                        )
                                        .child("...")
                                )
                                .child(
                                    svg()
                                        .path("assets/icons/chevron-right.svg")
                                        .size(px(14.0))
                                        .text_color(accent_secondary)
                                        .mx_1(),
                                )
                                // Dropdown menu with RPG styling
                                .when(show_ellipsis_menu, |s| {
                                    s.child(
                                        div()
                                            .absolute()
                                            .top(px(28.0))
                                            .left_0()
                                            .bg(menu_bg)
                                            .border_1()
                                            .border_color(border_color)
                                            .rounded_md()
                                            .shadow_lg()
                                            .py_1()
                                            .min_w(px(150.0))
                                            .children(hidden.into_iter().map(|seg| {
                                                let nav_path = seg.path.clone();
                                                div()
                                                    .id(SharedString::from(format!("hidden-{}", seg.name)))
                                                    .px_3()
                                                    .py_1p5()
                                                    .text_sm()
                                                    .text_color(text_light)
                                                    .cursor_pointer()
                                                    .hover(|h| h.bg(hover_bg).text_color(hover_color))
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(move |view, _event, window, cx| {
                                                            view.handle_segment_click(nav_path.clone(), window, cx);
                                                            view.breadcrumb.show_ellipsis_menu = false;
                                                        }),
                                                    )
                                                    .child(seg.name.clone())
                                            }))
                                    )
                                })
                        )
                    })
                    // Segment with RPG hover effect
                    .child(
                        div()
                            .id(SharedString::from(format!("segment-{}", i)))
                            .px(segment_padding)
                            .py_0p5()
                            .rounded_sm()
                            .text_color(text_light)
                            .cursor_pointer()
                            .hover(|s| s.text_color(hover_color).bg(hover_bg))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |view, _event, window, cx| {
                                    view.handle_segment_click(path.clone(), window, cx);
                                }),
                            )
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |view, _event, window, cx| {
                                    view.handle_context_menu(path_for_context.clone(), window, cx);
                                }),
                            )
                            .child(segment.name.clone())
                    )
            }))
            // Context menu with RPG styling
            .when(self.context_menu_path.is_some(), |s| {
                let menu_path = self.context_menu_path.clone().unwrap();
                s.child(
                    div()
                        .absolute()
                        .top(px(28.0))
                        .bg(menu_bg)
                        .border_1()
                        .border_color(border_color)
                        .rounded_md()
                        .shadow_lg()
                        .py_1()
                        .min_w(px(120.0))
                        .child(
                            div()
                                .id("copy-path-menu")
                                .px_3()
                                .py_1p5()
                                .text_sm()
                                .text_color(text_light)
                                .cursor_pointer()
                                .hover(|h| h.bg(hover_bg).text_color(hover_color))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |view, _event, window, cx| {
                                        view.copy_path_to_clipboard(&menu_path, window, cx);
                                        cx.notify();
                                    }),
                                )
                                .child("Copy Path")
                        )
                )
            })
    }
}

#[cfg(test)]
#[path = "breadcrumb_tests.rs"]
mod tests;
