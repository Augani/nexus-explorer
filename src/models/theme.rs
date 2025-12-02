use gpui::{Global, Rgba};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for themes
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ThemeId {
    DragonForge,
    FrostHaven,
    AncientTome,
    ShadowRealm,
    ElvenGlade,
}

impl Default for ThemeId {
    fn default() -> Self {
        Self::DragonForge
    }
}

/// Complete theme definition with colors, typography, and decorations
#[derive(Clone, Debug)]
pub struct Theme {
    pub id: ThemeId,
    pub name: &'static str,
    pub description: &'static str,
    pub colors: ThemeColors,
    pub typography: ThemeTypography,
    pub decorations: ThemeDecorations,
}

/// All color definitions for a theme
#[derive(Clone, Debug)]
pub struct ThemeColors {
    // Backgrounds - layered depth
    pub bg_void: Rgba,
    pub bg_primary: Rgba,
    pub bg_secondary: Rgba,
    pub bg_tertiary: Rgba,
    pub bg_hover: Rgba,
    pub bg_selected: Rgba,
    pub bg_active: Rgba,

    // Text hierarchy
    pub text_primary: Rgba,
    pub text_secondary: Rgba,
    pub text_muted: Rgba,
    pub text_inverse: Rgba,

    // Accent colors - dramatic, saturated
    pub accent_primary: Rgba,
    pub accent_secondary: Rgba,
    pub accent_glow: Rgba,

    // Semantic colors
    pub success: Rgba,
    pub warning: Rgba,
    pub error: Rgba,
    pub info: Rgba,

    // Borders and dividers
    pub border_subtle: Rgba,
    pub border_default: Rgba,
    pub border_emphasis: Rgba,
    pub border_ornate: Rgba,

    // File type colors
    pub folder_color: Rgba,
    pub folder_open_color: Rgba,
    pub file_code: Rgba,
    pub file_data: Rgba,
    pub file_media: Rgba,
    pub file_archive: Rgba,
    pub file_document: Rgba,

    // Terminal colors (16-color palette)
    pub terminal_bg: Rgba,
    pub terminal_fg: Rgba,
    pub terminal_cursor: Rgba,
    pub terminal_selection: Rgba,
    pub terminal_black: Rgba,
    pub terminal_red: Rgba,
    pub terminal_green: Rgba,
    pub terminal_yellow: Rgba,
    pub terminal_blue: Rgba,
    pub terminal_magenta: Rgba,
    pub terminal_cyan: Rgba,
    pub terminal_white: Rgba,
}

/// Typography settings for a theme
#[derive(Clone, Debug)]
pub struct ThemeTypography {
    pub font_display: &'static str,
    pub font_display_weight_light: u16,
    pub font_display_weight_bold: u16,

    pub font_body: &'static str,
    pub font_body_weight_normal: u16,
    pub font_body_weight_medium: u16,

    pub font_mono: &'static str,
    pub font_mono_weight: u16,

    // Size scale
    pub size_xs: f32,
    pub size_sm: f32,
    pub size_base: f32,
    pub size_lg: f32,
    pub size_xl: f32,
    pub size_2xl: f32,
    pub size_3xl: f32,

    // Letter spacing
    pub tracking_tight: f32,
    pub tracking_normal: f32,
    pub tracking_wide: f32,
}

/// Decorative elements for a theme
#[derive(Clone, Debug)]
pub struct ThemeDecorations {
    pub border_radius_sm: f32,
    pub border_radius_md: f32,
    pub border_radius_lg: f32,
    pub border_width: f32,

    pub use_ornate_borders: bool,
    pub corner_flourish: Option<CornerFlourish>,
    pub divider_style: DividerStyle,
    pub frame_style: FrameStyle,

    pub shadow_sm: ShadowConfig,
    pub shadow_md: ShadowConfig,
    pub shadow_lg: ShadowConfig,
    pub shadow_glow: ShadowConfig,
    pub shadow_inner: ShadowConfig,

    pub bg_pattern: Option<BackgroundPattern>,
    pub bg_noise_opacity: f32,
}

/// Corner flourish decoration
#[derive(Clone, Debug)]
pub struct CornerFlourish {
    pub svg_path: &'static str,
    pub size: f32,
    pub color_mode: FlourishColorMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlourishColorMode {
    Accent,
    Border,
    Gradient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameStyle {
    None,
    Simple,
    Double,
    Ornate,
    Beveled,
    Inset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DividerStyle {
    Simple,
    Ornate,
    Gradient,
    Dashed,
    Embossed,
    Runic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundPattern {
    None,
    Dots,
    Grid,
    Noise,
    Parchment,
    Leather,
    Volcanic,
    Crystalline,
    Mystical,
    Organic,
}

/// Shadow configuration
#[derive(Clone, Debug)]
pub struct ShadowConfig {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Rgba,
}

impl ShadowConfig {
    pub fn none() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            blur: 0.0,
            spread: 0.0,
            color: Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 },
        }
    }

    pub fn new(offset_x: f32, offset_y: f32, blur: f32, spread: f32, color: Rgba) -> Self {
        Self {
            offset_x,
            offset_y,
            blur,
            spread,
            color,
        }
    }
}


/// Helper to create Rgba from hex color
pub fn rgba_from_hex(hex: u32) -> Rgba {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    Rgba { r, g, b, a: 1.0 }
}

/// Helper to create Rgba from hex with alpha
pub fn rgba_from_hex_alpha(hex: u32, alpha: f32) -> Rgba {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    Rgba { r, g, b, a: alpha }
}

impl Default for ThemeTypography {
    fn default() -> Self {
        Self {
            font_display: "Crimson Pro",
            font_display_weight_light: 200,
            font_display_weight_bold: 900,
            font_body: "IBM Plex Sans",
            font_body_weight_normal: 400,
            font_body_weight_medium: 600,
            font_mono: "JetBrains Mono",
            font_mono_weight: 400,
            size_xs: 10.0,
            size_sm: 12.0,
            size_base: 14.0,
            size_lg: 18.0,
            size_xl: 24.0,
            size_2xl: 36.0,
            size_3xl: 48.0,
            tracking_tight: -0.02,
            tracking_normal: 0.0,
            tracking_wide: 0.05,
        }
    }
}

impl Default for ThemeDecorations {
    fn default() -> Self {
        Self {
            border_radius_sm: 4.0,
            border_radius_md: 8.0,
            border_radius_lg: 16.0,
            border_width: 1.0,
            use_ornate_borders: false,
            corner_flourish: None,
            divider_style: DividerStyle::Simple,
            frame_style: FrameStyle::Simple,
            shadow_sm: ShadowConfig::none(),
            shadow_md: ShadowConfig::none(),
            shadow_lg: ShadowConfig::none(),
            shadow_glow: ShadowConfig::none(),
            shadow_inner: ShadowConfig::none(),
            bg_pattern: None,
            bg_noise_opacity: 0.0,
        }
    }
}

impl ThemeColors {
    /// Check if all required color fields are set (non-transparent)
    pub fn is_complete(&self) -> bool {
        // Check that essential colors are not transparent
        self.bg_primary.a > 0.0
            && self.bg_secondary.a > 0.0
            && self.text_primary.a > 0.0
            && self.accent_primary.a > 0.0
            && self.border_default.a > 0.0
            && self.folder_color.a > 0.0
            && self.terminal_bg.a > 0.0
            && self.terminal_fg.a > 0.0
    }
}


impl Theme {
    /// Dragon Forge theme - Deep crimson and molten gold (Default)
    pub fn dragon_forge() -> Self {
        Self {
            id: ThemeId::DragonForge,
            name: "Dragon Forge",
            description: "Deep crimson and molten gold with volcanic atmosphere",
            colors: ThemeColors {
                // Backgrounds - volcanic depths
                bg_void: rgba_from_hex(0x050508),
                bg_primary: rgba_from_hex(0x0d0a0a),
                bg_secondary: rgba_from_hex(0x1a1414),
                bg_tertiary: rgba_from_hex(0x241c1c),
                bg_hover: rgba_from_hex(0x2d2222),
                bg_selected: rgba_from_hex_alpha(0xd43f3f, 0.2),
                bg_active: rgba_from_hex_alpha(0xd43f3f, 0.3),

                // Text - warm parchment tones
                text_primary: rgba_from_hex(0xf4e8dc),
                text_secondary: rgba_from_hex(0xc9b8a8),
                text_muted: rgba_from_hex(0x8b7b6b),
                text_inverse: rgba_from_hex(0x0d0a0a),

                // Accents - crimson and gold
                accent_primary: rgba_from_hex(0xd43f3f),
                accent_secondary: rgba_from_hex(0xf4b842),
                accent_glow: rgba_from_hex_alpha(0xd43f3f, 0.4),

                // Semantic
                success: rgba_from_hex(0x4ade80),
                warning: rgba_from_hex(0xf4b842),
                error: rgba_from_hex(0xef4444),
                info: rgba_from_hex(0x60a5fa),

                // Borders - gold trimmed
                border_subtle: rgba_from_hex(0x2d2222),
                border_default: rgba_from_hex(0x3d2d2d),
                border_emphasis: rgba_from_hex(0xf4b842),
                border_ornate: rgba_from_hex(0xf4b842),

                // File colors
                folder_color: rgba_from_hex(0xf4b842),
                folder_open_color: rgba_from_hex(0xffd700),
                file_code: rgba_from_hex(0x60a5fa),
                file_data: rgba_from_hex(0x4ade80),
                file_media: rgba_from_hex(0xf472b6),
                file_archive: rgba_from_hex(0xa78bfa),
                file_document: rgba_from_hex(0xfbbf24),

                // Terminal
                terminal_bg: rgba_from_hex(0x0d0a0a),
                terminal_fg: rgba_from_hex(0xf4e8dc),
                terminal_cursor: rgba_from_hex(0xf4b842),
                terminal_selection: rgba_from_hex_alpha(0xd43f3f, 0.3),
                terminal_black: rgba_from_hex(0x1a1414),
                terminal_red: rgba_from_hex(0xd43f3f),
                terminal_green: rgba_from_hex(0x4ade80),
                terminal_yellow: rgba_from_hex(0xf4b842),
                terminal_blue: rgba_from_hex(0x60a5fa),
                terminal_magenta: rgba_from_hex(0xf472b6),
                terminal_cyan: rgba_from_hex(0x22d3ee),
                terminal_white: rgba_from_hex(0xf4e8dc),
            },
            typography: ThemeTypography {
                font_display: "Playfair Display",
                font_display_weight_light: 200,
                font_display_weight_bold: 900,
                ..ThemeTypography::default()
            },
            decorations: ThemeDecorations {
                border_radius_sm: 4.0,
                border_radius_md: 8.0,
                border_radius_lg: 12.0,
                border_width: 1.0,
                use_ornate_borders: true,
                corner_flourish: Some(CornerFlourish {
                    svg_path: "M0,0 Q8,0 8,8 L8,16 Q8,8 16,8 L24,8",
                    size: 24.0,
                    color_mode: FlourishColorMode::Accent,
                }),
                divider_style: DividerStyle::Ornate,
                frame_style: FrameStyle::Ornate,
                shadow_sm: ShadowConfig::new(0.0, 2.0, 4.0, 0.0, rgba_from_hex_alpha(0x000000, 0.3)),
                shadow_md: ShadowConfig::new(0.0, 4.0, 8.0, 0.0, rgba_from_hex_alpha(0x000000, 0.4)),
                shadow_lg: ShadowConfig::new(0.0, 8.0, 16.0, 0.0, rgba_from_hex_alpha(0x000000, 0.5)),
                shadow_glow: ShadowConfig::new(0.0, 0.0, 16.0, 4.0, rgba_from_hex_alpha(0xd43f3f, 0.3)),
                shadow_inner: ShadowConfig::new(0.0, 2.0, 4.0, 0.0, rgba_from_hex_alpha(0x000000, 0.2)),
                bg_pattern: Some(BackgroundPattern::Volcanic),
                bg_noise_opacity: 0.03,
            },
        }
    }

    /// Frost Haven theme - Ice blues and aurora purples
    pub fn frost_haven() -> Self {
        Self {
            id: ThemeId::FrostHaven,
            name: "Frost Haven",
            description: "Ice blues and aurora purples with crystalline elegance",
            colors: ThemeColors {
                // Backgrounds - frozen depths
                bg_void: rgba_from_hex(0x030810),
                bg_primary: rgba_from_hex(0x0a1628),
                bg_secondary: rgba_from_hex(0x122240),
                bg_tertiary: rgba_from_hex(0x1a2d50),
                bg_hover: rgba_from_hex(0x223860),
                bg_selected: rgba_from_hex_alpha(0x6bd4ff, 0.2),
                bg_active: rgba_from_hex_alpha(0x6bd4ff, 0.3),

                // Text - frost white
                text_primary: rgba_from_hex(0xe8f4ff),
                text_secondary: rgba_from_hex(0xb8d4f0),
                text_muted: rgba_from_hex(0x6b8bb0),
                text_inverse: rgba_from_hex(0x0a1628),

                // Accents - ice and aurora
                accent_primary: rgba_from_hex(0x6bd4ff),
                accent_secondary: rgba_from_hex(0xb48aff),
                accent_glow: rgba_from_hex_alpha(0x6bd4ff, 0.4),

                // Semantic
                success: rgba_from_hex(0x4ade80),
                warning: rgba_from_hex(0xfbbf24),
                error: rgba_from_hex(0xf87171),
                info: rgba_from_hex(0x6bd4ff),

                // Borders - crystalline
                border_subtle: rgba_from_hex(0x1a2d50),
                border_default: rgba_from_hex(0x2a4070),
                border_emphasis: rgba_from_hex(0x6bd4ff),
                border_ornate: rgba_from_hex(0xb48aff),

                // File colors
                folder_color: rgba_from_hex(0x6bd4ff),
                folder_open_color: rgba_from_hex(0x8be0ff),
                file_code: rgba_from_hex(0xb48aff),
                file_data: rgba_from_hex(0x4ade80),
                file_media: rgba_from_hex(0xf472b6),
                file_archive: rgba_from_hex(0xa78bfa),
                file_document: rgba_from_hex(0xfbbf24),

                // Terminal
                terminal_bg: rgba_from_hex(0x0a1628),
                terminal_fg: rgba_from_hex(0xe8f4ff),
                terminal_cursor: rgba_from_hex(0x6bd4ff),
                terminal_selection: rgba_from_hex_alpha(0x6bd4ff, 0.3),
                terminal_black: rgba_from_hex(0x122240),
                terminal_red: rgba_from_hex(0xf87171),
                terminal_green: rgba_from_hex(0x4ade80),
                terminal_yellow: rgba_from_hex(0xfbbf24),
                terminal_blue: rgba_from_hex(0x6bd4ff),
                terminal_magenta: rgba_from_hex(0xb48aff),
                terminal_cyan: rgba_from_hex(0x22d3ee),
                terminal_white: rgba_from_hex(0xe8f4ff),
            },
            typography: ThemeTypography {
                font_display: "Bricolage Grotesque",
                font_display_weight_light: 300,
                font_display_weight_bold: 800,
                ..ThemeTypography::default()
            },
            decorations: ThemeDecorations {
                border_radius_sm: 6.0,
                border_radius_md: 10.0,
                border_radius_lg: 16.0,
                border_width: 1.0,
                use_ornate_borders: true,
                corner_flourish: Some(CornerFlourish {
                    svg_path: "M0,8 L8,0 L16,8 L8,16 Z",
                    size: 16.0,
                    color_mode: FlourishColorMode::Gradient,
                }),
                divider_style: DividerStyle::Gradient,
                frame_style: FrameStyle::Beveled,
                shadow_sm: ShadowConfig::new(0.0, 2.0, 4.0, 0.0, rgba_from_hex_alpha(0x000020, 0.3)),
                shadow_md: ShadowConfig::new(0.0, 4.0, 8.0, 0.0, rgba_from_hex_alpha(0x000020, 0.4)),
                shadow_lg: ShadowConfig::new(0.0, 8.0, 16.0, 0.0, rgba_from_hex_alpha(0x000020, 0.5)),
                shadow_glow: ShadowConfig::new(0.0, 0.0, 20.0, 6.0, rgba_from_hex_alpha(0x6bd4ff, 0.25)),
                shadow_inner: ShadowConfig::new(0.0, 2.0, 4.0, 0.0, rgba_from_hex_alpha(0x000020, 0.2)),
                bg_pattern: Some(BackgroundPattern::Crystalline),
                bg_noise_opacity: 0.02,
            },
        }
    }


    /// Ancient Tome theme - Parchment and leather browns
    pub fn ancient_tome() -> Self {
        Self {
            id: ThemeId::AncientTome,
            name: "Ancient Tome",
            description: "Parchment textures with leather browns and gold leaf",
            colors: ThemeColors {
                // Backgrounds - aged parchment
                bg_void: rgba_from_hex(0x1a1510),
                bg_primary: rgba_from_hex(0x2a2318),
                bg_secondary: rgba_from_hex(0x3a3020),
                bg_tertiary: rgba_from_hex(0x4a3d28),
                bg_hover: rgba_from_hex(0x5a4a30),
                bg_selected: rgba_from_hex_alpha(0xd4af37, 0.2),
                bg_active: rgba_from_hex_alpha(0xd4af37, 0.3),

                // Text - ink on parchment
                text_primary: rgba_from_hex(0xf5e6c8),
                text_secondary: rgba_from_hex(0xd4c4a8),
                text_muted: rgba_from_hex(0x8b7b5b),
                text_inverse: rgba_from_hex(0x1a1510),

                // Accents - gold leaf and leather
                accent_primary: rgba_from_hex(0xd4af37),
                accent_secondary: rgba_from_hex(0x8b4513),
                accent_glow: rgba_from_hex_alpha(0xd4af37, 0.3),

                // Semantic
                success: rgba_from_hex(0x6b8e23),
                warning: rgba_from_hex(0xd4af37),
                error: rgba_from_hex(0x8b0000),
                info: rgba_from_hex(0x4682b4),

                // Borders - embossed leather
                border_subtle: rgba_from_hex(0x3a3020),
                border_default: rgba_from_hex(0x5a4a30),
                border_emphasis: rgba_from_hex(0xd4af37),
                border_ornate: rgba_from_hex(0xd4af37),

                // File colors
                folder_color: rgba_from_hex(0x8b4513),
                folder_open_color: rgba_from_hex(0xa0522d),
                file_code: rgba_from_hex(0x4682b4),
                file_data: rgba_from_hex(0x6b8e23),
                file_media: rgba_from_hex(0x8b4513),
                file_archive: rgba_from_hex(0x654321),
                file_document: rgba_from_hex(0xd4af37),

                // Terminal
                terminal_bg: rgba_from_hex(0x1a1510),
                terminal_fg: rgba_from_hex(0xf5e6c8),
                terminal_cursor: rgba_from_hex(0xd4af37),
                terminal_selection: rgba_from_hex_alpha(0xd4af37, 0.3),
                terminal_black: rgba_from_hex(0x2a2318),
                terminal_red: rgba_from_hex(0x8b0000),
                terminal_green: rgba_from_hex(0x6b8e23),
                terminal_yellow: rgba_from_hex(0xd4af37),
                terminal_blue: rgba_from_hex(0x4682b4),
                terminal_magenta: rgba_from_hex(0x8b4513),
                terminal_cyan: rgba_from_hex(0x5f9ea0),
                terminal_white: rgba_from_hex(0xf5e6c8),
            },
            typography: ThemeTypography {
                font_display: "Newsreader",
                font_display_weight_light: 300,
                font_display_weight_bold: 800,
                font_body: "Source Sans 3",
                ..ThemeTypography::default()
            },
            decorations: ThemeDecorations {
                border_radius_sm: 2.0,
                border_radius_md: 4.0,
                border_radius_lg: 8.0,
                border_width: 2.0,
                use_ornate_borders: true,
                corner_flourish: Some(CornerFlourish {
                    svg_path: "M0,0 C4,0 8,4 8,8 C8,4 12,0 16,0",
                    size: 20.0,
                    color_mode: FlourishColorMode::Border,
                }),
                divider_style: DividerStyle::Embossed,
                frame_style: FrameStyle::Double,
                shadow_sm: ShadowConfig::new(2.0, 2.0, 4.0, 0.0, rgba_from_hex_alpha(0x000000, 0.4)),
                shadow_md: ShadowConfig::new(3.0, 3.0, 6.0, 0.0, rgba_from_hex_alpha(0x000000, 0.5)),
                shadow_lg: ShadowConfig::new(4.0, 4.0, 12.0, 0.0, rgba_from_hex_alpha(0x000000, 0.6)),
                shadow_glow: ShadowConfig::new(0.0, 0.0, 12.0, 2.0, rgba_from_hex_alpha(0xd4af37, 0.2)),
                shadow_inner: ShadowConfig::new(1.0, 1.0, 2.0, 0.0, rgba_from_hex_alpha(0x000000, 0.3)),
                bg_pattern: Some(BackgroundPattern::Parchment),
                bg_noise_opacity: 0.05,
            },
        }
    }

    /// Shadow Realm theme - Deep purples and ethereal glows
    pub fn shadow_realm() -> Self {
        Self {
            id: ThemeId::ShadowRealm,
            name: "Shadow Realm",
            description: "Deep purples with ethereal glows and void blacks",
            colors: ThemeColors {
                // Backgrounds - void depths
                bg_void: rgba_from_hex(0x050508),
                bg_primary: rgba_from_hex(0x0a0a14),
                bg_secondary: rgba_from_hex(0x14142a),
                bg_tertiary: rgba_from_hex(0x1e1e3a),
                bg_hover: rgba_from_hex(0x28284a),
                bg_selected: rgba_from_hex_alpha(0x9966ff, 0.2),
                bg_active: rgba_from_hex_alpha(0x9966ff, 0.3),

                // Text - ethereal glow
                text_primary: rgba_from_hex(0xe8e0ff),
                text_secondary: rgba_from_hex(0xb8a8e0),
                text_muted: rgba_from_hex(0x6b5b8b),
                text_inverse: rgba_from_hex(0x050508),

                // Accents - mystical purple
                accent_primary: rgba_from_hex(0x9966ff),
                accent_secondary: rgba_from_hex(0x4a0080),
                accent_glow: rgba_from_hex_alpha(0x9966ff, 0.5),

                // Semantic
                success: rgba_from_hex(0x00ff88),
                warning: rgba_from_hex(0xff9900),
                error: rgba_from_hex(0xff3366),
                info: rgba_from_hex(0x00ccff),

                // Borders - ethereal
                border_subtle: rgba_from_hex(0x1e1e3a),
                border_default: rgba_from_hex(0x3a3a5a),
                border_emphasis: rgba_from_hex(0x9966ff),
                border_ornate: rgba_from_hex(0x9966ff),

                // File colors
                folder_color: rgba_from_hex(0x9966ff),
                folder_open_color: rgba_from_hex(0xb388ff),
                file_code: rgba_from_hex(0x00ccff),
                file_data: rgba_from_hex(0x00ff88),
                file_media: rgba_from_hex(0xff66aa),
                file_archive: rgba_from_hex(0x6633cc),
                file_document: rgba_from_hex(0xff9900),

                // Terminal
                terminal_bg: rgba_from_hex(0x050508),
                terminal_fg: rgba_from_hex(0xe8e0ff),
                terminal_cursor: rgba_from_hex(0x9966ff),
                terminal_selection: rgba_from_hex_alpha(0x9966ff, 0.3),
                terminal_black: rgba_from_hex(0x14142a),
                terminal_red: rgba_from_hex(0xff3366),
                terminal_green: rgba_from_hex(0x00ff88),
                terminal_yellow: rgba_from_hex(0xff9900),
                terminal_blue: rgba_from_hex(0x00ccff),
                terminal_magenta: rgba_from_hex(0x9966ff),
                terminal_cyan: rgba_from_hex(0x00ffcc),
                terminal_white: rgba_from_hex(0xe8e0ff),
            },
            typography: ThemeTypography {
                font_display: "Crimson Pro",
                font_display_weight_light: 200,
                font_display_weight_bold: 900,
                ..ThemeTypography::default()
            },
            decorations: ThemeDecorations {
                border_radius_sm: 4.0,
                border_radius_md: 8.0,
                border_radius_lg: 12.0,
                border_width: 1.0,
                use_ornate_borders: true,
                corner_flourish: Some(CornerFlourish {
                    svg_path: "M0,12 Q6,6 12,0 Q6,6 0,12",
                    size: 18.0,
                    color_mode: FlourishColorMode::Gradient,
                }),
                divider_style: DividerStyle::Runic,
                frame_style: FrameStyle::Inset,
                shadow_sm: ShadowConfig::new(0.0, 2.0, 6.0, 0.0, rgba_from_hex_alpha(0x000000, 0.5)),
                shadow_md: ShadowConfig::new(0.0, 4.0, 12.0, 0.0, rgba_from_hex_alpha(0x000000, 0.6)),
                shadow_lg: ShadowConfig::new(0.0, 8.0, 24.0, 0.0, rgba_from_hex_alpha(0x000000, 0.7)),
                shadow_glow: ShadowConfig::new(0.0, 0.0, 24.0, 8.0, rgba_from_hex_alpha(0x9966ff, 0.4)),
                shadow_inner: ShadowConfig::new(0.0, 2.0, 6.0, 0.0, rgba_from_hex_alpha(0x000000, 0.4)),
                bg_pattern: Some(BackgroundPattern::Mystical),
                bg_noise_opacity: 0.04,
            },
        }
    }

    /// Elven Glade theme - Forest greens and moonlight silver
    pub fn elven_glade() -> Self {
        Self {
            id: ThemeId::ElvenGlade,
            name: "Elven Glade",
            description: "Forest greens with moonlight silver and organic patterns",
            colors: ThemeColors {
                // Backgrounds - forest depths
                bg_void: rgba_from_hex(0x050a08),
                bg_primary: rgba_from_hex(0x0a1810),
                bg_secondary: rgba_from_hex(0x142820),
                bg_tertiary: rgba_from_hex(0x1e3830),
                bg_hover: rgba_from_hex(0x284840),
                bg_selected: rgba_from_hex_alpha(0x228b22, 0.2),
                bg_active: rgba_from_hex_alpha(0x228b22, 0.3),

                // Text - moonlight silver
                text_primary: rgba_from_hex(0xe8f0e8),
                text_secondary: rgba_from_hex(0xc0c0c0),
                text_muted: rgba_from_hex(0x6b8b6b),
                text_inverse: rgba_from_hex(0x050a08),

                // Accents - forest and moonlight
                accent_primary: rgba_from_hex(0x228b22),
                accent_secondary: rgba_from_hex(0xc0c0c0),
                accent_glow: rgba_from_hex_alpha(0x228b22, 0.3),

                // Semantic
                success: rgba_from_hex(0x32cd32),
                warning: rgba_from_hex(0xdaa520),
                error: rgba_from_hex(0xcd5c5c),
                info: rgba_from_hex(0x87ceeb),

                // Borders - organic
                border_subtle: rgba_from_hex(0x1e3830),
                border_default: rgba_from_hex(0x3a5a4a),
                border_emphasis: rgba_from_hex(0x228b22),
                border_ornate: rgba_from_hex(0xc0c0c0),

                // File colors
                folder_color: rgba_from_hex(0x228b22),
                folder_open_color: rgba_from_hex(0x32cd32),
                file_code: rgba_from_hex(0x87ceeb),
                file_data: rgba_from_hex(0x32cd32),
                file_media: rgba_from_hex(0xdaa520),
                file_archive: rgba_from_hex(0x8b4513),
                file_document: rgba_from_hex(0xc0c0c0),

                // Terminal
                terminal_bg: rgba_from_hex(0x050a08),
                terminal_fg: rgba_from_hex(0xe8f0e8),
                terminal_cursor: rgba_from_hex(0xc0c0c0),
                terminal_selection: rgba_from_hex_alpha(0x228b22, 0.3),
                terminal_black: rgba_from_hex(0x142820),
                terminal_red: rgba_from_hex(0xcd5c5c),
                terminal_green: rgba_from_hex(0x32cd32),
                terminal_yellow: rgba_from_hex(0xdaa520),
                terminal_blue: rgba_from_hex(0x87ceeb),
                terminal_magenta: rgba_from_hex(0xba55d3),
                terminal_cyan: rgba_from_hex(0x20b2aa),
                terminal_white: rgba_from_hex(0xe8f0e8),
            },
            typography: ThemeTypography::default(),
            decorations: ThemeDecorations {
                border_radius_sm: 8.0,
                border_radius_md: 12.0,
                border_radius_lg: 20.0,
                border_width: 1.0,
                use_ornate_borders: true,
                corner_flourish: Some(CornerFlourish {
                    svg_path: "M0,16 C8,16 16,8 16,0 C16,8 24,16 32,16",
                    size: 32.0,
                    color_mode: FlourishColorMode::Accent,
                }),
                divider_style: DividerStyle::Gradient,
                frame_style: FrameStyle::Simple,
                shadow_sm: ShadowConfig::new(0.0, 2.0, 4.0, 0.0, rgba_from_hex_alpha(0x000000, 0.25)),
                shadow_md: ShadowConfig::new(0.0, 4.0, 8.0, 0.0, rgba_from_hex_alpha(0x000000, 0.35)),
                shadow_lg: ShadowConfig::new(0.0, 8.0, 16.0, 0.0, rgba_from_hex_alpha(0x000000, 0.45)),
                shadow_glow: ShadowConfig::new(0.0, 0.0, 16.0, 4.0, rgba_from_hex_alpha(0xc0c0c0, 0.2)),
                shadow_inner: ShadowConfig::new(0.0, 1.0, 3.0, 0.0, rgba_from_hex_alpha(0x000000, 0.2)),
                bg_pattern: Some(BackgroundPattern::Organic),
                bg_noise_opacity: 0.02,
            },
        }
    }

    /// Get theme by ID
    pub fn from_id(id: ThemeId) -> Self {
        match id {
            ThemeId::DragonForge => Self::dragon_forge(),
            ThemeId::FrostHaven => Self::frost_haven(),
            ThemeId::AncientTome => Self::ancient_tome(),
            ThemeId::ShadowRealm => Self::shadow_realm(),
            ThemeId::ElvenGlade => Self::elven_glade(),
        }
    }

    /// Get all available themes
    pub fn all_themes() -> Vec<Self> {
        vec![
            Self::dragon_forge(),
            Self::frost_haven(),
            Self::ancient_tome(),
            Self::shadow_realm(),
            Self::elven_glade(),
        ]
    }
}


/// Theme manager for handling theme selection and persistence
#[derive(Clone)]
pub struct ThemeManager {
    current_id: ThemeId,
    themes: HashMap<ThemeId, Theme>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut themes = HashMap::new();
        for theme in Theme::all_themes() {
            themes.insert(theme.id, theme);
        }

        Self {
            current_id: ThemeId::default(),
            themes,
        }
    }

    pub fn current_theme(&self) -> &Theme {
        self.themes.get(&self.current_id).unwrap_or_else(|| {
            self.themes.get(&ThemeId::DragonForge).expect("Default theme must exist")
        })
    }

    pub fn current_id(&self) -> ThemeId {
        self.current_id
    }

    pub fn set_theme(&mut self, id: ThemeId) {
        if self.themes.contains_key(&id) {
            self.current_id = id;
        }
    }

    pub fn available_themes(&self) -> Vec<&Theme> {
        self.themes.values().collect()
    }

    pub fn theme_by_id(&self, id: ThemeId) -> Option<&Theme> {
        self.themes.get(&id)
    }

    /// Save theme selection to config file
    pub fn save(&self) -> std::io::Result<()> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("nexus-explorer");
        
        std::fs::create_dir_all(&config_dir)?;
        
        let config_path = config_dir.join("theme.json");
        let config = ThemeConfig {
            theme_id: self.current_id,
        };
        
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        std::fs::write(config_path, json)
    }

    /// Load theme selection from config file
    pub fn load() -> std::io::Result<Self> {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("nexus-explorer")
            .join("theme.json");
        
        let mut manager = Self::new();
        
        if config_path.exists() {
            let json = std::fs::read_to_string(config_path)?;
            if let Ok(config) = serde_json::from_str::<ThemeConfig>(&json) {
                manager.current_id = config.theme_id;
            }
        }
        
        Ok(manager)
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Global for ThemeManager {}

/// Serializable theme configuration
#[derive(Serialize, Deserialize)]
struct ThemeConfig {
    theme_id: ThemeId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_theme_manager_default() {
        let manager = ThemeManager::new();
        assert_eq!(manager.current_id(), ThemeId::DragonForge);
    }

    #[test]
    fn test_theme_manager_set_theme() {
        let mut manager = ThemeManager::new();
        manager.set_theme(ThemeId::FrostHaven);
        assert_eq!(manager.current_id(), ThemeId::FrostHaven);
    }

    #[test]
    fn test_all_themes_available() {
        let manager = ThemeManager::new();
        let themes = manager.available_themes();
        assert_eq!(themes.len(), 5);
    }

    #[test]
    fn test_dragon_forge_colors_complete() {
        let theme = Theme::dragon_forge();
        assert!(theme.colors.is_complete());
    }

    #[test]
    fn test_frost_haven_colors_complete() {
        let theme = Theme::frost_haven();
        assert!(theme.colors.is_complete());
    }

    #[test]
    fn test_ancient_tome_colors_complete() {
        let theme = Theme::ancient_tome();
        assert!(theme.colors.is_complete());
    }

    #[test]
    fn test_shadow_realm_colors_complete() {
        let theme = Theme::shadow_realm();
        assert!(theme.colors.is_complete());
    }

    #[test]
    fn test_elven_glade_colors_complete() {
        let theme = Theme::elven_glade();
        assert!(theme.colors.is_complete());
    }

    fn arb_theme_id() -> impl Strategy<Value = ThemeId> {
        prop_oneof![
            Just(ThemeId::DragonForge),
            Just(ThemeId::FrostHaven),
            Just(ThemeId::AncientTome),
            Just(ThemeId::ShadowRealm),
            Just(ThemeId::ElvenGlade),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: ui-enhancements, Property 36: Theme Color Completeness**
        /// **Validates: Requirements 14.4**
        ///
        /// *For any* theme, all required color fields SHALL be set (non-transparent)
        /// to ensure consistent UI rendering.
        #[test]
        fn prop_theme_color_completeness(theme_id in arb_theme_id()) {
            let theme = Theme::from_id(theme_id);
            
            // All themes must have complete color definitions
            prop_assert!(
                theme.colors.is_complete(),
                "Theme {:?} must have complete color definitions",
                theme_id
            );
            
            // Verify specific critical colors are set
            prop_assert!(
                theme.colors.bg_primary.a > 0.0,
                "Theme {:?} must have bg_primary set",
                theme_id
            );
            prop_assert!(
                theme.colors.text_primary.a > 0.0,
                "Theme {:?} must have text_primary set",
                theme_id
            );
            prop_assert!(
                theme.colors.accent_primary.a > 0.0,
                "Theme {:?} must have accent_primary set",
                theme_id
            );
        }

        /// **Feature: ui-enhancements, Property 35: Theme Persistence**
        /// **Validates: Requirements 14.1, 14.10**
        ///
        /// *For any* theme selection, setting and getting the theme SHALL return
        /// the same theme ID, ensuring persistence consistency.
        #[test]
        fn prop_theme_persistence_round_trip(theme_id in arb_theme_id()) {
            let mut manager = ThemeManager::new();
            
            // Set the theme
            manager.set_theme(theme_id);
            
            // Verify the theme was set correctly
            prop_assert_eq!(
                manager.current_id(),
                theme_id,
                "Theme ID should match after setting"
            );
            
            // Verify we can retrieve the theme
            let theme = manager.current_theme();
            prop_assert_eq!(
                theme.id,
                theme_id,
                "Retrieved theme ID should match"
            );
        }

        /// **Feature: ui-enhancements, Property 37: Theme Application Consistency**
        /// **Validates: Requirements 14.3, 14.4**
        ///
        /// *For any* theme, the theme SHALL have consistent typography and decoration
        /// settings that are valid for rendering.
        #[test]
        fn prop_theme_application_consistency(theme_id in arb_theme_id()) {
            let theme = Theme::from_id(theme_id);
            
            // Typography must have valid font sizes
            prop_assert!(
                theme.typography.size_base > 0.0,
                "Theme {:?} must have positive base font size",
                theme_id
            );
            prop_assert!(
                theme.typography.size_sm < theme.typography.size_base,
                "Theme {:?} size_sm should be smaller than size_base",
                theme_id
            );
            prop_assert!(
                theme.typography.size_base < theme.typography.size_lg,
                "Theme {:?} size_base should be smaller than size_lg",
                theme_id
            );
            
            // Decorations must have valid border radii
            prop_assert!(
                theme.decorations.border_radius_sm >= 0.0,
                "Theme {:?} must have non-negative border_radius_sm",
                theme_id
            );
            prop_assert!(
                theme.decorations.border_radius_sm <= theme.decorations.border_radius_md,
                "Theme {:?} border_radius_sm should be <= border_radius_md",
                theme_id
            );
            prop_assert!(
                theme.decorations.border_radius_md <= theme.decorations.border_radius_lg,
                "Theme {:?} border_radius_md should be <= border_radius_lg",
                theme_id
            );
        }
    }
}


/// Helper trait for getting theme colors as gpui::Rgba
impl ThemeColors {
    /// Get background color for void/deepest areas
    pub fn bg_void_color(&self) -> gpui::Rgba {
        self.bg_void
    }

    /// Get primary background color
    pub fn bg_primary_color(&self) -> gpui::Rgba {
        self.bg_primary
    }

    /// Get secondary background color
    pub fn bg_secondary_color(&self) -> gpui::Rgba {
        self.bg_secondary
    }

    /// Get tertiary background color
    pub fn bg_tertiary_color(&self) -> gpui::Rgba {
        self.bg_tertiary
    }

    /// Get hover background color
    pub fn bg_hover_color(&self) -> gpui::Rgba {
        self.bg_hover
    }

    /// Get selected background color
    pub fn bg_selected_color(&self) -> gpui::Rgba {
        self.bg_selected
    }

    /// Get primary text color
    pub fn text_primary_color(&self) -> gpui::Rgba {
        self.text_primary
    }

    /// Get secondary text color
    pub fn text_secondary_color(&self) -> gpui::Rgba {
        self.text_secondary
    }

    /// Get muted text color
    pub fn text_muted_color(&self) -> gpui::Rgba {
        self.text_muted
    }

    /// Get primary accent color
    pub fn accent_primary_color(&self) -> gpui::Rgba {
        self.accent_primary
    }

    /// Get secondary accent color
    pub fn accent_secondary_color(&self) -> gpui::Rgba {
        self.accent_secondary
    }

    /// Get default border color
    pub fn border_default_color(&self) -> gpui::Rgba {
        self.border_default
    }

    /// Get subtle border color
    pub fn border_subtle_color(&self) -> gpui::Rgba {
        self.border_subtle
    }

    /// Get emphasis border color
    pub fn border_emphasis_color(&self) -> gpui::Rgba {
        self.border_emphasis
    }

    /// Get folder color
    pub fn folder_color_value(&self) -> gpui::Rgba {
        self.folder_color
    }
}

/// Current theme accessor - provides the active theme for UI components
/// This is a simple static accessor that returns the default theme.
/// In a full implementation, this would read from a global state.
pub fn current_theme() -> Theme {
    // For now, return the default Dragon Forge theme
    // In production, this would read from ThemeManager global state
    Theme::dragon_forge()
}

/// Get the current theme colors
pub fn theme_colors() -> ThemeColors {
    current_theme().colors
}

/// Get the current theme typography
pub fn theme_typography() -> ThemeTypography {
    current_theme().typography
}

/// Get the current theme decorations
pub fn theme_decorations() -> ThemeDecorations {
    current_theme().decorations
}
