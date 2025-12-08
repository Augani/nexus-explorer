use crate::models::ansi_parser::{color_from_256, AnsiParser, ParsedSegment, ANSI_COLORS};
use proptest::prelude::*;






proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_ansi_standard_foreground_colors(color_code in 30u8..=37u8) {
        let mut parser = AnsiParser::new();
        let escape_seq = format!("\x1B[{}m", color_code);
        parser.parse(escape_seq.as_bytes());

        let expected_color = ANSI_COLORS[(color_code - 30) as usize];
        prop_assert_eq!(
            parser.current_style().foreground,
            expected_color,
            "Standard foreground color {} should map to ANSI color {}",
            color_code,
            color_code - 30
        );
    }

    #[test]
    fn prop_ansi_standard_background_colors(color_code in 40u8..=47u8) {
        let mut parser = AnsiParser::new();
        let escape_seq = format!("\x1B[{}m", color_code);
        parser.parse(escape_seq.as_bytes());

        let expected_color = ANSI_COLORS[(color_code - 40) as usize];
        prop_assert_eq!(
            parser.current_style().background,
            expected_color,
            "Standard background color {} should map to ANSI color {}",
            color_code,
            color_code - 40
        );
    }

    #[test]
    fn prop_ansi_bright_foreground_colors(color_code in 90u8..=97u8) {
        let mut parser = AnsiParser::new();
        let escape_seq = format!("\x1B[{}m", color_code);
        parser.parse(escape_seq.as_bytes());

        let expected_color = ANSI_COLORS[(color_code - 90 + 8) as usize];
        prop_assert_eq!(
            parser.current_style().foreground,
            expected_color,
            "Bright foreground color {} should map to ANSI color {}",
            color_code,
            color_code - 90 + 8
        );
    }

    #[test]
    fn prop_ansi_bright_background_colors(color_code in 100u8..=107u8) {
        let mut parser = AnsiParser::new();
        let escape_seq = format!("\x1B[{}m", color_code);
        parser.parse(escape_seq.as_bytes());

        let expected_color = ANSI_COLORS[(color_code - 100 + 8) as usize];
        prop_assert_eq!(
            parser.current_style().background,
            expected_color,
            "Bright background color {} should map to ANSI color {}",
            color_code,
            color_code - 100 + 8
        );
    }

    #[test]
    fn prop_ansi_256_color_foreground(color_index in 0u8..=255u8) {
        let mut parser = AnsiParser::new();
        let escape_seq = format!("\x1B[38;5;{}m", color_index);
        parser.parse(escape_seq.as_bytes());

        let expected_color = color_from_256(color_index as usize);
        prop_assert_eq!(
            parser.current_style().foreground,
            expected_color,
            "256-color foreground {} should be correctly parsed",
            color_index
        );
    }

    #[test]
    fn prop_ansi_256_color_background(color_index in 0u8..=255u8) {
        let mut parser = AnsiParser::new();
        let escape_seq = format!("\x1B[48;5;{}m", color_index);
        parser.parse(escape_seq.as_bytes());

        let expected_color = color_from_256(color_index as usize);
        prop_assert_eq!(
            parser.current_style().background,
            expected_color,
            "256-color background {} should be correctly parsed",
            color_index
        );
    }

    #[test]
    fn prop_ansi_rgb_foreground(r in 0u8..=255u8, g in 0u8..=255u8, b in 0u8..=255u8) {
        let mut parser = AnsiParser::new();
        let escape_seq = format!("\x1B[38;2;{};{};{}m", r, g, b);
        parser.parse(escape_seq.as_bytes());

        let style = parser.current_style();
        let expected_r = r as f32 / 255.0;
        let expected_g = g as f32 / 255.0;
        let expected_b = b as f32 / 255.0;

        prop_assert!(
            (style.foreground.r - expected_r).abs() < 0.01,
            "RGB foreground red component should be {}, got {}",
            expected_r,
            style.foreground.r
        );
        prop_assert!(
            (style.foreground.g - expected_g).abs() < 0.01,
            "RGB foreground green component should be {}, got {}",
            expected_g,
            style.foreground.g
        );
        prop_assert!(
            (style.foreground.b - expected_b).abs() < 0.01,
            "RGB foreground blue component should be {}, got {}",
            expected_b,
            style.foreground.b
        );
    }

    #[test]
    fn prop_ansi_rgb_background(r in 0u8..=255u8, g in 0u8..=255u8, b in 0u8..=255u8) {
        let mut parser = AnsiParser::new();
        let escape_seq = format!("\x1B[48;2;{};{};{}m", r, g, b);
        parser.parse(escape_seq.as_bytes());

        let style = parser.current_style();
        let expected_r = r as f32 / 255.0;
        let expected_g = g as f32 / 255.0;
        let expected_b = b as f32 / 255.0;

        prop_assert!(
            (style.background.r - expected_r).abs() < 0.01,
            "RGB background red component should be {}, got {}",
            expected_r,
            style.background.r
        );
        prop_assert!(
            (style.background.g - expected_g).abs() < 0.01,
            "RGB background green component should be {}, got {}",
            expected_g,
            style.background.g
        );
        prop_assert!(
            (style.background.b - expected_b).abs() < 0.01,
            "RGB background blue component should be {}, got {}",
            expected_b,
            style.background.b
        );
    }

    #[test]
    fn prop_ansi_reset_clears_style(
        bold in any::<bool>(),
        italic in any::<bool>(),
        underline in any::<bool>()
    ) {
        let mut parser = AnsiParser::new();

        if bold {
            parser.parse(b"\x1B[1m");
        }
        if italic {
            parser.parse(b"\x1B[3m");
        }
        if underline {
            parser.parse(b"\x1B[4m");
        }

        parser.parse(b"\x1B[0m");

        let style = parser.current_style();
        prop_assert!(!style.bold, "Bold should be reset");
        prop_assert!(!style.italic, "Italic should be reset");
        prop_assert!(!style.underline, "Underline should be reset");
    }

    #[test]
    fn prop_text_preserves_style(text in "[a-zA-Z0-9 ]{1,50}") {
        let mut parser = AnsiParser::new();

        parser.parse(b"\x1B[1;31m");
        let style_before = parser.current_style().clone();

        let segments = parser.parse(text.as_bytes());

        prop_assert_eq!(
            parser.current_style(),
            &style_before,
            "Style should be preserved after parsing text"
        );

        if !text.is_empty() {
            prop_assert!(!segments.is_empty(), "Should have at least one segment");
            if let ParsedSegment::Text(_, seg_style) = &segments[0] {
                prop_assert_eq!(
                    seg_style,
                    &style_before,
                    "Text segment should have the current style"
                );
            }
        }
    }
}
