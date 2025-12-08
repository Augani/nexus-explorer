use std::path::PathBuf;

use gpui::{
    div, prelude::*, px, svg, App, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Window,
};

use crate::models::theme_colors;
use adabraka_ui::components::input::{InputEvent, InputState};

/// Actions that can be triggered from the symlink dialog
#[derive(Clone, Debug)]
pub enum SymlinkDialogAction {
    Create { target: PathBuf, link_path: PathBuf },
    Cancel,
}

/// Dialog for creating symbolic links
pub struct SymlinkDialog {
    target_path: PathBuf,
    link_name_input: Entity<InputState>,
    link_location_input: Entity<InputState>,
    focus_handle: FocusHandle,
    pending_action: Option<SymlinkDialogAction>,
}

impl SymlinkDialog {
    pub fn new(target_path: PathBuf, default_location: PathBuf, cx: &mut Context<Self>) -> Self {
        let target_name = target_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("link");
        let default_link_name = format!("{} link", target_name);

        let link_name_input = cx.new(|cx| {
            let mut state = InputState::new(cx);
            state.content = default_link_name.into();
            state.select_on_focus = true;
            state
        });

        let link_location_input = cx.new(|cx| {
            let mut state = InputState::new(cx);
            state.content = default_location.to_string_lossy().to_string().into();
            state
        });

        cx.subscribe(&link_name_input, |dialog: &mut Self, _, event: &InputEvent, cx| {
            if let InputEvent::Enter = event {
                dialog.submit(cx);
            }
        })
        .detach();

        cx.subscribe(&link_location_input, |dialog: &mut Self, _, event: &InputEvent, cx| {
            if let InputEvent::Enter = event {
                dialog.submit(cx);
            }
        })
        .detach();

        Self {
            target_path,
            link_name_input,
            link_location_input,
            focus_handle: cx.focus_handle(),
            pending_action: None,
        }
    }

    pub fn take_pending_action(&mut self) -> Option<SymlinkDialogAction> {
        self.pending_action.take()
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let link_name = self.link_name_input.read(cx).content.to_string();
        let link_location = self.link_location_input.read(cx).content.to_string();

        if link_name.is_empty() {
            return;
        }

        let link_path = PathBuf::from(&link_location).join(&link_name);

        self.pending_action = Some(SymlinkDialogAction::Create {
            target: self.target_path.clone(),
            link_path,
        });
        cx.notify();
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.pending_action = Some(SymlinkDialogAction::Cancel);
        cx.notify();
    }
}

impl Focusable for SymlinkDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SymlinkDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = theme_colors();
        let bg_primary = colors.bg_primary;
        let bg_secondary = colors.bg_secondary;
        let border_color = colors.border_default;
        let text_primary = colors.text_primary;
        let text_secondary = colors.text_secondary;
        let accent_primary = colors.accent_primary;
        let hover_bg = colors.bg_hover;

        let target_display = self.target_path.to_string_lossy().to_string();

        div()
            .id("symlink-dialog")
            .track_focus(&self.focus_handle)
            .w(px(480.0))
            .bg(bg_primary)
            .border_1()
            .border_color(border_color)
            .rounded_lg()
            .shadow_xl()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                // Header
                div()
                    .px_4()
                    .py_3()
                    .bg(bg_secondary)
                    .border_b_1()
                    .border_color(border_color)
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        svg()
                            .path("assets/icons/link-2.svg")
                            .size(px(18.0))
                            .text_color(accent_primary),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(text_primary)
                            .child("Create Symbolic Link"),
                    ),
            )
            .child(
                // Content
                div()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        // Target info
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(text_secondary)
                                    .child("Link Target"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_2()
                                    .bg(bg_secondary)
                                    .rounded_md()
                                    .border_1()
                                    .border_color(border_color)
                                    .text_sm()
                                    .text_color(text_primary)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(target_display),
                            ),
                    )
                    .child(
                        // Link name input
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(text_secondary)
                                    .child("Link Name"),
                            )
                            .child(self.link_name_input.clone()),
                    )
                    .child(
                        // Link location input
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(text_secondary)
                                    .child("Link Location"),
                            )
                            .child(self.link_location_input.clone()),
                    ),
            )
            .child(
                // Footer with buttons
                div()
                    .px_4()
                    .py_3()
                    .bg(bg_secondary)
                    .border_t_1()
                    .border_color(border_color)
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        div()
                            .id("cancel-btn")
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .border_1()
                            .border_color(border_color)
                            .text_sm()
                            .text_color(text_primary)
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg))
                            .on_click(cx.listener(|dialog, _, _, cx| {
                                dialog.cancel(cx);
                            }))
                            .child("Cancel"),
                    )
                    .child(
                        div()
                            .id("create-btn")
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .bg(accent_primary)
                            .text_sm()
                            .text_color(gpui::rgb(0xffffff))
                            .cursor_pointer()
                            .hover(|s| s.opacity(0.9))
                            .on_click(cx.listener(|dialog, _, _, cx| {
                                dialog.submit(cx);
                            }))
                            .child("Create Link"),
                    ),
            )
    }
}

/// Creates a symbolic link at the specified path pointing to the target
pub fn create_symbolic_link(target: &PathBuf, link_path: &PathBuf) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path)
    }

    #[cfg(windows)]
    {
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link_path)
        } else {
            std::os::windows::fs::symlink_file(target, link_path)
        }
    }
}
