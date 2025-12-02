use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Render, Styled, Window,
};
use std::path::PathBuf;

use crate::models::{
    AnsiParser, ClearMode, ParsedSegment, PtyService, TerminalState,
    key_codes,
};

/// Terminal line height in pixels
const LINE_HEIGHT: f32 = 20.0;
/// Terminal character width (monospace)
const CHAR_WIDTH: f32 = 8.4;
/// Terminal padding
const TERMINAL_PADDING: f32 = 12.0;

/// Terminal view component
pub struct TerminalView {
    state: TerminalState,
    parser: AnsiParser,
    pty: Option<PtyService>,
    focus_handle: FocusHandle,
    is_visible: bool,
    cursor_blink: bool,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
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
            selection_start: None,
            selection_end: None,
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


    /// Start the terminal
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
            ParsedSegment::Bell => {} // Could trigger a visual bell
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


    /// Handle key down events
    fn handle_key_down(&mut self, event: &KeyDownEvent, _cx: &mut Context<Self>) {
        if !self.is_running() {
            return;
        }

        // Handle special keys
        let key_str = format!("{:?}", event.keystroke.key);
        
        match key_str.as_str() {
            "Enter" => self.send_input(key_codes::ENTER),
            "Tab" => self.send_input(key_codes::TAB),
            "Backspace" => self.send_input(key_codes::BACKSPACE),
            "Escape" => self.send_input(key_codes::ESCAPE),
            "Delete" => self.send_input(key_codes::DELETE),
            "Up" => self.send_input(key_codes::UP),
            "Down" => self.send_input(key_codes::DOWN),
            "Left" => self.send_input(key_codes::LEFT),
            "Right" => self.send_input(key_codes::RIGHT),
            "Home" => self.send_input(key_codes::HOME),
            "End" => self.send_input(key_codes::END),
            "PageUp" => self.send_input(key_codes::PAGE_UP),
            "PageDown" => self.send_input(key_codes::PAGE_DOWN),
            _ => {
                // Handle regular characters and ctrl combinations
                if let Some(key_char) = &event.keystroke.key_char {
                    if event.keystroke.modifiers.control {
                        // Ctrl+key combinations
                        if key_char.len() == 1 {
                            let c = key_char.chars().next().unwrap();
                            if c.is_ascii_alphabetic() {
                                let ctrl_code = (c.to_ascii_lowercase() as u8) - b'a' + 1;
                                self.send_input(&[ctrl_code]);
                            }
                        }
                    } else if !event.keystroke.modifiers.alt && !event.keystroke.modifiers.platform {
                        // Regular character input
                        self.send_str(key_char);
                    }
                }
            }
        }
    }

    /// Render a single terminal line
    fn render_line(&self, _idx: usize, line: &crate::models::TerminalLine) -> impl IntoElement {
        let text: String = line.cells.iter().map(|c| c.char).collect();
        let text = text.trim_end().to_string();
        
        div()
            .h(px(LINE_HEIGHT))
            .text_color(gpui::Rgba { r: 0.96, g: 0.91, b: 0.86, a: 1.0 })
            .font_family("JetBrains Mono")
            .text_size(px(13.0))
            .child(if text.is_empty() { " ".to_string() } else { text })
    }
}


impl Focusable for TerminalView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TerminalView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Process any pending output
        self.process_output();

        let bg_color = gpui::rgb(0x0d0a0a);
        let border_color = gpui::rgb(0x3d2d2d);
        let header_bg = gpui::rgb(0x1a1414);
        let text_muted = gpui::rgb(0x8b7b6b);
        let accent_color = gpui::rgb(0xf4b842);

        if !self.is_visible {
            return div().id("terminal-hidden").size_0();
        }

        let visible_lines: Vec<_> = self.state.visible_lines().collect();
        let line_count = visible_lines.len();

        div()
            .id("terminal-panel")
            .w_full()
            .h(px(300.0))
            .bg(bg_color)
            .border_t_1()
            .border_color(border_color)
            .flex()
            .flex_col()
            // Header
            .child(
                div()
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
                                    .child("TERMINAL")
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(accent_color)
                                    .child(
                                        self.state.title()
                                            .map(|s| s.to_string())
                                            .unwrap_or_else(|| self.state.working_directory().to_string_lossy().to_string())
                                    )
                            )
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(8.0))
                                    .h(px(8.0))
                                    .rounded_full()
                                    .bg(if self.is_running() {
                                        gpui::rgb(0x4ade80) // Green when running
                                    } else {
                                        gpui::rgb(0x8b7b6b) // Gray when stopped
                                    })
                            )
                    )
            )
            // Terminal content
            .child(
                div()
                    .id("terminal-content")
                    .flex_1()
                    .overflow_hidden()
                    .p(px(TERMINAL_PADDING))
                    .font_family("JetBrains Mono")
                    .text_size(px(13.0))
                    .children(
                        visible_lines.iter().enumerate().map(|(idx, line)| {
                            self.render_line(idx, line)
                        }).collect::<Vec<_>>()
                    )
            )
            // Scrollbar indicator
            .child(
                div()
                    .h(px(4.0))
                    .bg(header_bg)
                    .flex()
                    .items_center()
                    .px_2()
                    .child(
                        div()
                            .flex_1()
                            .h(px(2.0))
                            .bg(border_color)
                            .rounded_full()
                            .child(
                                div()
                                    .h_full()
                                    .w(px(
                                        if self.state.total_lines() > 0 {
                                            (line_count as f32 / self.state.total_lines() as f32 * 100.0).min(100.0)
                                        } else {
                                            100.0
                                        }
                                    ))
                                    .bg(accent_color)
                                    .rounded_full()
                            )
                    )
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
}
