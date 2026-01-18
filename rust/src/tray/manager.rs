//! System tray manager with dynamic usage bar icon
//!
//! Creates a system tray icon that shows session and weekly usage as two horizontal bars

use image::{ImageBuffer, Rgba, RgbaImage};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

use super::icon::{LoadingPattern, UsageLevel};

const ICON_SIZE: u32 = 32;

/// Surprise animation types (matching macOS CodexBar)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SurpriseAnimation {
    /// No animation
    #[allow(dead_code)]
    None,
    /// Bars flash bright white briefly (like eyes blinking)
    Blink,
    /// Bars wiggle left/right (Claude arms/legs effect)
    Wiggle,
    /// Bars pulse in intensity
    Pulse,
    /// Rainbow color sweep
    Rainbow,
    /// Icon tilts slightly (Codex hat tilt effect)
    Tilt,
}

impl SurpriseAnimation {
    /// Get a random animation type
    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        match rng.random_range(0..5) {
            0 => SurpriseAnimation::Blink,
            1 => SurpriseAnimation::Wiggle,
            2 => SurpriseAnimation::Pulse,
            3 => SurpriseAnimation::Rainbow,
            _ => SurpriseAnimation::Tilt,
        }
    }

    /// Duration of the animation in frames (at ~60fps)
    pub fn duration_frames(&self) -> u32 {
        match self {
            SurpriseAnimation::None => 0,
            SurpriseAnimation::Blink => 8,     // Quick flash
            SurpriseAnimation::Wiggle => 20,   // Shake back and forth
            SurpriseAnimation::Pulse => 30,    // Slow pulse
            SurpriseAnimation::Rainbow => 40,  // Color sweep
            SurpriseAnimation::Tilt => 24,     // Tilt and return
        }
    }
}

/// Icon overlay types for status indicators
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum IconOverlay {
    /// No overlay - normal display
    #[default]
    None,
    /// Error state - grayed out icon with X
    Error,
    /// Stale data - dim icon with clock indicator
    #[allow(dead_code)]
    Stale,
    /// Incident - warning badge overlay
    Incident,
    /// Partial outage - orange badge
    Partial,
}

/// Provider usage data for merged icon mode
#[derive(Clone, Debug)]
pub struct ProviderUsage {
    pub name: String,
    pub session_percent: f64,
    #[allow(dead_code)]
    pub weekly_percent: f64,
}

/// System tray manager
pub struct TrayManager {
    tray_icon: TrayIcon,
}

impl TrayManager {
    /// Create a new tray manager with default icon
    pub fn new() -> anyhow::Result<Self> {
        let menu = Menu::new();
        let open_item = MenuItem::new("Open CodexBar", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit_item = MenuItem::new("Quit", true, None);

        menu.append(&open_item)?;
        menu.append(&separator)?;
        menu.append(&quit_item)?;

        let icon = create_bar_icon(0.0, 0.0, IconOverlay::None);

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("CodexBar - Loading...")
            .with_icon(icon)
            .build()?;

        Ok(Self { tray_icon })
    }

    /// Update the tray icon based on usage percentages (single provider mode)
    pub fn update_usage(&self, session_percent: f64, weekly_percent: f64, provider_name: &str) {
        let icon = create_bar_icon(session_percent, weekly_percent, IconOverlay::None);
        let _ = self.tray_icon.set_icon(Some(icon));

        let tooltip = format!(
            "{}: Session {}% | Weekly {}%",
            provider_name,
            session_percent as i32,
            weekly_percent as i32
        );
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
    }

    /// Update the tray icon with an overlay (error, stale, incident)
    pub fn update_usage_with_overlay(&self, session_percent: f64, weekly_percent: f64, provider_name: &str, overlay: IconOverlay) {
        let icon = create_bar_icon(session_percent, weekly_percent, overlay);
        let _ = self.tray_icon.set_icon(Some(icon));

        let status_suffix = match overlay {
            IconOverlay::None => "",
            IconOverlay::Error => " (Error)",
            IconOverlay::Stale => " (Stale)",
            IconOverlay::Incident => " (Incident)",
            IconOverlay::Partial => " (Partial Outage)",
        };

        let tooltip = format!(
            "{}: Session {}% | Weekly {}%{}",
            provider_name,
            session_percent as i32,
            weekly_percent as i32,
            status_suffix
        );
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
    }

    /// Show error state on the tray icon
    #[allow(dead_code)]
    pub fn show_error(&self, provider_name: &str, error_msg: &str) {
        let icon = create_bar_icon(0.0, 0.0, IconOverlay::Error);
        let _ = self.tray_icon.set_icon(Some(icon));
        let tooltip = format!("{}: {}", provider_name, error_msg);
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
    }

    /// Show stale data indicator
    #[allow(dead_code)]
    pub fn show_stale(&self, session_percent: f64, weekly_percent: f64, provider_name: &str, age_minutes: u64) {
        let icon = create_bar_icon(session_percent, weekly_percent, IconOverlay::Stale);
        let _ = self.tray_icon.set_icon(Some(icon));

        let tooltip = format!(
            "{}: Session {}% | Weekly {}% (data {}m old)",
            provider_name,
            session_percent as i32,
            weekly_percent as i32,
            age_minutes
        );
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
    }

    /// Update the tray icon showing credits mode (thicker bar when weekly exhausted)
    /// This shows a thick credits bar when weekly quota is exhausted but credits remain
    pub fn update_credits_mode(&self, credits_percent: f64, provider_name: &str) {
        let icon = create_credits_icon(credits_percent);
        let _ = self.tray_icon.set_icon(Some(icon));

        let tooltip = format!(
            "{}: Weekly quota exhausted | {:.0}% credits remaining",
            provider_name,
            credits_percent
        );
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
    }

    /// Update the tray icon showing multiple providers (merged mode)
    pub fn update_merged(&self, providers: &[ProviderUsage]) {
        if providers.is_empty() {
            let icon = create_bar_icon(0.0, 0.0, IconOverlay::None);
            let _ = self.tray_icon.set_icon(Some(icon));
            let _ = self.tray_icon.set_tooltip(Some("CodexBar - No providers"));
            return;
        }

        let icon = create_merged_icon(providers);
        let _ = self.tray_icon.set_icon(Some(icon));

        // Build tooltip with all providers
        let tooltip_lines: Vec<String> = providers
            .iter()
            .take(4) // Limit tooltip length
            .map(|p| format!("{}: {}%", p.name, p.session_percent as i32))
            .collect();
        let tooltip = format!("CodexBar\n{}", tooltip_lines.join("\n"));
        let _ = self.tray_icon.set_tooltip(Some(&tooltip));
    }

    /// Show loading animation on the tray icon
    pub fn show_loading(&self, pattern: LoadingPattern, phase: f64) {
        let primary = pattern.value(phase);
        let secondary = pattern.value(phase + pattern.secondary_offset());

        let icon = create_loading_icon(primary, secondary);
        let _ = self.tray_icon.set_icon(Some(icon));
        let _ = self.tray_icon.set_tooltip(Some("CodexBar - Loading..."));
    }

    /// Show a surprise animation frame
    pub fn show_surprise(&self, animation: SurpriseAnimation, frame: u32, session_percent: f64, weekly_percent: f64) {
        let icon = create_surprise_icon(animation, frame, session_percent, weekly_percent);
        let _ = self.tray_icon.set_icon(Some(icon));
    }

    /// Check for menu events
    pub fn check_events() -> Option<TrayMenuAction> {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            // Check event ID to determine action
            let id_str = event.id.0.as_str();
            if id_str.contains("1001") || id_str.to_lowercase().contains("quit") {
                return Some(TrayMenuAction::Quit);
            } else if id_str.contains("1000") || id_str.to_lowercase().contains("open") {
                return Some(TrayMenuAction::Open);
            }
        }
        None
    }
}

/// Tray menu actions
#[derive(Debug, Clone, Copy)]
pub enum TrayMenuAction {
    Open,
    Quit,
}

/// Create a bar icon showing session and weekly usage with optional overlay
fn create_bar_icon(session_percent: f64, weekly_percent: f64, overlay: IconOverlay) -> Icon {
    let mut img: RgbaImage = ImageBuffer::new(ICON_SIZE, ICON_SIZE);

    // Fill with transparent background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw rounded background - dimmed if error/stale
    let bg_alpha = match overlay {
        IconOverlay::Error | IconOverlay::Stale => 180,
        _ => 255,
    };
    let bg_color = Rgba([60, 60, 70, bg_alpha]);
    for y in 2..ICON_SIZE - 2 {
        for x in 2..ICON_SIZE - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    // Bar dimensions
    let bar_left = 4u32;
    let bar_right = ICON_SIZE - 4;
    let bar_width = bar_right - bar_left;

    // Color adjustment for error/stale states
    let color_adjust = |r: u8, g: u8, b: u8| -> (u8, u8, u8) {
        match overlay {
            IconOverlay::Error => {
                // Grayscale
                let gray = ((r as u16 + g as u16 + b as u16) / 3) as u8;
                (gray, gray, gray)
            }
            IconOverlay::Stale => {
                // Dim colors by 40%
                ((r as f32 * 0.6) as u8, (g as f32 * 0.6) as u8, (b as f32 * 0.6) as u8)
            }
            _ => (r, g, b),
        }
    };

    // Session bar (top, thicker) - y: 8 to 14
    let session_level = UsageLevel::from_percent(session_percent);
    let (sr, sg, sb) = session_level.color();
    let (sr, sg, sb) = color_adjust(sr, sg, sb);
    let session_fill = ((session_percent / 100.0) * bar_width as f64) as u32;

    // Track (gray)
    for y in 8..15 {
        for x in bar_left..bar_right {
            img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
        }
    }
    // Fill (colored)
    for y in 8..15 {
        for x in bar_left..(bar_left + session_fill).min(bar_right) {
            img.put_pixel(x, y, Rgba([sr, sg, sb, 255]));
        }
    }

    // Weekly bar (bottom, thinner) - y: 18 to 22
    let weekly_level = UsageLevel::from_percent(weekly_percent);
    let (wr, wg, wb) = weekly_level.color();
    let (wr, wg, wb) = color_adjust(wr, wg, wb);
    let weekly_fill = ((weekly_percent / 100.0) * bar_width as f64) as u32;

    // Track (gray)
    for y in 18..23 {
        for x in bar_left..bar_right {
            img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
        }
    }
    // Fill (colored)
    for y in 18..23 {
        for x in bar_left..(bar_left + weekly_fill).min(bar_right) {
            img.put_pixel(x, y, Rgba([wr, wg, wb, 255]));
        }
    }

    // Draw overlay badge
    draw_overlay_badge(&mut img, overlay);

    let rgba = img.into_raw();
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon")
}

/// Draw overlay badge on the icon (bottom-right corner)
fn draw_overlay_badge(img: &mut RgbaImage, overlay: IconOverlay) {
    match overlay {
        IconOverlay::None => {}
        IconOverlay::Error => {
            // Red X in bottom-right corner
            let badge_color = Rgba([255, 60, 60, 255]);
            // Draw a small X (6x6 pixels in corner)
            for i in 0..6 {
                // Diagonal line \
                let x = ICON_SIZE - 8 + i;
                let y = ICON_SIZE - 8 + i;
                if x < ICON_SIZE && y < ICON_SIZE {
                    img.put_pixel(x, y, badge_color);
                }
                // Diagonal line /
                let x2 = ICON_SIZE - 3 - i;
                let y2 = ICON_SIZE - 8 + i;
                if x2 < ICON_SIZE && y2 < ICON_SIZE {
                    img.put_pixel(x2, y2, badge_color);
                }
            }
        }
        IconOverlay::Stale => {
            // Clock indicator - small dot in corner
            let badge_color = Rgba([180, 180, 180, 255]);
            // Draw a small circle (clock symbol)
            for dy in 0..4 {
                for dx in 0..4 {
                    let x = ICON_SIZE - 6 + dx;
                    let y = ICON_SIZE - 6 + dy;
                    if x < ICON_SIZE && y < ICON_SIZE {
                        img.put_pixel(x, y, badge_color);
                    }
                }
            }
        }
        IconOverlay::Incident => {
            // Red warning badge
            let badge_color = Rgba([244, 67, 54, 255]);
            // Draw filled circle in corner
            for dy in 0..6 {
                for dx in 0..6 {
                    let x = ICON_SIZE - 8 + dx;
                    let y = ICON_SIZE - 8 + dy;
                    if x < ICON_SIZE && y < ICON_SIZE {
                        img.put_pixel(x, y, badge_color);
                    }
                }
            }
        }
        IconOverlay::Partial => {
            // Orange warning badge
            let badge_color = Rgba([255, 152, 0, 255]);
            // Draw filled circle in corner
            for dy in 0..6 {
                for dx in 0..6 {
                    let x = ICON_SIZE - 8 + dx;
                    let y = ICON_SIZE - 8 + dy;
                    if x < ICON_SIZE && y < ICON_SIZE {
                        img.put_pixel(x, y, badge_color);
                    }
                }
            }
        }
    }
}

/// Create a credits icon showing a thick single bar for credits mode
/// Used when weekly quota is exhausted but paid credits remain
fn create_credits_icon(credits_percent: f64) -> Icon {
    let mut img: RgbaImage = ImageBuffer::new(ICON_SIZE, ICON_SIZE);

    // Fill with transparent background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw rounded background
    let bg_color = Rgba([60, 60, 70, 255]);
    for y in 2..ICON_SIZE - 2 {
        for x in 2..ICON_SIZE - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    // Bar dimensions - thick bar for credits (16px like macOS version)
    let bar_left = 4u32;
    let bar_right = ICON_SIZE - 4;
    let bar_width = bar_right - bar_left;

    // Credits bar - centered and thick (y: 8 to 24)
    let bar_y_start = 8u32;
    let bar_y_end = 24u32;

    // Cyan/blue color for credits
    let credits_color = Rgba([64, 196, 255, 255]);
    let credits_fill = ((credits_percent / 100.0) * bar_width as f64) as u32;

    // Track (gray)
    for y in bar_y_start..bar_y_end {
        for x in bar_left..bar_right {
            img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
        }
    }
    // Fill (cyan)
    for y in bar_y_start..bar_y_end {
        for x in bar_left..(bar_left + credits_fill).min(bar_right) {
            img.put_pixel(x, y, credits_color);
        }
    }

    let rgba = img.into_raw();
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon")
}

/// Create a merged icon showing multiple providers stacked
fn create_merged_icon(providers: &[ProviderUsage]) -> Icon {
    let mut img: RgbaImage = ImageBuffer::new(ICON_SIZE, ICON_SIZE);

    // Fill with transparent background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw rounded background
    let bg_color = Rgba([60, 60, 70, 255]);
    for y in 2..ICON_SIZE - 2 {
        for x in 2..ICON_SIZE - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    // Bar dimensions
    let bar_left = 4u32;
    let bar_right = ICON_SIZE - 4;
    let bar_width = bar_right - bar_left;

    // Calculate bar positions based on provider count
    let provider_count = providers.len().min(4); // Max 4 bars
    if provider_count == 0 {
        let rgba = img.into_raw();
        return Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon");
    }

    // Calculate bar height and spacing to fit within icon
    let total_height = ICON_SIZE - 8; // Leave margin
    let bar_height = (total_height / provider_count as u32).min(6);
    let spacing = if provider_count > 1 {
        (total_height - (bar_height * provider_count as u32)) / (provider_count as u32 - 1).max(1)
    } else {
        0
    };

    for (i, provider) in providers.iter().take(4).enumerate() {
        let y_start = 4 + (i as u32 * (bar_height + spacing));
        let y_end = (y_start + bar_height).min(ICON_SIZE - 4);

        let level = UsageLevel::from_percent(provider.session_percent);
        let (r, g, b) = level.color();
        let fill_width = ((provider.session_percent / 100.0) * bar_width as f64) as u32;

        // Draw track (gray)
        for y in y_start..y_end {
            for x in bar_left..bar_right {
                img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
            }
        }

        // Draw fill (colored)
        for y in y_start..y_end {
            for x in bar_left..(bar_left + fill_width).min(bar_right) {
                img.put_pixel(x, y, Rgba([r, g, b, 255]));
            }
        }
    }

    let rgba = img.into_raw();
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon")
}

/// Create a loading animation icon with animated bars
fn create_loading_icon(primary_percent: f64, secondary_percent: f64) -> Icon {
    let mut img: RgbaImage = ImageBuffer::new(ICON_SIZE, ICON_SIZE);

    // Fill with transparent background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw rounded background
    let bg_color = Rgba([60, 60, 70, 255]);
    for y in 2..ICON_SIZE - 2 {
        for x in 2..ICON_SIZE - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    // Bar dimensions
    let bar_left = 4u32;
    let bar_right = ICON_SIZE - 4;
    let bar_width = bar_right - bar_left;

    // Loading color - cyan/blue gradient
    let loading_color = Rgba([64, 196, 255, 255]);

    // Primary bar (top) - y: 8 to 14
    let primary_fill = ((primary_percent / 100.0) * bar_width as f64) as u32;
    for y in 8..15 {
        for x in bar_left..bar_right {
            img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
        }
    }
    for y in 8..15 {
        for x in bar_left..(bar_left + primary_fill).min(bar_right) {
            img.put_pixel(x, y, loading_color);
        }
    }

    // Secondary bar (bottom) - y: 18 to 22
    let secondary_fill = ((secondary_percent / 100.0) * bar_width as f64) as u32;
    for y in 18..23 {
        for x in bar_left..bar_right {
            img.put_pixel(x, y, Rgba([80, 80, 90, 255]));
        }
    }
    for y in 18..23 {
        for x in bar_left..(bar_left + secondary_fill).min(bar_right) {
            img.put_pixel(x, y, loading_color);
        }
    }

    let rgba = img.into_raw();
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon")
}

/// Create a surprise animation icon frame
fn create_surprise_icon(animation: SurpriseAnimation, frame: u32, session_percent: f64, weekly_percent: f64) -> Icon {
    let mut img: RgbaImage = ImageBuffer::new(ICON_SIZE, ICON_SIZE);

    // Fill with transparent background
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }

    // Draw rounded background
    let bg_color = Rgba([60, 60, 70, 255]);
    for y in 2..ICON_SIZE - 2 {
        for x in 2..ICON_SIZE - 2 {
            img.put_pixel(x, y, bg_color);
        }
    }

    // Bar dimensions
    let bar_left = 4u32;
    let bar_right = ICON_SIZE - 4;
    let bar_width = bar_right - bar_left;

    // Calculate animation parameters
    let total_frames = animation.duration_frames().max(1);
    let progress = frame as f64 / total_frames as f64;

    // Color and position modifiers based on animation type
    let (color_mod, x_offset, y_offset) = match animation {
        SurpriseAnimation::None => ((1.0, 1.0, 1.0), 0i32, 0i32),
        SurpriseAnimation::Blink => {
            // Flash to white and back
            let flash = if progress < 0.5 {
                progress * 2.0  // Fade to white
            } else {
                (1.0 - progress) * 2.0  // Fade back
            };
            let blend = 1.0 + flash * 0.8;  // Boost brightness
            ((blend, blend, blend), 0, 0)
        }
        SurpriseAnimation::Wiggle => {
            // Shake left and right
            let shake = (progress * std::f64::consts::PI * 6.0).sin();  // 3 full oscillations
            let offset = (shake * 2.0) as i32;  // +/- 2 pixels
            ((1.0, 1.0, 1.0), offset, 0)
        }
        SurpriseAnimation::Pulse => {
            // Gentle pulse - grow and shrink brightness
            let pulse = (progress * std::f64::consts::PI * 2.0).sin();  // One full cycle
            let intensity = 1.0 + pulse * 0.3;  // +/- 30% brightness
            ((intensity, intensity, intensity), 0, 0)
        }
        SurpriseAnimation::Rainbow => {
            // Sweep through rainbow colors
            let hue = progress * 360.0;
            let (r, g, b) = hsv_to_rgb(hue, 0.8, 1.0);
            ((r as f64 / 255.0 * 2.0, g as f64 / 255.0 * 2.0, b as f64 / 255.0 * 2.0), 0, 0)
        }
        SurpriseAnimation::Tilt => {
            // Tilt effect - slight diagonal shift that returns
            let tilt = (progress * std::f64::consts::PI).sin();  // 0 -> 1 -> 0
            let x_off = (tilt * 2.0) as i32;  // +2 pixels at peak
            let y_off = (tilt * 1.0) as i32;  // +1 pixel at peak (slight diagonal)
            ((1.0, 1.0, 1.0), x_off, y_off)
        }
    };

    // Session bar (top) - y: 8 to 14
    let session_level = UsageLevel::from_percent(session_percent);
    let (sr, sg, sb) = session_level.color();
    let sr = ((sr as f64 * color_mod.0).min(255.0)) as u8;
    let sg = ((sg as f64 * color_mod.1).min(255.0)) as u8;
    let sb = ((sb as f64 * color_mod.2).min(255.0)) as u8;
    let session_fill = ((session_percent / 100.0) * bar_width as f64) as u32;

    // Track (gray)
    for y in 8..15 {
        for x in bar_left..bar_right {
            let adjusted_x = (x as i32 + x_offset).max(bar_left as i32).min(bar_right as i32 - 1) as u32;
            let adjusted_y = (y as i32 + y_offset).max(4).min(ICON_SIZE as i32 - 4) as u32;
            img.put_pixel(adjusted_x, adjusted_y, Rgba([80, 80, 90, 255]));
        }
    }
    // Fill (colored with animation)
    for y in 8..15 {
        for x in bar_left..(bar_left + session_fill).min(bar_right) {
            let adjusted_x = (x as i32 + x_offset).max(bar_left as i32).min(bar_right as i32 - 1) as u32;
            let adjusted_y = (y as i32 + y_offset).max(4).min(ICON_SIZE as i32 - 4) as u32;
            img.put_pixel(adjusted_x, adjusted_y, Rgba([sr, sg, sb, 255]));
        }
    }

    // Weekly bar (bottom) - y: 18 to 22
    let weekly_level = UsageLevel::from_percent(weekly_percent);
    let (wr, wg, wb) = weekly_level.color();
    let wr = ((wr as f64 * color_mod.0).min(255.0)) as u8;
    let wg = ((wg as f64 * color_mod.1).min(255.0)) as u8;
    let wb = ((wb as f64 * color_mod.2).min(255.0)) as u8;
    let weekly_fill = ((weekly_percent / 100.0) * bar_width as f64) as u32;

    // Track (gray)
    for y in 18..23 {
        for x in bar_left..bar_right {
            let adjusted_x = (x as i32 + x_offset).max(bar_left as i32).min(bar_right as i32 - 1) as u32;
            let adjusted_y = (y as i32 + y_offset).max(4).min(ICON_SIZE as i32 - 4) as u32;
            img.put_pixel(adjusted_x, adjusted_y, Rgba([80, 80, 90, 255]));
        }
    }
    // Fill (colored with animation)
    for y in 18..23 {
        for x in bar_left..(bar_left + weekly_fill).min(bar_right) {
            let adjusted_x = (x as i32 + x_offset).max(bar_left as i32).min(bar_right as i32 - 1) as u32;
            let adjusted_y = (y as i32 + y_offset).max(4).min(ICON_SIZE as i32 - 4) as u32;
            img.put_pixel(adjusted_x, adjusted_y, Rgba([wr, wg, wb, 255]));
        }
    }

    let rgba = img.into_raw();
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("Failed to create icon")
}

/// Convert HSV to RGB (h: 0-360, s: 0-1, v: 0-1)
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bar_icon() {
        // Just verify it doesn't panic
        let _icon = create_bar_icon(50.0, 25.0, IconOverlay::None);
        let _icon = create_bar_icon(0.0, 0.0, IconOverlay::None);
        let _icon = create_bar_icon(100.0, 100.0, IconOverlay::None);
    }

    #[test]
    fn test_create_bar_icon_with_overlays() {
        // Test all overlay types
        let _icon = create_bar_icon(50.0, 25.0, IconOverlay::Error);
        let _icon = create_bar_icon(50.0, 25.0, IconOverlay::Stale);
        let _icon = create_bar_icon(50.0, 25.0, IconOverlay::Incident);
        let _icon = create_bar_icon(50.0, 25.0, IconOverlay::Partial);
    }
}
