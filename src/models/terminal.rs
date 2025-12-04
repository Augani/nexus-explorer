use gpui::Rgba;
use std::path::PathBuf;

/// Default terminal dimensions
pub const DEFAULT_COLS: usize = 80;
pub const DEFAULT_ROWS: usize = 24;
pub const DEFAULT_SCROLLBACK: usize = 10000;

/// Style for a terminal cell
#[derive(Clone, Debug, PartialEq)]
pub struct CellStyle {
    pub foreground: Rgba,
    pub background: Rgba,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub inverse: bool,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            foreground: Rgba { r: 0.96, g: 0.91, b: 0.86, a: 1.0 },
            background: Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 },
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            dim: false,
            inverse: false,
        }
    }
}

impl CellStyle {
    pub fn with_foreground(mut self, color: Rgba) -> Self {
        self.foreground = color;
        self
    }

    pub fn with_background(mut self, color: Rgba) -> Self {
        self.background = color;
        self
    }

    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    pub fn with_underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }
}


/// A single cell in the terminal grid
#[derive(Clone, Debug)]
pub struct TerminalCell {
    pub char: char,
    pub style: CellStyle,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            char: ' ',
            style: CellStyle::default(),
        }
    }
}

impl TerminalCell {
    pub fn new(char: char, style: CellStyle) -> Self {
        Self { char, style }
    }

    pub fn with_char(char: char) -> Self {
        Self {
            char,
            style: CellStyle::default(),
        }
    }
}

/// A line of terminal cells
#[derive(Clone, Debug)]
pub struct TerminalLine {
    pub cells: Vec<TerminalCell>,
    pub wrapped: bool,
}

impl TerminalLine {
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![TerminalCell::default(); cols],
            wrapped: false,
        }
    }

    pub fn with_capacity(cols: usize) -> Self {
        let mut cells = Vec::with_capacity(cols);
        cells.resize_with(cols, TerminalCell::default);
        Self {
            cells,
            wrapped: false,
        }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn get(&self, col: usize) -> Option<&TerminalCell> {
        self.cells.get(col)
    }

    pub fn get_mut(&mut self, col: usize) -> Option<&mut TerminalCell> {
        self.cells.get_mut(col)
    }

    pub fn set(&mut self, col: usize, cell: TerminalCell) {
        if col < self.cells.len() {
            self.cells[col] = cell;
        }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = TerminalCell::default();
        }
        self.wrapped = false;
    }

    pub fn clear_from(&mut self, col: usize) {
        for i in col..self.cells.len() {
            self.cells[i] = TerminalCell::default();
        }
    }

    pub fn clear_to(&mut self, col: usize) {
        let end = col.min(self.cells.len());
        for i in 0..end {
            self.cells[i] = TerminalCell::default();
        }
    }

    /// Get the text content of this line (trimmed)
    pub fn text(&self) -> String {
        let mut s: String = self.cells.iter().map(|c| c.char).collect();
        s.truncate(s.trim_end().len());
        s
    }

    /// Resize the line to a new column count
    pub fn resize(&mut self, cols: usize) {
        self.cells.resize_with(cols, TerminalCell::default);
    }
}


/// Cursor position in the terminal
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorPosition {
    pub row: usize,
    pub col: usize,
}

impl CursorPosition {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    pub fn origin() -> Self {
        Self { row: 0, col: 0 }
    }
}

/// Terminal state holding all lines, cursor, and configuration
#[derive(Clone, Debug)]
pub struct TerminalState {
    /// All lines including scrollback buffer
    lines: Vec<TerminalLine>,
    /// Current cursor position (relative to viewport, not scrollback)
    cursor: CursorPosition,
    /// Number of columns
    cols: usize,
    /// Number of visible rows
    rows: usize,
    /// Scroll offset from bottom (0 = at bottom)
    scroll_offset: usize,
    /// Maximum scrollback lines
    max_scrollback: usize,
    /// Working directory for the terminal
    working_directory: PathBuf,
    /// Whether the terminal process is running
    is_running: bool,
    /// Current style for new characters
    current_style: CellStyle,
    /// Whether cursor is visible
    cursor_visible: bool,
    /// Saved cursor position (for save/restore)
    saved_cursor: Option<CursorPosition>,
    /// Title set by escape sequences
    title: Option<String>,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self::new(DEFAULT_COLS, DEFAULT_ROWS)
    }
}

impl TerminalState {
    pub fn new(cols: usize, rows: usize) -> Self {
        let mut lines = Vec::with_capacity(rows);
        for _ in 0..rows {
            lines.push(TerminalLine::new(cols));
        }

        Self {
            lines,
            cursor: CursorPosition::origin(),
            cols,
            rows,
            scroll_offset: 0,
            max_scrollback: DEFAULT_SCROLLBACK,
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            is_running: false,
            current_style: CellStyle::default(),
            cursor_visible: true,
            saved_cursor: None,
            title: None,
        }
    }

    pub fn with_working_directory(mut self, path: PathBuf) -> Self {
        self.working_directory = path;
        self
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cursor(&self) -> CursorPosition {
        self.cursor
    }

    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn working_directory(&self) -> &PathBuf {
        &self.working_directory
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn current_style(&self) -> &CellStyle {
        &self.current_style
    }

    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    pub fn scrollback_lines(&self) -> usize {
        self.lines.len().saturating_sub(self.rows)
    }

    pub fn max_scroll_offset(&self) -> usize {
        self.scrollback_lines()
    }


    pub fn set_running(&mut self, running: bool) {
        self.is_running = running;
    }

    pub fn set_working_directory(&mut self, path: PathBuf) {
        self.working_directory = path;
    }

    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }

    pub fn set_current_style(&mut self, style: CellStyle) {
        self.current_style = style;
    }

    pub fn reset_style(&mut self) {
        self.current_style = CellStyle::default();
    }

    /// Get a line by absolute index (including scrollback)
    pub fn line(&self, index: usize) -> Option<&TerminalLine> {
        self.lines.get(index)
    }

    /// Get a mutable line by absolute index
    pub fn line_mut(&mut self, index: usize) -> Option<&mut TerminalLine> {
        self.lines.get_mut(index)
    }

    /// Get visible lines based on current scroll offset
    pub fn visible_lines(&self) -> impl Iterator<Item = &TerminalLine> {
        let total = self.lines.len();
        let start = total.saturating_sub(self.rows + self.scroll_offset);
        let end = total.saturating_sub(self.scroll_offset);
        self.lines[start..end].iter()
    }

    /// Get the absolute line index for a viewport row
    fn viewport_to_absolute(&self, row: usize) -> usize {
        let total = self.lines.len();
        total.saturating_sub(self.rows) + row
    }

    /// Get the line at the current cursor position
    fn current_line_mut(&mut self) -> &mut TerminalLine {
        let idx = self.viewport_to_absolute(self.cursor.row);
        // Ensure we have enough lines
        while self.lines.len() <= idx {
            self.lines.push(TerminalLine::new(self.cols));
        }
        &mut self.lines[idx]
    }

    /// Write a character at the current cursor position
    pub fn write_char(&mut self, c: char) {
        if self.cursor.col >= self.cols {
            self.newline();
        }

        let style = self.current_style.clone();
        let col = self.cursor.col;
        let line = self.current_line_mut();
        line.set(col, TerminalCell::new(c, style));
        self.cursor.col += 1;
    }

    /// Write a string at the current cursor position
    pub fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '\n' => self.newline(),
                '\r' => self.carriage_return(),
                '\t' => self.tab(),
                '\x08' => self.backspace(),
                '\x07' => {}
                c if c.is_control() => {}
                c => self.write_char(c),
            }
        }
    }

    /// Move to a new line
    pub fn newline(&mut self) {
        self.cursor.col = 0;
        if self.cursor.row + 1 >= self.rows {
            self.scroll_up();
        } else {
            self.cursor.row += 1;
        }
    }

    /// Carriage return - move cursor to beginning of line
    pub fn carriage_return(&mut self) {
        self.cursor.col = 0;
    }

    /// Tab - move to next tab stop (every 8 columns)
    pub fn tab(&mut self) {
        let next_tab = ((self.cursor.col / 8) + 1) * 8;
        self.cursor.col = next_tab.min(self.cols - 1);
    }

    /// Backspace - move cursor back one position
    pub fn backspace(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }


    /// Scroll the terminal up by one line
    pub fn scroll_up(&mut self) {
        // Add a new line at the bottom
        self.lines.push(TerminalLine::new(self.cols));

        // Trim scrollback if needed
        while self.lines.len() > self.rows + self.max_scrollback {
            self.lines.remove(0);
        }
    }

    /// Scroll the terminal down by one line (reverse scroll)
    pub fn scroll_down(&mut self) {
        let insert_idx = self.lines.len().saturating_sub(self.rows);
        self.lines.insert(insert_idx, TerminalLine::new(self.cols));

        // Trim scrollback if needed
        while self.lines.len() > self.rows + self.max_scrollback {
            self.lines.remove(0);
        }
    }

    /// Move cursor to absolute position
    pub fn move_cursor_to(&mut self, row: usize, col: usize) {
        self.cursor.row = row.min(self.rows.saturating_sub(1));
        self.cursor.col = col.min(self.cols.saturating_sub(1));
    }

    /// Move cursor relative to current position
    pub fn move_cursor_by(&mut self, row_delta: i32, col_delta: i32) {
        let new_row = (self.cursor.row as i32 + row_delta).max(0) as usize;
        let new_col = (self.cursor.col as i32 + col_delta).max(0) as usize;
        self.move_cursor_to(new_row, new_col);
    }

    /// Move cursor up
    pub fn cursor_up(&mut self, n: usize) {
        self.cursor.row = self.cursor.row.saturating_sub(n);
    }

    /// Move cursor down
    pub fn cursor_down(&mut self, n: usize) {
        self.cursor.row = (self.cursor.row + n).min(self.rows.saturating_sub(1));
    }

    /// Move cursor forward (right)
    pub fn cursor_forward(&mut self, n: usize) {
        self.cursor.col = (self.cursor.col + n).min(self.cols.saturating_sub(1));
    }

    /// Move cursor backward (left)
    pub fn cursor_backward(&mut self, n: usize) {
        self.cursor.col = self.cursor.col.saturating_sub(n);
    }

    /// Save cursor position
    pub fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor);
    }

    /// Restore cursor position
    pub fn restore_cursor(&mut self) {
        if let Some(pos) = self.saved_cursor {
            self.cursor = pos;
        }
    }

    /// Clear the entire screen
    pub fn clear_screen(&mut self) {
        for line in &mut self.lines {
            line.clear();
        }
        self.cursor = CursorPosition::origin();
    }

    /// Clear from cursor to end of screen
    pub fn clear_to_end_of_screen(&mut self) {
        // Clear current line from cursor
        let idx = self.viewport_to_absolute(self.cursor.row);
        if let Some(line) = self.lines.get_mut(idx) {
            line.clear_from(self.cursor.col);
        }

        // Clear all lines below
        for i in (idx + 1)..self.lines.len() {
            self.lines[i].clear();
        }
    }

    /// Clear from beginning of screen to cursor
    pub fn clear_to_start_of_screen(&mut self) {
        let idx = self.viewport_to_absolute(self.cursor.row);

        // Clear all lines above
        let start = self.lines.len().saturating_sub(self.rows);
        for i in start..idx {
            self.lines[i].clear();
        }

        // Clear current line to cursor
        if let Some(line) = self.lines.get_mut(idx) {
            line.clear_to(self.cursor.col + 1);
        }
    }

    /// Clear the current line
    pub fn clear_line(&mut self) {
        let idx = self.viewport_to_absolute(self.cursor.row);
        if let Some(line) = self.lines.get_mut(idx) {
            line.clear();
        }
    }

    /// Clear from cursor to end of line
    pub fn clear_to_end_of_line(&mut self) {
        let idx = self.viewport_to_absolute(self.cursor.row);
        if let Some(line) = self.lines.get_mut(idx) {
            line.clear_from(self.cursor.col);
        }
    }

    /// Clear from beginning of line to cursor
    pub fn clear_to_start_of_line(&mut self) {
        let idx = self.viewport_to_absolute(self.cursor.row);
        if let Some(line) = self.lines.get_mut(idx) {
            line.clear_to(self.cursor.col + 1);
        }
    }


    /// Scroll viewport up (view older content)
    pub fn scroll_viewport_up(&mut self, lines: usize) {
        let max = self.max_scroll_offset();
        self.scroll_offset = (self.scroll_offset + lines).min(max);
    }

    /// Scroll viewport down (view newer content)
    pub fn scroll_viewport_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Scroll viewport to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Scroll viewport to top
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = self.max_scroll_offset();
    }

    /// Check if viewport is at bottom
    pub fn is_at_bottom(&self) -> bool {
        self.scroll_offset == 0
    }

    /// Resize the terminal
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;

        // Resize all existing lines
        for line in &mut self.lines {
            line.resize(cols);
        }

        // Ensure we have at least `rows` lines
        while self.lines.len() < rows {
            self.lines.push(TerminalLine::new(cols));
        }

        // Clamp cursor position
        self.cursor.row = self.cursor.row.min(rows.saturating_sub(1));
        self.cursor.col = self.cursor.col.min(cols.saturating_sub(1));

        self.scroll_offset = self.scroll_offset.min(self.max_scroll_offset());
    }

    /// Reset the terminal to initial state
    pub fn reset(&mut self) {
        self.lines.clear();
        for _ in 0..self.rows {
            self.lines.push(TerminalLine::new(self.cols));
        }
        self.cursor = CursorPosition::origin();
        self.scroll_offset = 0;
        self.current_style = CellStyle::default();
        self.cursor_visible = true;
        self.saved_cursor = None;
        self.title = None;
    }

    /// Get all content as a string (for debugging/testing)
    pub fn content_as_string(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get visible content as a string
    pub fn visible_content_as_string(&self) -> String {
        self.visible_lines()
            .map(|line| line.text())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_state_creation() {
        let state = TerminalState::new(80, 24);
        assert_eq!(state.cols(), 80);
        assert_eq!(state.rows(), 24);
        assert_eq!(state.cursor(), CursorPosition::origin());
        assert!(state.cursor_visible());
        assert!(!state.is_running());
    }

    #[test]
    fn test_write_char() {
        let mut state = TerminalState::new(80, 24);
        state.write_char('H');
        state.write_char('i');
        assert_eq!(state.cursor().col, 2);
        assert_eq!(state.cursor().row, 0);
    }

    #[test]
    fn test_write_str() {
        let mut state = TerminalState::new(80, 24);
        state.write_str("Hello");
        assert_eq!(state.cursor().col, 5);
    }

    #[test]
    fn test_newline() {
        let mut state = TerminalState::new(80, 24);
        state.write_str("Line 1");
        state.newline();
        state.write_str("Line 2");
        assert_eq!(state.cursor().row, 1);
        assert_eq!(state.cursor().col, 6);
    }

    #[test]
    fn test_cursor_movement() {
        let mut state = TerminalState::new(80, 24);
        state.move_cursor_to(5, 10);
        assert_eq!(state.cursor(), CursorPosition::new(5, 10));

        state.cursor_up(2);
        assert_eq!(state.cursor().row, 3);

        state.cursor_down(1);
        assert_eq!(state.cursor().row, 4);

        state.cursor_forward(5);
        assert_eq!(state.cursor().col, 15);

        state.cursor_backward(3);
        assert_eq!(state.cursor().col, 12);
    }

    #[test]
    fn test_clear_screen() {
        let mut state = TerminalState::new(80, 24);
        state.write_str("Some content");
        state.clear_screen();
        assert_eq!(state.cursor(), CursorPosition::origin());
    }

    #[test]
    fn test_scroll() {
        let mut state = TerminalState::new(80, 5);
        for i in 0..10 {
            state.write_str(&format!("Line {}", i));
            state.newline();
        }
        assert!(state.scrollback_lines() > 0);
    }

    #[test]
    fn test_resize() {
        let mut state = TerminalState::new(80, 24);
        state.write_str("Test");
        state.resize(40, 12);
        assert_eq!(state.cols(), 40);
        assert_eq!(state.rows(), 12);
    }

    #[test]
    fn test_cell_style() {
        let style = CellStyle::default()
            .with_bold(true)
            .with_foreground(Rgba { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        assert!(style.bold);
        assert_eq!(style.foreground.r, 1.0);
    }
}
