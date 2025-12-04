use crate::models::terminal::CellStyle;
use gpui::Rgba;

/// Standard ANSI color palette (16 colors)
pub const ANSI_COLORS: [Rgba; 16] = [
    // Normal colors (0-7)
    Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    },
    Rgba {
        r: 0.8,
        g: 0.2,
        b: 0.2,
        a: 1.0,
    },
    Rgba {
        r: 0.2,
        g: 0.8,
        b: 0.2,
        a: 1.0,
    },
    Rgba {
        r: 0.8,
        g: 0.8,
        b: 0.2,
        a: 1.0,
    },
    Rgba {
        r: 0.2,
        g: 0.4,
        b: 0.8,
        a: 1.0,
    },
    Rgba {
        r: 0.8,
        g: 0.2,
        b: 0.8,
        a: 1.0,
    },
    Rgba {
        r: 0.2,
        g: 0.8,
        b: 0.8,
        a: 1.0,
    },
    Rgba {
        r: 0.8,
        g: 0.8,
        b: 0.8,
        a: 1.0,
    },
    // Bright colors (8-15)
    Rgba {
        r: 0.4,
        g: 0.4,
        b: 0.4,
        a: 1.0,
    },
    Rgba {
        r: 1.0,
        g: 0.4,
        b: 0.4,
        a: 1.0,
    },
    Rgba {
        r: 0.4,
        g: 1.0,
        b: 0.4,
        a: 1.0,
    },
    Rgba {
        r: 1.0,
        g: 1.0,
        b: 0.4,
        a: 1.0,
    },
    Rgba {
        r: 0.4,
        g: 0.6,
        b: 1.0,
        a: 1.0,
    },
    Rgba {
        r: 1.0,
        g: 0.4,
        b: 1.0,
        a: 1.0,
    },
    Rgba {
        r: 0.4,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    },
    Rgba {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    },
];

/// Default foreground color
pub const DEFAULT_FG: Rgba = Rgba {
    r: 0.96,
    g: 0.91,
    b: 0.86,
    a: 1.0,
};
/// Default background color (transparent)
pub const DEFAULT_BG: Rgba = Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

/// Parser state machine states
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParserState {
    Ground,
    Escape,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    CsiPrivate,
    OscString,
}

/// Parsed segment from ANSI input
#[derive(Clone, Debug, PartialEq)]
pub enum ParsedSegment {
    Text(String, CellStyle),
    CursorUp(usize),
    CursorDown(usize),
    CursorForward(usize),
    CursorBackward(usize),
    CursorPosition(usize, usize),
    CursorSave,
    CursorRestore,
    ClearScreen(ClearMode),
    ClearLine(ClearMode),
    SetTitle(String),
    Bell,
    Backspace,
    Tab,
    LineFeed,
    CarriageReturn,
    ScrollUp(usize),
    ScrollDown(usize),
}

/// Clear mode for screen/line clearing
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClearMode {
    ToEnd,
    ToStart,
    All,
    Scrollback,
}

impl ClearMode {
    fn from_param(param: usize) -> Self {
        match param {
            0 => ClearMode::ToEnd,
            1 => ClearMode::ToStart,
            2 => ClearMode::All,
            3 => ClearMode::Scrollback,
            _ => ClearMode::ToEnd,
        }
    }
}

/// ANSI escape sequence parser
pub struct AnsiParser {
    state: ParserState,
    params: Vec<u16>,
    intermediate: Vec<u8>,
    osc_string: String,
    current_style: CellStyle,
    default_fg: Rgba,
    default_bg: Rgba,
    utf8_buffer: Vec<u8>,
    utf8_remaining: usize,
}

impl Default for AnsiParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AnsiParser {
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            params: Vec::with_capacity(16),
            intermediate: Vec::with_capacity(4),
            osc_string: String::new(),
            current_style: CellStyle::default(),
            default_fg: DEFAULT_FG,
            default_bg: DEFAULT_BG,
            utf8_buffer: Vec::with_capacity(4),
            utf8_remaining: 0,
        }
    }

    pub fn with_colors(default_fg: Rgba, default_bg: Rgba) -> Self {
        let mut parser = Self::new();
        parser.default_fg = default_fg;
        parser.default_bg = default_bg;
        parser.current_style.foreground = default_fg;
        parser
    }

    pub fn current_style(&self) -> &CellStyle {
        &self.current_style
    }

    pub fn reset(&mut self) {
        self.state = ParserState::Ground;
        self.params.clear();
        self.intermediate.clear();
        self.osc_string.clear();
        self.utf8_buffer.clear();
        self.utf8_remaining = 0;
        self.current_style = CellStyle {
            foreground: self.default_fg,
            background: self.default_bg,
            ..CellStyle::default()
        };
    }

    /// Parse input bytes and return parsed segments
    pub fn parse(&mut self, input: &[u8]) -> Vec<ParsedSegment> {
        let mut segments = Vec::new();
        let mut text_buffer = String::new();

        for &byte in input {
            match self.state {
                ParserState::Ground => {
                    self.handle_ground(byte, &mut text_buffer, &mut segments);
                }
                ParserState::Escape => {
                    self.handle_escape(byte, &mut text_buffer, &mut segments);
                }
                ParserState::CsiEntry | ParserState::CsiParam => {
                    self.handle_csi(byte, &mut text_buffer, &mut segments);
                }
                ParserState::CsiIntermediate => {
                    self.handle_csi_intermediate(byte, &mut text_buffer, &mut segments);
                }
                ParserState::CsiPrivate => {
                    self.handle_csi_private(byte, &mut text_buffer, &mut segments);
                }
                ParserState::OscString => {
                    self.handle_osc(byte, &mut segments);
                }
            }
        }

        // Flush remaining text
        if !text_buffer.is_empty() {
            segments.push(ParsedSegment::Text(
                std::mem::take(&mut text_buffer),
                self.current_style.clone(),
            ));
        }

        segments
    }

    fn handle_ground(
        &mut self,
        byte: u8,
        text_buffer: &mut String,
        segments: &mut Vec<ParsedSegment>,
    ) {
        if self.utf8_remaining > 0 {
            if (byte & 0xC0) == 0x80 {
                // Valid continuation byte
                self.utf8_buffer.push(byte);
                self.utf8_remaining -= 1;
                if self.utf8_remaining == 0 {
                    // Complete UTF-8 sequence
                    if let Ok(s) = std::str::from_utf8(&self.utf8_buffer) {
                        text_buffer.push_str(s);
                    }
                    self.utf8_buffer.clear();
                }
                return;
            } else {
                self.utf8_buffer.clear();
                self.utf8_remaining = 0;
            }
        }

        match byte {
            0x1B => {
                // ESC - start escape sequence
                if !text_buffer.is_empty() {
                    segments.push(ParsedSegment::Text(
                        std::mem::take(text_buffer),
                        self.current_style.clone(),
                    ));
                }
                self.state = ParserState::Escape;
            }
            0x07 => {
                if !text_buffer.is_empty() {
                    segments.push(ParsedSegment::Text(
                        std::mem::take(text_buffer),
                        self.current_style.clone(),
                    ));
                }
                segments.push(ParsedSegment::Bell);
            }
            0x08 => {
                if !text_buffer.is_empty() {
                    segments.push(ParsedSegment::Text(
                        std::mem::take(text_buffer),
                        self.current_style.clone(),
                    ));
                }
                segments.push(ParsedSegment::Backspace);
            }
            0x09 => {
                if !text_buffer.is_empty() {
                    segments.push(ParsedSegment::Text(
                        std::mem::take(text_buffer),
                        self.current_style.clone(),
                    ));
                }
                segments.push(ParsedSegment::Tab);
            }
            0x0A => {
                if !text_buffer.is_empty() {
                    segments.push(ParsedSegment::Text(
                        std::mem::take(text_buffer),
                        self.current_style.clone(),
                    ));
                }
                segments.push(ParsedSegment::LineFeed);
            }
            0x0D => {
                if !text_buffer.is_empty() {
                    segments.push(ParsedSegment::Text(
                        std::mem::take(text_buffer),
                        self.current_style.clone(),
                    ));
                }
                segments.push(ParsedSegment::CarriageReturn);
            }
            0x00..=0x1F => {
                // Other control characters - ignore
            }
            0x20..=0x7F => {
                // ASCII printable character
                text_buffer.push(byte as char);
            }
            0xC0..=0xDF => {
                // Start of 2-byte UTF-8 sequence
                self.utf8_buffer.clear();
                self.utf8_buffer.push(byte);
                self.utf8_remaining = 1;
            }
            0xE0..=0xEF => {
                // Start of 3-byte UTF-8 sequence
                self.utf8_buffer.clear();
                self.utf8_buffer.push(byte);
                self.utf8_remaining = 2;
            }
            0xF0..=0xF7 => {
                // Start of 4-byte UTF-8 sequence
                self.utf8_buffer.clear();
                self.utf8_buffer.push(byte);
                self.utf8_remaining = 3;
            }
            _ => {
                // Invalid byte - ignore
            }
        }
    }

    fn handle_escape(
        &mut self,
        byte: u8,
        text_buffer: &mut String,
        segments: &mut Vec<ParsedSegment>,
    ) {
        match byte {
            b'[' => {
                self.state = ParserState::CsiEntry;
                self.params.clear();
                self.intermediate.clear();
            }
            b']' => {
                self.state = ParserState::OscString;
                self.osc_string.clear();
            }
            b'7' => {
                segments.push(ParsedSegment::CursorSave);
                self.state = ParserState::Ground;
            }
            b'8' => {
                segments.push(ParsedSegment::CursorRestore);
                self.state = ParserState::Ground;
            }
            b'D' => {
                // Index (scroll up)
                segments.push(ParsedSegment::ScrollUp(1));
                self.state = ParserState::Ground;
            }
            b'M' => {
                // Reverse index (scroll down)
                segments.push(ParsedSegment::ScrollDown(1));
                self.state = ParserState::Ground;
            }
            b'c' => {
                self.reset();
                self.state = ParserState::Ground;
            }
            _ => {
                // Unknown escape sequence - output as text
                text_buffer.push('\x1B');
                if let Some(c) = char::from_u32(byte as u32) {
                    text_buffer.push(c);
                }
                self.state = ParserState::Ground;
            }
        }
    }

    fn handle_csi(
        &mut self,
        byte: u8,
        text_buffer: &mut String,
        segments: &mut Vec<ParsedSegment>,
    ) {
        match byte {
            b'?' => {
                // DEC private mode - switch to private mode parsing
                self.state = ParserState::CsiPrivate;
            }
            b'>' | b'=' | b'!' => {
                // Other CSI modifiers - switch to private mode (ignore these sequences)
                self.state = ParserState::CsiPrivate;
            }
            b'0'..=b'9' => {
                // Parameter digit
                self.state = ParserState::CsiParam;
                let digit = (byte - b'0') as u16;
                if let Some(last) = self.params.last_mut() {
                    *last = last.saturating_mul(10).saturating_add(digit);
                } else {
                    self.params.push(digit);
                }
            }
            b';' => {
                // Parameter separator
                self.state = ParserState::CsiParam;
                if self.params.is_empty() {
                    self.params.push(0);
                }
                self.params.push(0);
            }
            b' '..=b'/' => {
                // Intermediate byte
                self.intermediate.push(byte);
                self.state = ParserState::CsiIntermediate;
            }
            b'@'..=b'~' => {
                // Final byte - execute CSI sequence
                self.execute_csi(byte, segments);
                self.state = ParserState::Ground;
            }
            _ => {
                // Invalid - abort silently (don't output garbage)
                self.state = ParserState::Ground;
            }
        }
    }

    fn handle_csi_private(
        &mut self,
        byte: u8,
        _text_buffer: &mut String,
        _segments: &mut Vec<ParsedSegment>,
    ) {
        match byte {
            b'0'..=b'9' | b';' => {
                // Continue consuming parameters
            }
            b'@'..=b'~' => {
                // Final byte - sequence complete, ignore it
                self.state = ParserState::Ground;
            }
            _ => {
                // Invalid - abort
                self.state = ParserState::Ground;
            }
        }
    }

    fn handle_csi_intermediate(
        &mut self,
        byte: u8,
        text_buffer: &mut String,
        segments: &mut Vec<ParsedSegment>,
    ) {
        match byte {
            b' '..=b'/' => {
                self.intermediate.push(byte);
            }
            b'@'..=b'~' => {
                self.execute_csi(byte, segments);
                self.state = ParserState::Ground;
            }
            _ => {
                text_buffer.push_str("\x1B[");
                self.state = ParserState::Ground;
            }
        }
    }

    fn handle_osc(&mut self, byte: u8, segments: &mut Vec<ParsedSegment>) {
        match byte {
            0x07 | 0x9C => {
                // BEL or ST - end of OSC
                self.execute_osc(segments);
                self.state = ParserState::Ground;
            }
            0x1B => {
                // Might be ST (\x1B\\)
                // For simplicity, treat as end
                self.execute_osc(segments);
                self.state = ParserState::Ground;
            }
            0x20..=0x7E => {
                // Printable ASCII
                self.osc_string.push(byte as char);
            }
            _ => {
                // For non-ASCII in OSC, just skip (titles should be ASCII anyway)
            }
        }
    }

    fn execute_osc(&mut self, segments: &mut Vec<ParsedSegment>) {
        // Parse OSC command
        if let Some(idx) = self.osc_string.find(';') {
            let cmd = &self.osc_string[..idx];
            let arg = &self.osc_string[idx + 1..];
            match cmd {
                "0" | "2" => {
                    segments.push(ParsedSegment::SetTitle(arg.to_string()));
                }
                _ => {}
            }
        }
        self.osc_string.clear();
    }

    fn execute_csi(&mut self, final_byte: u8, segments: &mut Vec<ParsedSegment>) {
        let param_or = |params: &[u16], idx: usize, default: usize| -> usize {
            let val = params.get(idx).copied().unwrap_or(0) as usize;
            if val == 0 {
                default
            } else {
                val
            }
        };

        match final_byte {
            b'A' => {
                // CUU - Cursor Up
                segments.push(ParsedSegment::CursorUp(param_or(&self.params, 0, 1)));
            }
            b'B' => {
                // CUD - Cursor Down
                segments.push(ParsedSegment::CursorDown(param_or(&self.params, 0, 1)));
            }
            b'C' => {
                // CUF - Cursor Forward
                segments.push(ParsedSegment::CursorForward(param_or(&self.params, 0, 1)));
            }
            b'D' => {
                // CUB - Cursor Backward
                segments.push(ParsedSegment::CursorBackward(param_or(&self.params, 0, 1)));
            }
            b'H' | b'f' => {
                // CUP - Cursor Position
                let row = param_or(&self.params, 0, 1).saturating_sub(1);
                let col = param_or(&self.params, 1, 1).saturating_sub(1);
                segments.push(ParsedSegment::CursorPosition(row, col));
            }
            b'J' => {
                // ED - Erase Display
                let mode = ClearMode::from_param(param_or(&self.params, 0, 0));
                segments.push(ParsedSegment::ClearScreen(mode));
            }
            b'K' => {
                // EL - Erase Line
                let mode = ClearMode::from_param(param_or(&self.params, 0, 0));
                segments.push(ParsedSegment::ClearLine(mode));
            }
            b'S' => {
                segments.push(ParsedSegment::ScrollUp(param_or(&self.params, 0, 1)));
            }
            b'T' => {
                // SD - Scroll Down
                segments.push(ParsedSegment::ScrollDown(param_or(&self.params, 0, 1)));
            }
            b'm' => {
                // SGR - Select Graphic Rendition
                self.execute_sgr();
            }
            b's' => {
                // SCP - Save Cursor Position
                segments.push(ParsedSegment::CursorSave);
            }
            b'u' => {
                // RCP - Restore Cursor Position
                segments.push(ParsedSegment::CursorRestore);
            }
            _ => {
                // Unknown CSI sequence - ignore
            }
        }
    }

    fn execute_sgr(&mut self) {
        if self.params.is_empty() {
            self.params.push(0);
        }

        let mut i = 0;
        while i < self.params.len() {
            let code = self.params[i] as usize;
            match code {
                0 => {
                    self.current_style = CellStyle {
                        foreground: self.default_fg,
                        background: self.default_bg,
                        ..CellStyle::default()
                    };
                }
                1 => self.current_style.bold = true,
                2 => self.current_style.dim = true,
                3 => self.current_style.italic = true,
                4 => self.current_style.underline = true,
                7 => self.current_style.inverse = true,
                9 => self.current_style.strikethrough = true,
                21 | 22 => {
                    self.current_style.bold = false;
                    self.current_style.dim = false;
                }
                23 => self.current_style.italic = false,
                24 => self.current_style.underline = false,
                27 => self.current_style.inverse = false,
                29 => self.current_style.strikethrough = false,
                30..=37 => {
                    // Standard foreground colors
                    self.current_style.foreground = ANSI_COLORS[code - 30];
                }
                38 => {
                    // Extended foreground color
                    if let Some(color) = self.parse_extended_color(&mut i) {
                        self.current_style.foreground = color;
                    }
                }
                39 => {
                    // Default foreground
                    self.current_style.foreground = self.default_fg;
                }
                40..=47 => {
                    // Standard background colors
                    self.current_style.background = ANSI_COLORS[code - 40];
                }
                48 => {
                    // Extended background color
                    if let Some(color) = self.parse_extended_color(&mut i) {
                        self.current_style.background = color;
                    }
                }
                49 => {
                    // Default background
                    self.current_style.background = self.default_bg;
                }
                90..=97 => {
                    // Bright foreground colors
                    self.current_style.foreground = ANSI_COLORS[code - 90 + 8];
                }
                100..=107 => {
                    // Bright background colors
                    self.current_style.background = ANSI_COLORS[code - 100 + 8];
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn parse_extended_color(&self, i: &mut usize) -> Option<Rgba> {
        if *i + 1 >= self.params.len() {
            return None;
        }

        let mode = self.params[*i + 1];
        match mode {
            2 => {
                // RGB color: 38;2;r;g;b
                if *i + 4 >= self.params.len() {
                    return None;
                }
                let r = self.params[*i + 2] as f32 / 255.0;
                let g = self.params[*i + 3] as f32 / 255.0;
                let b = self.params[*i + 4] as f32 / 255.0;
                *i += 4;
                Some(Rgba { r, g, b, a: 1.0 })
            }
            5 => {
                // 256-color palette: 38;5;n
                if *i + 2 >= self.params.len() {
                    return None;
                }
                let n = self.params[*i + 2] as usize;
                *i += 2;
                Some(color_from_256(n))
            }
            _ => None,
        }
    }
}

/// Convert 256-color palette index to Rgba
pub fn color_from_256(n: usize) -> Rgba {
    match n {
        0..=15 => ANSI_COLORS[n],
        16..=231 => {
            // 6x6x6 color cube
            let n = n - 16;
            let r = (n / 36) % 6;
            let g = (n / 6) % 6;
            let b = n % 6;
            Rgba {
                r: if r == 0 {
                    0.0
                } else {
                    (r * 40 + 55) as f32 / 255.0
                },
                g: if g == 0 {
                    0.0
                } else {
                    (g * 40 + 55) as f32 / 255.0
                },
                b: if b == 0 {
                    0.0
                } else {
                    (b * 40 + 55) as f32 / 255.0
                },
                a: 1.0,
            }
        }
        232..=255 => {
            let gray = ((n - 232) * 10 + 8) as f32 / 255.0;
            Rgba {
                r: gray,
                g: gray,
                b: gray,
                a: 1.0,
            }
        }
        _ => DEFAULT_FG,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let mut parser = AnsiParser::new();
        let segments = parser.parse(b"Hello, World!");
        assert_eq!(segments.len(), 1);
        if let ParsedSegment::Text(text, _) = &segments[0] {
            assert_eq!(text, "Hello, World!");
        } else {
            panic!("Expected Text segment");
        }
    }

    #[test]
    fn test_newline() {
        let mut parser = AnsiParser::new();
        let segments = parser.parse(b"Line1\nLine2");
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], ParsedSegment::Text(t, _) if t == "Line1"));
        assert!(matches!(&segments[1], ParsedSegment::LineFeed));
        assert!(matches!(&segments[2], ParsedSegment::Text(t, _) if t == "Line2"));
    }

    #[test]
    fn test_cursor_movement() {
        let mut parser = AnsiParser::new();

        let segments = parser.parse(b"\x1B[5A");
        assert_eq!(segments, vec![ParsedSegment::CursorUp(5)]);

        let segments = parser.parse(b"\x1B[3B");
        assert_eq!(segments, vec![ParsedSegment::CursorDown(3)]);

        let segments = parser.parse(b"\x1B[2C");
        assert_eq!(segments, vec![ParsedSegment::CursorForward(2)]);

        let segments = parser.parse(b"\x1B[4D");
        assert_eq!(segments, vec![ParsedSegment::CursorBackward(4)]);
    }

    #[test]
    fn test_cursor_position() {
        let mut parser = AnsiParser::new();
        let segments = parser.parse(b"\x1B[10;20H");
        assert_eq!(segments, vec![ParsedSegment::CursorPosition(9, 19)]);
    }

    #[test]
    fn test_clear_screen() {
        let mut parser = AnsiParser::new();

        let segments = parser.parse(b"\x1B[2J");
        assert_eq!(segments, vec![ParsedSegment::ClearScreen(ClearMode::All)]);

        let segments = parser.parse(b"\x1B[J");
        assert_eq!(segments, vec![ParsedSegment::ClearScreen(ClearMode::ToEnd)]);
    }

    #[test]
    fn test_clear_line() {
        let mut parser = AnsiParser::new();

        let segments = parser.parse(b"\x1B[K");
        assert_eq!(segments, vec![ParsedSegment::ClearLine(ClearMode::ToEnd)]);

        let segments = parser.parse(b"\x1B[1K");
        assert_eq!(segments, vec![ParsedSegment::ClearLine(ClearMode::ToStart)]);
    }

    #[test]
    fn test_sgr_reset() {
        let mut parser = AnsiParser::new();
        parser.current_style.bold = true;
        parser.parse(b"\x1B[0m");
        assert!(!parser.current_style().bold);
    }

    #[test]
    fn test_sgr_bold() {
        let mut parser = AnsiParser::new();
        parser.parse(b"\x1B[1m");
        assert!(parser.current_style().bold);
    }

    #[test]
    fn test_sgr_foreground_color() {
        let mut parser = AnsiParser::new();
        parser.parse(b"\x1B[31m");
        assert_eq!(parser.current_style().foreground, ANSI_COLORS[1]);
    }

    #[test]
    fn test_sgr_background_color() {
        let mut parser = AnsiParser::new();
        parser.parse(b"\x1B[44m");
        assert_eq!(parser.current_style().background, ANSI_COLORS[4]);
    }

    #[test]
    fn test_sgr_bright_colors() {
        let mut parser = AnsiParser::new();
        parser.parse(b"\x1B[91m");
        assert_eq!(parser.current_style().foreground, ANSI_COLORS[9]);
    }

    #[test]
    fn test_sgr_256_color() {
        let mut parser = AnsiParser::new();
        parser.parse(b"\x1B[38;5;196m");
        // Color 196 is in the 6x6x6 cube
        let expected = color_from_256(196);
        assert_eq!(parser.current_style().foreground, expected);
    }

    #[test]
    fn test_sgr_rgb_color() {
        let mut parser = AnsiParser::new();
        parser.parse(b"\x1B[38;2;255;128;64m");
        let style = parser.current_style();
        assert!((style.foreground.r - 1.0).abs() < 0.01);
        assert!((style.foreground.g - 0.502).abs() < 0.01);
        assert!((style.foreground.b - 0.251).abs() < 0.01);
    }

    #[test]
    fn test_osc_title() {
        let mut parser = AnsiParser::new();
        let segments = parser.parse(b"\x1B]0;My Title\x07");
        assert_eq!(
            segments,
            vec![ParsedSegment::SetTitle("My Title".to_string())]
        );
    }

    #[test]
    fn test_mixed_content() {
        let mut parser = AnsiParser::new();
        let segments = parser.parse(b"\x1B[31mRed\x1B[0m Normal");

        assert_eq!(segments.len(), 2);
        if let ParsedSegment::Text(text, style) = &segments[0] {
            assert_eq!(text, "Red");
            assert_eq!(style.foreground, ANSI_COLORS[1]);
        }
        if let ParsedSegment::Text(text, _) = &segments[1] {
            assert_eq!(text, " Normal");
        }
    }

    #[test]
    fn test_color_from_256_standard() {
        for i in 0..16 {
            assert_eq!(color_from_256(i), ANSI_COLORS[i]);
        }
    }

    #[test]
    fn test_color_from_256_grayscale() {
        let gray = color_from_256(232);
        assert!(gray.r > 0.0 && gray.r < 0.1);
        assert_eq!(gray.r, gray.g);
        assert_eq!(gray.g, gray.b);
    }
}
