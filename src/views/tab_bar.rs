use gpui::{
    div, prelude::*, px, svg, App, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, SharedString, Styled, Window,
};

use crate::models::{theme_colors, TabId, TabState};

pub struct TabBarView {
    tab_state: TabState,
    focus_handle: FocusHandle,
    pending_navigation: Option<TabId>,
    pending_close: Option<TabId>,
    pending_new_tab: bool,
    scroll_offset: f32,
    max_visible_tabs: usize,
    hovered_tab: Option<TabId>,
    dragging_tab: Option<TabId>,
}

impl TabBarView {
    pub fn new(initial_path: std::path::PathBuf, cx: &mut Context<Self>) -> Self {
        Self {
            tab_state: TabState::new(initial_path),
            focus_handle: cx.focus_handle(),
            pending_navigation: None,
            pending_close: None,
            pending_new_tab: false,
            scroll_offset: 0.0,
            max_visible_tabs: 12,
            hovered_tab: None,
            dragging_tab: None,
        }
    }

    pub fn with_tab_state(tab_state: TabState, cx: &mut Context<Self>) -> Self {
        Self {
            tab_state,
            focus_handle: cx.focus_handle(),
            pending_navigation: None,
            pending_close: None,
            pending_new_tab: false,
            scroll_offset: 0.0,
            max_visible_tabs: 12,
            hovered_tab: None,
            dragging_tab: None,
        }
    }

    pub fn tab_state(&self) -> &TabState {
        &self.tab_state
    }

    pub fn tab_state_mut(&mut self) -> &mut TabState {
        &mut self.tab_state
    }

    pub fn open_tab(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) -> TabId {
        let id = self.tab_state.open_tab(path);
        self.ensure_tab_visible(self.tab_state.active_index());
        cx.notify();
        id
    }

    pub fn close_tab(&mut self, id: TabId, cx: &mut Context<Self>) -> bool {
        let result = self.tab_state.close_tab(id);
        cx.notify();
        result
    }

    pub fn switch_to(&mut self, id: TabId, cx: &mut Context<Self>) -> bool {
        let result = self.tab_state.switch_to(id);
        if result {
            self.ensure_tab_visible(self.tab_state.active_index());
        }
        cx.notify();
        result
    }

    pub fn active_path(&self) -> &std::path::Path {
        &self.tab_state.active_tab().path
    }

    pub fn navigate_to(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        self.tab_state.navigate_active_to(path);
        cx.notify();
    }

    pub fn take_pending_navigation(&mut self) -> Option<TabId> {
        self.pending_navigation.take()
    }

    pub fn take_pending_close(&mut self) -> Option<TabId> {
        self.pending_close.take()
    }

    pub fn take_pending_new_tab(&mut self) -> bool {
        std::mem::take(&mut self.pending_new_tab)
    }

    pub fn tab_count(&self) -> usize {
        self.tab_state.tab_count()
    }

    fn ensure_tab_visible(&mut self, index: usize) {
        let scroll_offset = self.scroll_offset as usize;
        if index < scroll_offset {
            self.scroll_offset = index as f32;
        } else if index >= scroll_offset + self.max_visible_tabs {
            self.scroll_offset = (index - self.max_visible_tabs + 1) as f32;
        }
    }

    fn has_overflow(&self) -> bool {
        self.tab_state.tab_count() > self.max_visible_tabs
    }

    fn scroll_left(&mut self, cx: &mut Context<Self>) {
        self.scroll_offset = (self.scroll_offset - 1.0).max(0.0);
        cx.notify();
    }

    fn scroll_right(&mut self, cx: &mut Context<Self>) {
        let max_offset =
            (self.tab_state.tab_count() as f32 - self.max_visible_tabs as f32).max(0.0);
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
        self.pending_new_tab = true;
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
        self.open_tab(home, cx);
    }

    fn handle_tab_middle_click(&mut self, id: TabId, cx: &mut Context<Self>) {
        self.handle_tab_close(id, cx);
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
        let tabs = self.tab_state.tabs().to_vec();
        let active_index = self.tab_state.active_index();
        let has_overflow = self.has_overflow();
        let scroll_offset = self.scroll_offset as usize;
        let entity = cx.entity().clone();

        div()
            .id("tab-bar")
            .h(px(38.0))
            .bg(theme.bg_secondary)
            .border_b_1()
            .border_color(theme.border_default)
            .flex()
            .items_center()
            .px_1()
            .gap_0p5()
            .when(has_overflow && scroll_offset > 0, |s| {
                let entity = entity.clone();
                s.child(
                    div()
                        .id("scroll-left")
                        .p_1()
                        .rounded_sm()
                        .cursor_pointer()
                        .hover(|h| h.bg(theme.bg_hover))
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            entity.update(cx, |view, cx| view.scroll_left(cx));
                        })
                        .child(
                            svg()
                                .path("assets/icons/chevron-left.svg")
                                .size(px(14.0))
                                .text_color(theme.text_muted),
                        ),
                )
            })
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap_0p5()
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
                                let is_pinned = tab.pinned;
                                let needs_refresh = tab.needs_refresh;
                                let entity_click = entity.clone();
                                let entity_close = entity.clone();
                                let entity_middle = entity.clone();

                                div()
                                    .id(SharedString::from(format!("tab-{}", tab_id.0)))
                                    .h(px(30.0))
                                    .min_w(px(if is_pinned { 40.0 } else { 100.0 }))
                                    .max_w(px(200.0))
                                    .px_3()
                                    .rounded_md()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .cursor_pointer()
                                    .when(is_active, |s| {
                                        s.bg(theme.bg_tertiary)
                                            .border_1()
                                            .border_color(theme.border_subtle)
                                    })
                                    .when(!is_active, |s| s.hover(|h| h.bg(theme.bg_hover)))
                                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                        entity_click.update(cx, |view, cx| {
                                            view.handle_tab_click(tab_id, cx);
                                        });
                                    })
                                    .on_mouse_down(MouseButton::Middle, move |_, _, cx| {
                                        entity_middle.update(cx, |view, cx| {
                                            view.handle_tab_middle_click(tab_id, cx);
                                        });
                                    })
                                    .child(
                                        svg()
                                            .path("assets/icons/folder.svg")
                                            .size(px(14.0))
                                            .text_color(if is_active {
                                                theme.accent_primary
                                            } else {
                                                theme.text_muted
                                            }),
                                    )
                                    .when(!is_pinned, |s| {
                                        s.child(
                                            div()
                                                .text_xs()
                                                .font_weight(if is_active {
                                                    gpui::FontWeight::MEDIUM
                                                } else {
                                                    gpui::FontWeight::NORMAL
                                                })
                                                .text_color(if is_active {
                                                    theme.text_primary
                                                } else {
                                                    theme.text_muted
                                                })
                                                .flex_1()
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(title),
                                        )
                                    })
                                    .when(needs_refresh, |s| {
                                        s.child(
                                            div()
                                                .w(px(6.0))
                                                .h(px(6.0))
                                                .rounded_full()
                                                .bg(theme.warning),
                                        )
                                    })
                                    .when(!is_pinned, |s| {
                                        s.child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "close-{}",
                                                    tab_id.0
                                                )))
                                                .w(px(16.0))
                                                .h(px(16.0))
                                                .rounded_sm()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .cursor_pointer()
                                                .hover(|h| h.bg(theme.bg_hover))
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    move |_, _, cx| {
                                                        entity_close.update(cx, |view, cx| {
                                                            view.handle_tab_close(tab_id, cx);
                                                        });
                                                    },
                                                )
                                                .child(
                                                    svg()
                                                        .path("assets/icons/x.svg")
                                                        .size(px(12.0))
                                                        .text_color(theme.text_muted),
                                                ),
                                        )
                                    })
                            }),
                    ),
            )
            .when(
                has_overflow && scroll_offset < tabs.len().saturating_sub(self.max_visible_tabs),
                |s| {
                    let entity = entity.clone();
                    s.child(
                        div()
                            .id("scroll-right")
                            .p_1()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|h| h.bg(theme.bg_hover))
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                entity.update(cx, |view, cx| view.scroll_right(cx));
                            })
                            .child(
                                svg()
                                    .path("assets/icons/chevron-right.svg")
                                    .size(px(14.0))
                                    .text_color(theme.text_muted),
                            ),
                    )
                },
            )
            .child(
                div()
                    .id("new-tab")
                    .w(px(28.0))
                    .h(px(28.0))
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|h| h.bg(theme.bg_hover))
                    .on_mouse_down(MouseButton::Left, {
                        let entity = entity.clone();
                        move |_, _, cx| {
                            entity.update(cx, |view, cx| view.handle_new_tab(cx));
                        }
                    })
                    .child(
                        svg()
                            .path("assets/icons/plus.svg")
                            .size(px(14.0))
                            .text_color(theme.text_muted),
                    ),
            )
    }
}
