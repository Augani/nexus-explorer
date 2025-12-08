use std::path::PathBuf;

use gpui::{
    div, prelude::*, px, svg, App, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Window,
};

use crate::models::{theme_colors, BatchRename, RenamePreview};
use adabraka_ui::components::input::{InputEvent, InputState};


#[derive(Clone, Debug)]
pub enum BatchRenameDialogAction {
    Apply { renamed_paths: Vec<PathBuf> },
    Cancel,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenameMode {
    Pattern,
    FindReplace,
}


pub struct BatchRenameDialog {
    batch_rename: BatchRename,
    mode: RenameMode,
    pattern_input: Entity<InputState>,
    find_input: Entity<InputState>,
    replace_input: Entity<InputState>,
    use_regex: bool,
    case_insensitive: bool,
    focus_handle: FocusHandle,
    pending_action: Option<BatchRenameDialogAction>,
}

impl BatchRenameDialog {
    pub fn new(files: Vec<PathBuf>, cx: &mut Context<Self>) -> Self {
        let batch_rename = BatchRename::new(files);

        let pattern_input = cx.new(|cx| {
            let mut state = InputState::new(cx);
            state.placeholder = "e.g., photo_{n} or {name}_backup".into();
            state.select_on_focus = true;
            state
        });

        let find_input = cx.new(|cx| {
            let mut state = InputState::new(cx);
            state.placeholder = "Text to find...".into();
            state
        });

        let replace_input = cx.new(|cx| {
            let mut state = InputState::new(cx);
            state.placeholder = "Replace with...".into();
            state
        });

        cx.subscribe(&pattern_input, |dialog: &mut Self, input, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                if dialog.mode == RenameMode::Pattern {
                    let content = input.read(cx).content.to_string();
                    dialog.batch_rename.set_pattern(&content);
                    cx.notify();
                }
            }
        })
        .detach();

        cx.subscribe(&find_input, |dialog: &mut Self, input, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                if dialog.mode == RenameMode::FindReplace {
                    let find = input.read(cx).content.to_string();
                    let replace = dialog.replace_input.read(cx).content.to_string();
                    dialog.batch_rename.set_find_replace_with_options(
                        &find,
                        &replace,
                        dialog.use_regex,
                        dialog.case_insensitive,
                    );
                    cx.notify();
                }
            }
        })
        .detach();

        cx.subscribe(&replace_input, |dialog: &mut Self, input, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                if dialog.mode == RenameMode::FindReplace {
                    let find = dialog.find_input.read(cx).content.to_string();
                    let replace = input.read(cx).content.to_string();
                    dialog.batch_rename.set_find_replace_with_options(
                        &find,
                        &replace,
                        dialog.use_regex,
                        dialog.case_insensitive,
                    );
                    cx.notify();
                }
            }
        })
        .detach();

        Self {
            batch_rename,
            mode: RenameMode::Pattern,
            pattern_input,
            find_input,
            replace_input,
            use_regex: false,
            case_insensitive: false,
            focus_handle: cx.focus_handle(),
            pending_action: None,
        }
    }

    pub fn take_pending_action(&mut self) -> Option<BatchRenameDialogAction> {
        self.pending_action.take()
    }

    fn set_mode(&mut self, mode: RenameMode, cx: &mut Context<Self>) {
        self.mode = mode;
        self.update_preview(cx);
        cx.notify();
    }

    fn toggle_regex(&mut self, cx: &mut Context<Self>) {
        self.use_regex = !self.use_regex;
        self.update_preview(cx);
        cx.notify();
    }

    fn toggle_case_insensitive(&mut self, cx: &mut Context<Self>) {
        self.case_insensitive = !self.case_insensitive;
        self.update_preview(cx);
        cx.notify();
    }

    fn update_preview(&mut self, cx: &mut Context<Self>) {
        match self.mode {
            RenameMode::Pattern => {
                let pattern = self.pattern_input.read(cx).content.to_string();
                self.batch_rename.set_pattern(&pattern);
            }
            RenameMode::FindReplace => {
                let find = self.find_input.read(cx).content.to_string();
                let replace = self.replace_input.read(cx).content.to_string();
                self.batch_rename.set_find_replace_with_options(
                    &find,
                    &replace,
                    self.use_regex,
                    self.case_insensitive,
                );
            }
        }
    }

    fn apply(&mut self, cx: &mut Context<Self>) {
        if self.batch_rename.has_conflicts() {
            return;
        }

        match self.batch_rename.apply() {
            Ok(renamed_paths) => {
                self.pending_action = Some(BatchRenameDialogAction::Apply { renamed_paths });
            }
            Err(_) => {
            }
        }
        cx.notify();
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.pending_action = Some(BatchRenameDialogAction::Cancel);
        cx.notify();
    }

}


impl Focusable for BatchRenameDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BatchRenameDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = theme_colors();
        let bg_primary = colors.bg_primary;
        let bg_secondary = colors.bg_secondary;
        let border_color = colors.border_default;
        let text_primary = colors.text_primary;
        let text_secondary = colors.text_secondary;
        let accent_primary = colors.accent_primary;
        let hover_bg = colors.bg_hover;
        let error_color = colors.error;

        let file_count = self.batch_rename.files().len();
        let rename_count = self.batch_rename.rename_count();
        let has_conflicts = self.batch_rename.has_conflicts();
        let conflict_count = self.batch_rename.conflicts().len();

        let preview_items: Vec<_> = self.batch_rename.preview().to_vec();
        let colors_clone = colors.clone();

        div()
            .id("batch-rename-dialog")
            .track_focus(&self.focus_handle)
            .w(px(600.0))
            .max_h(px(500.0))
            .bg(bg_primary)
            .border_1()
            .border_color(border_color)
            .rounded_lg()
            .shadow_xl()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
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
                            .path("assets/icons/pen.svg")
                            .size(px(18.0))
                            .text_color(accent_primary),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(text_primary)
                            .child(format!("Batch Rename ({} files)", file_count)),
                    ),
            )
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(border_color)
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("pattern-tab")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .text_sm()
                            .cursor_pointer()
                            .when(self.mode == RenameMode::Pattern, |el| {
                                el.bg(accent_primary).text_color(gpui::rgb(0xffffff))
                            })
                            .when(self.mode != RenameMode::Pattern, |el| {
                                el.text_color(text_secondary).hover(|s| s.bg(hover_bg))
                            })
                            .on_click(cx.listener(|dialog, _, _, cx| {
                                dialog.set_mode(RenameMode::Pattern, cx);
                            }))
                            .child("Pattern"),
                    )
                    .child(
                        div()
                            .id("find-replace-tab")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .text_sm()
                            .cursor_pointer()
                            .when(self.mode == RenameMode::FindReplace, |el| {
                                el.bg(accent_primary).text_color(gpui::rgb(0xffffff))
                            })
                            .when(self.mode != RenameMode::FindReplace, |el| {
                                el.text_color(text_secondary).hover(|s| s.bg(hover_bg))
                            })
                            .on_click(cx.listener(|dialog, _, _, cx| {
                                dialog.set_mode(RenameMode::FindReplace, cx);
                            }))
                            .child("Find & Replace"),
                    ),
            )
            .child(
                div()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .flex_1()
                    .overflow_hidden()
                    .when(self.mode == RenameMode::Pattern, |el| {
                        el.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(text_secondary)
                                        .child("Pattern (use {n}, {name}, {date}, {ext})"),
                                )
                                .child(self.pattern_input.clone()),
                        )
                    })
                    .when(self.mode == RenameMode::FindReplace, |el| {
                        el.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(text_secondary)
                                                .child("Find"),
                                        )
                                        .child(self.find_input.clone()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(text_secondary)
                                                .child("Replace"),
                                        )
                                        .child(self.replace_input.clone()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap_4()
                                        .child(
                                            div()
                                                .id("regex-toggle")
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .cursor_pointer()
                                                .on_click(cx.listener(|dialog, _, _, cx| {
                                                    dialog.toggle_regex(cx);
                                                }))
                                                .child(
                                                    div()
                                                        .size(px(14.0))
                                                        .rounded_sm()
                                                        .border_1()
                                                        .border_color(border_color)
                                                        .when(self.use_regex, |el| {
                                                            el.bg(accent_primary)
                                                        }),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(text_secondary)
                                                        .child("Use Regex"),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .id("case-toggle")
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .cursor_pointer()
                                                .on_click(cx.listener(|dialog, _, _, cx| {
                                                    dialog.toggle_case_insensitive(cx);
                                                }))
                                                .child(
                                                    div()
                                                        .size(px(14.0))
                                                        .rounded_sm()
                                                        .border_1()
                                                        .border_color(border_color)
                                                        .when(self.case_insensitive, |el| {
                                                            el.bg(accent_primary)
                                                        }),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(text_secondary)
                                                        .child("Case Insensitive"),
                                                ),
                                        ),
                                ),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .overflow_hidden()
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(text_secondary)
                                            .child("Preview"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(if has_conflicts { error_color } else { text_secondary })
                                            .child(if has_conflicts {
                                                format!("{} conflicts detected", conflict_count)
                                            } else {
                                                format!("{} files will be renamed", rename_count)
                                            }),
                                    ),
                            )
                            .child({
                                let mut preview_container = div()
                                    .id("preview-list")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .bg(bg_secondary)
                                    .rounded_md()
                                    .border_1()
                                    .border_color(border_color)
                                    .p_2();

                                for preview in &preview_items {
                                    let name_changed = preview.original != preview.new_name;
                                    let arrow_color = if preview.has_conflict {
                                        error_color
                                    } else if name_changed {
                                        colors_clone.success
                                    } else {
                                        text_secondary
                                    };

                                    let item = div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .py_1()
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_sm()
                                                .text_color(text_secondary)
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(preview.original.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(arrow_color)
                                                .child("→"),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_sm()
                                                .text_color(if preview.has_conflict { error_color } else { text_primary })
                                                .font_weight(if name_changed { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(preview.new_name.clone()),
                                        )
                                        .when(preview.has_conflict, |el| {
                                            el.child(
                                                div()
                                                    .text_xs()
                                                    .text_color(error_color)
                                                    .child("⚠ Conflict"),
                                            )
                                        });

                                    preview_container = preview_container.child(item);
                                }

                                preview_container
                            }),
                    ),
            )
            .child(
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
                            .id("apply-btn")
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .text_sm()
                            .cursor_pointer()
                            .when(has_conflicts || rename_count == 0, |el| {
                                el.bg(bg_secondary)
                                    .text_color(text_secondary)
                                    .cursor_not_allowed()
                            })
                            .when(!has_conflicts && rename_count > 0, |el| {
                                el.bg(accent_primary)
                                    .text_color(gpui::rgb(0xffffff))
                                    .hover(|s| s.opacity(0.9))
                                    .on_click(cx.listener(|dialog, _, _, cx| {
                                        dialog.apply(cx);
                                    }))
                            })
                            .child(format!("Rename {} Files", rename_count)),
                    ),
            )
    }
}
