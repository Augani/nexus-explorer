use gpui::{
    div, prelude::*, px, App, Context, Entity, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, SharedString, Styled, Timer, Window,
};
use std::time::Duration;

use crate::models::theme_colors;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToastVariant {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u64,
    pub title: SharedString,
    pub description: Option<SharedString>,
    pub variant: ToastVariant,
    pub duration_ms: u64,
}

impl Toast {
    pub fn new(id: u64, title: impl Into<SharedString>) -> Self {
        Self {
            id,
            title: title.into(),
            description: None,
            variant: ToastVariant::Info,
            duration_ms: 4000,
        }
    }

    pub fn description(mut self, desc: impl Into<SharedString>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn variant(mut self, variant: ToastVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn success(mut self) -> Self {
        self.variant = ToastVariant::Success;
        self
    }

    pub fn error(mut self) -> Self {
        self.variant = ToastVariant::Error;
        self
    }

    pub fn warning(mut self) -> Self {
        self.variant = ToastVariant::Warning;
        self
    }

    pub fn duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }
}

pub struct ToastManager {
    toasts: Vec<Toast>,
    next_id: u64,
    focus_handle: FocusHandle,
}

impl ToastManager {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            toasts: Vec::new(),
            next_id: 1,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn show(&mut self, toast: Toast, cx: &mut Context<Self>) {
        let id = toast.id;
        let duration = toast.duration_ms;
        
        self.toasts.push(toast);
        
        // Schedule auto-dismiss
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(duration)).await;
            let _ = this.update(cx, |this, cx| {
                this.dismiss(id, cx);
            });
        })
        .detach();
        
        cx.notify();
    }

    pub fn show_success(&mut self, title: impl Into<SharedString>, cx: &mut Context<Self>) {
        let id = self.next_id;
        self.next_id += 1;
        let toast = Toast::new(id, title).success();
        self.show(toast, cx);
    }

    pub fn show_error(&mut self, title: impl Into<SharedString>, cx: &mut Context<Self>) {
        let id = self.next_id;
        self.next_id += 1;
        let toast = Toast::new(id, title).error().duration(6000);
        self.show(toast, cx);
    }

    pub fn show_info(&mut self, title: impl Into<SharedString>, cx: &mut Context<Self>) {
        let id = self.next_id;
        self.next_id += 1;
        let toast = Toast::new(id, title);
        self.show(toast, cx);
    }

    pub fn dismiss(&mut self, id: u64, cx: &mut Context<Self>) {
        self.toasts.retain(|t| t.id != id);
        cx.notify();
    }

    pub fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

impl Focusable for ToastManager {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ToastManager {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();

        if self.toasts.is_empty() {
            return div().into_any_element();
        }

        div()
            .absolute()
            .bottom(px(80.0))
            .right(px(16.0))
            .flex()
            .flex_col_reverse()
            .gap_2()
            .max_w(px(380.0))
            .children(self.toasts.iter().map(|toast| {
                let (bg, border, icon_color) = match toast.variant {
                    ToastVariant::Info => (theme.bg_tertiary, theme.border_default, theme.accent_primary),
                    ToastVariant::Success => (theme.bg_tertiary, theme.success, theme.success),
                    ToastVariant::Warning => (theme.bg_tertiary, theme.warning, theme.warning),
                    ToastVariant::Error => (theme.bg_tertiary, theme.error, theme.error),
                };

                let icon = match toast.variant {
                    ToastVariant::Info => "ℹ",
                    ToastVariant::Success => "✓",
                    ToastVariant::Warning => "⚠",
                    ToastVariant::Error => "✕",
                };

                div()
                    .id(("toast", toast.id))
                    .flex()
                    .items_start()
                    .gap_3()
                    .w_full()
                    .min_w(px(280.0))
                    .bg(bg)
                    .border_l_4()
                    .border_color(border)
                    .rounded_md()
                    .p_3()
                    .shadow_lg()
                    .child(
                        div()
                            .text_base()
                            .text_color(icon_color)
                            .child(icon)
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text_primary)
                                    .child(toast.title.clone())
                            )
                            .when_some(toast.description.clone(), |this, desc| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.text_secondary)
                                        .child(desc)
                                )
                            })
                    )
            }))
            .into_any_element()
    }
}
