use gpui::{
    actions, div, prelude::*, px, svg, App, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement, Render, Styled,
    Window,
};

use crate::models::SearchEngine;

// Define actions for search input
actions!(search_input, [ClearSearch, EscapeSearch]);

/// Search input state for managing query and focus
pub struct SearchInput {
    query: String,
    placeholder: String,
}

/// View wrapper for SearchInput with GPUI integration
pub struct SearchInputView {
    search_input: SearchInput,
    focus_handle: FocusHandle,
    search_engine: Option<Entity<SearchEngine>>,
    pending_query_update: Option<String>,
}

impl SearchInput {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            placeholder: "Search files, commands...".to_string(),
        }
    }

    pub fn with_placeholder(placeholder: impl Into<String>) -> Self {
        Self {
            query: String::new(),
            placeholder: placeholder.into(),
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }

    pub fn clear(&mut self) {
        self.query.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }
}

impl Default for SearchInput {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchInputView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            search_input: SearchInput::new(),
            focus_handle: cx.focus_handle(),
            search_engine: None,
            pending_query_update: None,
        }
    }

    pub fn with_search_engine(mut self, engine: Entity<SearchEngine>) -> Self {
        self.search_engine = Some(engine);
        self
    }

    pub fn query(&self) -> &str {
        self.search_input.query()
    }

    pub fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.search_input.set_query(query.clone());
        
        if let Some(engine) = &self.search_engine {
            engine.update(cx, |engine, _| {
                engine.set_pattern(&query);
            });
        }
        
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.search_input.clear();
        
        if let Some(engine) = &self.search_engine {
            engine.update(cx, |engine, _| {
                engine.set_pattern("");
            });
        }
        
        cx.notify();
    }

    pub fn is_empty(&self) -> bool {
        self.search_input.is_empty()
    }

    pub fn take_pending_query(&mut self) -> Option<String> {
        self.pending_query_update.take()
    }

    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus_handle);
    }

    fn handle_escape(&mut self, _: &EscapeSearch, _window: &mut Window, cx: &mut Context<Self>) {
        self.clear(cx);
    }

    fn handle_clear(&mut self, _: &ClearSearch, _window: &mut Window, cx: &mut Context<Self>) {
        self.clear(cx);
    }

    /// Register key bindings for search input
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("escape", EscapeSearch, Some("SearchInput")),
        ]);
    }
}

impl Focusable for SearchInputView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SearchInputView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg_input = gpui::rgb(0x161b22);
        let border_color = gpui::rgb(0x30363d);
        let border_focus = gpui::rgb(0x1f6feb);
        let text_gray = gpui::rgb(0x6e7681);
        let text_light = gpui::rgb(0xc9d1d9);
        let icon_color = gpui::rgb(0x6e7681);

        let is_focused = self.focus_handle.is_focused(window);
        let query = self.search_input.query.clone();
        let placeholder = self.search_input.placeholder.clone();
        let is_empty = query.is_empty();

        let entity = cx.entity().clone();

        div()
            .id("search-input-container")
            .key_context("SearchInput")
            .relative()
            .w_full()
            .on_action(cx.listener(Self::handle_escape))
            .on_action(cx.listener(Self::handle_clear))
            .child(
                div()
                    .id("search-input")
                    .w_full()
                    .bg(bg_input)
                    .text_xs()
                    .rounded_md()
                    .border_1()
                    .when(is_focused, |s| s.border_color(border_focus))
                    .when(!is_focused, |s| s.border_color(border_color))
                    .py_1p5()
                    .pl(px(32.0))
                    .pr_3()
                    .flex()
                    .items_center()
                    .cursor_text()
                    .relative()
                    .track_focus(&self.focus_handle)
                    .child(
                        svg()
                            .path("assets/icons/search.svg")
                            .size(px(13.0))
                            .text_color(icon_color)
                            .absolute()
                            .left(px(10.0))
                            .top(px(6.0)),
                    )
                    .when(is_empty, |s| {
                        s.child(
                            div()
                                .text_color(text_gray)
                                .child(placeholder),
                        )
                    })
                    .when(!is_empty, |s| {
                        s.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_0()
                                .child(
                                    div()
                                        .text_color(text_light)
                                        .child(query.clone()),
                                )
                                .when(is_focused, |s| {
                                    s.child(
                                        div()
                                            .w(px(1.0))
                                            .h(px(14.0))
                                            .bg(text_light),
                                    )
                                }),
                        )
                    })
                    .when(!is_empty, |s| {
                        let entity = entity.clone();
                        s.child(
                            div()
                                .id("clear-search")
                                .absolute()
                                .right(px(8.0))
                                .top(px(5.0))
                                .p_0p5()
                                .rounded_sm()
                                .cursor_pointer()
                                .hover(|h| h.bg(gpui::rgb(0x21262d)))
                                .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                                    entity.update(cx, |view, cx| {
                                        view.clear(cx);
                                    });
                                })
                                .child(
                                    div()
                                        .text_color(text_gray)
                                        .text_xs()
                                        .child("✕"),
                                ),
                        )
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_input_new() {
        let input = SearchInput::new();
        assert!(input.is_empty());
    }

    #[test]
    fn test_search_input_set_query() {
        let mut input = SearchInput::new();
        input.set_query("test".to_string());
        assert_eq!(input.query(), "test");
        assert!(!input.is_empty());
    }

    #[test]
    fn test_search_input_clear() {
        let mut input = SearchInput::new();
        input.set_query("test".to_string());
        input.clear();
        assert!(input.is_empty());
    }
}
