use gpui::{Global, Rgba};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;


#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ThemeId {
    Light,
    Dark,
    DragonForge,
    FrostHaven,
    AncientTome,
    ShadowRealm,
    ElvenGlade,
}

impl Default for ThemeId {
    fn default() -> Self {
        Self::Dark
    }
}


#[derive(Clone, Debug)]
pub struct Theme {
    pub id: ThemeId,
    pub name: &'static str,
    pub description: &'static str,
    pub colors: ThemeColors,
    pub typography: ThemeTypography,
    pub decorations: ThemeDecorations,
}


#[derive(Clone, Debug)]
pub struct ThemeColors {
    pub bg_void: Rgba,
    pub bg_primary: Rgba,
    pub bg_secondary: Rgba,
    pub bg_tertiary: Rgba,
    pub bg_hover: Rgba,
    pub bg_selected: Rgba,
    pub bg_active: Rgba,

    pub text_primary: Rgba,
    pub text_secondary: Rgba,
    pub text_muted: Rgba,
    pub text_inverse: Rgba,

    pub accent_primary: Rgba,
    pub accent_secondary: Rgba,
    pub accent_glow: Rgba,

    pub success: Rgba,
    pub warning: Rgba,
    pub error: Rgba,
    pub info: Rgba,

    pub border_subtle: Rgba,
    pub border_default: Rgba,
    pub border_emphasis: Rgba,
    pub border_ornate: Rgba,

    pub folder_color: Rgba,
    pub folder_open_color: Rgba,
    pub file_code: Rgba,
    pub file_data: Rgba,
    pub file_media: Rgba,
    pub file_archive: Rgba,
    pub file_document: Rgba,

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

    pub size_xs: f32,
    pub size_sm: f32,
    pub size_base: f32,
    pub size_lg: f32,
    pub size_xl: f32,
    pub size_2xl: f32,
    pub size_3xl: f32,

    pub tracking_tight: f32,
    pub tracking_normal: f32,
    pub tracking_wide: f32,
}


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
            color: Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
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


pub fn rgba_from_hex(hex: u32) -> Rgba {
    let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let b = (hex & 0xFF) as f32 / 255.0;
    Rgba { r, g, b, a: 1.0 }
}


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

    pub fn is_complete(&self) -> bool {
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

    pub fn light() -> Self {
        Self {
            id: ThemeId::Light,
            name: "Light",
            description: "Clean and minimal light theme",
            colors: ThemeColors {
                bg_void: rgba_from_hex(0xf5f5f5),
                bg_primary: rgba_from_hex(0xffffff),
                bg_secondary: rgba_from_hex(0xf8f8f8),
                bg_tertiary: rgba_from_hex(0xf0f0f0),
                bg_hover: rgba_from_hex(0xe8e8e8),
                bg_selected: rgba_from_hex_alpha(0x0066cc, 0.1),
                bg_active: rgba_from_hex_alpha(0x0066cc, 0.15),

                text_primary: rgba_from_hex(0x1a1a1a),
                text_secondary: rgba_from_hex(0x4a4a4a),
                text_muted: rgba_from_hex(0x8a8a8a),
                text_inverse: rgba_from_hex(0xffffff),

                accent_primary: rgba_from_hex(0x0066cc),
                accent_secondary: rgba_from_hex(0x0088ff),
                accent_glow: rgba_from_hex_alpha(0x0066cc, 0.2),

                success: rgba_from_hex(0x22c55e),
                warning: rgba_from_hex(0xf59e0b),
                error: rgba_from_hex(0xef4444),
                info: rgba_from_hex(0x3b82f6),

                border_subtle: rgba_from_hex(0xe5e5e5),
                border_default: rgba_from_hex(0xd4d4d4),
                border_emphasis: rgba_from_hex(0x0066cc),
                border_ornate: rgba_from_hex(0x0066cc),

                folder_color: rgba_from_hex(0x0066cc),
                folder_open_color: rgba_from_hex(0x0088ff),
                file_code: rgba_from_hex(0x7c3aed),
                file_data: rgba_from_hex(0x22c55e),
                file_media: rgba_from_hex(0xec4899),
                file_archive: rgba_from_hex(0xf59e0b),
                file_document: rgba_from_hex(0x3b82f6),

                terminal_bg: rgba_from_hex(0xffffff),
                terminal_fg: rgba_from_hex(0x1a1a1a),
                terminal_cursor: rgba_from_hex(0x0066cc),
                terminal_selection: rgba_from_hex_alpha(0x0066cc, 0.2),
                terminal_black: rgba_from_hex(0x1a1a1a),
                terminal_red: rgba_from_hex(0xdc2626),
                terminal_green: rgba_from_hex(0x16a34a),
                terminal_yellow: rgba_from_hex(0xca8a04),
                terminal_blue: rgba_from_hex(0x2563eb),
                terminal_magenta: rgba_from_hex(0x9333ea),
                terminal_cyan: rgba_from_hex(0x0891b2),
                terminal_white: rgba_from_hex(0xf5f5f5),
            },
            typography: ThemeTypography::default(),
            decorations: ThemeDecorations {
                border_radius_sm: 4.0,
                border_radius_md: 6.0,
                border_radius_lg: 8.0,
                border_width: 1.0,
                use_ornate_borders: false,
                corner_flourish: None,
                divider_style: DividerStyle::Simple,
                frame_style: FrameStyle::Simple,
                shadow_sm: ShadowConfig::new(
                    0.0,
                    1.0,
                    2.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.05),
                ),
                shadow_md: ShadowConfig::new(
                    0.0,
                    2.0,
                    4.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.1),
                ),
                shadow_lg: ShadowConfig::new(
                    0.0,
                    4.0,
                    8.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.15),
                ),
                shadow_glow: ShadowConfig::none(),
                shadow_inner: ShadowConfig::none(),
                bg_pattern: None,
                bg_noise_opacity: 0.0,
            },
        }
    }


    pub fn dark() -> Self {
        Self {
            id: ThemeId::Dark,
            name: "Dark",
            description: "Clean and minimal dark theme",
            colors: ThemeColors {
                bg_void: rgba_from_hex(0x0a0a0a),
                bg_primary: rgba_from_hex(0x141414),
                bg_secondary: rgba_from_hex(0x1e1e1e),
                bg_tertiary: rgba_from_hex(0x282828),
                bg_hover: rgba_from_hex(0x323232),
                bg_selected: rgba_from_hex_alpha(0x3b82f6, 0.2),
                bg_active: rgba_from_hex_alpha(0x3b82f6, 0.3),

                text_primary: rgba_from_hex(0xf5f5f5),
                text_secondary: rgba_from_hex(0xb4b4b4),
                text_muted: rgba_from_hex(0x737373),
                text_inverse: rgba_from_hex(0x141414),

                accent_primary: rgba_from_hex(0x3b82f6),
                accent_secondary: rgba_from_hex(0x60a5fa),
                accent_glow: rgba_from_hex_alpha(0x3b82f6, 0.3),

                success: rgba_from_hex(0x22c55e),
                warning: rgba_from_hex(0xf59e0b),
                error: rgba_from_hex(0xef4444),
                info: rgba_from_hex(0x3b82f6),

                border_subtle: rgba_from_hex(0x282828),
                border_default: rgba_from_hex(0x3f3f3f),
                border_emphasis: rgba_from_hex(0x3b82f6),
                border_ornate: rgba_from_hex(0x3b82f6),

                folder_color: rgba_from_hex(0x60a5fa),
                folder_open_color: rgba_from_hex(0x93c5fd),
                file_code: rgba_from_hex(0xa78bfa),
                file_data: rgba_from_hex(0x4ade80),
                file_media: rgba_from_hex(0xf472b6),
                file_archive: rgba_from_hex(0xfbbf24),
                file_document: rgba_from_hex(0x60a5fa),

                terminal_bg: rgba_from_hex(0x141414),
                terminal_fg: rgba_from_hex(0xf5f5f5),
                terminal_cursor: rgba_from_hex(0x3b82f6),
                terminal_selection: rgba_from_hex_alpha(0x3b82f6, 0.3),
                terminal_black: rgba_from_hex(0x1e1e1e),
                terminal_red: rgba_from_hex(0xef4444),
                terminal_green: rgba_from_hex(0x22c55e),
                terminal_yellow: rgba_from_hex(0xfbbf24),
                terminal_blue: rgba_from_hex(0x3b82f6),
                terminal_magenta: rgba_from_hex(0xa855f7),
                terminal_cyan: rgba_from_hex(0x22d3ee),
                terminal_white: rgba_from_hex(0xf5f5f5),
            },
            typography: ThemeTypography::default(),
            decorations: ThemeDecorations {
                border_radius_sm: 4.0,
                border_radius_md: 6.0,
                border_radius_lg: 8.0,
                border_width: 1.0,
                use_ornate_borders: false,
                corner_flourish: None,
                divider_style: DividerStyle::Simple,
                frame_style: FrameStyle::Simple,
                shadow_sm: ShadowConfig::new(
                    0.0,
                    1.0,
                    2.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.2),
                ),
                shadow_md: ShadowConfig::new(
                    0.0,
                    2.0,
                    4.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.3),
                ),
                shadow_lg: ShadowConfig::new(
                    0.0,
                    4.0,
                    8.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.4),
                ),
                shadow_glow: ShadowConfig::none(),
                shadow_inner: ShadowConfig::none(),
                bg_pattern: None,
                bg_noise_opacity: 0.0,
            },
        }
    }


    pub fn dragon_forge() -> Self {
        Self {
            id: ThemeId::DragonForge,
            name: "Dragon Forge",
            description: "Deep crimson and molten gold with volcanic atmosphere",
            colors: ThemeColors {
                bg_void: rgba_from_hex(0x050508),
                bg_primary: rgba_from_hex(0x0d0a0a),
                bg_secondary: rgba_from_hex(0x1a1414),
                bg_tertiary: rgba_from_hex(0x241c1c),
                bg_hover: rgba_from_hex(0x2d2222),
                bg_selected: rgba_from_hex_alpha(0xd43f3f, 0.2),
                bg_active: rgba_from_hex_alpha(0xd43f3f, 0.3),

                text_primary: rgba_from_hex(0xf4e8dc),
                text_secondary: rgba_from_hex(0xc9b8a8),
                text_muted: rgba_from_hex(0x8b7b6b),
                text_inverse: rgba_from_hex(0x0d0a0a),

                accent_primary: rgba_from_hex(0xd43f3f),
                accent_secondary: rgba_from_hex(0xf4b842),
                accent_glow: rgba_from_hex_alpha(0xd43f3f, 0.4),

                success: rgba_from_hex(0x4ade80),
                warning: rgba_from_hex(0xf4b842),
                error: rgba_from_hex(0xef4444),
                info: rgba_from_hex(0x60a5fa),

                border_subtle: rgba_from_hex(0x2d2222),
                border_default: rgba_from_hex(0x3d2d2d),
                border_emphasis: rgba_from_hex(0xf4b842),
                border_ornate: rgba_from_hex(0xf4b842),

                folder_color: rgba_from_hex(0xf4b842),
                folder_open_color: rgba_from_hex(0xffd700),
                file_code: rgba_from_hex(0x60a5fa),
                file_data: rgba_from_hex(0x4ade80),
                file_media: rgba_from_hex(0xf472b6),
                file_archive: rgba_from_hex(0xa78bfa),
                file_document: rgba_from_hex(0xfbbf24),

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
                shadow_sm: ShadowConfig::new(
                    0.0,
                    2.0,
                    4.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.3),
                ),
                shadow_md: ShadowConfig::new(
                    0.0,
                    4.0,
                    8.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.4),
                ),
                shadow_lg: ShadowConfig::new(
                    0.0,
                    8.0,
                    16.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.5),
                ),
                shadow_glow: ShadowConfig::new(
                    0.0,
                    0.0,
                    16.0,
                    4.0,
                    rgba_from_hex_alpha(0xd43f3f, 0.3),
                ),
                shadow_inner: ShadowConfig::new(
                    0.0,
                    2.0,
                    4.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.2),
                ),
                bg_pattern: Some(BackgroundPattern::Volcanic),
                bg_noise_opacity: 0.03,
            },
        }
    }


    pub fn frost_haven() -> Self {
        Self {
            id: ThemeId::FrostHaven,
            name: "Frost Haven",
            description: "Ice blues and aurora purples with crystalline elegance",
            colors: ThemeColors {
                bg_void: rgba_from_hex(0x030810),
                bg_primary: rgba_from_hex(0x0a1628),
                bg_secondary: rgba_from_hex(0x122240),
                bg_tertiary: rgba_from_hex(0x1a2d50),
                bg_hover: rgba_from_hex(0x223860),
                bg_selected: rgba_from_hex_alpha(0x6bd4ff, 0.2),
                bg_active: rgba_from_hex_alpha(0x6bd4ff, 0.3),

                text_primary: rgba_from_hex(0xe8f4ff),
                text_secondary: rgba_from_hex(0xb8d4f0),
                text_muted: rgba_from_hex(0x6b8bb0),
                text_inverse: rgba_from_hex(0x0a1628),

                accent_primary: rgba_from_hex(0x6bd4ff),
                accent_secondary: rgba_from_hex(0xb48aff),
                accent_glow: rgba_from_hex_alpha(0x6bd4ff, 0.4),

                success: rgba_from_hex(0x4ade80),
                warning: rgba_from_hex(0xfbbf24),
                error: rgba_from_hex(0xf87171),
                info: rgba_from_hex(0x6bd4ff),

                border_subtle: rgba_from_hex(0x1a2d50),
                border_default: rgba_from_hex(0x2a4070),
                border_emphasis: rgba_from_hex(0x6bd4ff),
                border_ornate: rgba_from_hex(0xb48aff),

                folder_color: rgba_from_hex(0x6bd4ff),
                folder_open_color: rgba_from_hex(0x8be0ff),
                file_code: rgba_from_hex(0xb48aff),
                file_data: rgba_from_hex(0x4ade80),
                file_media: rgba_from_hex(0xf472b6),
                file_archive: rgba_from_hex(0xa78bfa),
                file_document: rgba_from_hex(0xfbbf24),

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
                shadow_sm: ShadowConfig::new(
                    0.0,
                    2.0,
                    4.0,
                    0.0,
                    rgba_from_hex_alpha(0x000020, 0.3),
                ),
                shadow_md: ShadowConfig::new(
                    0.0,
                    4.0,
                    8.0,
                    0.0,
                    rgba_from_hex_alpha(0x000020, 0.4),
                ),
                shadow_lg: ShadowConfig::new(
                    0.0,
                    8.0,
                    16.0,
                    0.0,
                    rgba_from_hex_alpha(0x000020, 0.5),
                ),
                shadow_glow: ShadowConfig::new(
                    0.0,
                    0.0,
                    20.0,
                    6.0,
                    rgba_from_hex_alpha(0x6bd4ff, 0.25),
                ),
                shadow_inner: ShadowConfig::new(
                    0.0,
                    2.0,
                    4.0,
                    0.0,
                    rgba_from_hex_alpha(0x000020, 0.2),
                ),
                bg_pattern: Some(BackgroundPattern::Crystalline),
                bg_noise_opacity: 0.02,
            },
        }
    }


    pub fn ancient_tome() -> Self {
        Self {
            id: ThemeId::AncientTome,
            name: "Ancient Tome",
            description: "Parchment textures with leather browns and gold leaf",
            colors: ThemeColors {
                bg_void: rgba_from_hex(0x1a1510),
                bg_primary: rgba_from_hex(0x2a2318),
                bg_secondary: rgba_from_hex(0x3a3020),
                bg_tertiary: rgba_from_hex(0x4a3d28),
                bg_hover: rgba_from_hex(0x5a4a30),
                bg_selected: rgba_from_hex_alpha(0xd4af37, 0.2),
                bg_active: rgba_from_hex_alpha(0xd4af37, 0.3),

                text_primary: rgba_from_hex(0xf5e6c8),
                text_secondary: rgba_from_hex(0xd4c4a8),
                text_muted: rgba_from_hex(0x8b7b5b),
                text_inverse: rgba_from_hex(0x1a1510),

                accent_primary: rgba_from_hex(0xd4af37),
                accent_secondary: rgba_from_hex(0x8b4513),
                accent_glow: rgba_from_hex_alpha(0xd4af37, 0.3),

                success: rgba_from_hex(0x6b8e23),
                warning: rgba_from_hex(0xd4af37),
                error: rgba_from_hex(0x8b0000),
                info: rgba_from_hex(0x4682b4),

                border_subtle: rgba_from_hex(0x3a3020),
                border_default: rgba_from_hex(0x5a4a30),
                border_emphasis: rgba_from_hex(0xd4af37),
                border_ornate: rgba_from_hex(0xd4af37),

                folder_color: rgba_from_hex(0x8b4513),
                folder_open_color: rgba_from_hex(0xa0522d),
                file_code: rgba_from_hex(0x4682b4),
                file_data: rgba_from_hex(0x6b8e23),
                file_media: rgba_from_hex(0x8b4513),
                file_archive: rgba_from_hex(0x654321),
                file_document: rgba_from_hex(0xd4af37),

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
                shadow_sm: ShadowConfig::new(
                    2.0,
                    2.0,
                    4.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.4),
                ),
                shadow_md: ShadowConfig::new(
                    3.0,
                    3.0,
                    6.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.5),
                ),
                shadow_lg: ShadowConfig::new(
                    4.0,
                    4.0,
                    12.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.6),
                ),
                shadow_glow: ShadowConfig::new(
                    0.0,
                    0.0,
                    12.0,
                    2.0,
                    rgba_from_hex_alpha(0xd4af37, 0.2),
                ),
                shadow_inner: ShadowConfig::new(
                    1.0,
                    1.0,
                    2.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.3),
                ),
                bg_pattern: Some(BackgroundPattern::Parchment),
                bg_noise_opacity: 0.05,
            },
        }
    }


    pub fn shadow_realm() -> Self {
        Self {
            id: ThemeId::ShadowRealm,
            name: "Shadow Realm",
            description: "Deep purples with ethereal glows and void blacks",
            colors: ThemeColors {
                bg_void: rgba_from_hex(0x050508),
                bg_primary: rgba_from_hex(0x0a0a14),
                bg_secondary: rgba_from_hex(0x14142a),
                bg_tertiary: rgba_from_hex(0x1e1e3a),
                bg_hover: rgba_from_hex(0x28284a),
                bg_selected: rgba_from_hex_alpha(0x9966ff, 0.2),
                bg_active: rgba_from_hex_alpha(0x9966ff, 0.3),

                text_primary: rgba_from_hex(0xe8e0ff),
                text_secondary: rgba_from_hex(0xb8a8e0),
                text_muted: rgba_from_hex(0x6b5b8b),
                text_inverse: rgba_from_hex(0x050508),

                accent_primary: rgba_from_hex(0x9966ff),
                accent_secondary: rgba_from_hex(0x4a0080),
                accent_glow: rgba_from_hex_alpha(0x9966ff, 0.5),

                success: rgba_from_hex(0x00ff88),
                warning: rgba_from_hex(0xff9900),
                error: rgba_from_hex(0xff3366),
                info: rgba_from_hex(0x00ccff),

                border_subtle: rgba_from_hex(0x1e1e3a),
                border_default: rgba_from_hex(0x3a3a5a),
                border_emphasis: rgba_from_hex(0x9966ff),
                border_ornate: rgba_from_hex(0x9966ff),

                folder_color: rgba_from_hex(0x9966ff),
                folder_open_color: rgba_from_hex(0xb388ff),
                file_code: rgba_from_hex(0x00ccff),
                file_data: rgba_from_hex(0x00ff88),
                file_media: rgba_from_hex(0xff66aa),
                file_archive: rgba_from_hex(0x6633cc),
                file_document: rgba_from_hex(0xff9900),

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
                shadow_sm: ShadowConfig::new(
                    0.0,
                    2.0,
                    6.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.5),
                ),
                shadow_md: ShadowConfig::new(
                    0.0,
                    4.0,
                    12.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.6),
                ),
                shadow_lg: ShadowConfig::new(
                    0.0,
                    8.0,
                    24.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.7),
                ),
                shadow_glow: ShadowConfig::new(
                    0.0,
                    0.0,
                    24.0,
                    8.0,
                    rgba_from_hex_alpha(0x9966ff, 0.4),
                ),
                shadow_inner: ShadowConfig::new(
                    0.0,
                    2.0,
                    6.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.4),
                ),
                bg_pattern: Some(BackgroundPattern::Mystical),
                bg_noise_opacity: 0.04,
            },
        }
    }


    pub fn elven_glade() -> Self {
        Self {
            id: ThemeId::ElvenGlade,
            name: "Elven Glade",
            description: "Forest greens with moonlight silver and organic patterns",
            colors: ThemeColors {
                bg_void: rgba_from_hex(0x050a08),
                bg_primary: rgba_from_hex(0x0a1810),
                bg_secondary: rgba_from_hex(0x142820),
                bg_tertiary: rgba_from_hex(0x1e3830),
                bg_hover: rgba_from_hex(0x284840),
                bg_selected: rgba_from_hex_alpha(0x228b22, 0.2),
                bg_active: rgba_from_hex_alpha(0x228b22, 0.3),

                text_primary: rgba_from_hex(0xe8f0e8),
                text_secondary: rgba_from_hex(0xc0c0c0),
                text_muted: rgba_from_hex(0x6b8b6b),
                text_inverse: rgba_from_hex(0x050a08),

                accent_primary: rgba_from_hex(0x228b22),
                accent_secondary: rgba_from_hex(0xc0c0c0),
                accent_glow: rgba_from_hex_alpha(0x228b22, 0.3),

                success: rgba_from_hex(0x32cd32),
                warning: rgba_from_hex(0xdaa520),
                error: rgba_from_hex(0xcd5c5c),
                info: rgba_from_hex(0x87ceeb),

                border_subtle: rgba_from_hex(0x1e3830),
                border_default: rgba_from_hex(0x3a5a4a),
                border_emphasis: rgba_from_hex(0x228b22),
                border_ornate: rgba_from_hex(0xc0c0c0),

                folder_color: rgba_from_hex(0x228b22),
                folder_open_color: rgba_from_hex(0x32cd32),
                file_code: rgba_from_hex(0x87ceeb),
                file_data: rgba_from_hex(0x32cd32),
                file_media: rgba_from_hex(0xdaa520),
                file_archive: rgba_from_hex(0x8b4513),
                file_document: rgba_from_hex(0xc0c0c0),

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
                shadow_sm: ShadowConfig::new(
                    0.0,
                    2.0,
                    4.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.25),
                ),
                shadow_md: ShadowConfig::new(
                    0.0,
                    4.0,
                    8.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.35),
                ),
                shadow_lg: ShadowConfig::new(
                    0.0,
                    8.0,
                    16.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.45),
                ),
                shadow_glow: ShadowConfig::new(
                    0.0,
                    0.0,
                    16.0,
                    4.0,
                    rgba_from_hex_alpha(0xc0c0c0, 0.2),
                ),
                shadow_inner: ShadowConfig::new(
                    0.0,
                    1.0,
                    3.0,
                    0.0,
                    rgba_from_hex_alpha(0x000000, 0.2),
                ),
                bg_pattern: Some(BackgroundPattern::Organic),
                bg_noise_opacity: 0.02,
            },
        }
    }


    pub fn from_id(id: ThemeId) -> Self {
        match id {
            ThemeId::Light => Self::light(),
            ThemeId::Dark => Self::dark(),
            ThemeId::DragonForge => Self::dragon_forge(),
            ThemeId::FrostHaven => Self::frost_haven(),
            ThemeId::AncientTome => Self::ancient_tome(),
            ThemeId::ShadowRealm => Self::shadow_realm(),
            ThemeId::ElvenGlade => Self::elven_glade(),
        }
    }


    pub fn all_themes() -> Vec<Self> {
        vec![
            Self::light(),
            Self::dark(),
            Self::dragon_forge(),
            Self::frost_haven(),
            Self::ancient_tome(),
            Self::shadow_realm(),
            Self::elven_glade(),
        ]
    }
}


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
            self.themes
                .get(&ThemeId::DragonForge)
                .expect("Default theme must exist")
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
        assert_eq!(manager.current_id(), ThemeId::Dark);
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
        assert_eq!(themes.len(), 7);
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






        #[test]
        fn prop_theme_color_completeness(theme_id in arb_theme_id()) {
            let theme = Theme::from_id(theme_id);

            prop_assert!(
                theme.colors.is_complete(),
                "Theme {:?} must have complete color definitions",
                theme_id
            );

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






        #[test]
        fn prop_theme_persistence_round_trip(theme_id in arb_theme_id()) {
            let mut manager = ThemeManager::new();

            manager.set_theme(theme_id);

            prop_assert_eq!(
                manager.current_id(),
                theme_id,
                "Theme ID should match after setting"
            );

            let theme = manager.current_theme();
            prop_assert_eq!(
                theme.id,
                theme_id,
                "Retrieved theme ID should match"
            );
        }






        #[test]
        fn prop_theme_application_consistency(theme_id in arb_theme_id()) {
            let theme = Theme::from_id(theme_id);

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



#[derive(Clone, Debug)]
pub struct GradientConfig {
    pub gradient_type: GradientType,
    pub stops: Vec<(f32, Rgba)>,
    pub angle: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientType {
    Linear,
    Radial,
    Conic,
}

impl GradientConfig {

    pub fn vertical(top: Rgba, bottom: Rgba) -> Self {
        Self {
            gradient_type: GradientType::Linear,
            stops: vec![(0.0, top), (1.0, bottom)],
            angle: 180.0,
        }
    }


    pub fn radial(center: Rgba, edge: Rgba) -> Self {
        Self {
            gradient_type: GradientType::Radial,
            stops: vec![(0.0, center), (1.0, edge)],
            angle: 0.0,
        }
    }


    pub fn depth(top: Rgba, middle: Rgba, bottom: Rgba) -> Self {
        Self {
            gradient_type: GradientType::Linear,
            stops: vec![(0.0, top), (0.5, middle), (1.0, bottom)],
            angle: 180.0,
        }
    }
}


#[derive(Clone, Debug)]
pub struct BackgroundEffect {

    pub base_color: Rgba,

    pub gradient: Option<GradientConfig>,

    pub pattern: Option<BackgroundPattern>,

    pub noise_opacity: f32,

    pub glow: Option<(Rgba, f32)>,
}

impl BackgroundEffect {

    pub fn solid(color: Rgba) -> Self {
        Self {
            base_color: color,
            gradient: None,
            pattern: None,
            noise_opacity: 0.0,
            glow: None,
        }
    }


    pub fn with_gradient(mut self, gradient: GradientConfig) -> Self {
        self.gradient = Some(gradient);
        self
    }


    pub fn with_pattern(mut self, pattern: BackgroundPattern) -> Self {
        self.pattern = Some(pattern);
        self
    }


    pub fn with_noise(mut self, opacity: f32) -> Self {
        self.noise_opacity = opacity.clamp(0.0, 1.0);
        self
    }


    pub fn with_glow(mut self, color: Rgba, intensity: f32) -> Self {
        self.glow = Some((color, intensity.clamp(0.0, 1.0)));
        self
    }
}

impl Theme {

    pub fn content_background(&self) -> BackgroundEffect {
        let base = self.colors.bg_primary;
        let void = self.colors.bg_void;

        let mut effect = BackgroundEffect::solid(base)
            .with_gradient(GradientConfig::vertical(
                rgba_from_hex_alpha(rgba_to_hex(&void), 0.3),
                rgba_from_hex_alpha(rgba_to_hex(&base), 1.0),
            ))
            .with_noise(self.decorations.bg_noise_opacity);

        if let Some(pattern) = self.decorations.bg_pattern {
            effect = effect.with_pattern(pattern);
        }

        effect
    }


    pub fn sidebar_background(&self) -> BackgroundEffect {
        let base = self.colors.bg_secondary;

        let mut effect =
            BackgroundEffect::solid(base).with_noise(self.decorations.bg_noise_opacity * 0.5);

        if let Some(pattern) = self.decorations.bg_pattern {
            effect = effect.with_pattern(pattern);
        }

        effect
    }


    pub fn toolbar_background(&self) -> BackgroundEffect {
        let base = self.colors.bg_secondary;
        let tertiary = self.colors.bg_tertiary;

        BackgroundEffect::solid(base)
            .with_gradient(GradientConfig::vertical(tertiary, base))
            .with_noise(self.decorations.bg_noise_opacity * 0.3)
    }


    pub fn card_background(&self) -> BackgroundEffect {
        let base = self.colors.bg_tertiary;

        let mut effect =
            BackgroundEffect::solid(base).with_noise(self.decorations.bg_noise_opacity * 0.5);

        if self.decorations.use_ornate_borders {
            effect = effect.with_glow(self.colors.accent_glow, 0.1);
        }

        effect
    }


    pub fn terminal_background(&self) -> BackgroundEffect {
        BackgroundEffect::solid(self.colors.terminal_bg)
            .with_noise(self.decorations.bg_noise_opacity * 0.2)
    }
}


fn rgba_to_hex(color: &Rgba) -> u32 {
    let r = (color.r * 255.0) as u32;
    let g = (color.g * 255.0) as u32;
    let b = (color.b * 255.0) as u32;
    (r << 16) | (g << 8) | b
}


impl BackgroundPattern {

    pub fn overlay_color(&self, theme: &Theme) -> Rgba {
        match self {
            BackgroundPattern::None => rgba_from_hex_alpha(0x000000, 0.0),
            BackgroundPattern::Dots => {
                rgba_from_hex_alpha(rgba_to_hex(&theme.colors.text_muted), 0.05)
            }
            BackgroundPattern::Grid => {
                rgba_from_hex_alpha(rgba_to_hex(&theme.colors.border_subtle), 0.08)
            }
            BackgroundPattern::Noise => rgba_from_hex_alpha(0x808080, 0.03),
            BackgroundPattern::Parchment => rgba_from_hex_alpha(0xd4c4a8, 0.05),
            BackgroundPattern::Leather => rgba_from_hex_alpha(0x8b4513, 0.04),
            BackgroundPattern::Volcanic => rgba_from_hex_alpha(0xd43f3f, 0.03),
            BackgroundPattern::Crystalline => rgba_from_hex_alpha(0x6bd4ff, 0.04),
            BackgroundPattern::Mystical => rgba_from_hex_alpha(0x9966ff, 0.05),
            BackgroundPattern::Organic => rgba_from_hex_alpha(0x228b22, 0.03),
        }
    }


    pub fn scale(&self) -> f32 {
        match self {
            BackgroundPattern::None => 1.0,
            BackgroundPattern::Dots => 8.0,
            BackgroundPattern::Grid => 16.0,
            BackgroundPattern::Noise => 1.0,
            BackgroundPattern::Parchment => 32.0,
            BackgroundPattern::Leather => 24.0,
            BackgroundPattern::Volcanic => 48.0,
            BackgroundPattern::Crystalline => 32.0,
            BackgroundPattern::Mystical => 64.0,
            BackgroundPattern::Organic => 40.0,
        }
    }
}



#[derive(Clone, Debug)]
pub struct OrnateBorderConfig {
    pub width: f32,
    pub color: Rgba,
    pub style: FrameStyle,
    pub corner_flourish: Option<CornerFlourish>,
    pub glow_color: Option<Rgba>,
    pub glow_blur: f32,
}

impl OrnateBorderConfig {

    pub fn simple(width: f32, color: Rgba) -> Self {
        Self {
            width,
            color,
            style: FrameStyle::Simple,
            corner_flourish: None,
            glow_color: None,
            glow_blur: 0.0,
        }
    }


    pub fn ornate(width: f32, color: Rgba, flourish: CornerFlourish) -> Self {
        Self {
            width,
            color,
            style: FrameStyle::Ornate,
            corner_flourish: Some(flourish),
            glow_color: None,
            glow_blur: 0.0,
        }
    }


    pub fn with_glow(mut self, color: Rgba, blur: f32) -> Self {
        self.glow_color = Some(color);
        self.glow_blur = blur;
        self
    }


    pub fn inner_border_color(&self) -> Rgba {
        rgba_from_hex_alpha(rgba_to_hex(&self.color), 0.5)
    }


    pub fn has_glow(&self) -> bool {
        self.glow_color.is_some() && self.glow_blur > 0.0
    }
}


pub mod flourish_paths {

    pub const CURVED: &str = "M0,0 Q8,0 8,8 L8,16 Q8,8 16,8 L24,8";


    pub const DIAMOND: &str = "M0,8 L8,0 L16,8 L8,16 Z";


    pub const SCROLL: &str = "M0,0 C4,0 8,4 8,8 C8,4 12,0 16,0";


    pub const SWIRL: &str = "M0,12 Q6,6 12,0 Q6,6 0,12";


    pub const VINE: &str = "M0,16 C8,16 16,8 16,0 C16,8 24,16 32,16";


    pub const CELTIC: &str = "M0,0 L8,8 L0,16 M8,0 L0,8 L8,16";


    pub const BRACKET: &str = "M0,12 L0,4 Q0,0 4,0 L12,0";


    pub const MEDIEVAL: &str = "M0,0 Q4,4 0,8 Q4,4 8,8 Q4,4 8,0 Q4,4 0,0";
}


impl CornerFlourish {

    pub fn dragon_forge() -> Self {
        Self {
            svg_path: flourish_paths::CURVED,
            size: 24.0,
            color_mode: FlourishColorMode::Accent,
        }
    }


    pub fn frost_haven() -> Self {
        Self {
            svg_path: flourish_paths::DIAMOND,
            size: 16.0,
            color_mode: FlourishColorMode::Gradient,
        }
    }


    pub fn ancient_tome() -> Self {
        Self {
            svg_path: flourish_paths::SCROLL,
            size: 20.0,
            color_mode: FlourishColorMode::Border,
        }
    }


    pub fn shadow_realm() -> Self {
        Self {
            svg_path: flourish_paths::SWIRL,
            size: 18.0,
            color_mode: FlourishColorMode::Gradient,
        }
    }


    pub fn elven_glade() -> Self {
        Self {
            svg_path: flourish_paths::VINE,
            size: 32.0,
            color_mode: FlourishColorMode::Accent,
        }
    }
}


#[derive(Clone, Debug)]
pub struct DividerConfig {
    pub style: DividerStyle,
    pub color: Rgba,
    pub thickness: f32,
    pub ornament_color: Option<Rgba>,
}

impl DividerConfig {

    pub fn simple(color: Rgba, thickness: f32) -> Self {
        Self {
            style: DividerStyle::Simple,
            color,
            thickness,
            ornament_color: None,
        }
    }


    pub fn ornate(color: Rgba, ornament_color: Rgba) -> Self {
        Self {
            style: DividerStyle::Ornate,
            color,
            thickness: 1.0,
            ornament_color: Some(ornament_color),
        }
    }


    pub fn ornament_char(&self) -> &'static str {
        match self.style {
            DividerStyle::Simple => "",
            DividerStyle::Ornate => "❖",
            DividerStyle::Gradient => "◆",
            DividerStyle::Dashed => "─",
            DividerStyle::Embossed => "═",
            DividerStyle::Runic => "᛭",
        }
    }
}

impl Theme {

    pub fn section_divider(&self) -> DividerConfig {
        match self.decorations.divider_style {
            DividerStyle::Simple => DividerConfig::simple(self.colors.border_subtle, 1.0),
            DividerStyle::Ornate => {
                DividerConfig::ornate(self.colors.border_default, self.colors.accent_secondary)
            }
            DividerStyle::Gradient => DividerConfig {
                style: DividerStyle::Gradient,
                color: self.colors.accent_primary,
                thickness: 1.0,
                ornament_color: None,
            },
            DividerStyle::Dashed => DividerConfig::simple(self.colors.border_default, 1.0),
            DividerStyle::Embossed => DividerConfig {
                style: DividerStyle::Embossed,
                color: self.colors.border_emphasis,
                thickness: 2.0,
                ornament_color: Some(self.colors.bg_tertiary),
            },
            DividerStyle::Runic => {
                DividerConfig::ornate(self.colors.accent_primary, self.colors.accent_glow)
            }
        }
    }
}


#[derive(Clone, Debug)]
pub struct FrameConfig {
    pub style: FrameStyle,
    pub border_width: f32,
    pub border_color: Rgba,
    pub inner_border_color: Option<Rgba>,
    pub corner_radius: f32,
    pub shadow: Option<ShadowConfig>,
}

impl FrameConfig {

    pub fn simple(border_width: f32, border_color: Rgba, corner_radius: f32) -> Self {
        Self {
            style: FrameStyle::Simple,
            border_width,
            border_color,
            inner_border_color: None,
            corner_radius,
            shadow: None,
        }
    }


    pub fn double(
        border_width: f32,
        outer_color: Rgba,
        inner_color: Rgba,
        corner_radius: f32,
    ) -> Self {
        Self {
            style: FrameStyle::Double,
            border_width,
            border_color: outer_color,
            inner_border_color: Some(inner_color),
            corner_radius,
            shadow: None,
        }
    }


    pub fn with_shadow(mut self, shadow: ShadowConfig) -> Self {
        self.shadow = Some(shadow);
        self
    }
}

impl Theme {

    pub fn panel_frame(&self) -> FrameConfig {
        let base = FrameConfig::simple(
            self.decorations.border_width,
            self.colors.border_default,
            self.decorations.border_radius_md,
        );

        match self.decorations.frame_style {
            FrameStyle::None => base,
            FrameStyle::Simple => base,
            FrameStyle::Double => FrameConfig::double(
                self.decorations.border_width,
                self.colors.border_default,
                self.colors.border_subtle,
                self.decorations.border_radius_md,
            )
            .with_shadow(self.decorations.shadow_md.clone()),
            FrameStyle::Ornate => base.with_shadow(self.decorations.shadow_glow.clone()),
            FrameStyle::Beveled => FrameConfig {
                style: FrameStyle::Beveled,
                border_width: self.decorations.border_width * 2.0,
                border_color: self.colors.border_emphasis,
                inner_border_color: Some(self.colors.bg_tertiary),
                corner_radius: self.decorations.border_radius_sm,
                shadow: Some(self.decorations.shadow_inner.clone()),
            },
            FrameStyle::Inset => FrameConfig {
                style: FrameStyle::Inset,
                border_width: self.decorations.border_width,
                border_color: self.colors.bg_void,
                inner_border_color: Some(self.colors.border_subtle),
                corner_radius: self.decorations.border_radius_sm,
                shadow: Some(self.decorations.shadow_inner.clone()),
            },
        }
    }


    pub fn card_frame(&self) -> FrameConfig {
        FrameConfig::simple(
            self.decorations.border_width,
            self.colors.border_subtle,
            self.decorations.border_radius_md,
        )
        .with_shadow(self.decorations.shadow_sm.clone())
    }


    pub fn modal_frame(&self) -> FrameConfig {
        FrameConfig::simple(
            self.decorations.border_width,
            self.colors.border_emphasis,
            self.decorations.border_radius_lg,
        )
        .with_shadow(self.decorations.shadow_lg.clone())
    }
}

impl Theme {

    pub fn panel_border(&self) -> OrnateBorderConfig {
        let mut config =
            OrnateBorderConfig::simple(self.decorations.border_width, self.colors.border_default);

        if self.decorations.use_ornate_borders {
            config.style = self.decorations.frame_style;
            config.corner_flourish = self.decorations.corner_flourish.clone();

            config = config.with_glow(self.colors.accent_glow, 8.0);
        }

        config
    }


    pub fn emphasis_border(&self) -> OrnateBorderConfig {
        let mut config = OrnateBorderConfig::simple(
            self.decorations.border_width * 2.0,
            self.colors.border_emphasis,
        );

        if self.decorations.use_ornate_borders {
            config.style = FrameStyle::Double;
            config.corner_flourish = self.decorations.corner_flourish.clone();
            config = config.with_glow(self.colors.accent_primary, 12.0);
        }

        config
    }


    pub fn section_divider_color(&self) -> Rgba {
        match self.decorations.divider_style {
            DividerStyle::Simple => self.colors.border_subtle,
            DividerStyle::Ornate => self.colors.border_ornate,
            DividerStyle::Gradient => self.colors.accent_secondary,
            DividerStyle::Dashed => self.colors.border_default,
            DividerStyle::Embossed => self.colors.border_emphasis,
            DividerStyle::Runic => self.colors.accent_primary,
        }
    }
}


impl ThemeColors {

    pub fn bg_void_color(&self) -> gpui::Rgba {
        self.bg_void
    }


    pub fn bg_primary_color(&self) -> gpui::Rgba {
        self.bg_primary
    }


    pub fn bg_secondary_color(&self) -> gpui::Rgba {
        self.bg_secondary
    }


    pub fn bg_tertiary_color(&self) -> gpui::Rgba {
        self.bg_tertiary
    }


    pub fn bg_hover_color(&self) -> gpui::Rgba {
        self.bg_hover
    }


    pub fn bg_selected_color(&self) -> gpui::Rgba {
        self.bg_selected
    }


    pub fn text_primary_color(&self) -> gpui::Rgba {
        self.text_primary
    }


    pub fn text_secondary_color(&self) -> gpui::Rgba {
        self.text_secondary
    }


    pub fn text_muted_color(&self) -> gpui::Rgba {
        self.text_muted
    }


    pub fn accent_primary_color(&self) -> gpui::Rgba {
        self.accent_primary
    }


    pub fn accent_secondary_color(&self) -> gpui::Rgba {
        self.accent_secondary
    }


    pub fn border_default_color(&self) -> gpui::Rgba {
        self.border_default
    }


    pub fn border_subtle_color(&self) -> gpui::Rgba {
        self.border_subtle
    }


    pub fn border_emphasis_color(&self) -> gpui::Rgba {
        self.border_emphasis
    }


    pub fn folder_color_value(&self) -> gpui::Rgba {
        self.folder_color
    }
}

use std::sync::atomic::{AtomicU8, Ordering};


static CURRENT_THEME_ID: AtomicU8 = AtomicU8::new(1);


fn theme_id_to_u8(id: ThemeId) -> u8 {
    match id {
        ThemeId::Light => 0,
        ThemeId::Dark => 1,
        ThemeId::DragonForge => 2,
        ThemeId::FrostHaven => 3,
        ThemeId::AncientTome => 4,
        ThemeId::ShadowRealm => 5,
        ThemeId::ElvenGlade => 6,
    }
}


fn u8_to_theme_id(val: u8) -> ThemeId {
    match val {
        0 => ThemeId::Light,
        1 => ThemeId::Dark,
        2 => ThemeId::DragonForge,
        3 => ThemeId::FrostHaven,
        4 => ThemeId::AncientTome,
        5 => ThemeId::ShadowRealm,
        6 => ThemeId::ElvenGlade,
        _ => ThemeId::Dark,
    }
}


pub fn set_current_theme(id: ThemeId) {
    CURRENT_THEME_ID.store(theme_id_to_u8(id), Ordering::SeqCst);
}


pub fn current_theme_id() -> ThemeId {
    u8_to_theme_id(CURRENT_THEME_ID.load(Ordering::SeqCst))
}


pub fn current_theme() -> Theme {
    Theme::from_id(current_theme_id())
}


pub fn theme_colors() -> ThemeColors {
    current_theme().colors
}


pub fn theme_typography() -> ThemeTypography {
    current_theme().typography
}


pub fn theme_decorations() -> ThemeDecorations {
    current_theme().decorations
}


pub fn save_theme_selection(id: ThemeId) {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("nexus-explorer");

    if std::fs::create_dir_all(&config_dir).is_err() {
        return;
    }

    let config_path = config_dir.join("theme.json");
    let config = serde_json::json!({
        "theme_id": id
    });

    if let Ok(json) = serde_json::to_string_pretty(&config) {
        let _ = std::fs::write(config_path, json);
    }
}


pub fn load_theme_selection() -> ThemeId {
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("nexus-explorer")
        .join("theme.json");

    if config_path.exists() {
        if let Ok(json) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&json) {
                if let Some(theme_id) = config.get("theme_id") {
                    if let Ok(id) = serde_json::from_value::<ThemeId>(theme_id.clone()) {
                        return id;
                    }
                }
            }
        }
    }

    ThemeId::default()
}
