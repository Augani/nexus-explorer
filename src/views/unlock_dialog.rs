/*
 * Unlock Dialog for Encrypted Volumes
 * 
 * Provides a dialog for unlocking BitLocker (Windows) and LUKS (Linux) encrypted volumes.
 * Requirements: 21.1-21.5 (Encrypted Volume Support)
 */

use gpui::{
    div, prelude::*, px, svg, App, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Window,
};

use crate::models::{theme_colors, EncryptedVolumeInfo, UnlockCredential};
use adabraka_ui::components::input::{InputEvent, InputState};

#[derive(Clone, Debug)]
pub enum UnlockDialogAction {
    Unlock { device_id: String, credential: UnlockCredential },
    Cancel,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CredentialType {
    Password,
    RecoveryKey,
}

pub struct UnlockDialog {
    volume_info: EncryptedVolumeInfo,
    password_input: Entity<InputState>,
    credential_type: CredentialType,
    error_message: Option<String>,
    is_unlocking: bool,
    focus_handle: FocusHandle,
    pending_action: Option<UnlockDialogAction>,
}

impl UnlockDialog {
    pub fn new(volume_info: EncryptedVolumeInfo, cx: &mut Context<Self>) -> Self {
        let password_input = cx.new(|cx| {
            let mut state = InputState::new(cx);
            state.placeholder = "Enter password...".into();
            state.masked = true;
            state
        });

        cx.subscribe(&password_input, |dialog: &mut Self, _, event: &InputEvent, cx| {
            if let InputEvent::Enter = event {
                dialog.submit(cx);
            }
        })
        .detach();

        Self {
            volume_info,
            password_input,
            credential_type: CredentialType::Password,
            error_message: None,
            is_unlocking: false,
            focus_handle: cx.focus_handle(),
            pending_action: None,
        }
    }

    pub fn take_pending_action(&mut self) -> Option<UnlockDialogAction> {
        self.pending_action.take()
    }

    pub fn set_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.error_message = Some(error);
        self.is_unlocking = false;
        cx.notify();
    }

    pub fn set_unlocking(&mut self, unlocking: bool, cx: &mut Context<Self>) {
        self.is_unlocking = unlocking;
        if unlocking {
            self.error_message = None;
        }
        cx.notify();
    }

    fn toggle_credential_type(&mut self, cx: &mut Context<Self>) {
        self.credential_type = match self.credential_type {
            CredentialType::Password => CredentialType::RecoveryKey,
            CredentialType::RecoveryKey => CredentialType::Password,
        };
        
        self.password_input.update(cx, |state, _| {
            state.content = "".into();
            state.placeholder = match self.credential_type {
                CredentialType::Password => "Enter password...".into(),
                CredentialType::RecoveryKey => "Enter recovery key...".into(),
            };
            state.masked = matches!(self.credential_type, CredentialType::Password);
        });
        
        self.error_message = None;
        cx.notify();
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.is_unlocking {
            return;
        }

        let input_value = self.password_input.read(cx).content.to_string();
        
        if input_value.is_empty() {
            self.error_message = Some("Please enter a password or recovery key".to_string());
            cx.notify();
            return;
        }

        let credential = match self.credential_type {
            CredentialType::Password => UnlockCredential::Password(input_value),
            CredentialType::RecoveryKey => UnlockCredential::RecoveryKey(input_value),
        };

        self.pending_action = Some(UnlockDialogAction::Unlock {
            device_id: self.volume_info.device_id.clone(),
            credential,
        });
        cx.notify();
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.pending_action = Some(UnlockDialogAction::Cancel);
        cx.notify();
    }
}

impl Focusable for UnlockDialog {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for UnlockDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = theme_colors();
        let bg_primary = colors.bg_primary;
        let bg_secondary = colors.bg_secondary;
        let border_color = colors.border_default;
        let text_primary = colors.text_primary;
        let text_secondary = colors.text_secondary;
        let accent_primary = colors.accent_primary;
        let hover_bg = colors.bg_hover;
        let error_color = gpui::rgb(0xf85149);

        let encryption_name = self.volume_info.encryption_type.display_name();
        let device_display = &self.volume_info.device_id;
        let label_display = self.volume_info.label.clone().unwrap_or_else(|| "Encrypted Volume".to_string());
        
        let is_password_mode = self.credential_type == CredentialType::Password;
        let toggle_text = if is_password_mode {
            "Use recovery key instead"
        } else {
            "Use password instead"
        };

        let error_message = self.error_message.clone();
        let is_unlocking = self.is_unlocking;

        div()
            .id("unlock-dialog")
            .track_focus(&self.focus_handle)
            .w(px(420.0))
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
                            .path("assets/icons/lock.svg")
                            .size(px(18.0))
                            .text_color(accent_primary),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(text_primary)
                            .child(format!("Unlock {} Volume", encryption_name)),
                    ),
            )
            .child(
                div()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_4()
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
                                    .child("Volume"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_2()
                                    .bg(bg_secondary)
                                    .rounded_md()
                                    .border_1()
                                    .border_color(border_color)
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        svg()
                                            .path("assets/icons/hard-drive.svg")
                                            .size(px(16.0))
                                            .text_color(text_secondary),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(text_primary)
                                                    .child(label_display),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(text_secondary)
                                                    .child(device_display.clone()),
                                            ),
                                    ),
                            ),
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
                                    .child(if is_password_mode { "Password" } else { "Recovery Key" }),
                            )
                            .child(self.password_input.clone()),
                    )
                    .child(
                        div()
                            .id("toggle-credential-type")
                            .text_xs()
                            .text_color(accent_primary)
                            .cursor_pointer()
                            .hover(|s| s.underline())
                            .on_click(cx.listener(|dialog, _, _, cx| {
                                dialog.toggle_credential_type(cx);
                            }))
                            .child(toggle_text),
                    )
                    .when_some(error_message, |el, msg| {
                        el.child(
                            div()
                                .px_3()
                                .py_2()
                                .bg(gpui::rgba(0xf8514920))
                                .rounded_md()
                                .border_1()
                                .border_color(error_color)
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    svg()
                                        .path("assets/icons/triangle-alert.svg")
                                        .size(px(14.0))
                                        .text_color(error_color),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(error_color)
                                        .child(msg),
                                ),
                        )
                    }),
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
                            .id("unlock-btn")
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .bg(accent_primary)
                            .text_sm()
                            .text_color(gpui::rgb(0xffffff))
                            .cursor_pointer()
                            .when(is_unlocking, |el| el.opacity(0.6).cursor_default())
                            .when(!is_unlocking, |el| el.hover(|s| s.opacity(0.9)))
                            .on_click(cx.listener(|dialog, _, _, cx| {
                                dialog.submit(cx);
                            }))
                            .child(if is_unlocking { "Unlocking..." } else { "Unlock" }),
                    ),
            )
    }
}
