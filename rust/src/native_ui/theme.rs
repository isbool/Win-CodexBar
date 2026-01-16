//! Theme colors and styling for macOS-style menubar popup

use egui::Color32;

/// Clean white macOS-style theme
pub struct Theme;

impl Theme {
    // ─────────────────────────────────────────────────────────────
    // Background
    // ─────────────────────────────────────────────────────────────

    /// Main background - very light gray
    pub const BG_PRIMARY: Color32 = Color32::from_rgb(248, 248, 250);

    /// Card background - white
    pub const CARD_BG: Color32 = Color32::from_rgb(255, 255, 255);

    // ─────────────────────────────────────────────────────────────
    // Tab colors
    // ─────────────────────────────────────────────────────────────

    /// Tab container background - light purple/blue tint
    pub const TAB_CONTAINER: Color32 = Color32::from_rgb(235, 235, 245);

    /// Tab inactive - light gray
    pub const TAB_INACTIVE: Color32 = Color32::from_rgb(240, 240, 245);

    /// Tab active - vibrant blue
    pub const TAB_ACTIVE: Color32 = Color32::from_rgb(50, 120, 255);

    /// Tab text inactive
    pub const TAB_TEXT_INACTIVE: Color32 = Color32::from_rgb(100, 100, 115);

    /// Tab text active
    pub const TAB_TEXT_ACTIVE: Color32 = Color32::WHITE;

    // ─────────────────────────────────────────────────────────────
    // Text colors
    // ─────────────────────────────────────────────────────────────

    /// Primary text - dark
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(30, 30, 35);

    /// Secondary text - muted
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(100, 100, 110);

    /// Muted text - light gray
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(140, 140, 150);

    // ─────────────────────────────────────────────────────────────
    // Separator
    // ─────────────────────────────────────────────────────────────

    /// Thin separator line
    pub const SEPARATOR: Color32 = Color32::from_rgb(230, 230, 235);

    /// Card border - subtle gray
    pub const CARD_BORDER: Color32 = Color32::from_rgb(225, 225, 230);

    // ─────────────────────────────────────────────────────────────
    // Usage colors
    // ─────────────────────────────────────────────────────────────

    /// Green - low usage / behind pace (good)
    pub const GREEN: Color32 = Color32::from_rgb(52, 199, 89);
    pub const USAGE_GREEN: Color32 = Self::GREEN;

    /// Yellow/Orange - medium usage
    pub const YELLOW: Color32 = Color32::from_rgb(255, 179, 64);

    /// Orange - high usage / ahead of pace (warning)
    pub const ORANGE: Color32 = Color32::from_rgb(255, 149, 0);
    pub const USAGE_ORANGE: Color32 = Self::ORANGE;

    /// Red - critical usage
    pub const RED: Color32 = Color32::from_rgb(255, 69, 58);

    /// Progress bar track - light gray
    pub const PROGRESS_TRACK: Color32 = Color32::from_rgb(230, 230, 235);

    /// Get usage color based on percentage
    pub fn usage_color(percent: f64) -> Color32 {
        if percent <= 25.0 {
            Self::GREEN
        } else if percent <= 50.0 {
            Self::YELLOW
        } else if percent <= 75.0 {
            Self::ORANGE
        } else {
            Self::RED
        }
    }
}

/// Provider icons
pub fn provider_icon(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        "codex" => "✦",
        "claude" => "✴",
        "cursor" => "▷",
        "gemini" => "✦",
        "copilot" => "✦",
        "antigravity" => "✦",
        "factory" | "windsurf" => "◎",
        "zed" | "zed ai" => "Z",
        "kiro" => "K",
        "vertexai" | "vertex ai" => "△",
        "augment" => "A",
        "minimax" => "M",
        "opencode" => "○",
        "kimi" => "🌙",
        "kimik2" | "kimi k2" => "🌙",
        _ => "•",
    }
}
