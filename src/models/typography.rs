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

// ============================================================================
// Font Families
// ============================================================================

/// Display font for headers, titles, and decorative text
/// Crimson Pro is an elegant serif font with excellent weight range
pub const FONT_DISPLAY: &str = "Crimson Pro";

/// Body font for file names, labels, and general UI text
/// IBM Plex Sans provides technical clarity with good readability
pub const FONT_BODY: &str = "IBM Plex Sans";

/// Monospace font for code, terminal, file sizes, and technical data
/// JetBrains Mono is optimized for code readability
pub const FONT_MONO: &str = "JetBrains Mono";

/// Fallback font stack for display text
pub const FONT_DISPLAY_FALLBACK: &str = "Georgia, Times New Roman, serif";

/// Fallback font stack for body text
pub const FONT_BODY_FALLBACK: &str = "Inter, -apple-system, BlinkMacSystemFont, sans-serif";

/// Fallback font stack for monospace text
pub const FONT_MONO_FALLBACK: &str = "Menlo, Monaco, Consolas, monospace";

// ============================================================================
// Font Weights - Extreme Contrasts for Visual Impact
// ============================================================================

/// Extra light weight for elegant, delicate headers
pub const WEIGHT_THIN: u16 = 200;

/// Light weight for subtle emphasis
pub const WEIGHT_LIGHT: u16 = 300;

/// Regular weight for body text only
pub const WEIGHT_REGULAR: u16 = 400;

/// Medium weight for slight emphasis
pub const WEIGHT_MEDIUM: u16 = 500;

/// Semi-bold weight for moderate emphasis
pub const WEIGHT_SEMIBOLD: u16 = 600;

/// Bold weight for strong emphasis
pub const WEIGHT_BOLD: u16 = 700;

/// Extra bold weight for very strong emphasis
pub const WEIGHT_EXTRABOLD: u16 = 800;

/// Black weight for maximum impact headers
pub const WEIGHT_BLACK: u16 = 900;

// ============================================================================
// Font Sizes - 3x+ Jumps for Dramatic Hierarchy
// ============================================================================

/// Micro size for badges and tiny labels (9px)
pub const SIZE_MICRO: f32 = 9.0;

/// Extra small for status indicators and timestamps (10px)
pub const SIZE_XS: f32 = 10.0;

/// Small for secondary text, metadata, file sizes (12px)
pub const SIZE_SM: f32 = 12.0;

/// Base size for body text and file names (14px)
pub const SIZE_BASE: f32 = 14.0;

/// Large for subheadings and section titles (18px)
pub const SIZE_LG: f32 = 18.0;

/// Extra large for panel headers (24px) - 3x jump from SM
pub const SIZE_XL: f32 = 24.0;

/// 2XL for page titles (36px) - 3x jump from SM
pub const SIZE_2XL: f32 = 36.0;

/// 3XL for hero text and splash screens (48px)
pub const SIZE_3XL: f32 = 48.0;

/// Display size for dramatic app title (72px)
pub const SIZE_DISPLAY: f32 = 72.0;

// ============================================================================
// Letter Spacing - Refinement for Different Contexts
// ============================================================================

/// Tight tracking for large display text (-0.02em)
pub const TRACKING_TIGHT: f32 = -0.02;

/// Normal tracking for body text (0)
pub const TRACKING_NORMAL: f32 = 0.0;

/// Wide tracking for small caps and labels (0.05em)
pub const TRACKING_WIDE: f32 = 0.05;

/// Ultra wide tracking for decorative headers (0.15em)
pub const TRACKING_ULTRA: f32 = 0.15;

// ============================================================================
// Line Heights
// ============================================================================

/// Tight line height for compact UI elements
pub const LINE_HEIGHT_TIGHT: f32 = 1.2;

/// Normal line height for body text
pub const LINE_HEIGHT_NORMAL: f32 = 1.5;

/// Relaxed line height for readability
pub const LINE_HEIGHT_RELAXED: f32 = 1.75;

// ============================================================================
// Spacing Scale - 4px Base Unit with Larger Jumps
// ============================================================================

/// No spacing
pub const SPACE_0: f32 = 0.0;

/// Tight spacing: icon-to-text gaps (4px)
pub const SPACE_1: f32 = 4.0;

/// Compact spacing: list items (8px)
pub const SPACE_2: f32 = 8.0;

/// Default spacing: button padding (12px)
pub const SPACE_3: f32 = 12.0;

/// Comfortable spacing: card padding (16px)
pub const SPACE_4: f32 = 16.0;

/// Medium spacing: between related elements (20px)
pub const SPACE_5: f32 = 20.0;

/// Spacious spacing: section gaps (24px)
pub const SPACE_6: f32 = 24.0;

/// Large spacing: panel margins (32px)
pub const SPACE_8: f32 = 32.0;

/// Extra large spacing: major sections (48px)
pub const SPACE_12: f32 = 48.0;

/// Huge spacing: page margins (64px)
pub const SPACE_16: f32 = 64.0;

// ============================================================================
// Component-Specific Spacing
// ============================================================================

/// Sidebar configuration
pub mod sidebar {
    /// Sidebar width (280px as per design spec)
    pub const WIDTH: f32 = 280.0;
    
    /// Item height in sidebar
    pub const ITEM_HEIGHT: f32 = 36.0;
    
    /// Horizontal padding for items
    pub const ITEM_PADDING_X: f32 = 16.0;
    
    /// Vertical padding for items
    pub const ITEM_PADDING_Y: f32 = 8.0;
    
    /// Gap between sections
    pub const SECTION_GAP: f32 = 24.0;
    
    /// Icon size in sidebar
    pub const ICON_SIZE: f32 = 18.0;
    
    /// Gap between icon and text
    pub const ICON_GAP: f32 = 12.0;
    
    /// Section header font size
    pub const HEADER_SIZE: f32 = 10.0;
    
    /// Section header letter spacing
    pub const HEADER_TRACKING: f32 = 0.1;
}

/// File list configuration
pub mod file_list {
    /// Row height (40px as per design spec)
    pub const ROW_HEIGHT: f32 = 40.0;
    
    /// Horizontal padding for rows
    pub const ROW_PADDING_X: f32 = 16.0;
    
    /// Icon size in file list
    pub const ICON_SIZE: f32 = 20.0;
    
    /// Gap between icon and text
    pub const ICON_GAP: f32 = 12.0;
    
    /// Gap between columns
    pub const COLUMN_GAP: f32 = 8.0;
    
    /// Header row height
    pub const HEADER_HEIGHT: f32 = 36.0;
    
    /// Footer/status bar height
    pub const FOOTER_HEIGHT: f32 = 28.0;
}

/// Toolbar configuration
pub mod toolbar {
    /// Toolbar height (52px as per design spec)
    pub const HEIGHT: f32 = 52.0;
    
    /// Button size
    pub const BUTTON_SIZE: f32 = 36.0;
    
    /// Gap between buttons
    pub const BUTTON_GAP: f32 = 8.0;
    
    /// Gap between toolbar sections
    pub const SECTION_GAP: f32 = 16.0;
    
    /// Horizontal padding
    pub const PADDING_X: f32 = 16.0;
    
    /// Breadcrumb segment padding
    pub const BREADCRUMB_PADDING: f32 = 8.0;
}

/// Terminal layout configuration
pub mod terminal_layout {
    /// Minimum terminal height
    pub const MIN_HEIGHT: f32 = 200.0;
    
    /// Default terminal height
    pub const DEFAULT_HEIGHT: f32 = 300.0;
    
    /// Line height multiplier
    pub const LINE_HEIGHT: f32 = 1.5;
    
    /// Padding around terminal content
    pub const PADDING: f32 = 16.0;
    
    /// Tab bar height
    pub const TAB_HEIGHT: f32 = 32.0;
}

/// Preview panel configuration
pub mod preview {
    /// Preview panel width
    pub const WIDTH: f32 = 360.0;
    
    /// Header height
    pub const HEADER_HEIGHT: f32 = 64.0;
    
    /// Content padding
    pub const PADDING: f32 = 20.0;
    
    /// Metadata row height
    pub const METADATA_ROW_HEIGHT: f32 = 28.0;
}

/// Grid view configuration
pub mod grid {
    /// Item width
    pub const ITEM_WIDTH: f32 = 120.0;
    
    /// Item height
    pub const ITEM_HEIGHT: f32 = 140.0;
    
    /// Icon size
    pub const ICON_SIZE: f32 = 64.0;
    
    /// Gap between items
    pub const GAP: f32 = 16.0;
    
    /// Minimum columns
    pub const MIN_COLUMNS: usize = 2;
}

/// Tab bar configuration
pub mod tab_bar {
    /// Tab bar height
    pub const HEIGHT: f32 = 36.0;
    
    /// Tab padding
    pub const TAB_PADDING_X: f32 = 16.0;
    
    /// Close button size
    pub const CLOSE_BUTTON_SIZE: f32 = 16.0;
    
    /// Gap between tabs
    pub const TAB_GAP: f32 = 2.0;
}

/// Status bar configuration
pub mod status_bar {
    /// Status bar height
    pub const HEIGHT: f32 = 28.0;
    
    /// Horizontal padding
    pub const PADDING_X: f32 = 16.0;
    
    /// Gap between items
    pub const ITEM_GAP: f32 = 16.0;
    
    /// Divider width
    pub const DIVIDER_WIDTH: f32 = 1.0;
}

// ============================================================================
// Border Radii
// ============================================================================

/// Small border radius for buttons and inputs
pub const RADIUS_SM: f32 = 4.0;

/// Medium border radius for cards and panels
pub const RADIUS_MD: f32 = 8.0;

/// Large border radius for modals and overlays
pub const RADIUS_LG: f32 = 16.0;

/// Extra large border radius for special elements
pub const RADIUS_XL: f32 = 24.0;

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the display font family as SharedString
pub fn display_font() -> SharedString {
    SharedString::from(FONT_DISPLAY)
}

/// Get the body font family as SharedString
pub fn body_font() -> SharedString {
    SharedString::from(FONT_BODY)
}

/// Get the mono font family as SharedString
pub fn mono_font() -> SharedString {
    SharedString::from(FONT_MONO)
}

/// Convert em-based tracking to pixels for a given font size
pub fn tracking_to_px(tracking_em: f32, font_size: f32) -> f32 {
    tracking_em * font_size
}

/// Typography preset for display headers
pub struct DisplayTypography {
    pub font_family: &'static str,
    pub font_size: f32,
    pub font_weight: u16,
    pub letter_spacing: f32,
    pub line_height: f32,
}

impl DisplayTypography {
    /// Large display header (hero text)
    pub const HERO: Self = Self {
        font_family: FONT_DISPLAY,
        font_size: SIZE_DISPLAY,
        font_weight: WEIGHT_THIN,
        letter_spacing: TRACKING_TIGHT,
        line_height: LINE_HEIGHT_TIGHT,
    };
    
    /// Page title
    pub const PAGE_TITLE: Self = Self {
        font_family: FONT_DISPLAY,
        font_size: SIZE_2XL,
        font_weight: WEIGHT_LIGHT,
        letter_spacing: TRACKING_TIGHT,
        line_height: LINE_HEIGHT_TIGHT,
    };
    
    /// Section header
    pub const SECTION_HEADER: Self = Self {
        font_family: FONT_DISPLAY,
        font_size: SIZE_XL,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_NORMAL,
    };
    
    /// Panel header
    pub const PANEL_HEADER: Self = Self {
        font_family: FONT_BODY,
        font_size: SIZE_LG,
        font_weight: WEIGHT_SEMIBOLD,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_NORMAL,
    };
}

/// Typography preset for body text
pub struct BodyTypography {
    pub font_family: &'static str,
    pub font_size: f32,
    pub font_weight: u16,
    pub letter_spacing: f32,
    pub line_height: f32,
}

impl BodyTypography {
    /// Primary body text (file names)
    pub const PRIMARY: Self = Self {
        font_family: FONT_BODY,
        font_size: SIZE_BASE,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_NORMAL,
    };
    
    /// Secondary body text (metadata)
    pub const SECONDARY: Self = Self {
        font_family: FONT_BODY,
        font_size: SIZE_SM,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_NORMAL,
    };
    
    /// Label text (section headers)
    pub const LABEL: Self = Self {
        font_family: FONT_BODY,
        font_size: SIZE_XS,
        font_weight: WEIGHT_BOLD,
        letter_spacing: TRACKING_WIDE,
        line_height: LINE_HEIGHT_TIGHT,
    };
    
    /// Caption text (timestamps, hints)
    pub const CAPTION: Self = Self {
        font_family: FONT_BODY,
        font_size: SIZE_XS,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_NORMAL,
    };
}

/// Typography preset for monospace text
pub struct MonoTypography {
    pub font_family: &'static str,
    pub font_size: f32,
    pub font_weight: u16,
    pub letter_spacing: f32,
    pub line_height: f32,
}

impl MonoTypography {
    /// Code text
    pub const CODE: Self = Self {
        font_family: FONT_MONO,
        font_size: SIZE_SM,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: LINE_HEIGHT_RELAXED,
    };
    
    /// Terminal text
    pub const TERMINAL: Self = Self {
        font_family: FONT_MONO,
        font_size: SIZE_SM,
        font_weight: WEIGHT_REGULAR,
        letter_spacing: TRACKING_NORMAL,
        line_height: 1.5,
    };
    
    /// File size text
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
        // Sizes should increase monotonically
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
        // Sidebar
        assert_eq!(sidebar::WIDTH, 280.0);
        assert_eq!(sidebar::ITEM_PADDING_X, 16.0);
        assert_eq!(sidebar::SECTION_GAP, 24.0);
        
        // File list
        assert_eq!(file_list::ROW_HEIGHT, 40.0);
        assert_eq!(file_list::ICON_SIZE, 20.0);
        assert_eq!(file_list::ICON_GAP, 12.0);
        
        // Toolbar
        assert_eq!(toolbar::HEIGHT, 52.0);
        assert_eq!(toolbar::BUTTON_SIZE, 36.0);
    }

    #[test]
    fn test_tracking_to_px() {
        // At 14px font size, -0.02em tracking = -0.28px
        let result = tracking_to_px(TRACKING_TIGHT, SIZE_BASE);
        assert!((result - (-0.28)).abs() < 0.01);
        
        // At 36px font size, 0.05em tracking = 1.8px
        let result = tracking_to_px(TRACKING_WIDE, SIZE_2XL);
        assert!((result - 1.8).abs() < 0.01);
    }
}
