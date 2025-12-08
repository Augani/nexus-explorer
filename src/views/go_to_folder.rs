use std::path::PathBuf;

use gpui::{
    div, prelude::*, px, svg, App, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, SharedString, Styled, Window,
};

use crate::models::{theme_colors, BookmarkManager};


pub struct GoToFolderView {
    input_value: String,
    focus_handle: FocusHandle,
    is_visible: bool,
    recent_locations: Vec<PathBuf>,
    bookmarks: Vec<(String, PathBuf)>,
    error_message: Option<String>,
    pending_navigation: Option<PathBuf>,
}

impl GoToFolderView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            input_value: String::new(),
            focus_handle: cx.focus_handle(),
            is_visible: false,
            recent_locations: Vec::new(),
            bookmarks: Vec::new(),
            error_message: None,
            pending_navigation: None,
        }
    }


    pub fn show(&mut self, cx: &mut Context<Self>) {
        self.is_visible = true;
        self.input_value.clear();
        self.error_message = None;
        cx.notify();
    }


    pub fn hide(&mut self, cx: &mut Context<Self>) {
        self.is_visible = false;
        self.input_value.clear();
        self.error_message = None;
        cx.notify();
    }


    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.is_visible {
            self.hide(cx);
        } else {
            self.show(cx);
        }
    }


    pub fn is_visible(&self) -> bool {
        self.is_visible
    }


    pub fn update_recent(&mut self, manager: &BookmarkManager) {
        self.recent_locations = manager.recent().iter().cloned().collect();
        self.bookmarks = manager
            .bookmarks()
            .iter()
            .map(|b| (b.name.clone(), b.path.clone()))
            .collect();
    }


    pub fn take_pending_navigation(&mut self) -> Option<PathBuf> {
        self.pending_navigation.take()
    }


    pub fn set_input(&mut self, text: String, cx: &mut Context<Self>) {
        self.input_value = text;
        self.error_message = None;
        cx.notify();
    }


    pub fn input_value(&self) -> &str {
        &self.input_value
    }


    pub fn navigate(&mut self, cx: &mut Context<Self>) {
        let path = self.expand_path(&self.input_value);

        if path.exists() && path.is_dir() {
            self.pending_navigation = Some(path);
            self.hide(cx);
        } else if path.exists() {
            self.error_message = Some("Path is not a directory".to_string());
            cx.notify();
        } else {
            self.error_message = Some("Path does not exist".to_string());
            cx.notify();
        }
    }


    fn navigate_to(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if path.exists() && path.is_dir() {
            self.pending_navigation = Some(path);
            self.hide(cx);
        }
    }


    fn expand_path(&self, input: &str) -> PathBuf {
        if input.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                return home.join(&input[1..].trim_start_matches('/'));
            }
        }
        PathBuf::from(input)
    }


    fn get_suggestions(&self) -> Vec<(String, PathBuf)> {
        let input_lower = self.input_value.to_lowercase();
        let mut suggestions = Vec::new();

        for (name, path) in &self.bookmarks {
            if name.to_lowercase().contains(&input_lower)
                || path.to_string_lossy().to_lowercase().contains(&input_lower)
            {
                suggestions.push((format!("📌 {}", name), path.clone()));
            }
        }

        for path in &self.recent_locations {
            let path_str = path.to_string_lossy();
            if path_str.to_lowercase().contains(&input_lower) {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown");
                suggestions.push((format!("🕐 {}", name), path.clone()));
            }
        }

        suggestions.truncate(10);
        suggestions
    }
}

impl Focusable for GoToFolderView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GoToFolderView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_visible {
            return div().into_any_element();
        }

        let theme = theme_colors();
        let input_value = self.input_value.clone();
        let error_message = self.error_message.clone();
        let suggestions = self.get_suggestions();
        let recent_locations = self.recent_locations.clone();

        div()
            .id("go-to-folder-overlay")
            .absolute()
            .inset_0()
            .bg(gpui::rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, _event, _window, cx| {
                    view.hide(cx);
                }),
            )
            .child(
                div()
                    .id("go-to-folder-dialog")
                    .w(px(500.0))
                    .max_h(px(400.0))
                    .bg(theme.bg_secondary)
                    .border_1()
                    .border_color(theme.border_default)
                    .rounded_lg()
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_event, _window, _cx| {
                    })
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(theme.border_subtle)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_primary)
                                    .child("Go to Folder"),
                            )
                            .child(
                                div()
                                    .id("close-btn")
                                    .cursor_pointer()
                                    .p_1()
                                    .rounded_md()
                                    .hover(|s| s.bg(theme.bg_hover))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|view, _event, _window, cx| {
                                            view.hide(cx);
                                        }),
                                    )
                                    .child(
                                        svg()
                                            .path("assets/icons/x.svg")
                                            .size(px(16.0))
                                            .text_color(theme.text_secondary),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py_2()
                                    .bg(theme.bg_primary)
                                    .border_1()
                                    .border_color(if error_message.is_some() {
                                        theme.error
                                    } else {
                                        theme.border_default
                                    })
                                    .rounded_md()
                                    .child(
                                        svg()
                                            .path("assets/icons/folder.svg")
                                            .size(px(16.0))
                                            .text_color(theme.text_secondary),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_sm()
                                            .text_color(theme.text_primary)
                                            .child(if input_value.is_empty() {
                                                div()
                                                    .text_color(theme.text_muted)
                                                    .child("Enter path (e.g., ~/Documents)")
                                            } else {
                                                div().child(input_value.clone())
                                            }),
                                    ),
                            )
                            .when(error_message.is_some(), |s| {
                                s.child(
                                    div()
                                        .mt_2()
                                        .text_xs()
                                        .text_color(theme.error)
                                        .child(error_message.unwrap_or_default()),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .px_4()
                            .pb_3()
                            .when(!suggestions.is_empty() && !input_value.is_empty(), |s| {
                                s.child(
                                    div()
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(theme.text_muted)
                                        .mb_2()
                                        .child("SUGGESTIONS"),
                                )
                                .children(
                                    suggestions.into_iter().map(|(name, path)| {
                                        let path_clone = path.clone();
                                        let path_display = path.to_string_lossy().to_string();

                                        div()
                                            .id(SharedString::from(format!(
                                                "suggestion-{}",
                                                path_display
                                            )))
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .px_2()
                                            .py_1p5()
                                            .rounded_md()
                                            .cursor_pointer()
                                            .text_sm()
                                            .text_color(theme.text_secondary)
                                            .hover(|h| {
                                                h.bg(theme.bg_hover).text_color(theme.text_primary)
                                            })
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(move |view, _event, _window, cx| {
                                                    view.navigate_to(path_clone.clone(), cx);
                                                }),
                                            )
                                            .child(name)
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .text_xs()
                                                    .text_color(theme.text_muted)
                                                    .overflow_hidden()
                                                    .child(path_display),
                                            )
                                    }),
                                )
                            })
                            .when(
                                input_value.is_empty() && !recent_locations.is_empty(),
                                |s| {
                                    s.child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.text_muted)
                                            .mb_2()
                                            .child("RECENT LOCATIONS"),
                                    )
                                    .children(
                                        recent_locations.into_iter().take(10).map(|path| {
                                            let path_clone = path.clone();
                                            let name = path
                                                .file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("Unknown")
                                                .to_string();
                                            let path_display = path.to_string_lossy().to_string();

                                            div()
                                                .id(SharedString::from(format!(
                                                    "recent-{}",
                                                    path_display
                                                )))
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .px_2()
                                                .py_1p5()
                                                .rounded_md()
                                                .cursor_pointer()
                                                .text_sm()
                                                .text_color(theme.text_secondary)
                                                .hover(|h| {
                                                    h.bg(theme.bg_hover)
                                                        .text_color(theme.text_primary)
                                                })
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(
                                                        move |view, _event, _window, cx| {
                                                            view.navigate_to(
                                                                path_clone.clone(),
                                                                cx,
                                                            );
                                                        },
                                                    ),
                                                )
                                                .child(
                                                    svg()
                                                        .path("assets/icons/folder.svg")
                                                        .size(px(14.0))
                                                        .text_color(theme.accent_primary),
                                                )
                                                .child(name)
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .text_xs()
                                                        .text_color(theme.text_muted)
                                                        .overflow_hidden()
                                                        .child(path_display),
                                                )
                                        }),
                                    )
                                },
                            ),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .border_t_1()
                            .border_color(theme.border_subtle)
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child("Press Enter to navigate")
                            .child("Esc to close"),
                    ),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    fn expand_path_helper(input: &str) -> PathBuf {
        if input.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                return home.join(&input[1..].trim_start_matches('/'));
            }
        }
        PathBuf::from(input)
    }

    #[test]
    fn test_expand_path_tilde() {
        if let Some(home) = dirs::home_dir() {
            let expanded = expand_path_helper("~/Documents");
            assert_eq!(expanded, home.join("Documents"));

            let expanded = expand_path_helper("~");
            assert_eq!(expanded, home);
        }

        let expanded = expand_path_helper("/usr/local");
        assert_eq!(expanded, PathBuf::from("/usr/local"));
    }
}
