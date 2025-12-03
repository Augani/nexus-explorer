use gpui::{
    actions, div, prelude::*, px, svg, App, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement, Render, Styled,
    Window, ElementInputHandler, UTF16Selection, Bounds, Pixels, Point, EntityInputHandler,
};
use std::ops::Range;

use crate::models::SearchEngine;

actions!(search_input, [ClearSearch, EscapeSearch, Backspace, Delete, SelectAll]);

/// Search input state for managing query and focus
pub struct SearchInput {
    query: String,
    placeholder: String,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_bounds: Option<Bounds<Pixels>>,
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
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_bounds: None,
        }
    }

    pub fn with_placeholder(placeholder: impl Into<String>) -> Self {
        Self {
            query: String::new(),
            placeholder: placeholder.into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_bounds: None,
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        let len = self.query.len();
        self.selected_range = len..len;
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.selected_range = 0..0;
    }

    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.query[..offset]
            .char_indices()
            .next_back()
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.query[offset..]
            .char_indices()
            .nth(1)
            .map(|(idx, _)| offset + idx)
            .unwrap_or(self.query.len())
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.query.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.query.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
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

    fn handle_backspace(&mut self, _: &Backspace, _window: &mut Window, cx: &mut Context<Self>) {
        if self.search_input.selected_range.is_empty() {
            let cursor = self.search_input.cursor_offset();
            let prev = self.search_input.previous_boundary(cursor);
            self.search_input.selected_range = prev..cursor;
        }
        if !self.search_input.selected_range.is_empty() {
            let range = self.search_input.selected_range.clone();
            self.search_input.query = format!(
                "{}{}",
                &self.search_input.query[..range.start],
                &self.search_input.query[range.end..]
            );
            self.search_input.selected_range = range.start..range.start;
            self.notify_search_engine(cx);
        }
        cx.notify();
    }

    fn handle_delete(&mut self, _: &Delete, _window: &mut Window, cx: &mut Context<Self>) {
        if self.search_input.selected_range.is_empty() {
            let cursor = self.search_input.cursor_offset();
            let next = self.search_input.next_boundary(cursor);
            self.search_input.selected_range = cursor..next;
        }
        if !self.search_input.selected_range.is_empty() {
            let range = self.search_input.selected_range.clone();
            self.search_input.query = format!(
                "{}{}",
                &self.search_input.query[..range.start],
                &self.search_input.query[range.end..]
            );
            self.search_input.selected_range = range.start..range.start;
            self.notify_search_engine(cx);
        }
        cx.notify();
    }

    fn handle_select_all(&mut self, _: &SelectAll, _window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.selected_range = 0..self.search_input.query.len();
        cx.notify();
    }

    fn notify_search_engine(&self, cx: &mut Context<Self>) {
        if let Some(engine) = &self.search_engine {
            let query = self.search_input.query.clone();
            engine.update(cx, |engine, _| {
                engine.set_pattern(&query);
            });
        }
    }

    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("escape", EscapeSearch, Some("SearchInput")),
            KeyBinding::new("backspace", Backspace, Some("SearchInput")),
            KeyBinding::new("delete", Delete, Some("SearchInput")),
            #[cfg(target_os = "macos")]
            KeyBinding::new("cmd-a", SelectAll, Some("SearchInput")),
            #[cfg(not(target_os = "macos"))]
            KeyBinding::new("ctrl-a", SelectAll, Some("SearchInput")),
        ]);
    }
}

impl EntityInputHandler for SearchInputView {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.search_input.range_from_utf16(&range_utf16);
        actual_range.replace(self.search_input.range_to_utf16(&range));
        Some(self.search_input.query[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.search_input.range_to_utf16(&self.search_input.selected_range),
            reversed: self.search_input.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.search_input.marked_range
            .as_ref()
            .map(|range| self.search_input.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.search_input.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.search_input.range_from_utf16(r))
            .or(self.search_input.marked_range.clone())
            .unwrap_or(self.search_input.selected_range.clone());

        self.search_input.query = format!(
            "{}{}{}",
            &self.search_input.query[..range.start],
            new_text,
            &self.search_input.query[range.end..]
        );
        
        let new_cursor = range.start + new_text.len();
        self.search_input.selected_range = new_cursor..new_cursor;
        self.search_input.marked_range = None;

        self.notify_search_engine(cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.search_input.range_from_utf16(r))
            .or(self.search_input.marked_range.clone())
            .unwrap_or(self.search_input.selected_range.clone());

        self.search_input.query = format!(
            "{}{}{}",
            &self.search_input.query[..range.start],
            new_text,
            &self.search_input.query[range.end..]
        );

        if !new_text.is_empty() {
            self.search_input.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.search_input.marked_range = None;
        }

        self.search_input.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|r| self.search_input.range_from_utf16(r))
            .map(|new_range| new_range.start + range.start..new_range.end + range.start)
            .unwrap_or_else(|| {
                let pos = range.start + new_text.len();
                pos..pos
            });

        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.search_input.offset_to_utf16(self.search_input.query.len()))
    }
}

impl Focusable for SearchInputView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SearchInputView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use crate::models::theme_colors;
        let theme = theme_colors();
        let bg_input = theme.bg_secondary;
        let border_color = theme.border_default;
        let border_focus = theme.accent_primary;
        let text_gray = theme.text_muted;
        let text_light = theme.text_primary;
        let icon_color = theme.text_muted;
        let selection_bg = gpui::rgba(0x1f6feb40);
        let cursor_color = theme.accent_primary;

        let is_focused = self.focus_handle.is_focused(window);
        let query = self.search_input.query.clone();
        let placeholder = self.search_input.placeholder.clone();
        let is_empty = query.is_empty();
        let selected_range = self.search_input.selected_range.clone();
        let cursor_pos = self.search_input.cursor_offset();

        let entity = cx.entity().clone();

        div()
            .id("search-input-container")
            .key_context("SearchInput")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::handle_escape))
            .on_action(cx.listener(Self::handle_clear))
            .on_action(cx.listener(Self::handle_backspace))
            .on_action(cx.listener(Self::handle_delete))
            .on_action(cx.listener(Self::handle_select_all))
            .relative()
            .w_full()
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
                    .child(
                        svg()
                            .path("assets/icons/search.svg")
                            .size(px(13.0))
                            .text_color(icon_color)
                            .absolute()
                            .left(px(10.0))
                            .top(px(6.0)),
                    )
                    .child(
                        div()
                            .id("search-text-input")
                            .flex_1()
                            .overflow_hidden()
                            .child({
                                let entity_for_input = entity.clone();
                                SearchTextElement { 
                                    input: entity_for_input,
                                    query: query.clone(),
                                    placeholder: placeholder.clone(),
                                    is_focused,
                                    selected_range: selected_range.clone(),
                                    cursor_pos,
                                    text_color: text_light,
                                    placeholder_color: text_gray,
                                    selection_bg,
                                    cursor_color,
                                }
                            })
                    )
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
                                .hover(|h| h.bg(theme.bg_hover))
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

/// Custom element that handles text input via window.handle_input()
struct SearchTextElement {
    input: Entity<SearchInputView>,
    query: String,
    placeholder: String,
    is_focused: bool,
    selected_range: Range<usize>,
    cursor_pos: usize,
    text_color: gpui::Rgba,
    placeholder_color: gpui::Rgba,
    selection_bg: gpui::Rgba,
    cursor_color: gpui::Rgba,
}

impl IntoElement for SearchTextElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl gpui::Element for SearchTextElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let mut style = gpui::Style::default();
        style.size.width = gpui::relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        ()
    }

    fn paint(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        
        // This is the key call that enables text input
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        // Render text
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        
        let (display_text, color): (String, gpui::Rgba) = if self.query.is_empty() {
            (self.placeholder.clone(), self.placeholder_color)
        } else {
            (self.query.clone(), self.text_color)
        };

        let run = gpui::TextRun {
            len: display_text.len(),
            font: style.font(),
            color: color.into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let line = window.text_system().shape_line(
            display_text.into(),
            font_size,
            &[run],
            None,
        );
        
        // Draw selection if any
        if self.is_focused && !self.selected_range.is_empty() && !self.query.is_empty() {
            let start_x = line.x_for_index(self.selected_range.start);
            let end_x = line.x_for_index(self.selected_range.end);
            let selection_bounds = Bounds::from_corners(
                gpui::point(bounds.left() + start_x, bounds.top()),
                gpui::point(bounds.left() + end_x, bounds.bottom()),
            );
            window.paint_quad(gpui::fill(selection_bounds, self.selection_bg));
        }

        // Draw text
        let _ = line.paint(bounds.origin, window.line_height(), window, cx);

        // Draw cursor if focused and no selection
        if self.is_focused && self.selected_range.is_empty() {
            let cursor_x = if self.query.is_empty() {
                px(0.0)
            } else {
                line.x_for_index(self.cursor_pos)
            };
            let cursor_bounds = Bounds::new(
                gpui::point(bounds.left() + cursor_x, bounds.top()),
                gpui::size(px(1.5), bounds.size.height),
            );
            window.paint_quad(gpui::fill(cursor_bounds, self.cursor_color));
        }

        // Store bounds for hit testing
        self.input.update(cx, |input, _| {
            input.search_input.last_bounds = Some(bounds);
        });
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
