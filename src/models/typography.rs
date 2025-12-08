//! Typography and Spacing System for RPG-Inspired UI
//!
//! This module defines the typography scale, font families, weights, and spacing
//! constants used throughout the Nexus Explorer application. The design follows
//! RPG-inspired aesthetics with dramatic size contrasts and ornate styling.
//!
//! ## Font Families
//! - **Crimson Pro**: Display font for headers and titles (serif, elegant)
//! - **IBM Plex Sans**: Body font for file names and labels (sans-serif, technical)
//! - **JetBrains Mono**: Monospace font for code, terminal, and file sizes
//!
//! ## Design Philosophy
//! - Use 3x+ size jumps for dramatic hierarchy (not incremental 1.5x scaling)
//! - Weight extremes (200 thin vs 900 black) for visual impact
//! - Generous spacing for readability and RPG aesthetic

use gpui::SharedString;




pub const FONT_DISPLAY: &str = "Crimson Pro";



pub const FONT_BODY: &str = "IBM Plex Sans";



pub const FONT_MONO: &str = "JetBrains Mono";


pub const FONT_DISPLAY_FALLBACK: &str = "Georgia, Times New Roman, serif";


pub const FONT_BODY_FALLBACK: &str = "Inter, -apple-system, BlinkMacSystemFont, sans-serif";


pub const FONT_MONO_FALLBACK: &str = "Menlo, Monaco, Consolas, monospace";



pub const WEIGHT_THIN: u16 = 200;


pub const WEIGHT_LIGHT: u16 = 300;


pub const WEIGHT_REGULAR: u16 = 400;


pub const WEIGHT_MEDIUM: u16 = 500;


pub const WEIGHT_SEMIBOLD: u16 = 600;


pub const WEIGHT_BOLD: u16 = 700;


pub const WEIGHT_EXTRABOLD: u16 = 800;


pub const WEIGHT_BLACK: u16 = 900;



pub const SIZE_MICRO: f32 = 9.0;


pub const SIZE_XS: f32 = 10.0;


pub const SIZE_SM: f32 = 12.0;


pub const SIZE_BASE: f32 = 14.0;


pub const SIZE_LG: f32 = 18.0;


pub const SIZE_XL: f32 = 24.0;


pub const SIZE_2XL: f32 = 36.0;


pub const SIZE_3XL: f32 = 48.0;


pub const SIZE_DISPLAY: f32 = 72.0;



pub const TRACKING_TIGHT: f32 = -0.02;


pub const TRACKING_NORMAL: f32 = 0.0;


pub const TRACKING_WIDE: f32 = 0.05;


pub const TRACKING_ULTRA: f32 = 0.15;



pub const LINE_HEIGHT_TIGHT: f32 = 1.2;


pub const LINE_HEIGHT_NORMAL: f32 = 1.5;


pub const LINE_HEIGHT_RELAXED: f32 = 1.75;



pub const SPACE_0: f32 = 0.0;


pub const SPACE_1: f32 = 4.0;


pub const SPACE_2: f32 = 8.0;


pub const SPACE_3: f32 = 12.0;


pub const SPACE_4: f32 = 16.0;


pub const SPACE_5: f32 = 20.0;


pub const SPACE_6: f32 = 24.0;


pub const SPACE_8: f32 = 32.0;


pub const SPACE_12: f32 = 48.0;


pub const SPACE_16: f32 = 64.0;



pub mod sidebar {

    pub const WIDTH: f32 = 280.0;


    pub const ITEM_HEIGHT: f32 = 36.0;


    pub const ITEM_PADDING_X: f32 = 16.0;


    pub const ITEM_PADDING_Y: f32 = 8.0;


    pub const SECTION_GAP: f32 = 24.0;


    pub const ICON_SIZE: f32 = 18.0;


    pub const ICON_GAP: f32 = 12.0;


    pub const HEADER_SIZE: f32 = 10.0;


    pub const HEADER_TRACKING: f32 = 0.1;
}


pub mod file_list {

    pub const ROW_HEIGHT: f32 = 40.0;


    pub const ROW_PADDING_X: f32 = 16.0;


    pub const ICON_SIZE: f32 = 20.0;


    pub const ICON_GAP: f32 = 12.0;


    pub const COLUMN_GAP: f32 = 8.0;


    pub const HEADER_HEIGHT: f32 = 36.0;


    pub const FOOTER_HEIGHT: f32 = 28.0;
}


pub mod toolbar {

    pub const HEIGHT: f32 = 52.0;


    pub const BUTTON_SIZE: f32 = 36.0;


    pub const BUTTON_GAP: f32 = 8.0;


    pub const SECTION_GAP: f32 = 16.0;


    pub const PADDING_X: f32 = 16.0;


    pub const BREADCRUMB_PADDING: f32 = 8.0;
}


pub mod terminal_layout {

    pub const MIN_HEIGHT: f32 = 200.0;


    pub const DEFAULT_HEIGHT: f32 = 300.0;


    pub const LINE_HEIGHT: f32 = 1.5;


    pub const PADDING: f32 = 16.0;


    pub const TAB_HEIGHT: f32 = 32.0;
}


pub mod preview {

    pub const WIDTH: f32 = 360.0;


    pub const HEADER_HEIGHT: f32 = 64.0;


    pub const PADDING: f32 = 20.0;


    pub const METADATA_ROW_HEIGHT: f32 = 28.0;
}


pub mod grid {

    pub const ITEM_WIDTH: f32 = 120.0;


    pub const ITEM_HEIGHT: f32 = 140.0;


    pub const ICON_SIZE: f32 = 64.0;


    pub const GAP: f32 = 16.0;


    pub const MIN_COLUMNS: usize = 2;
}


pub mod tab_bar {

    pub const HEIGHT: f32 = 36.0;


    pub const TAB_PADDING_X: f32 = 16.0;


    pub const CLOSE_BUTTON_SIZE: f32 = 16.0;


    pub const TAB_GAP: f32 = 2.0;
}


pub mod status_bar {

    pub const HEIGHT: f32 = 28.0;


    pub const PADDING_X: f32 = 16.0;


    pub const ITEM_GAP: f32 = 16.0;


    pub const DIVIDER_WIDTH: f32 = 1.0;
}



pub const RADIUS_SM: f32 = 4.0;


pub const RADIUS_MD: f32 = 8.0;


pub const RADIUS_LG: f32 = 16.0;


pub const RADIUS_XL: f32 = 24.0;



pub fn display_font() -> SharedString {
    SharedString::from(FONT_DISPLAY)
}


pub fn body_font() -> SharedString {
    SharedString::from(FONT_BODY)
}


pub fn mono_font() -> SharedString {
    SharedString::from(FONT_MONO)
}


pub fn tracking_to_px(tracking_em: f32, font_size: f32) -> f32 {
    tracking_em * font_size
}


pub struct DisplayTypography {
    pub font_family: &'static str,
    pub font_size: f32,
    pub font_weight: u16,
    pub letter_spacing: f32,
    pub line_height: f32,
}

impl DisplayTypography {

    pub const HERO: Self = Self {
        font_family: FONT_DISPLAY,
        font_size: SIZE_DISPLAY,
        font_weight: WEIGHT_THIN,
        letter_spacing: TRACKING_TIGHT,
        line_height: LINE_HEIGHT_TIGHT,
    };


    pub const PAGE_TITLE: Self = Self {
        font_family: FONT_DISPLAY,
        font_size: SIZE_2XL,
        font_weight: WEIGHT_LIGHT,
        letter_spacing: TRACKING_TIGHT,
        line_height: LINE_HEIGHT_TIGHT,
    };


    pub const SECTION_HEADER: Self = Self {
        font_family: FONT_DISPLAY,
        font_size: SIZE_XL,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_NORMAL,
    };


    pub const PANEL_HEADER: Self = Self {
        font_family: FONT_BODY,
        font_size: SIZE_LG,
        font_weight: WEIGHT_SEMIBOLD,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_NORMAL,
    };
}


pub struct BodyTypography {
    pub font_family: &'static str,
    pub font_size: f32,
    pub font_weight: u16,
    pub letter_spacing: f32,
    pub line_height: f32,
}

impl BodyTypography {

    pub const PRIMARY: Self = Self {
        font_family: FONT_BODY,
        font_size: SIZE_BASE,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_NORMAL,
    };


    pub const SECONDARY: Self = Self {
        font_family: FONT_BODY,
        font_size: SIZE_SM,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_NORMAL,
    };


    pub const LABEL: Self = Self {
        font_family: FONT_BODY,
        font_size: SIZE_XS,
        font_weight: WEIGHT_BOLD,
        letter_spacing: TRACKING_WIDE,
        line_height: LINE_HEIGHT_TIGHT,
    };


    pub const CAPTION: Self = Self {
        font_family: FONT_BODY,
        font_size: SIZE_XS,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_NORMAL,
    };
}


pub struct MonoTypography {
    pub font_family: &'static str,
    pub font_size: f32,
    pub font_weight: u16,
    pub letter_spacing: f32,
    pub line_height: f32,
}

impl MonoTypography {

    pub const CODE: Self = Self {
        font_family: FONT_MONO,
        font_size: SIZE_SM,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_RELAXED,
    };


    pub const TERMINAL: Self = Self {
        font_family: FONT_MONO,
        font_size: SIZE_SM,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: 1.5,
    };


    pub const FILE_SIZE: Self = Self {
        font_family: FONT_MONO,
        font_size: SIZE_XS,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_NORMAL,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_constants() {
        assert_eq!(FONT_DISPLAY, "Crimson Pro");
        assert_eq!(FONT_BODY, "IBM Plex Sans");
        assert_eq!(FONT_MONO, "JetBrains Mono");
    }

    #[test]
    fn test_size_hierarchy() {
        assert!(SIZE_MICRO < SIZE_XS);
        assert!(SIZE_XS < SIZE_SM);
        assert!(SIZE_SM < SIZE_BASE);
        assert!(SIZE_BASE < SIZE_LG);
        assert!(SIZE_LG < SIZE_XL);
        assert!(SIZE_XL < SIZE_2XL);
        assert!(SIZE_2XL < SIZE_3XL);
        assert!(SIZE_3XL < SIZE_DISPLAY);
    }

    #[test]
    fn test_weight_hierarchy() {
        assert!(WEIGHT_THIN < WEIGHT_LIGHT);
        assert!(WEIGHT_LIGHT < WEIGHT_REGULAR);
        assert!(WEIGHT_REGULAR < WEIGHT_MEDIUM);
        assert!(WEIGHT_MEDIUM < WEIGHT_SEMIBOLD);
        assert!(WEIGHT_SEMIBOLD < WEIGHT_BOLD);
        assert!(WEIGHT_BOLD < WEIGHT_EXTRABOLD);
        assert!(WEIGHT_EXTRABOLD < WEIGHT_BLACK);
    }

    #[test]
    fn test_spacing_hierarchy() {
        assert!(SPACE_0 < SPACE_1);
        assert!(SPACE_1 < SPACE_2);
        assert!(SPACE_2 < SPACE_3);
        assert!(SPACE_3 < SPACE_4);
        assert!(SPACE_4 < SPACE_5);
        assert!(SPACE_5 < SPACE_6);
        assert!(SPACE_6 < SPACE_8);
        assert!(SPACE_8 < SPACE_12);
        assert!(SPACE_12 < SPACE_16);
    }

    #[test]
    fn test_component_spacing() {
        assert_eq!(sidebar::WIDTH, 280.0);
        assert_eq!(sidebar::ITEM_PADDING_X, 16.0);
        assert_eq!(sidebar::SECTION_GAP, 24.0);

        assert_eq!(file_list::ROW_HEIGHT, 40.0);
        assert_eq!(file_list::ICON_SIZE, 20.0);
        assert_eq!(file_list::ICON_GAP, 12.0);

        assert_eq!(toolbar::HEIGHT, 52.0);
        assert_eq!(toolbar::BUTTON_SIZE, 36.0);
    }

    #[test]
    fn test_tracking_to_px() {
        let result = tracking_to_px(TRACKING_TIGHT, SIZE_BASE);
        assert!((result - (-0.28)).abs() < 0.01);

        let result = tracking_to_px(TRACKING_WIDE, SIZE_2XL);
        assert!((result - 1.8).abs() < 0.01);
    }
}
