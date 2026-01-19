//! Theme: "Midnight Terminal" - Dark developer aesthetic
//!
//! A bold, distinctive dark theme with vibrant neon accents,
//! glassmorphism effects, and glowing status indicators.

#![allow(dead_code)]

use egui::Color32;

/// Midnight Terminal Theme
pub struct Theme;

impl Theme {
    // ═══════════════════════════════════════════════════════════════════
    // BACKGROUNDS - Deep charcoal with subtle warmth
    // ═══════════════════════════════════════════════════════════════════

    /// Main background - deep charcoal
    pub const BG_PRIMARY: Color32 = Color32::from_rgb(18, 18, 24);

    /// Secondary background - slightly lighter
    pub const BG_SECONDARY: Color32 = Color32::from_rgb(24, 24, 32);

    /// Card background - glassmorphism dark
    pub const CARD_BG: Color32 = Color32::from_rgb(32, 32, 42);

    /// Card background hover
    pub const CARD_BG_HOVER: Color32 = Color32::from_rgb(40, 40, 52);

    /// Elevated surface
    pub const SURFACE_ELEVATED: Color32 = Color32::from_rgb(45, 45, 58);

    // ═══════════════════════════════════════════════════════════════════
    // ACCENT COLORS - Vibrant neon palette
    // ═══════════════════════════════════════════════════════════════════

    /// Primary accent - Electric cyan
    pub const ACCENT_PRIMARY: Color32 = Color32::from_rgb(0, 212, 255);

    /// Secondary accent - Hot magenta
    pub const ACCENT_SECONDARY: Color32 = Color32::from_rgb(255, 0, 128);

    /// Tertiary accent - Lime green
    pub const ACCENT_TERTIARY: Color32 = Color32::from_rgb(0, 255, 136);

    /// Accent muted - Soft purple
    pub const ACCENT_MUTED: Color32 = Color32::from_rgb(138, 43, 226);

    // ═══════════════════════════════════════════════════════════════════
    // TAB COLORS
    // ═══════════════════════════════════════════════════════════════════

    /// Tab container - dark glass
    pub const TAB_CONTAINER: Color32 = Color32::from_rgb(28, 28, 38);

    /// Tab inactive
    pub const TAB_INACTIVE: Color32 = Color32::from_rgb(38, 38, 48);

    /// Tab active - gradient start (we'll simulate with solid)
    pub const TAB_ACTIVE: Color32 = Color32::from_rgb(0, 180, 216);

    /// Tab text inactive
    pub const TAB_TEXT_INACTIVE: Color32 = Color32::from_rgb(120, 120, 140);

    /// Tab text active
    pub const TAB_TEXT_ACTIVE: Color32 = Color32::WHITE;

    // ═══════════════════════════════════════════════════════════════════
    // TEXT COLORS
    // ═══════════════════════════════════════════════════════════════════

    /// Primary text - bright white
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 240, 245);

    /// Secondary text
    #[allow(dead_code)]
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(180, 180, 195);

    /// Muted text
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 100, 120);

    /// Dimmed text
    pub const TEXT_DIM: Color32 = Color32::from_rgb(70, 70, 85);

    // ═══════════════════════════════════════════════════════════════════
    // BORDERS & SEPARATORS
    // ═══════════════════════════════════════════════════════════════════

    /// Separator line
    pub const SEPARATOR: Color32 = Color32::from_rgb(50, 50, 65);

    /// Card border - subtle glow
    pub const CARD_BORDER: Color32 = Color32::from_rgb(60, 60, 80);

    /// Card border accent
    pub const CARD_BORDER_ACCENT: Color32 = Color32::from_rgb(0, 150, 180);

    // ═══════════════════════════════════════════════════════════════════
    // USAGE STATUS COLORS - Vibrant with glow effect
    // ═══════════════════════════════════════════════════════════════════

    /// Green - excellent (0-25%)
    pub const GREEN: Color32 = Color32::from_rgb(0, 255, 136);
    pub const USAGE_GREEN: Color32 = Self::GREEN;

    /// Cyan - good (25-50%)
    pub const CYAN: Color32 = Color32::from_rgb(0, 212, 255);

    /// Yellow - caution (50-75%)
    pub const YELLOW: Color32 = Color32::from_rgb(255, 214, 0);

    /// Orange - warning (75-90%)
    pub const ORANGE: Color32 = Color32::from_rgb(255, 140, 0);
    pub const USAGE_ORANGE: Color32 = Self::ORANGE;

    /// Red - critical (90-100%)
    pub const RED: Color32 = Color32::from_rgb(255, 60, 80);

    /// Progress bar track
    pub const PROGRESS_TRACK: Color32 = Color32::from_rgb(40, 40, 55);

    // ═══════════════════════════════════════════════════════════════════
    // SPECIAL EFFECTS (use methods for alpha colors)
    // ═══════════════════════════════════════════════════════════════════

    /// Get shadow color
    pub fn shadow() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 0, 0, 80)
    }

    /// Get glow overlay color
    pub fn glow_overlay() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 212, 255, 20)
    }

    /// Get progress glow color
    pub fn progress_glow() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 212, 255, 30)
    }

    /// Success badge
    pub const BADGE_SUCCESS: Color32 = Color32::from_rgb(0, 200, 120);

    /// Warning badge
    pub const BADGE_WARNING: Color32 = Color32::from_rgb(255, 180, 0);

    /// Error badge
    pub const BADGE_ERROR: Color32 = Color32::from_rgb(255, 70, 90);

    /// Info badge
    pub const BADGE_INFO: Color32 = Color32::from_rgb(80, 160, 255);

    // ═══════════════════════════════════════════════════════════════════
    // METHODS
    // ═══════════════════════════════════════════════════════════════════

    /// Get usage color based on percentage - vibrant gradient
    pub fn usage_color(percent: f64) -> Color32 {
        if percent <= 25.0 {
            Self::GREEN
        } else if percent <= 50.0 {
            Self::CYAN
        } else if percent <= 75.0 {
            Self::YELLOW
        } else if percent <= 90.0 {
            Self::ORANGE
        } else {
            Self::RED
        }
    }

    /// Get a dimmed version of usage color for track
    pub fn usage_track_color(percent: f64) -> Color32 {
        let base = Self::usage_color(percent);
        Color32::from_rgba_unmultiplied(
            base.r() / 4,
            base.g() / 4,
            base.b() / 4,
            60,
        )
    }

    /// Get glow color for usage
    pub fn usage_glow_color(percent: f64) -> Color32 {
        let base = Self::usage_color(percent);
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 40)
    }
}

/// Provider icons - distinctive symbols
pub fn provider_icon(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        "codex" => "◆",
        "claude" => "◈",
        "cursor" => "▶",
        "gemini" => "✧",
        "copilot" => "⬡",
        "antigravity" => "◉",
        "factory" | "windsurf" => "◎",
        "zed" | "zai" => "Z",
        "kiro" => "K",
        "vertexai" | "vertex ai" => "△",
        "augment" => "A",
        "minimax" => "M",
        "opencode" => "○",
        "kimi" => "☾",
        "kimik2" | "kimi k2" => "☾",
        "amp" => "⚡",
        "synthetic" => "◇",
        _ => "●",
    }
}

/// Provider brand colors
pub fn provider_color(name: &str) -> Color32 {
    match name.to_lowercase().as_str() {
        "codex" => Color32::from_rgb(16, 163, 127),   // OpenAI green
        "claude" => Color32::from_rgb(204, 119, 68),   // Anthropic orange
        "cursor" => Color32::from_rgb(138, 43, 226),   // Purple
        "gemini" => Color32::from_rgb(66, 133, 244),   // Google blue
        "copilot" => Color32::from_rgb(36, 41, 47),    // GitHub dark
        "antigravity" => Color32::from_rgb(0, 212, 255), // Cyan
        "factory" | "windsurf" => Color32::from_rgb(0, 200, 150),
        "zed" | "zai" => Color32::from_rgb(255, 100, 50),
        "kiro" => Color32::from_rgb(255, 165, 0),
        "vertexai" | "vertex ai" => Color32::from_rgb(66, 133, 244),
        "augment" => Color32::from_rgb(100, 200, 255),
        "minimax" => Color32::from_rgb(255, 100, 150),
        "opencode" => Color32::from_rgb(200, 200, 200),
        "kimi" | "kimik2" | "kimi k2" => Color32::from_rgb(100, 100, 255),
        "amp" => Color32::from_rgb(255, 90, 60),       // Sourcegraph orange-red
        "synthetic" => Color32::from_rgb(160, 120, 255), // Synthetic purple
        _ => Theme::ACCENT_PRIMARY,
    }
}
