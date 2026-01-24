//! Theme: Modern Refined Dark
//!
//! A rich, sophisticated dark theme with depth and atmosphere.
//! Inspired by premium apps like Linear, Raycast, and Arc.

#![allow(dead_code)]

use egui::Color32;

/// Modern Refined Dark Theme
pub struct Theme;

impl Theme {
    // ═══════════════════════════════════════════════════════════════════
    // BACKGROUNDS - Rich, layered dark palette with subtle warmth
    // ═══════════════════════════════════════════════════════════════════

    /// Deep background - rich charcoal with subtle warmth
    pub const BG_PRIMARY: Color32 = Color32::from_rgb(18, 18, 22);

    /// Secondary background - elevated layer
    pub const BG_SECONDARY: Color32 = Color32::from_rgb(24, 24, 30);

    /// Tertiary background - for nested elements
    pub const BG_TERTIARY: Color32 = Color32::from_rgb(32, 32, 40);

    /// Card/panel background - glass-like elevated surface
    pub const CARD_BG: Color32 = Color32::from_rgb(28, 28, 36);

    /// Card background on hover - subtle lift
    pub const CARD_BG_HOVER: Color32 = Color32::from_rgb(38, 38, 48);

    /// Elevated surface (modals, popovers)
    pub const SURFACE_ELEVATED: Color32 = Color32::from_rgb(36, 36, 46);

    /// Input field background
    pub const INPUT_BG: Color32 = Color32::from_rgb(22, 22, 28);

    // ═══════════════════════════════════════════════════════════════════
    // ACCENT COLORS - Vibrant, modern palette
    // ═══════════════════════════════════════════════════════════════════

    /// Primary accent - Electric blue
    pub const ACCENT_PRIMARY: Color32 = Color32::from_rgb(59, 130, 246);

    /// Primary accent hover
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(96, 165, 250);

    /// Primary accent muted
    pub const ACCENT_MUTED: Color32 = Color32::from_rgb(59, 130, 246);

    /// Secondary accent - Soft purple
    pub const ACCENT_SECONDARY: Color32 = Color32::from_rgb(139, 92, 246);

    /// Tertiary accent - Subtle glow
    pub const ACCENT_TERTIARY: Color32 = Color32::from_rgb(99, 102, 241);

    // ═══════════════════════════════════════════════════════════════════
    // TAB COLORS - Refined tab styling
    // ═══════════════════════════════════════════════════════════════════

    /// Tab container background
    pub const TAB_CONTAINER: Color32 = Color32::from_rgb(22, 22, 28);

    /// Tab inactive state
    pub const TAB_INACTIVE: Color32 = Color32::from_rgb(32, 32, 40);

    /// Tab active state
    pub const TAB_ACTIVE: Color32 = Color32::from_rgb(59, 130, 246);

    /// Tab text when inactive
    pub const TAB_TEXT_INACTIVE: Color32 = Color32::from_rgb(120, 120, 140);

    /// Tab text when active
    pub const TAB_TEXT_ACTIVE: Color32 = Color32::WHITE;

    // ═══════════════════════════════════════════════════════════════════
    // TEXT COLORS - Clear hierarchy with soft tones
    // ═══════════════════════════════════════════════════════════════════

    /// Primary text - Soft white with warmth
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 240, 245);

    /// Secondary text - Labels, descriptions
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 180);

    /// Muted text - Hints, placeholders
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 100, 120);

    /// Dimmed text - Disabled states
    pub const TEXT_DIM: Color32 = Color32::from_rgb(70, 70, 85);

    /// Section header text
    pub const TEXT_SECTION: Color32 = Color32::from_rgb(120, 120, 140);

    // ═══════════════════════════════════════════════════════════════════
    // BORDERS & SEPARATORS - Subtle, refined
    // ═══════════════════════════════════════════════════════════════════

    /// Separator line
    pub const SEPARATOR: Color32 = Color32::from_rgb(45, 45, 55);

    /// Card/panel border - subtle glass effect
    pub const CARD_BORDER: Color32 = Color32::from_rgb(50, 50, 65);

    /// Focused/accent border
    pub const CARD_BORDER_ACCENT: Color32 = Color32::from_rgb(59, 130, 246);

    /// Subtle border for inputs
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(55, 55, 70);

    // ═══════════════════════════════════════════════════════════════════
    // USAGE/STATUS COLORS - Vibrant, clear
    // ═══════════════════════════════════════════════════════════════════

    /// Green - Success (0-50% usage)
    pub const GREEN: Color32 = Color32::from_rgb(34, 197, 94);
    pub const USAGE_GREEN: Color32 = Self::GREEN;

    /// Blue - Primary/Info
    pub const BLUE: Color32 = Color32::from_rgb(59, 130, 246);

    /// Yellow - Caution (50-75% usage)
    pub const YELLOW: Color32 = Color32::from_rgb(250, 204, 21);

    /// Orange - Warning (75-90% usage)
    pub const ORANGE: Color32 = Color32::from_rgb(251, 146, 60);
    pub const USAGE_ORANGE: Color32 = Self::ORANGE;

    /// Red - Critical (90-100% usage)
    pub const RED: Color32 = Color32::from_rgb(239, 68, 68);

    /// Cyan - Info/credits
    pub const CYAN: Color32 = Color32::from_rgb(34, 211, 238);

    /// Progress bar track - subtle with depth
    pub const PROGRESS_TRACK: Color32 = Color32::from_rgb(40, 40, 52);

    // ═══════════════════════════════════════════════════════════════════
    // BADGES - Status indicators with glow
    // ═══════════════════════════════════════════════════════════════════

    /// Success badge
    pub const BADGE_SUCCESS: Color32 = Color32::from_rgb(34, 197, 94);

    /// Warning badge
    pub const BADGE_WARNING: Color32 = Color32::from_rgb(251, 146, 60);

    /// Error badge
    pub const BADGE_ERROR: Color32 = Color32::from_rgb(239, 68, 68);

    /// Info badge
    pub const BADGE_INFO: Color32 = Color32::from_rgb(59, 130, 246);

    // ═══════════════════════════════════════════════════════════════════
    // SPECIAL EFFECTS - Depth and atmosphere
    // ═══════════════════════════════════════════════════════════════════

    /// Get shadow color - deeper for more dimension
    pub fn shadow() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 0, 0, 80)
    }

    /// Get subtle overlay for selected states
    pub fn selection_overlay() -> Color32 {
        Color32::from_rgba_unmultiplied(59, 130, 246, 25)
    }

    /// Hover overlay - gentle highlight
    pub fn hover_overlay() -> Color32 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 6)
    }

    /// Glow overlay for active elements
    pub fn glow_overlay() -> Color32 {
        Color32::from_rgba_unmultiplied(59, 130, 246, 20)
    }

    /// Progress glow
    pub fn progress_glow() -> Color32 {
        Color32::from_rgba_unmultiplied(59, 130, 246, 30)
    }

    /// Gradient start (for backgrounds)
    pub fn gradient_start() -> Color32 {
        Color32::from_rgba_unmultiplied(59, 130, 246, 8)
    }

    /// Gradient end
    pub fn gradient_end() -> Color32 {
        Color32::from_rgba_unmultiplied(139, 92, 246, 5)
    }

    // ═══════════════════════════════════════════════════════════════════
    // METHODS - Usage-based coloring
    // ═══════════════════════════════════════════════════════════════════

    /// Get usage color based on percentage
    pub fn usage_color(percent: f64) -> Color32 {
        if percent <= 50.0 {
            Self::GREEN
        } else if percent <= 75.0 {
            Self::YELLOW
        } else if percent <= 90.0 {
            Self::ORANGE
        } else {
            Self::RED
        }
    }

    /// Get a dimmed version of usage color for track
    pub fn usage_track_color(_percent: f64) -> Color32 {
        Self::PROGRESS_TRACK
    }

    /// Get subtle glow color for usage
    pub fn usage_glow_color(percent: f64) -> Color32 {
        let base = Self::usage_color(percent);
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 35)
    }

    /// Get menu item hover background
    pub fn menu_hover() -> Color32 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 8)
    }

    /// Button gradient top
    pub fn button_gradient_top() -> Color32 {
        Color32::from_rgb(70, 145, 255)
    }

    /// Button gradient bottom
    pub fn button_gradient_bottom() -> Color32 {
        Color32::from_rgb(50, 120, 230)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROVIDER ICONS - Clean symbols with personality
// ═══════════════════════════════════════════════════════════════════════════

/// Provider icons - distinctive symbols
pub fn provider_icon(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        "codex" => "◆",
        "claude" => "◈",
        "cursor" => "▸",
        "gemini" => "✦",
        "copilot" => "⬡",
        "antigravity" => "◉",
        "factory" | "droid" => "◎",
        "zai" | "z.ai" => "Z",
        "kiro" => "K",
        "vertexai" | "vertex ai" => "△",
        "augment" => "A",
        "minimax" => "M",
        "opencode" => "○",
        "kimi" => "☽",
        "kimik2" | "kimi k2" => "☽",
        "amp" => "⚡",
        "synthetic" => "◇",
        "jetbrains" | "jetbrains ai" => "J",
        _ => "●",
    }
}

/// Provider brand colors - vibrant and recognizable
pub fn provider_color(name: &str) -> Color32 {
    match name.to_lowercase().as_str() {
        "codex" => Color32::from_rgb(16, 185, 129),       // Emerald green
        "claude" => Color32::from_rgb(217, 119, 87),       // Warm terracotta
        "cursor" => Color32::from_rgb(147, 112, 219),      // Medium purple
        "gemini" => Color32::from_rgb(66, 153, 225),       // Sky blue
        "copilot" => Color32::from_rgb(139, 148, 158),     // Slate gray
        "antigravity" => Color32::from_rgb(56, 189, 248),  // Bright cyan
        "factory" | "droid" => Color32::from_rgb(52, 211, 153),
        "zai" | "z.ai" => Color32::from_rgb(251, 146, 60),
        "kiro" => Color32::from_rgb(251, 191, 36),
        "vertexai" | "vertex ai" => Color32::from_rgb(96, 165, 250),
        "augment" => Color32::from_rgb(125, 211, 252),
        "minimax" => Color32::from_rgb(244, 114, 182),
        "opencode" => Color32::from_rgb(203, 213, 225),
        "kimi" | "kimik2" | "kimi k2" => Color32::from_rgb(129, 140, 248),
        "amp" => Color32::from_rgb(248, 113, 113),
        "synthetic" => Color32::from_rgb(167, 139, 250),
        "jetbrains" | "jetbrains ai" => Color32::from_rgb(252, 165, 165),
        _ => Theme::ACCENT_PRIMARY,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SPACING CONSTANTS - Generous, comfortable layout
// ═══════════════════════════════════════════════════════════════════════════

/// Spacing constants for consistent layout
pub struct Spacing;

impl Spacing {
    pub const XXS: f32 = 4.0;
    pub const XS: f32 = 8.0;
    pub const SM: f32 = 12.0;
    pub const MD: f32 = 16.0;
    pub const LG: f32 = 24.0;
    pub const XL: f32 = 32.0;
    pub const XXL: f32 = 48.0;
}

/// Rounding constants - softer, modern feel
pub struct Radius;

impl Radius {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 6.0;
    pub const MD: f32 = 10.0;
    pub const LG: f32 = 14.0;
    pub const XL: f32 = 18.0;
    pub const PILL: f32 = 100.0;
}

/// Font sizes - macOS-inspired clear hierarchy
pub struct FontSize;

impl FontSize {
    pub const XS: f32 = 11.0;
    pub const SM: f32 = 12.0;
    pub const BASE: f32 = 13.0;
    pub const MD: f32 = 14.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 18.0;
    pub const XXL: f32 = 22.0;
}
