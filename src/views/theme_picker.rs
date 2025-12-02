use gpui::{
    div, prelude::*, px, App, Context, FocusHandle, Focusable, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Render, Styled, Window,
};

use crate::models::{Theme, ThemeId, theme_colors};

/// Theme picker view for selecting RPG themes
pub struct ThemePickerView {
    focus_handle: FocusHandle,
    selected_theme: ThemeId,
    hovered_theme: Option<ThemeId>,
    is_visible: bool,
}

/// Alias for backwards compatibility
pub type ThemePicker = ThemePickerView;

impl ThemePickerView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            selected_theme: ThemeId::default(),
            hovered_theme: None,
            is_visible: false,
        }
    }

    pub fn show(&mut self, cx: &mut Context<Self>) {
        self.is_visible = true;
        cx.notify();
    }

    pub fn hide(&mut self, cx: &mut Context<Self>) {
        self.is_visible = false;
        cx.notify();
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.is_visible = !self.is_visible;
        cx.notify();
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    pub fn selected_theme(&self) -> ThemeId {
        self.selected_theme
    }

    pub fn set_selected_theme(&mut self, id: ThemeId, cx: &mut Context<Self>) {
        self.selected_theme = id;
        cx.notify();
    }


    fn render_theme_card(&self, theme: &Theme, is_selected: bool, is_hovered: bool) -> impl IntoElement {
        let colors = &theme.colors;
        let border_color = if is_selected {
            colors.accent_primary
        } else if is_hovered {
            colors.border_emphasis
        } else {
            colors.border_default
        };

        div()
            .w(px(200.0))
            .h(px(160.0))
            .rounded_lg()
            .border_2()
            .border_color(border_color)
            .bg(colors.bg_secondary)
            .overflow_hidden()
            .cursor_pointer()
            .child(
                div()
                    .h(px(100.0))
                    .bg(colors.bg_primary)
                    .p_2()
                    .child(
                        div()
                            .flex()
                            .h_full()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(40.0))
                                    .h_full()
                                    .bg(colors.bg_secondary)
                                    .rounded_sm()
                                    .p_1()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(div().h(px(6.0)).w(px(30.0)).bg(colors.folder_color).rounded_sm())
                                            .child(div().h(px(6.0)).w(px(26.0)).bg(colors.text_muted).rounded_sm())
                                            .child(div().h(px(6.0)).w(px(28.0)).bg(colors.text_muted).rounded_sm()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .bg(colors.bg_tertiary)
                                    .rounded_sm()
                                    .p_1()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(div().h(px(8.0)).bg(colors.bg_selected).rounded_sm())
                                            .child(div().h(px(8.0)).bg(colors.bg_hover).rounded_sm())
                                            .child(div().h(px(8.0)).bg(colors.bg_hover).rounded_sm()),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .h(px(60.0))
                    .bg(colors.bg_tertiary)
                    .p_2()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(colors.text_primary)
                            .child(theme.name),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(div().w(px(16.0)).h(px(16.0)).rounded_full().bg(colors.accent_primary))
                            .child(div().w(px(16.0)).h(px(16.0)).rounded_full().bg(colors.accent_secondary))
                            .child(div().w(px(16.0)).h(px(16.0)).rounded_full().bg(colors.folder_color))
                            .child(div().w(px(16.0)).h(px(16.0)).rounded_full().bg(colors.success)),
                    ),
            )
    }
}

impl Focusable for ThemePickerView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ThemePickerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_visible {
            return div().into_any_element();
        }

        let current_theme = theme_colors();
        let themes = Theme::all_themes();
        let selected = self.selected_theme;
        let hovered = self.hovered_theme;

        div()
            .absolute()
            .inset_0()
            .bg(gpui::rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                view.hide(cx);
            }))
            .child(
                div()
                    .w(px(700.0))
                    .bg(current_theme.bg_secondary)
                    .rounded_xl()
                    .border_1()
                    .border_color(current_theme.border_default)
                    .p_6()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xl()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(current_theme.text_primary)
                                            .child("Choose Your Theme"),
                                    )
                                    .child(
                                        div()
                                            .id("close-theme-picker")
                                            .cursor_pointer()
                                            .text_color(current_theme.text_muted)
                                            .hover(|s| s.text_color(current_theme.text_primary))
                                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                                view.hide(cx);
                                            }))
                                            .child("✕"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(current_theme.text_secondary)
                                    .child("Select an RPG-inspired theme to customize your explorer"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_4()
                                    .justify_center()
                                    .children(themes.iter().enumerate().map(|(idx, theme)| {
                                        let theme_id = theme.id;
                                        let is_selected = theme_id == selected;
                                        let is_hovered = hovered == Some(theme_id);
                                        
                                        div()
                                            .id(("theme-card", idx))
                                            .on_mouse_down(MouseButton::Left, cx.listener(move |view, _, _, cx| {
                                                view.set_selected_theme(theme_id, cx);
                                            }))
                                            .child(self.render_theme_card(theme, is_selected, is_hovered))
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }
}
