use gpui::{
    div, prelude::*, px, App, Context, FocusHandle, Focusable, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Render, Styled, Window,
};
use std::time::{Duration, Instant};

use crate::models::{Theme, ThemeColors, ThemeId, theme_colors};

/// Callback type for theme selection
pub type OnThemeSelect = Box<dyn Fn(ThemeId) + 'static>;

/// Animation state for theme transitions
#[derive(Clone, Debug)]
struct TransitionState {
    from_theme: Option<ThemeId>,
    to_theme: ThemeId,
    start_time: Instant,
    duration: Duration,
}

impl TransitionState {
    fn new(to_theme: ThemeId) -> Self {
        Self {
            from_theme: None,
            to_theme,
            start_time: Instant::now(),
            duration: Duration::from_millis(250),
        }
    }

    fn with_from(mut self, from: ThemeId) -> Self {
        self.from_theme = Some(from);
        self
    }

    fn progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed();
        (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
    }

    fn is_complete(&self) -> bool {
        self.progress() >= 1.0
    }
}

/// Theme picker view for selecting RPG themes with animated previews
pub struct ThemePickerView {
    focus_handle: FocusHandle,
    selected_theme: ThemeId,
    is_visible: bool,
    transition: Option<TransitionState>,
    on_theme_select: Option<OnThemeSelect>,
    hovered_theme: Option<ThemeId>,
}

/// Alias for backwards compatibility
pub type ThemePicker = ThemePickerView;

impl ThemePickerView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            selected_theme: ThemeId::default(),
            is_visible: false,
            transition: None,
            on_theme_select: None,
            hovered_theme: None,
        }
    }

    /// Set the callback for when a theme is selected
    pub fn on_theme_select<F>(mut self, callback: F) -> Self
    where
        F: Fn(ThemeId) + 'static,
    {
        self.on_theme_select = Some(Box::new(callback));
        self
    }

    /// Set the initial selected theme
    pub fn with_selected_theme(mut self, theme_id: ThemeId) -> Self {
        self.selected_theme = theme_id;
        self
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
        if self.selected_theme != id {
            self.transition = Some(
                TransitionState::new(id).with_from(self.selected_theme)
            );
            self.selected_theme = id;
            cx.notify();
        }
    }

    /// Set hovered theme for preview effect
    fn set_hovered_theme(&mut self, theme_id: Option<ThemeId>, cx: &mut Context<Self>) {
        if self.hovered_theme != theme_id {
            self.hovered_theme = theme_id;
            cx.notify();
        }
    }
}

/// Render a mini preview of the file explorer UI with theme colors
fn render_mini_preview(theme: &Theme) -> impl IntoElement {
    let colors = &theme.colors;
    
    div()
        .h(px(100.0))
        .bg(colors.bg_primary)
        .rounded_t_lg()
        .overflow_hidden()
        .child(
            div()
                .flex()
                .h_full()
                .child(
                    // Sidebar preview
                    div()
                        .w(px(50.0))
                        .h_full()
                        .bg(colors.bg_secondary)
                        .border_r_1()
                        .border_color(colors.border_subtle)
                        .p_1()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .h(px(4.0))
                                        .w(px(32.0))
                                        .bg(colors.text_muted)
                                        .rounded_sm()
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(2.0))
                                                .child(div().w(px(8.0)).h(px(8.0)).bg(colors.folder_color).rounded_sm())
                                                .child(div().h(px(4.0)).w(px(20.0)).bg(colors.text_secondary).rounded_sm())
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(2.0))
                                                .child(div().w(px(8.0)).h(px(8.0)).bg(colors.folder_color).rounded_sm())
                                                .child(div().h(px(4.0)).w(px(18.0)).bg(colors.text_secondary).rounded_sm())
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(2.0))
                                                .child(div().w(px(8.0)).h(px(8.0)).bg(colors.accent_primary).rounded_sm())
                                                .child(div().h(px(4.0)).w(px(22.0)).bg(colors.text_secondary).rounded_sm())
                                        )
                                )
                        ),
                )
                .child(
                    // Main content area
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(
                            // Toolbar
                            div()
                                .h(px(16.0))
                                .bg(colors.bg_tertiary)
                                .border_b_1()
                                .border_color(colors.border_subtle)
                                .px_1()
                                .flex()
                                .items_center()
                                .gap(px(2.0))
                                .child(div().w(px(8.0)).h(px(8.0)).bg(colors.text_muted).rounded_sm())
                                .child(div().w(px(40.0)).h(px(6.0)).bg(colors.bg_hover).rounded_sm())
                        )
                        .child(
                            // File list
                            div()
                                .flex_1()
                                .bg(colors.bg_primary)
                                .p_1()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .h(px(12.0))
                                                .bg(colors.bg_selected)
                                                .rounded_sm()
                                                .px_1()
                                                .flex()
                                                .items_center()
                                                .gap(px(2.0))
                                                .child(div().w(px(8.0)).h(px(8.0)).bg(colors.folder_color).rounded_sm())
                                                .child(div().h(px(4.0)).w(px(30.0)).bg(colors.text_primary).rounded_sm())
                                        )
                                        .child(
                                            div()
                                                .h(px(12.0))
                                                .px_1()
                                                .flex()
                                                .items_center()
                                                .gap(px(2.0))
                                                .child(div().w(px(8.0)).h(px(8.0)).bg(colors.file_code).rounded_sm())
                                                .child(div().h(px(4.0)).w(px(35.0)).bg(colors.text_secondary).rounded_sm())
                                        )
                                        .child(
                                            div()
                                                .h(px(12.0))
                                                .px_1()
                                                .flex()
                                                .items_center()
                                                .gap(px(2.0))
                                                .child(div().w(px(8.0)).h(px(8.0)).bg(colors.file_data).rounded_sm())
                                                .child(div().h(px(4.0)).w(px(28.0)).bg(colors.text_secondary).rounded_sm())
                                        )
                                        .child(
                                            div()
                                                .h(px(12.0))
                                                .px_1()
                                                .flex()
                                                .items_center()
                                                .gap(px(2.0))
                                                .child(div().w(px(8.0)).h(px(8.0)).bg(colors.file_media).rounded_sm())
                                                .child(div().h(px(4.0)).w(px(32.0)).bg(colors.text_secondary).rounded_sm())
                                        )
                                ),
                        ),
                ),
        )
}

/// Render color swatches showing the theme's key colors
fn render_color_swatches(theme: &Theme) -> impl IntoElement {
    let colors = &theme.colors;
    
    div()
        .flex()
        .gap(px(4.0))
        .child(
            div()
                .w(px(20.0))
                .h(px(20.0))
                .rounded_full()
                .bg(colors.accent_primary)
                .border_1()
                .border_color(colors.border_default)
        )
        .child(
            div()
                .w(px(20.0))
                .h(px(20.0))
                .rounded_full()
                .bg(colors.accent_secondary)
                .border_1()
                .border_color(colors.border_default)
        )
        .child(
            div()
                .w(px(20.0))
                .h(px(20.0))
                .rounded_full()
                .bg(colors.folder_color)
                .border_1()
                .border_color(colors.border_default)
        )
        .child(
            div()
                .w(px(20.0))
                .h(px(20.0))
                .rounded_full()
                .bg(colors.success)
                .border_1()
                .border_color(colors.border_default)
        )
        .child(
            div()
                .w(px(20.0))
                .h(px(20.0))
                .rounded_full()
                .bg(colors.file_code)
                .border_1()
                .border_color(colors.border_default)
        )
}

/// Render a theme card with live preview
fn render_theme_card(theme: &Theme, is_selected: bool, current_colors: &ThemeColors) -> impl IntoElement {
    render_theme_card_animated(theme, is_selected, false, current_colors, false, 1.0)
}

/// Render a theme card with live preview and animation support
fn render_theme_card_animated(
    theme: &Theme, 
    is_selected: bool, 
    is_hovered: bool,
    current_colors: &ThemeColors,
    is_transitioning: bool,
    transition_progress: f32,
) -> impl IntoElement {
    let colors = &theme.colors;
    let hover_border = colors.border_emphasis;
    
    let card_bg = if is_selected {
        colors.bg_tertiary
    } else if is_hovered {
        current_colors.bg_hover
    } else {
        current_colors.bg_secondary
    };

    div()
        .w(px(220.0))
        .rounded_lg()
        .overflow_hidden()
        .cursor_pointer()
        .bg(card_bg)
        // Apply scale animation on hover
        .when(is_hovered && !is_selected, |s| {
            s.shadow_lg()
        })
        // Selected state with accent border and glow effect
        .when(is_selected, |s| {
            s.border_2()
                .border_color(colors.accent_primary)
                .shadow_lg()
        })
        // Transitioning animation - pulse effect
        .when(is_transitioning, |s| {
            s.opacity(0.9 + 0.1 * transition_progress)
        })
        .when(!is_selected, |s| {
            s.border_1()
                .border_color(if is_hovered { hover_border } else { current_colors.border_default })
        })
        .child(render_mini_preview(theme))
        .child(
            div()
                .p_3()
                .bg(if is_selected { colors.bg_secondary } else { current_colors.bg_tertiary })
                .border_t_1()
                .border_color(if is_selected { colors.border_subtle } else { current_colors.border_subtle })
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
                                .text_sm()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(if is_selected { colors.text_primary } else { current_colors.text_primary })
                                .child(theme.name)
                        )
                        .when(is_selected, |s| {
                            s.child(
                                div()
                                    .w(px(20.0))
                                    .h(px(20.0))
                                    .rounded_full()
                                    .bg(colors.accent_primary)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(colors.text_inverse)
                                            .child("✓")
                                    )
                            )
                        })
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if is_selected { colors.text_secondary } else { current_colors.text_muted })
                        .line_height(px(16.0))
                        .child(theme.description)
                )
                .child(render_color_swatches(theme))
        )
}

impl Focusable for ThemePickerView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ThemePickerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_visible {
            return div().into_any_element();
        }

        let current_theme = theme_colors();
        let themes = Theme::all_themes();
        let selected = self.selected_theme;

        let transition_progress = self.transition
            .as_ref()
            .map(|t| t.progress())
            .unwrap_or(1.0);
        
        let is_transitioning = transition_progress < 1.0;
        
        // Calculate crossfade opacity for smooth transition
        let crossfade_opacity = if is_transitioning {
            // Ease-out cubic for smooth deceleration
            let t = transition_progress;
            1.0 - (1.0 - t).powi(3)
        } else {
            1.0
        };

        div()
            .absolute()
            .inset_0()
            .bg(gpui::rgba(0x00000099))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                view.hide(cx);
            }))
            .child(
                div()
                    .w(px(780.0))
                    .max_h(px(620.0))
                    .bg(current_theme.bg_secondary)
                    .rounded_xl()
                    .border_1()
                    .border_color(current_theme.border_default)
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    // Apply crossfade animation to the entire dialog
                    .when(is_transitioning, |s| {
                        s.opacity(0.95 + 0.05 * crossfade_opacity)
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            // Header
                            .child(
                                div()
                                    .px_6()
                                    .py_4()
                                    .bg(current_theme.bg_tertiary)
                                    .border_b_1()
                                    .border_color(current_theme.border_subtle)
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xl()
                                                    .font_weight(gpui::FontWeight::BOLD)
                                                    .text_color(current_theme.text_primary)
                                                    .child("🎨 Choose Your Theme")
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(current_theme.text_secondary)
                                                    .child("Select an RPG-inspired theme to customize your explorer")
                                            )
                                    )
                                    .child(
                                        div()
                                            .id("close-theme-picker")
                                            .w(px(32.0))
                                            .h(px(32.0))
                                            .rounded_md()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .text_color(current_theme.text_muted)
                                            .hover(|s| s
                                                .bg(current_theme.bg_hover)
                                                .text_color(current_theme.text_primary)
                                            )
                                            .on_mouse_down(MouseButton::Left, cx.listener(|view, _, _, cx| {
                                                view.hide(cx);
                                            }))
                                            .child("✕")
                                    )
                            )
                            // Theme cards grid
                            .child(
                                div()
                                    .p_6()
                                    .flex()
                                    .flex_wrap()
                                    .gap_4()
                                    .justify_center()
                                    .overflow_hidden()
                                    .max_h(px(450.0))
                                    .children(themes.iter().enumerate().map(|(idx, theme)| {
                                        let theme_id = theme.id;
                                        let is_selected = theme_id == selected;
                                        
                                        div()
                                            .id(("theme-card", idx))
                                            .on_mouse_down(MouseButton::Left, cx.listener(move |view, _, _, cx| {
                                                view.set_selected_theme(theme_id, cx);
                                                // Call the callback if set
                                                if let Some(callback) = view.on_theme_select.take() {
                                                    callback(theme_id);
                                                    view.on_theme_select = Some(callback);
                                                }
                                            }))
                                            .child(render_theme_card_animated(theme, is_selected, false, &current_theme, is_transitioning && is_selected, crossfade_opacity))
                                    }))
                            )
                            // Footer with status
                            .child(
                                div()
                                    .px_6()
                                    .py_3()
                                    .bg(current_theme.bg_tertiary)
                                    .border_t_1()
                                    .border_color(current_theme.border_subtle)
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .w(px(8.0))
                                                    .h(px(8.0))
                                                    .rounded_full()
                                                    .bg(Theme::from_id(selected).colors.accent_primary)
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(current_theme.text_secondary)
                                                    .child(format!("Active: {}", Theme::from_id(selected).name))
                                            )
                                    )
                                    .when(is_transitioning, |s| {
                                        s.child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(current_theme.accent_primary)
                                                        .child("✨ Applying theme...")
                                                )
                                                .child(
                                                    div()
                                                        .w(px(80.0))
                                                        .h(px(4.0))
                                                        .bg(current_theme.bg_hover)
                                                        .rounded_full()
                                                        .overflow_hidden()
                                                        .child(
                                                            div()
                                                                .h_full()
                                                                .w(px(80.0 * crossfade_opacity))
                                                                .bg(current_theme.accent_primary)
                                                                .rounded_full()
                                                        )
                                                )
                                        )
                                    })
                                    .when(!is_transitioning, |s| {
                                        s.child(
                                            div()
                                                .text_xs()
                                                .text_color(current_theme.text_muted)
                                                .child("Click a theme to apply")
                                        )
                                    })
                            )
                    )
            )
            .into_any_element()
    }
}
