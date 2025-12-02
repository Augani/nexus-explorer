use gpui::{
    div, prelude::*, px, svg, App, Context, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Render, SharedString, Styled,
    Window,
};

use crate::models::{TabId, TabState, theme_colors};

/// View wrapper for TabBar with GPUI integration
pub struct TabBarView {
    tab_state: TabState,
    focus_handle: FocusHandle,
    pending_navigation: Option<TabId>,
    pending_close: Option<TabId>,
    scroll_offset: f32,
    max_visible_tabs: usize,
}

impl TabBarView {
    pub fn new(initial_path: std::path::PathBuf, cx: &mut Context<Self>) -> Self {
        Self {
            tab_state: TabState::new(initial_path),
            focus_handle: cx.focus_handle(),
            pending_navigation: None,
            pending_close: None,
            scroll_offset: 0.0,
            max_visible_tabs: 10,
        }
    }

    pub fn with_tab_state(tab_state: TabState, cx: &mut Context<Self>) -> Self {
        Self {
            tab_state,
            focus_handle: cx.focus_handle(),
            pending_navigation: None,
            pending_close: None,
            scroll_offset: 0.0,
            max_visible_tabs: 10,
        }
    }

    /// Get a reference to the tab state
    pub fn tab_state(&self) -> &TabState {
        &self.tab_state
    }

    /// Get a mutable reference to the tab state
    pub fn tab_state_mut(&mut self) -> &mut TabState {
        &mut self.tab_state
    }

    /// Open a new tab for the given path
    pub fn open_tab(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) -> TabId {
        let id = self.tab_state.open_tab(path);
        cx.notify();
        id
    }

    /// Close a tab by ID
    pub fn close_tab(&mut self, id: TabId, cx: &mut Context<Self>) -> bool {
        let result = self.tab_state.close_tab(id);
        cx.notify();
        result
    }

    /// Switch to a tab by ID
    pub fn switch_to(&mut self, id: TabId, cx: &mut Context<Self>) -> bool {
        let result = self.tab_state.switch_to(id);
        cx.notify();
        result
    }

    /// Get the active tab's path
    pub fn active_path(&self) -> &std::path::Path {
        &self.tab_state.active_tab().path
    }

    /// Update the active tab's path
    pub fn update_active_path(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        self.tab_state.update_active_path(path);
        cx.notify();
    }

    /// Take pending navigation (tab switch request)
    pub fn take_pending_navigation(&mut self) -> Option<TabId> {
        self.pending_navigation.take()
    }

    /// Take pending close request
    pub fn take_pending_close(&mut self) -> Option<TabId> {
        self.pending_close.take()
    }

    /// Get the number of tabs
    pub fn tab_count(&self) -> usize {
        self.tab_state.tab_count()
    }

    /// Check if there are more tabs than can be displayed
    pub fn has_overflow(&self) -> bool {
        self.tab_state.tab_count() > self.max_visible_tabs
    }

    /// Scroll tabs left
    pub fn scroll_left(&mut self, cx: &mut Context<Self>) {
        self.scroll_offset = (self.scroll_offset - 1.0).max(0.0);
        cx.notify();
    }

    /// Scroll tabs right
    pub fn scroll_right(&mut self, cx: &mut Context<Self>) {
        let max_offset = (self.tab_state.tab_count() as f32 - self.max_visible_tabs as f32).max(0.0);
        self.scroll_offset = (self.scroll_offset + 1.0).min(max_offset);
        cx.notify();
    }

    fn handle_tab_click(&mut self, id: TabId, cx: &mut Context<Self>) {
        self.pending_navigation = Some(id);
        self.tab_state.switch_to(id);
        cx.notify();
    }

    fn handle_tab_close(&mut self, id: TabId, cx: &mut Context<Self>) {
        self.pending_close = Some(id);
        self.tab_state.close_tab(id);
        cx.notify();
    }

    fn handle_new_tab(&mut self, cx: &mut Context<Self>) {
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
        self.open_tab(home, cx);
    }
}

impl Focusable for TabBarView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TabBarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme_colors();
        let bg_dark = theme.bg_secondary;
        let bg_active = theme.bg_tertiary;
        let border_color = theme.border_default;
        let text_primary = theme.text_primary;
        let text_muted = theme.text_muted;
        let hover_bg = theme.bg_hover;
        let accent = theme.accent_primary;

        let tabs = self.tab_state.tabs().to_vec();
        let active_index = self.tab_state.active_index();
        let has_overflow = self.has_overflow();
        let scroll_offset = self.scroll_offset as usize;

        let entity = cx.entity().clone();

        div()
            .id("tab-bar")
            .h(px(36.0))
            .bg(bg_dark)
            .border_b_1()
            .border_color(border_color)
            .flex()
            .items_center()
            .px_2()
            .gap_1()
            // Scroll left button (if overflow)
            .when(has_overflow && scroll_offset > 0, |s| {
                let entity = entity.clone();
                s.child(
                    div()
                        .id("scroll-left")
                        .p_1()
                        .rounded_sm()
                        .cursor_pointer()
                        .hover(|h| h.bg(hover_bg))
                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                            entity.update(cx, |view, cx| {
                                view.scroll_left(cx);
                            });
                        })
                        .child(
                            svg()
                                .path("assets/icons/chevron-left.svg")
                                .size(px(14.0))
                                .text_color(text_muted),
                        ),
                )
            })
            // Tab container
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .overflow_hidden()
                    .children(
                        tabs.iter()
                            .enumerate()
                            .skip(scroll_offset)
                            .take(self.max_visible_tabs)
                            .map(|(index, tab)| {
                                let is_active = index == active_index;
                                let tab_id = tab.id;
                                let title = tab.title.clone();
                                let needs_refresh = tab.needs_refresh;
                                let entity_for_click = entity.clone();
                                let entity_for_close = entity.clone();

                                div()
                                    .id(SharedString::from(format!("tab-{}", tab_id.0)))
                                    .h(px(28.0))
                                    .px_3()
                                    .rounded_t_md()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .cursor_pointer()
                                    .when(is_active, |s| {
                                        s.bg(bg_active)
                                            .border_b_2()
                                            .border_color(accent)
                                    })
                                    .when(!is_active, |s| {
                                        s.hover(|h| h.bg(hover_bg))
                                    })
                                    .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                        entity_for_click.update(cx, |view, cx| {
                                            view.handle_tab_click(tab_id, cx);
                                        });
                                    })
                                    // Folder icon
                                    .child(
                                        svg()
                                            .path("assets/icons/folder.svg")
                                            .size(px(14.0))
                                            .text_color(if is_active { accent } else { text_muted }),
                                    )
                                    // Tab title
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(if is_active { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                                            .text_color(if is_active { text_primary } else { text_muted })
                                            .max_w(px(120.0))
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(title),
                                    )
                                    // Refresh indicator
                                    .when(needs_refresh, |s| {
                                        s.child(
                                            div()
                                                .w(px(6.0))
                                                .h(px(6.0))
                                                .rounded_full()
                                                .bg(theme.warning),
                                        )
                                    })
                                    // Close button
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("close-tab-{}", tab_id.0)))
                                            .p_0p5()
                                            .rounded_sm()
                                            .cursor_pointer()
                                            .hover(|h| h.bg(gpui::rgb(0x3d4148)))
                                            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                                entity_for_close.update(cx, |view, cx| {
                                                    view.handle_tab_close(tab_id, cx);
                                                });
                                            })
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(text_muted)
                                                    .child("×"),
                                            ),
                                    )
                            }),
                    ),
            )
            // Scroll right button (if overflow)
            .when(has_overflow && scroll_offset < tabs.len().saturating_sub(self.max_visible_tabs), |s| {
                let entity = entity.clone();
                s.child(
                    div()
                        .id("scroll-right")
                        .p_1()
                        .rounded_sm()
                        .cursor_pointer()
                        .hover(|h| h.bg(hover_bg))
                        .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                            entity.update(cx, |view, cx| {
                                view.scroll_right(cx);
                            });
                        })
                        .child(
                            svg()
                                .path("assets/icons/chevron-right.svg")
                                .size(px(14.0))
                                .text_color(text_muted),
                        ),
                )
            })
            // New tab button
            .child(
                div()
                    .id("new-tab")
                    .p_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|h| h.bg(hover_bg))
                    .on_mouse_down(MouseButton::Left, {
                        let entity = entity.clone();
                        move |_event, _window, cx| {
                            entity.update(cx, |view, cx| {
                                view.handle_new_tab(cx);
                            });
                        }
                    })
                    .child(
                        svg()
                            .path("assets/icons/file-plus.svg")
                            .size(px(14.0))
                            .text_color(text_muted),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_tab_bar_creation() {
        // Basic test - actual GPUI context tests would need more setup
        let state = TabState::new(PathBuf::from("/home"));
        assert_eq!(state.tab_count(), 1);
    }
}
