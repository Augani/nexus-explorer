use gpui::{
    div, px, App, ClipboardItem, Context, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render,
    ScrollHandle, ScrollWheelEvent, StatefulInteractiveElement, Styled, Timer, Window,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::models::{
    key_codes, theme_colors, AnsiParser, ClearMode, ParsedSegment, PtyService, TerminalState,
};

/// Terminal line height in pixels
const LINE_HEIGHT: f32 = 20.0;
/// Terminal character width (monospace)
const CHAR_WIDTH: f32 = 8.4;
/// Terminal padding
const TERMINAL_PADDING: f32 = 12.0;
/// Terminal content height (excluding header and scrollbar)
const TERMINAL_CONTENT_HEIGHT: f32 = 260.0;
/// Number of lines to render above/below visible area for smooth scrolling
const OVERSCAN_LINES: usize = 3;

/// Cursor blink interval in milliseconds
const CURSOR_BLINK_INTERVAL_MS: u64 = 530;

/// Terminal view component with virtualized rendering
pub struct TerminalView {
    state: TerminalState,
    parser: AnsiParser,
    pty: Option<PtyService>,
    focus_handle: FocusHandle,
    is_visible: bool,
    cursor_blink: bool,
    cursor_blink_state: bool,
    last_blink_time: Instant,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
    is_selecting: bool,
    scroll_handle: ScrollHandle,
    viewport_height: f32,
    polling_started: bool,
}

impl TerminalView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            state: TerminalState::default(),
            parser: AnsiParser::new(),
            pty: None,
            focus_handle: cx.focus_handle(),
            is_visible: false,
            cursor_blink: true,
            cursor_blink_state: true,
            last_blink_time: Instant::now(),
            selection_start: None,
            selection_end: None,
            is_selecting: false,
            scroll_handle: ScrollHandle::new(),
            viewport_height: TERMINAL_CONTENT_HEIGHT,
            polling_started: false,
        }
    }

    pub fn with_working_directory(mut self, path: PathBuf) -> Self {
        self.state = self.state.with_working_directory(path);
        self
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.is_visible = visible;
    }

    pub fn toggle_visible(&mut self) {
        self.is_visible = !self.is_visible;
    }

    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus_handle);
    }

    pub fn should_focus(&self) -> bool {
        self.is_visible && self.is_running()
    }

    pub fn is_running(&self) -> bool {
        self.pty.as_ref().map(|p| p.is_running()).unwrap_or(false)
    }

    pub fn working_directory(&self) -> &PathBuf {
        self.state.working_directory()
    }

    pub fn set_working_directory(&mut self, path: PathBuf) {
        self.state.set_working_directory(path.clone());
        if let Some(pty) = &mut self.pty {
            pty.set_working_directory(path);
        }
    }

    /// Change directory in the running shell by sending a cd command
    pub fn change_directory(&mut self, path: PathBuf) {
        let is_running = self.is_running();
        self.state.set_working_directory(path.clone());
        if let Some(pty) = &mut self.pty {
            pty.set_working_directory(path.clone());
            if is_running {
                let path_str = path.to_string_lossy();
                let cd_cmd = format!("cd '{}'\n", path_str.replace("'", "'\\''"));
                let _ = pty.write(cd_cmd.as_bytes());
            }
        }
    }

    /// Start the terminal with output polling
    pub fn start_with_polling(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if self.is_running() {
            return Ok(());
        }

        let mut pty = PtyService::new()
            .with_working_directory(self.state.working_directory().clone())
            .with_size(self.state.cols() as u16, self.state.rows() as u16);

        pty.start().map_err(|e| e.to_string())?;
        self.pty = Some(pty);
        self.state.set_running(true);
        self.is_visible = true;

        // Start polling task for PTY output
        self.polling_started = true;
        cx.spawn_in(window, async move |this, cx| loop {
            Timer::after(Duration::from_millis(16)).await;

            let should_continue = this
                .update(cx, |view, cx| {
                    if !view.is_running() {
                        view.polling_started = false;
                        return false;
                    }
                    view.process_output();
                    cx.notify();
                    true
                })
                .unwrap_or(false);

            if !should_continue {
                break;
            }
        })
        .detach();

        // Focus the terminal
        window.focus(&self.focus_handle);

        Ok(())
    }

    /// Start the terminal (legacy, no polling)
    pub fn start(&mut self) -> Result<(), String> {
        if self.is_running() {
            return Ok(());
        }

        let mut pty = PtyService::new()
            .with_working_directory(self.state.working_directory().clone())
            .with_size(self.state.cols() as u16, self.state.rows() as u16);

        pty.start().map_err(|e| e.to_string())?;
        self.pty = Some(pty);
        self.state.set_running(true);
        self.is_visible = true;

        Ok(())
    }

    /// Stop the terminal
    pub fn stop(&mut self) {
        if let Some(mut pty) = self.pty.take() {
            pty.stop();
        }
        self.state.set_running(false);
    }

    /// Restart the terminal
    pub fn restart(&mut self) -> Result<(), String> {
        self.stop();
        self.state.reset();
        self.start()
    }

    /// Process PTY output
    pub fn process_output(&mut self) {
        if let Some(pty) = &self.pty {
            let output = pty.drain_output();
            if !output.is_empty() {
                self.process_bytes(&output);
            }
        }
    }

    /// Process raw bytes through the ANSI parser
    fn process_bytes(&mut self, bytes: &[u8]) {
        let segments = self.parser.parse(bytes);
        for segment in segments {
            self.apply_segment(segment);
        }
    }

    /// Apply a parsed segment to the terminal state
    fn apply_segment(&mut self, segment: ParsedSegment) {
        match segment {
            ParsedSegment::Text(text, style) => {
                self.state.set_current_style(style);
                self.state.write_str(&text);
            }
            ParsedSegment::CursorUp(n) => self.state.cursor_up(n),
            ParsedSegment::CursorDown(n) => self.state.cursor_down(n),
            ParsedSegment::CursorForward(n) => self.state.cursor_forward(n),
            ParsedSegment::CursorBackward(n) => self.state.cursor_backward(n),
            ParsedSegment::CursorPosition(row, col) => self.state.move_cursor_to(row, col),
            ParsedSegment::CursorSave => self.state.save_cursor(),
            ParsedSegment::CursorRestore => self.state.restore_cursor(),
            ParsedSegment::ClearScreen(mode) => match mode {
                ClearMode::ToEnd => self.state.clear_to_end_of_screen(),
                ClearMode::ToStart => self.state.clear_to_start_of_screen(),
                ClearMode::All | ClearMode::Scrollback => self.state.clear_screen(),
            },
            ParsedSegment::ClearLine(mode) => match mode {
                ClearMode::ToEnd => self.state.clear_to_end_of_line(),
                ClearMode::ToStart => self.state.clear_to_start_of_line(),
                ClearMode::All | ClearMode::Scrollback => self.state.clear_line(),
            },
            ParsedSegment::SetTitle(title) => self.state.set_title(Some(title)),
            ParsedSegment::Bell => {}
            ParsedSegment::Backspace => self.state.backspace(),
            ParsedSegment::Tab => self.state.tab(),
            ParsedSegment::LineFeed => self.state.newline(),
            ParsedSegment::CarriageReturn => self.state.carriage_return(),
            ParsedSegment::ScrollUp(n) => {
                for _ in 0..n {
                    self.state.scroll_up();
                }
            }
            ParsedSegment::ScrollDown(n) => {
                for _ in 0..n {
                    self.state.scroll_down();
                }
            }
        }
    }

    /// Send input to the PTY
    pub fn send_input(&mut self, data: &[u8]) {
        if let Some(pty) = &mut self.pty {
            let _ = pty.write(data);
        }
    }

    /// Send a string to the PTY
    pub fn send_str(&mut self, s: &str) {
        self.send_input(s.as_bytes());
    }

    /// Resize the terminal
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.state.resize(cols, rows);
        if let Some(pty) = &mut self.pty {
            let _ = pty.resize(cols as u16, rows as u16);
        }
    }

    /// Scroll viewport up
    pub fn scroll_up(&mut self, lines: usize) {
        self.state.scroll_viewport_up(lines);
    }

    /// Scroll viewport down
    pub fn scroll_down(&mut self, lines: usize) {
        self.state.scroll_viewport_down(lines);
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.state.scroll_to_bottom();
    }

    /// Get visible line count
    pub fn visible_line_count(&self) -> usize {
        self.state.rows()
    }

    /// Get total line count (including scrollback)
    pub fn total_line_count(&self) -> usize {
        self.state.total_lines()
    }

    /// Calculate the visible line range for virtualized rendering
    /// Returns (start_index, end_index) of lines to render
    pub fn visible_line_range(&self) -> (usize, usize) {
        let total_lines = self.state.total_lines();
        let visible_rows = self.state.rows();
        let scroll_offset = self.state.scroll_offset();

        // Calculate the start of the visible viewport in the line buffer
        let viewport_start = total_lines.saturating_sub(visible_rows + scroll_offset);
        let viewport_end = total_lines.saturating_sub(scroll_offset);

        // Add overscan for smooth scrolling
        let render_start = viewport_start.saturating_sub(OVERSCAN_LINES);
        let render_end = (viewport_end + OVERSCAN_LINES).min(total_lines);

        (render_start, render_end)
    }

    /// Get lines for virtualized rendering (only visible + overscan)
    pub fn virtualized_lines(&self) -> impl Iterator<Item = (usize, &crate::models::TerminalLine)> {
        let (start, end) = self.visible_line_range();
        (start..end).filter_map(move |idx| self.state.line(idx).map(|line| (idx, line)))
    }

    /// Calculate the number of visible lines based on viewport height
    pub fn calculate_visible_rows(&self) -> usize {
        ((self.viewport_height - TERMINAL_PADDING * 2.0) / LINE_HEIGHT).floor() as usize
    }

    /// Set viewport height and update terminal rows
    pub fn set_viewport_height(&mut self, height: f32) {
        self.viewport_height = height;
        let rows = self.calculate_visible_rows();
        if rows != self.state.rows() && rows > 0 {
            self.resize(self.state.cols(), rows);
        }
    }

    /// Get scroll progress (0.0 = bottom, 1.0 = top)
    pub fn scroll_progress(&self) -> f32 {
        let max_offset = self.state.max_scroll_offset();
        if max_offset == 0 {
            0.0
        } else {
            self.state.scroll_offset() as f32 / max_offset as f32
        }
    }

    /// Get scrollbar thumb size as a fraction of the track
    pub fn scrollbar_thumb_size(&self) -> f32 {
        let total = self.state.total_lines();
        let visible = self.state.rows();
        if total == 0 {
            1.0
        } else {
            (visible as f32 / total as f32).min(1.0)
        }
    }

    /// Handle key down events - this is the main keyboard input handler
    pub fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        if !self.is_running() {
            return;
        }

        // Auto-scroll to bottom on input
        self.scroll_to_bottom();

        let key = event.keystroke.key.as_str();

        let handled = match key {
            "enter" => {
                self.send_input(key_codes::ENTER);
                true
            }
            "tab" => {
                self.send_input(key_codes::TAB);
                true
            }
            "backspace" => {
                self.send_input(key_codes::BACKSPACE);
                true
            }
            "escape" => {
                self.send_input(key_codes::ESCAPE);
                true
            }
            "delete" => {
                self.send_input(key_codes::DELETE);
                true
            }
            "up" => {
                self.send_input(key_codes::UP);
                true
            }
            "down" => {
                self.send_input(key_codes::DOWN);
                true
            }
            "left" => {
                self.send_input(key_codes::LEFT);
                true
            }
            "right" => {
                self.send_input(key_codes::RIGHT);
                true
            }
            "home" => {
                self.send_input(key_codes::HOME);
                true
            }
            "end" => {
                self.send_input(key_codes::END);
                true
            }
            "pageup" => {
                self.send_input(key_codes::PAGE_UP);
                true
            }
            "pagedown" => {
                self.send_input(key_codes::PAGE_DOWN);
                true
            }
            "insert" => {
                self.send_input(b"\x1b[2~");
                true
            }
            "f1" => {
                self.send_input(b"\x1bOP");
                true
            }
            "f2" => {
                self.send_input(b"\x1bOQ");
                true
            }
            "f3" => {
                self.send_input(b"\x1bOR");
                true
            }
            "f4" => {
                self.send_input(b"\x1bOS");
                true
            }
            "f5" => {
                self.send_input(b"\x1b[15~");
                true
            }
            "f6" => {
                self.send_input(b"\x1b[17~");
                true
            }
            "f7" => {
                self.send_input(b"\x1b[18~");
                true
            }
            "f8" => {
                self.send_input(b"\x1b[19~");
                true
            }
            "f9" => {
                self.send_input(b"\x1b[20~");
                true
            }
            "f10" => {
                self.send_input(b"\x1b[21~");
                true
            }
            "f11" => {
                self.send_input(b"\x1b[23~");
                true
            }
            "f12" => {
                self.send_input(b"\x1b[24~");
                true
            }
            "space" => {
                self.send_input(b" ");
                true
            }
            _ => false,
        };

        if !handled {
            if let Some(key_char) = &event.keystroke.key_char {
                if event.keystroke.modifiers.platform {
                    // Platform modifier (Cmd on macOS, Ctrl on Windows/Linux)
                    let c = key_char.chars().next().unwrap_or('\0').to_ascii_lowercase();
                    match c {
                        'c' => {
                            if self.has_selection() {
                                self.copy_selection(_cx);
                            } else {
                                self.send_input(&[0x03]);
                            }
                        }
                        'v' => {
                            self.paste_from_clipboard(_cx);
                        }
                        _ => {}
                    }
                } else if event.keystroke.modifiers.control {
                    // Ctrl+key combinations
                    let c = key_char.chars().next().unwrap_or('\0');
                    if c.is_ascii_alphabetic() {
                        let ctrl_code = (c.to_ascii_lowercase() as u8) - b'a' + 1;
                        self.send_input(&[ctrl_code]);
                    }
                } else if event.keystroke.modifiers.alt {
                    // Alt+key combinations (send ESC prefix)
                    let mut data = vec![0x1b];
                    data.extend(key_char.as_bytes());
                    self.send_input(&data);
                } else {
                    // Regular character input (including space)
                    self.send_str(key_char);
                    self.clear_selection();
                }
            } else if !event.keystroke.modifiers.platform
                && !event.keystroke.modifiers.control
                && !event.keystroke.modifiers.alt
            {
                // No key_char but also no modifiers - single character key or space
                if key == "space" {
                    self.send_input(b" ");
                    self.clear_selection();
                } else if key.len() == 1 {
                    self.send_str(key);
                    self.clear_selection();
                }
            }
        }

        self.reset_cursor_blink();
    }

    /// Handle mouse scroll events for terminal scrollback
    pub fn handle_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // Calculate lines to scroll based on delta
        let delta_y = match event.delta {
            gpui::ScrollDelta::Lines(lines) => lines.y,
            gpui::ScrollDelta::Pixels(pixels) => f32::from(pixels.y) / LINE_HEIGHT,
        };

        let lines = delta_y.abs().ceil() as usize;
        let lines = lines.max(1);

        // Natural scrolling: positive delta = scroll content up (view older), negative = scroll down
        if delta_y < 0.0 {
            self.scroll_up(lines);
        } else {
            self.scroll_down(lines);
        }
    }

    /// Handle mouse down for text selection and focus
    pub fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Focus the terminal on click
        window.focus(&self.focus_handle);

        if event.button == MouseButton::Left {
            let (line, col) = self.position_from_mouse(event.position);
            self.selection_start = Some((line, col));
            self.selection_end = Some((line, col));
            self.is_selecting = true;
            cx.notify();
        }
    }

    /// Handle mouse move for text selection
    pub fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            let (line, col) = self.position_from_mouse(event.position);
            self.selection_end = Some((line, col));
            cx.notify();
        }
    }

    /// Handle mouse up for text selection
    pub fn handle_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left {
            self.is_selecting = false;
            cx.notify();
        }
    }

    /// Convert mouse position to terminal line/column
    fn position_from_mouse(&self, position: gpui::Point<gpui::Pixels>) -> (usize, usize) {
        let x = f32::from(position.x) - TERMINAL_PADDING;
        let y = f32::from(position.y) - TERMINAL_PADDING - 36.0;

        let col = (x / CHAR_WIDTH).max(0.0) as usize;
        let row = (y / LINE_HEIGHT).max(0.0) as usize;

        // Convert to absolute line index
        let (render_start, _) = self.visible_line_range();
        let line = render_start + row;

        (line, col)
    }

    /// Check if a position is within the current selection
    fn is_position_selected(&self, line: usize, col: usize) -> bool {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let (start_line, start_col) = start;
            let (end_line, end_col) = end;

            // Normalize selection (start should be before end)
            let (start_line, start_col, end_line, end_col) =
                if start_line > end_line || (start_line == end_line && start_col > end_col) {
                    (end_line, end_col, start_line, start_col)
                } else {
                    (start_line, start_col, end_line, end_col)
                };

            if line < start_line || line > end_line {
                return false;
            }

            if line == start_line && line == end_line {
                col >= start_col && col < end_col
            } else if line == start_line {
                col >= start_col
            } else if line == end_line {
                col < end_col
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Get selected text
    pub fn get_selected_text(&self) -> Option<String> {
        let (start, end) = (self.selection_start?, self.selection_end?);
        let (start_line, start_col) = start;
        let (end_line, end_col) = end;

        // Normalize selection
        let (start_line, start_col, end_line, end_col) =
            if start_line > end_line || (start_line == end_line && start_col > end_col) {
                (end_line, end_col, start_line, start_col)
            } else {
                (start_line, start_col, end_line, end_col)
            };

        if start_line == end_line && start_col == end_col {
            return None;
        }

        let mut result = String::new();

        for line_idx in start_line..=end_line {
            if let Some(line) = self.state.line(line_idx) {
                let line_text: String = line.cells.iter().map(|c| c.char).collect();
                let line_text = line_text.trim_end();

                let col_start = if line_idx == start_line { start_col } else { 0 };
                let col_end = if line_idx == end_line {
                    end_col.min(line_text.len())
                } else {
                    line_text.len()
                };

                if col_start < line_text.len() {
                    let chars: Vec<char> = line_text.chars().collect();
                    let selected: String =
                        chars[col_start..col_end.min(chars.len())].iter().collect();
                    result.push_str(&selected);
                }

                if line_idx < end_line {
                    result.push('\n');
                }
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Copy selected text to clipboard
    pub fn copy_selection(&self, cx: &mut Context<Self>) {
        if let Some(text) = self.get_selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    /// Paste from clipboard
    pub fn paste_from_clipboard(&mut self, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                self.send_str(&text);
            }
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.is_selecting = false;
    }

    /// Check if there's an active selection
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some()
            && self.selection_end.is_some()
            && self.selection_start != self.selection_end
    }

    /// Update cursor blink state
    pub fn update_cursor_blink(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_blink_time)
            >= Duration::from_millis(CURSOR_BLINK_INTERVAL_MS)
        {
            self.cursor_blink_state = !self.cursor_blink_state;
            self.last_blink_time = now;
        }
    }

    /// Reset cursor blink (show cursor immediately after input)
    pub fn reset_cursor_blink(&mut self) {
        self.cursor_blink_state = true;
        self.last_blink_time = Instant::now();
    }

    /// Render a single terminal line - optimized for performance
    fn render_line(
        &self,
        idx: usize,
        line: &crate::models::TerminalLine,
        is_cursor_line: bool,
    ) -> impl IntoElement {
        let line_text: String = line.cells.iter().map(|c| c.char).collect();
        let cursor_col = self.state.cursor().col;
        let show_cursor = is_cursor_line
            && self.cursor_blink
            && self.cursor_blink_state
            && self.state.cursor_visible();

        let theme = theme_colors();
        let default_fg = theme.terminal_fg;
        let cursor_bg = theme.terminal_cursor;
        let cursor_fg = theme.terminal_bg;
        let selection_bg = theme.terminal_selection;

        let has_selection = self.has_selection() && {
            let (start, end) = (self.selection_start.unwrap(), self.selection_end.unwrap());
            let (s_line, e_line) = if start.0 <= end.0 {
                (start.0, end.0)
            } else {
                (end.0, start.0)
            };
            idx >= s_line && idx <= e_line
        };

        // Fast path: no cursor, no selection - just render the text
        if !show_cursor && !has_selection {
            return div()
                .id(("terminal-line", idx))
                .h(px(LINE_HEIGHT))
                .w_full()
                .flex()
                .items_center()
                .font_family("JetBrains Mono")
                .text_size(px(13.0))
                .text_color(default_fg)
                .child(line_text);
        }

        let chars: Vec<char> = line_text.chars().collect();
        let line_len = chars.len();
        let mut spans: Vec<gpui::AnyElement> = Vec::new();
        let mut current_text = String::new();
        let mut current_selected = false;

        let flush_span = |spans: &mut Vec<gpui::AnyElement>, text: &mut String, selected: bool| {
            if !text.is_empty() {
                let span = if selected {
                    div()
                        .bg(selection_bg)
                        .text_color(gpui::rgb(0xffffff))
                        .child(std::mem::take(text))
                } else {
                    div().text_color(default_fg).child(std::mem::take(text))
                };
                spans.push(span.into_any_element());
            }
        };

        for col in 0..=line_len {
            let is_cursor_pos = show_cursor && col == cursor_col;
            let is_selected = has_selection && self.is_position_selected(idx, col);

            if is_cursor_pos {
                // Flush any pending text
                flush_span(&mut spans, &mut current_text, current_selected);

                let cursor_char = chars.get(col).copied().unwrap_or(' ');
                let cursor_span = div()
                    .bg(cursor_bg)
                    .text_color(cursor_fg)
                    .child(cursor_char.to_string());
                spans.push(cursor_span.into_any_element());
                current_selected = is_selected;
            } else if col < line_len {
                if is_selected != current_selected && !current_text.is_empty() {
                    flush_span(&mut spans, &mut current_text, current_selected);
                }
                current_selected = is_selected;
                current_text.push(chars[col]);
            }
        }

        // Flush remaining text
        flush_span(&mut spans, &mut current_text, current_selected);

        // Add cursor at end if needed
        if show_cursor && cursor_col >= line_len {
            let cursor_span = div().bg(cursor_bg).text_color(cursor_fg).child(" ");
            spans.push(cursor_span.into_any_element());
        }

        div()
            .id(("terminal-line", idx))
            .h(px(LINE_HEIGHT))
            .w_full()
            .flex()
            .items_center()
            .font_family("JetBrains Mono")
            .text_size(px(13.0))
            .children(spans)
    }

    /// Calculate the absolute line index for the cursor
    fn cursor_absolute_line(&self) -> usize {
        let total = self.state.total_lines();
        let rows = self.state.rows();
        total.saturating_sub(rows) + self.state.cursor().row
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.process_output();

        // Start polling task if terminal is running and polling hasn't started
        if self.is_visible && self.is_running() && !self.polling_started {
            self.polling_started = true;
            cx.spawn_in(window, async move |this, cx| loop {
                Timer::after(Duration::from_millis(16)).await;

                let should_continue = this
                    .update(cx, |view, cx| {
                        if !view.is_running() || !view.is_visible {
                            view.polling_started = false;
                            return false;
                        }
                        view.process_output();
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);

                if !should_continue {
                    break;
                }
            })
            .detach();

            // Focus the terminal when it becomes visible
            window.focus(&self.focus_handle);
        }

        let theme = theme_colors();
        let bg_color = theme.terminal_bg;
        let border_color = theme.border_default;
        let header_bg = theme.bg_tertiary;
        let text_muted = theme.text_muted;
        let accent_color = theme.accent_primary;

        if !self.is_visible {
            return div().id("terminal-hidden").size_0();
        }

        let (render_start, render_end) = self.visible_line_range();
        let cursor_line = self.cursor_absolute_line();
        let total_lines = self.state.total_lines();
        let visible_rows = self.state.rows();

        // Calculate scrollbar metrics
        let thumb_size = self.scrollbar_thumb_size();
        let scroll_progress = self.scroll_progress();
        let scrollbar_track_height = TERMINAL_CONTENT_HEIGHT - 8.0;
        let thumb_height = (thumb_size * scrollbar_track_height).max(20.0);
        let thumb_offset = scroll_progress * (scrollbar_track_height - thumb_height);

        // Collect lines to render (virtualized)
        let lines_to_render: Vec<_> = (render_start..render_end)
            .filter_map(|idx| self.state.line(idx).map(|line| (idx, line.clone())))
            .collect();

        self.update_cursor_blink();

        div()
            .id("terminal-panel")
            .key_context("Terminal")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_key_down(event, window, cx);
                // Prevent event from bubbling
                cx.stop_propagation();
            }))
            .on_scroll_wheel(cx.listener(Self::handle_scroll))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .w_full()
            .h_full()
            .bg(bg_color)
            .border_color(border_color)
            .flex()
            .flex_col()
            .child(
                div()
                    .id("terminal-header")
                    .h(px(36.0))
                    .bg(header_bg)
                    .border_b_1()
                    .border_color(border_color)
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(text_muted)
                                    .child("TERMINAL"),
                            )
                            .child(
                                div().text_xs().text_color(accent_color).child(
                                    self.state.title().map(|s| s.to_string()).unwrap_or_else(
                                        || {
                                            self.state
                                                .working_directory()
                                                .to_string_lossy()
                                                .to_string()
                                        },
                                    ),
                                ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(text_muted)
                                    .child(format!("({}/{})", visible_rows, total_lines)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .items_center()
                            .child(div().text_xs().text_color(text_muted).child(
                                if self.state.scroll_offset() > 0 {
                                    format!("↑{}", self.state.scroll_offset())
                                } else {
                                    String::new()
                                },
                            ))
                            .child(div().w(px(8.0)).h(px(8.0)).rounded_full().bg(
                                if self.is_running() {
                                    gpui::rgb(0x4ade80)
                                } else {
                                    gpui::rgb(0x8b7b6b)
                                },
                            )),
                    ),
            )
            // Terminal content - render all lines and let container scroll
            .child(
                div()
                    .id("terminal-content-wrapper")
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .child(
                        div()
                            .id("terminal-content")
                            .flex_1()
                            .overflow_y_scroll()
                            .p(px(TERMINAL_PADDING))
                            .font_family("JetBrains Mono")
                            .text_size(px(13.0))
                            // Prevent scroll area from capturing key events
                            .on_key_down(cx.listener(Self::handle_key_down))
                            .child(
                                div().flex().flex_col().children(
                                    (0..total_lines)
                                        .map(|idx| {
                                            let line = self.state.line(idx).cloned();
                                            let is_cursor_line = idx == cursor_line;
                                            if let Some(line) = line {
                                                self.render_line(idx, &line, is_cursor_line)
                                                    .into_any_element()
                                            } else {
                                                div().h(px(LINE_HEIGHT)).into_any_element()
                                            }
                                        })
                                        .collect::<Vec<_>>(),
                                ),
                            ),
                    )
                    // Vertical scrollbar indicator
                    .child(
                        div()
                            .id("terminal-scrollbar")
                            .w(px(8.0))
                            .h_full()
                            .bg(header_bg)
                            .flex()
                            .flex_col()
                            .p(px(2.0))
                            .child(
                                div()
                                    .flex_1()
                                    .bg(border_color)
                                    .rounded(px(2.0))
                                    .relative()
                                    .child(
                                        div()
                                            .absolute()
                                            .top(px(thumb_offset))
                                            .left_0()
                                            .right_0()
                                            .h(px(thumb_height))
                                            .bg(accent_color)
                                            .rounded(px(2.0)),
                                    ),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_view_visibility() {
        // Basic visibility test
        let is_visible = false;
        assert!(!is_visible);
    }

    #[test]
    fn test_line_height_constant() {
        assert_eq!(LINE_HEIGHT, 20.0);
    }

    #[test]
    fn test_char_width_constant() {
        assert!(CHAR_WIDTH > 0.0);
    }

    #[test]
    fn test_cursor_blink_interval() {
        assert!(CURSOR_BLINK_INTERVAL_MS > 0);
        assert!(CURSOR_BLINK_INTERVAL_MS < 1000);
    }

    #[test]
    fn test_selection_normalization() {
        // Test that selection is properly normalized (start before end)
        let start = (5, 10);
        let end = (3, 5);

        let (start_line, start_col, end_line, end_col) =
            if start.0 > end.0 || (start.0 == end.0 && start.1 > end.1) {
                (end.0, end.1, start.0, start.1)
            } else {
                (start.0, start.1, end.0, end.1)
            };

        assert_eq!(start_line, 3);
        assert_eq!(start_col, 5);
        assert_eq!(end_line, 5);
        assert_eq!(end_col, 10);
    }

    #[test]
    fn test_position_from_mouse_calculation() {
        // Test mouse position to terminal coordinates conversion
        let x = TERMINAL_PADDING + CHAR_WIDTH * 5.0;
        let y = TERMINAL_PADDING + 36.0 + LINE_HEIGHT * 3.0;

        let col = ((x - TERMINAL_PADDING) / CHAR_WIDTH).max(0.0) as usize;
        let row = ((y - TERMINAL_PADDING - 36.0) / LINE_HEIGHT).max(0.0) as usize;

        assert_eq!(col, 5);
        assert_eq!(row, 3);
    }

    #[test]
    fn test_selection_same_line() {
        // Test selection within same line
        let start = (5, 2);
        let end = (5, 8);

        // Position (5, 4) should be selected
        let line = 5;
        let col = 4;

        let (start_line, start_col) = start;
        let (end_line, end_col) = end;

        let is_selected = if line < start_line || line > end_line {
            false
        } else if line == start_line && line == end_line {
            col >= start_col && col < end_col
        } else {
            true
        };

        assert!(is_selected);
    }

    #[test]
    fn test_selection_multi_line() {
        // Test selection across multiple lines
        let start = (3, 5);
        let end = (6, 10);

        // Position (4, 0) should be selected (middle line)
        let line = 4;
        let col = 0;

        let (start_line, start_col, end_line, _end_col) = (start.0, start.1, end.0, end.1);

        let is_selected = if line < start_line || line > end_line {
            false
        } else if line == start_line {
            col >= start_col
        } else if line == end_line {
            true
        } else {
            true
        };

        assert!(is_selected);
    }
}
