use std::time::Duration;

use gpui::{
    div, prelude::*, px, svg, App, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, Rgba, SharedString, Styled, Window,
};

use crate::models::{theme_colors, FileOperation, OperationId, OperationStatus, OperationType};

/
fn with_alpha(color: Rgba, alpha: f32) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha,
    }
}

/
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/
fn format_speed(bytes_per_sec: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes_per_sec >= GB {
        format!("{:.1} GB/s", bytes_per_sec as f64 / GB as f64)
    } else if bytes_per_sec >= MB {
        format!("{:.1} MB/s", bytes_per_sec as f64 / MB as f64)
    } else if bytes_per_sec >= KB {
        format!("{:.1} KB/s", bytes_per_sec as f64 / KB as f64)
    } else {
        format!("{} B/s", bytes_per_sec)
    }
}

/
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size >= TB {
        format!("{:.1} TB", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressPanelAction {
    Cancel(OperationId),
    Skip(OperationId),
    Retry(OperationId),
    Dismiss(OperationId),
    DismissAll,
}

/
pub struct ProgressPanelView {
    operations: Vec<FileOperation>,
    focus_handle: FocusHandle,
    pending_action: Option<ProgressPanelAction>,
    is_expanded: bool,
}

impl ProgressPanelView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            operations: Vec::new(),
            focus_handle: cx.focus_handle(),
            pending_action: None,
            is_expanded: true,
        }
    }

    /
    pub fn update_operations(&mut self, operations: Vec<FileOperation>, cx: &mut Context<Self>) {
        self.operations = operations;
        cx.notify();
    }

    /
    pub fn take_pending_action(&mut self) -> Option<ProgressPanelAction> {
        self.pending_action.take()
    }

    /
    pub fn has_operations(&self) -> bool {
        !self.operations.is_empty()
    }

    /
    pub fn has_active_operations(&self) -> bool {
        self.operations.iter().any(|op| op.status.is_active())
    }

    /
    pub fn toggle_expanded(&mut self, cx: &mut Context<Self>) {
        self.is_expanded = !self.is_expanded;
        cx.notify();
    }

    fn render_operation(&self, op: &FileOperation, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        let bg_color = theme.bg_tertiary;
        let border_color = theme.border_default;
        let text_primary = theme.text_primary;
        let text_muted = theme.text_muted;
        let accent = theme.accent_primary;
        let success = theme.success;
        let error = theme.error;
        let hover_bg = theme.bg_hover;

        let op_id = op.id;
        let op_type = op.op_type;
        let status = op.status.clone();
        let progress = op.progress.clone();
        let current_file = progress.current_file.clone();
        let has_error = op.current_error.is_some();
        let error_msg = op.current_error.as_ref().map(|e| e.message.clone());
        let skipped_count = op.error_state.skipped_count;

        let percentage = progress.percentage();
        let is_active = status.is_active();
        let is_completed = matches!(status, OperationStatus::Completed);
        let is_failed = matches!(status, OperationStatus::Failed(_));
        let is_cancelled = matches!(status, OperationStatus::Cancelled);
        let is_paused = matches!(status, OperationStatus::Paused);

        div()
            .id(SharedString::from(format!("operation-{}", op_id.0)))
            .w_full()
            .p_3()
            .bg(bg_color)
            .rounded_md()
            .border_1()
            .border_color(border_color)
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                svg()
                                    .path(match op_type {
                                        OperationType::Copy => "assets/icons/copy.svg",
                                        OperationType::Move => "assets/icons/arrow-right.svg",
                                        OperationType::Delete => "assets/icons/trash-2.svg",
                                    })
                                    .size(px(16.0))
                                    .text_color(if is_completed {
                                        success
                                    } else if is_failed {
                                        error
                                    } else {
                                        accent
                                    }),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(text_primary)
                                    .child(format!("{}", op_type)),
                            )
                            .child(div().text_xs().text_color(text_muted).child(format!(
                                "{}/{} files",
                                progress.completed_files, progress.total_files
                            )))
                            .when(skipped_count > 0, |el| {
                                el.child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.warning)
                                        .child(format!("({} skipped)", skipped_count)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(match &status {
                                        OperationStatus::Completed => success,
                                        OperationStatus::Failed(_) => error,
                                        OperationStatus::Cancelled => text_muted,
                                        OperationStatus::Paused => theme.warning,
                                        _ => text_muted,
                                    })
                                    .child(match &status {
                                        OperationStatus::Pending => "Pending".to_string(),
                                        OperationStatus::Running => format!("{:.0}%", percentage),
                                        OperationStatus::Paused => {
                                            "Error - Action Required".to_string()
                                        }
                                        OperationStatus::Completed => "Completed".to_string(),
                                        OperationStatus::Failed(_) => "Failed".to_string(),
                                        OperationStatus::Cancelled => "Cancelled".to_string(),
                                    }),
                            )
                            .when(is_active, |el| {
                                el.child(
                                    div()
                                        .id(SharedString::from(format!("cancel-{}", op_id.0)))
                                        .px_2()
                                        .py_1()
                                        .rounded_sm()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(hover_bg))
                                        .text_xs()
                                        .text_color(error)
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |view, _event, _window, cx| {
                                                view.pending_action =
                                                    Some(ProgressPanelAction::Cancel(op_id));
                                                cx.notify();
                                            }),
                                        )
                                        .child("Cancel"),
                                )
                            })
                            .when(!is_active, |el| {
                                el.child(
                                    div()
                                        .id(SharedString::from(format!("dismiss-{}", op_id.0)))
                                        .px_2()
                                        .py_1()
                                        .rounded_sm()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(hover_bg))
                                        .text_xs()
                                        .text_color(text_muted)
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |view, _event, _window, cx| {
                                                view.pending_action =
                                                    Some(ProgressPanelAction::Dismiss(op_id));
                                                cx.notify();
                                            }),
                                        )
                                        .child("Dismiss"),
                                )
                            }),
                    ),
            )
            .when(is_active, |el| {
                el.child(
                    div()
                        .w_full()
                        .h(px(4.0))
                        .bg(border_color)
                        .rounded_full()
                        .overflow_hidden()
                        .child(
                            div()
                                .h_full()
                                .w(gpui::relative(percentage / 100.0))
                                .bg(accent)
                                .rounded_full(),
                        ),
                )
            })
            .when(matches!(status, OperationStatus::Running), |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_xs()
                        .text_color(text_muted)
                        .child(
                            div()
                                .max_w(px(200.0))
                                .truncate()
                                .child(current_file.unwrap_or_else(|| "Preparing...".to_string())),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .when(progress.speed_bytes_per_sec > 0, |el| {
                                    el.child(format_speed(progress.speed_bytes_per_sec))
                                })
                                .when(progress.estimated_remaining.as_secs() > 0, |el| {
                                    el.child(format!(
                                        "~{} remaining",
                                        format_duration(progress.estimated_remaining)
                                    ))
                                }),
                        ),
                )
            })
            .when(progress.total_bytes > 0 && is_active, |el| {
                el.child(div().text_xs().text_color(text_muted).child(format!(
                    "{} / {}",
                    format_size(progress.transferred_bytes),
                    format_size(progress.total_bytes)
                )))
            })
            .when(is_failed, |el| {
                let msg = if let OperationStatus::Failed(ref m) = status {
                    m.clone()
                } else {
                    "Unknown error".to_string()
                };
                el.child(
                    div()
                        .w_full()
                        .p_2()
                        .bg(with_alpha(error, 0.1))
                        .rounded_sm()
                        .text_xs()
                        .text_color(error)
                        .child(msg),
                )
            })
            .when(has_error, |el| {
                let is_recoverable = op
                    .current_error
                    .as_ref()
                    .map(|e| e.is_recoverable)
                    .unwrap_or(false);
                let user_msg = op
                    .current_error
                    .as_ref()
                    .map(|e| e.user_message())
                    .unwrap_or_else(|| {
                        error_msg
                            .clone()
                            .unwrap_or_else(|| "An error occurred".to_string())
                    });

                el.child(
                    div()
                        .w_full()
                        .p_2()
                        .bg(with_alpha(error, 0.1))
                        .border_1()
                        .border_color(with_alpha(error, 0.3))
                        .rounded_sm()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .gap_2()
                                .child(
                                    svg()
                                        .path("assets/icons/triangle-alert.svg")
                                        .size(px(14.0))
                                        .text_color(error)
                                        .flex_shrink_0(),
                                )
                                .child(div().text_xs().text_color(error).child(user_msg)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .justify_end()
                                .child(
                                    div()
                                        .id(SharedString::from(format!("skip-{}", op_id.0)))
                                        .px_3()
                                        .py_1()
                                        .rounded_sm()
                                        .cursor_pointer()
                                        .bg(hover_bg)
                                        .hover(|s| s.bg(border_color))
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(text_primary)
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |view, _event, _window, cx| {
                                                view.pending_action =
                                                    Some(ProgressPanelAction::Skip(op_id));
                                                cx.notify();
                                            }),
                                        )
                                        .child("Skip"),
                                )
                                .when(is_recoverable, |el| {
                                    el.child(
                                        div()
                                            .id(SharedString::from(format!("retry-{}", op_id.0)))
                                            .px_3()
                                            .py_1()
                                            .rounded_sm()
                                            .cursor_pointer()
                                            .bg(with_alpha(accent, 0.2))
                                            .hover(|s| s.bg(with_alpha(accent, 0.3)))
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(accent)
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(move |view, _event, _window, cx| {
                                                    view.pending_action =
                                                        Some(ProgressPanelAction::Retry(op_id));
                                                    cx.notify();
                                                }),
                                            )
                                            .child("Retry"),
                                    )
                                })
                                .child(
                                    div()
                                        .id(SharedString::from(format!("cancel-err-{}", op_id.0)))
                                        .px_3()
                                        .py_1()
                                        .rounded_sm()
                                        .cursor_pointer()
                                        .bg(with_alpha(error, 0.15))
                                        .hover(|s| s.bg(with_alpha(error, 0.25)))
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(error)
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |view, _event, _window, cx| {
                                                view.pending_action =
                                                    Some(ProgressPanelAction::Cancel(op_id));
                                                cx.notify();
                                            }),
                                        )
                                        .child("Cancel All"),
                                ),
                        ),
                )
            })
    }
}

impl Focusable for ProgressPanelView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProgressPanelView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        let bg_color = theme.bg_secondary;
        let border_color = theme.border_default;
        let text_primary = theme.text_primary;
        let text_muted = theme.text_muted;
        let hover_bg = theme.bg_hover;

        let has_operations = self.has_operations();
        let has_active = self.has_active_operations();
        let is_expanded = self.is_expanded;
        let operations = self.operations.clone();
        let active_count = operations.iter().filter(|o| o.status.is_active()).count();
        let completed_count = operations.iter().filter(|o| o.status.is_finished()).count();

        if !has_operations {
            return div().into_any_element();
        }

        div()
            .id("progress-panel")
            .w_full()
            .max_w(px(400.0))
            .bg(bg_color)
            .border_1()
            .border_color(border_color)
            .rounded_lg()
            .shadow_lg()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .id("progress-panel-header")
                    .w_full()
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(border_color)
                    .cursor_pointer()
                    .hover(|s| s.bg(hover_bg))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|view, _event, _window, cx| {
                            view.toggle_expanded(cx);
                        }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                svg()
                                    .path(if is_expanded {
                                        "assets/icons/chevron-down.svg"
                                    } else {
                                        "assets/icons/chevron-right.svg"
                                    })
                                    .size(px(14.0))
                                    .text_color(text_muted),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(text_primary)
                                    .child("File Operations"),
                            )
                            .when(active_count > 0, |el| {
                                el.child(
                                    div()
                                        .px_1p5()
                                        .py_0p5()
                                        .rounded_full()
                                        .bg(with_alpha(theme.accent_primary, 0.2))
                                        .text_xs()
                                        .text_color(theme.accent_primary)
                                        .child(format!("{} active", active_count)),
                                )
                            }),
                    )
                    .when(completed_count > 0, |el| {
                        el.child(
                            div()
                                .id("dismiss-all")
                                .px_2()
                                .py_1()
                                .rounded_sm()
                                .cursor_pointer()
                                .hover(|s| s.bg(hover_bg))
                                .text_xs()
                                .text_color(text_muted)
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|view, _event, _window, cx| {
                                        view.pending_action = Some(ProgressPanelAction::DismissAll);
                                        cx.notify();
                                    }),
                                )
                                .child("Clear completed"),
                        )
                    }),
            )
            .when(is_expanded, |el| {
                el.child(
                    div()
                        .w_full()
                        .max_h(px(300.0))
                        .p_2()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children(operations.iter().map(|op| self.render_operation(op, cx))),
                )
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
    }

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(500), "500 B/s");
        assert_eq!(format_speed(1024), "1.0 KB/s");
        assert_eq!(format_speed(1048576), "1.0 MB/s");
        assert_eq!(format_speed(1073741824), "1.0 GB/s");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
        assert_eq!(format_size(1099511627776), "1.0 TB");
    }
}
