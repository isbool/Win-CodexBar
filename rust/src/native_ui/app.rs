//! Main egui application - macOS-style menubar popup
//! Clean, working implementation with proper layout

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Rect, RichText, Rounding, Stroke, Vec2};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::preferences::PreferencesWindow;
use super::theme::{provider_icon, Theme};
use crate::core::{FetchContext, Provider, ProviderId, ProviderFetchResult, RateWindow};
use crate::providers::*;
use crate::settings::{ManualCookies, Settings};
use crate::status::{fetch_provider_status, get_status_page_url, StatusLevel};
use crate::tray::{IconOverlay, LoadingPattern, ProviderUsage, SurpriseAnimation, TrayManager, TrayMenuAction};

const MAIN_PROVIDER_COUNT: usize = 6;

#[derive(Clone, Debug)]
pub struct ProviderData {
    pub name: String,
    pub display_name: String,
    pub session_percent: Option<f64>,
    pub session_reset: Option<String>,
    pub weekly_percent: Option<f64>,
    pub weekly_reset: Option<String>,
    pub model_percent: Option<f64>,
    pub model_name: Option<String>,
    pub plan: Option<String>,
    pub error: Option<String>,
    pub dashboard_url: Option<String>,
    // Pace info: difference from expected usage (negative = behind, positive = ahead)
    pub pace_percent: Option<f64>,
    pub pace_lasts_to_reset: bool,
    // Cost info
    pub cost_used: Option<String>,
    pub cost_limit: Option<String>,
    pub cost_period: Option<String>,
    // Credits info (separate from cost - shows as a bar)
    pub credits_remaining: Option<f64>,
    pub credits_percent: Option<f64>,
    // Status/incident info
    pub status_level: StatusLevel,
    pub status_description: Option<String>,
}

impl ProviderData {
    fn from_result(id: ProviderId, result: &ProviderFetchResult, metadata: &crate::core::ProviderMetadata) -> Self {
        let snapshot = &result.usage;
        // Calculate pace based on primary rate window
        let (pace_percent, pace_lasts) = calculate_pace(&snapshot.primary);

        // Extract cost info if available
        let (cost_used, cost_limit, cost_period, credits_remaining, credits_percent) = if let Some(ref cost) = result.cost {
            // Check if this is a "Credits" type (Codex) vs regular cost
            if cost.period == "Credits" {
                // For credits, `used` is actually the balance (credits remaining)
                // Full scale is 1000 credits per the original macOS app
                const FULL_SCALE_CREDITS: f64 = 1000.0;
                let remaining = cost.used;
                let percent = (remaining / FULL_SCALE_CREDITS * 100.0).clamp(0.0, 100.0);
                (None, None, None, Some(remaining), Some(percent))
            } else {
                (
                    Some(cost.format_used()),
                    cost.format_limit(),
                    Some(cost.period.clone()),
                    None,
                    None,
                )
            }
        } else {
            (None, None, None, None, None)
        };

        Self {
            name: id.cli_name().to_string(),
            display_name: id.display_name().to_string(),
            session_percent: Some(snapshot.primary.used_percent),
            session_reset: snapshot.primary.resets_at.map(format_reset_time),
            weekly_percent: snapshot.secondary.as_ref().map(|s| s.used_percent),
            weekly_reset: snapshot.secondary.as_ref().and_then(|s| s.resets_at.map(format_reset_time)),
            model_percent: snapshot.model_specific.as_ref().map(|m| m.used_percent),
            model_name: snapshot.model_specific.as_ref().and_then(|m| m.reset_description.clone()),
            plan: snapshot.login_method.clone(),
            error: None,
            dashboard_url: metadata.dashboard_url.map(|s| s.to_string()),
            pace_percent,
            pace_lasts_to_reset: pace_lasts,
            cost_used,
            cost_limit,
            cost_period,
            credits_remaining,
            credits_percent,
            status_level: StatusLevel::Unknown, // Will be updated by status fetch
            status_description: None,
        }
    }

    fn from_error(id: ProviderId, error: String) -> Self {
        Self {
            name: id.cli_name().to_string(),
            display_name: id.display_name().to_string(),
            session_percent: None,
            session_reset: None,
            weekly_percent: None,
            weekly_reset: None,
            model_percent: None,
            model_name: None,
            plan: None,
            error: Some(error),
            dashboard_url: None,
            pace_percent: None,
            pace_lasts_to_reset: false,
            cost_used: None,
            cost_limit: None,
            cost_period: None,
            credits_remaining: None,
            credits_percent: None,
            status_level: StatusLevel::Unknown,
            status_description: None,
        }
    }
}

fn format_reset_time(reset: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = reset - now;

    if diff.num_seconds() <= 0 {
        return "Resetting...".to_string();
    }

    let hours = diff.num_hours();
    let minutes = (diff.num_minutes() % 60).abs();

    if hours >= 24 {
        let days = hours / 24;
        let remaining_hours = hours % 24;
        format!("{}d {}h", days, remaining_hours)
    } else {
        format!("{}h {}m", hours, minutes)
    }
}

/// Calculate pace: how much you're ahead/behind expected usage
/// Returns (pace_percent, lasts_to_reset)
/// pace_percent: negative means behind (using less than expected), positive means ahead
/// lasts_to_reset: true if current usage rate will last until reset
fn calculate_pace(rate_window: &RateWindow) -> (Option<f64>, bool) {
    let Some(window_minutes) = rate_window.window_minutes else {
        return (None, false);
    };
    let Some(resets_at) = rate_window.resets_at else {
        return (None, false);
    };

    let now = chrono::Utc::now();
    let time_remaining = resets_at - now;
    let remaining_minutes = time_remaining.num_minutes() as f64;

    if remaining_minutes <= 0.0 {
        return (None, false);
    }

    let total_minutes = window_minutes as f64;
    let elapsed_minutes = total_minutes - remaining_minutes;

    if elapsed_minutes <= 0.0 {
        return (None, false);
    }

    // Expected usage at this point in the window (linear model)
    let expected_percent = (elapsed_minutes / total_minutes) * 100.0;
    let actual_percent = rate_window.used_percent;

    // Pace: actual - expected
    // Negative = behind (using less), Positive = ahead (using more)
    let pace = actual_percent - expected_percent;

    // Will usage last to reset? If actual < expected, yes
    let lasts_to_reset = actual_percent <= expected_percent;

    (Some(pace), lasts_to_reset)
}

/// Generate a random delay for the next surprise animation (30 seconds to 5 minutes)
fn random_surprise_delay() -> Duration {
    use rand::Rng;
    let mut rng = rand::rng();
    let secs = rng.random_range(30..300);
    Duration::from_secs(secs)
}

struct SharedState {
    providers: Vec<ProviderData>,
    last_refresh: Instant,
    is_refreshing: bool,
    loading_pattern: LoadingPattern,
    loading_phase: f64,
    // Surprise animation state
    surprise_animation: Option<SurpriseAnimation>,
    surprise_frame: u32,
    next_surprise_time: Instant,
}

pub struct CodexBarApp {
    state: Arc<Mutex<SharedState>>,
    selected_provider: usize,
    settings: Settings,
    tray_manager: Option<TrayManager>,
    preferences_window: PreferencesWindow,
}

impl CodexBarApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load Windows symbol font
        let mut fonts = FontDefinitions::default();
        if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
            fonts.font_data.insert(
                "segoe_symbols".to_owned(),
                FontData::from_owned(font_data).into(),
            );
            fonts.families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .push("segoe_symbols".to_owned());
        }
        cc.egui_ctx.set_fonts(fonts);

        let settings = Settings::load();
        let enabled_ids = settings.get_enabled_provider_ids();

        // Create placeholder providers immediately so UI shows right away
        let placeholders: Vec<ProviderData> = enabled_ids
            .iter()
            .map(|&id| ProviderData {
                name: id.cli_name().to_string(),
                display_name: id.display_name().to_string(),
                session_percent: None,
                session_reset: None,
                weekly_percent: None,
                weekly_reset: None,
                model_percent: None,
                model_name: None,
                plan: None,
                error: None,
                dashboard_url: None,
                pace_percent: None,
                pace_lasts_to_reset: false,
                cost_used: None,
                cost_limit: None,
                cost_period: None,
                credits_remaining: None,
                credits_percent: None,
                status_level: StatusLevel::Unknown,
                status_description: None,
            })
            .collect();

        let state = Arc::new(Mutex::new(SharedState {
            providers: placeholders,
            last_refresh: Instant::now() - Duration::from_secs(999), // Force immediate refresh
            is_refreshing: false,
            loading_pattern: LoadingPattern::random(),
            loading_phase: 0.0,
            surprise_animation: None,
            surprise_frame: 0,
            next_surprise_time: Instant::now() + random_surprise_delay(),
        }));

        // Initialize system tray
        let tray_manager = match TrayManager::new() {
            Ok(tm) => Some(tm),
            Err(e) => {
                tracing::warn!("Failed to create tray manager: {}", e);
                None
            }
        };

        Self {
            state,
            selected_provider: 0,
            settings,
            tray_manager,
            preferences_window: PreferencesWindow::new(),
        }
    }

    fn refresh_providers(&self) {
        let state = Arc::clone(&self.state);
        let enabled_ids = self.settings.get_enabled_provider_ids();
        let manual_cookies = ManualCookies::load();

        std::thread::spawn(move || {
            if let Ok(mut s) = state.lock() {
                s.is_refreshing = true;
                s.loading_pattern = LoadingPattern::random();
                s.loading_phase = 0.0;
            }

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Spawn all provider fetches concurrently
                let handles: Vec<_> = enabled_ids
                    .iter()
                    .enumerate()
                    .map(|(idx, &id)| {
                        // Create fetch context with manual cookie if available
                        let manual_cookie = manual_cookies.get(id.cli_name()).map(|s| s.to_string());
                        let ctx = FetchContext {
                            manual_cookie_header: manual_cookie,
                            ..FetchContext::default()
                        };
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            let provider = create_provider(id);
                            let metadata = provider.metadata().clone();
                            let provider_name = id.cli_name().to_string();

                            // Fetch usage and status in parallel
                            let (usage_result, status_result) = tokio::join!(
                                async {
                                    tokio::time::timeout(
                                        std::time::Duration::from_secs(5),
                                        provider.fetch_usage(&ctx)
                                    ).await
                                },
                                async {
                                    tokio::time::timeout(
                                        std::time::Duration::from_secs(5),
                                        fetch_provider_status(&provider_name)
                                    ).await
                                }
                            );

                            // Create provider data from usage result
                            let mut result = match usage_result {
                                Ok(Ok(result)) => ProviderData::from_result(id, &result, &metadata),
                                Ok(Err(e)) => ProviderData::from_error(id, e.to_string()),
                                Err(_) => ProviderData::from_error(id, "Timeout".to_string()),
                            };

                            // Update status from status result
                            if let Ok(Some(status)) = status_result {
                                result.status_level = status.level;
                                result.status_description = Some(status.description);
                            }

                            // Update UI immediately as each provider completes
                            if let Ok(mut s) = state.lock() {
                                if idx < s.providers.len() {
                                    s.providers[idx] = result;
                                }
                            }
                        })
                    })
                    .collect();

                // Wait for all to complete
                for handle in handles {
                    let _ = handle.await;
                }
            });

            if let Ok(mut s) = state.lock() {
                s.last_refresh = Instant::now();
                s.is_refreshing = false;
            }
        });
    }
}

fn create_provider(id: ProviderId) -> Box<dyn Provider> {
    match id {
        ProviderId::Claude => Box::new(ClaudeProvider::new()),
        ProviderId::Codex => Box::new(CodexProvider::new()),
        ProviderId::Cursor => Box::new(CursorProvider::new()),
        ProviderId::Gemini => Box::new(GeminiProvider::new()),
        ProviderId::Copilot => Box::new(CopilotProvider::new()),
        ProviderId::Antigravity => Box::new(AntigravityProvider::new()),
        ProviderId::Factory => Box::new(FactoryProvider::new()),
        ProviderId::Zai => Box::new(ZaiProvider::new()),
        ProviderId::Kiro => Box::new(KiroProvider::new()),
        ProviderId::VertexAI => Box::new(VertexAIProvider::new()),
        ProviderId::Augment => Box::new(AugmentProvider::new()),
        ProviderId::MiniMax => Box::new(MiniMaxProvider::new()),
        ProviderId::OpenCode => Box::new(OpenCodeProvider::new()),
        ProviderId::Kimi => Box::new(KimiProvider::new()),
        ProviderId::KimiK2 => Box::new(KimiK2Provider::new()),
    }
}

impl eframe::App for CodexBarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Auto-refresh check (skip if interval is 0 = Manual)
        let should_refresh = {
            if self.settings.refresh_interval_secs == 0 {
                // Manual mode - no auto refresh
                false
            } else if let Ok(state) = self.state.lock() {
                !state.is_refreshing && state.last_refresh.elapsed() > Duration::from_secs(self.settings.refresh_interval_secs)
            } else {
                false
            }
        };
        if should_refresh {
            self.refresh_providers();
        }

        // Get state
        let (providers, is_refreshing, loading_pattern, loading_phase, surprise_state) = {
            if let Ok(mut state) = self.state.lock() {
                // Advance loading animation phase
                if state.is_refreshing {
                    state.loading_phase += 0.05; // Advance animation
                    if state.loading_phase > 1.0 {
                        state.loading_phase -= 1.0;
                    }
                }

                // Handle surprise animations (only if enabled in settings)
                let surprise = if self.settings.surprise_animations && !state.is_refreshing {
                    if let Some(anim) = state.surprise_animation {
                        // Advance animation frame
                        state.surprise_frame += 1;
                        if state.surprise_frame >= anim.duration_frames() {
                            // Animation complete
                            state.surprise_animation = None;
                            state.surprise_frame = 0;
                            state.next_surprise_time = Instant::now() + random_surprise_delay();
                            None
                        } else {
                            Some((anim, state.surprise_frame))
                        }
                    } else if Instant::now() >= state.next_surprise_time {
                        // Time for a new surprise!
                        let anim = SurpriseAnimation::random();
                        state.surprise_animation = Some(anim);
                        state.surprise_frame = 0;
                        Some((anim, 0))
                    } else {
                        None
                    }
                } else {
                    None
                };

                (state.providers.clone(), state.is_refreshing, state.loading_pattern, state.loading_phase, surprise)
            } else {
                (Vec::new(), false, LoadingPattern::default(), 0.0, None)
            }
        };

        // Request repaint - faster during loading or surprise animation
        ctx.request_repaint_after(if is_refreshing || surprise_state.is_some() {
            Duration::from_millis(50) // ~20fps for smooth animation
        } else {
            Duration::from_secs(1)
        });

        // Update tray icon with selected provider's usage
        if let Some(ref tray) = self.tray_manager {
            if is_refreshing {
                // Show loading animation
                tray.show_loading(loading_pattern, loading_phase);
            } else if let Some((anim, frame)) = surprise_state {
                // Show surprise animation (only in single provider mode for now)
                if let Some(provider) = providers.get(self.selected_provider) {
                    let session = provider.session_percent.unwrap_or(0.0);
                    let weekly = provider.weekly_percent.unwrap_or(session);
                    tray.show_surprise(anim, frame, session, weekly);
                }
            } else if self.settings.merge_tray_icons {
                // Merged mode: show all providers in one icon
                let provider_usages: Vec<ProviderUsage> = providers
                    .iter()
                    .filter(|p| p.session_percent.is_some())
                    .map(|p| ProviderUsage {
                        name: p.display_name.clone(),
                        session_percent: p.session_percent.unwrap_or(0.0),
                        weekly_percent: p.weekly_percent.unwrap_or(0.0),
                    })
                    .collect();
                tray.update_merged(&provider_usages);
            } else {
                // Single provider mode with overlay support
                if let Some(provider) = providers.get(self.selected_provider) {
                    let session = provider.session_percent.unwrap_or(0.0);
                    let weekly = provider.weekly_percent.unwrap_or(session);

                    // Check for credits mode: weekly exhausted but credits remain
                    // Weekly is exhausted if used >= 99% (showing almost 100%)
                    let weekly_exhausted = weekly >= 99.0;
                    let has_credits = provider.credits_percent.is_some() && provider.credits_percent.unwrap_or(0.0) > 0.0;

                    if weekly_exhausted && has_credits {
                        // Credits mode: show thick credits bar
                        tray.update_credits_mode(provider.credits_percent.unwrap_or(0.0), &provider.display_name);
                    } else {
                        // Determine overlay based on provider state
                        let overlay = if provider.error.is_some() {
                            IconOverlay::Error
                        } else {
                            match provider.status_level {
                                StatusLevel::Major => IconOverlay::Incident,
                                StatusLevel::Partial => IconOverlay::Partial,
                                StatusLevel::Degraded => IconOverlay::Partial,
                                _ => IconOverlay::None,
                            }
                        };

                        if overlay != IconOverlay::None {
                            tray.update_usage_with_overlay(session, weekly, &provider.display_name, overlay);
                        } else {
                            tray.update_usage(session, weekly, &provider.display_name);
                        }
                    }
                }
            }

            // Check for tray menu events
            if let Some(action) = TrayManager::check_events() {
                match action {
                    TrayMenuAction::Quit => std::process::exit(0),
                    TrayMenuAction::Open => {
                        // Window is already open, just bring to front
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                }
            }
        }

        // Apply clean style
        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill = Theme::BG_PRIMARY;
        style.visuals.panel_fill = Theme::BG_PRIMARY;
        ctx.set_style(style);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Theme::BG_PRIMARY).inner_margin(16.0))
            .show(ctx, |ui| {
                // ════════════════════════════════════════════════════════════
                // PROVIDERS CARD - Tab bar + More Providers together
                // ════════════════════════════════════════════════════════════

                egui::Frame::none()
                    .fill(Theme::CARD_BG)
                    .rounding(Rounding::same(14.0))
                    .inner_margin(12.0)
                    .stroke(Stroke::new(1.0, Theme::CARD_BORDER))
                    .show(ui, |ui| {
                        // Main tab bar
                        egui::Frame::none()
                            .fill(Theme::TAB_CONTAINER)
                            .rounding(Rounding::same(10.0))
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                    let main_count = providers.len().min(MAIN_PROVIDER_COUNT);
                                    for idx in 0..main_count {
                                        let provider = &providers[idx];
                                        let is_selected = idx == self.selected_provider;
                                        let icon = provider_icon(&provider.name);

                                        let (bg, text_color) = if is_selected {
                                            (Theme::TAB_ACTIVE, Theme::TAB_TEXT_ACTIVE)
                                        } else {
                                            (Color32::TRANSPARENT, Theme::TAB_TEXT_INACTIVE)
                                        };

                                        let btn = egui::Button::new(
                                            RichText::new(format!("{}\n{}", icon, provider.display_name))
                                                .size(11.0)
                                                .color(text_color),
                                        )
                                        .fill(bg)
                                        .stroke(Stroke::NONE)
                                        .rounding(Rounding::same(8.0))
                                        .min_size(Vec2::new(52.0, 44.0));

                                        if ui.add(btn).clicked() {
                                            self.selected_provider = idx;
                                        }
                                    }

                                    if is_refreshing {
                                        ui.spinner();
                                    }
                                });
                            });

                        // More Providers - directly below main tabs, inside same card
                        if providers.len() > MAIN_PROVIDER_COUNT {
                            ui.add_space(8.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);

                                ui.label(
                                    RichText::new("More:")
                                        .size(10.0)
                                        .color(Theme::TEXT_MUTED),
                                );

                                for idx in MAIN_PROVIDER_COUNT..providers.len() {
                                    let provider = &providers[idx];
                                    let is_selected = idx == self.selected_provider;
                                    let icon = provider_icon(&provider.name);

                                    let (bg, text_color) = if is_selected {
                                        (Theme::TAB_ACTIVE, Color32::WHITE)
                                    } else {
                                        (Theme::TAB_INACTIVE, Theme::TAB_TEXT_INACTIVE)
                                    };

                                    let btn = egui::Button::new(
                                        RichText::new(format!("{} {}", icon, provider.display_name))
                                            .size(9.0)
                                            .color(text_color),
                                    )
                                    .fill(bg)
                                    .stroke(Stroke::NONE)
                                    .rounding(Rounding::same(5.0));

                                    if ui.add(btn).clicked() {
                                        self.selected_provider = idx;
                                    }
                                }
                            });
                        }
                    });

                ui.add_space(12.0);

                // ════════════════════════════════════════════════════════════
                // PROVIDER DETAIL
                // ════════════════════════════════════════════════════════════

                if let Some(provider) = providers.get(self.selected_provider) {
                    egui::Frame::none()
                        .fill(Theme::CARD_BG)
                        .rounding(Rounding::same(14.0))
                        .inner_margin(16.0)
                        .stroke(Stroke::new(1.0, Theme::CARD_BORDER))
                        .show(ui, |ui| {
                            // Header row
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&provider.display_name)
                                            .size(18.0)
                                            .color(Theme::TEXT_PRIMARY)
                                            .strong(),
                                    );
                                    ui.label(
                                        RichText::new("Updated just now")
                                            .size(11.0)
                                            .color(Theme::TEXT_MUTED),
                                    );
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                    // Status badge (show if not operational)
                                    if provider.status_level != StatusLevel::Operational && provider.status_level != StatusLevel::Unknown {
                                        let (badge_color, badge_text) = match provider.status_level {
                                            StatusLevel::Degraded => (Theme::YELLOW, "⚠"),
                                            StatusLevel::Partial => (Theme::ORANGE, "⚠"),
                                            StatusLevel::Major => (Theme::RED, "⛔"),
                                            _ => (Theme::TEXT_MUTED, "?"),
                                        };
                                        egui::Frame::none()
                                            .fill(badge_color)
                                            .rounding(Rounding::same(6.0))
                                            .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(badge_text)
                                                        .size(10.0)
                                                        .color(Color32::WHITE),
                                                );
                                            });
                                        ui.add_space(4.0);
                                    }

                                    if let Some(plan) = &provider.plan {
                                        egui::Frame::none()
                                            .fill(Theme::TAB_ACTIVE)
                                            .rounding(Rounding::same(6.0))
                                            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(plan)
                                                        .size(10.0)
                                                        .color(Color32::WHITE)
                                                        .strong(),
                                                );
                                            });
                                    }
                                });
                            });

                            ui.add_space(14.0);

                            // Usage sections
                            if provider.error.is_some() {
                                ui.label(
                                    RichText::new("Unable to fetch usage data")
                                        .size(12.0)
                                        .color(Theme::TEXT_MUTED),
                                );
                            } else {
                                if let Some(pct) = provider.session_percent {
                                    draw_usage_bar(ui, "Session", pct, provider.session_reset.as_deref());
                                    ui.add_space(10.0);
                                }

                                if let Some(pct) = provider.weekly_percent {
                                    draw_usage_bar(ui, "Weekly", pct, provider.weekly_reset.as_deref());
                                    ui.add_space(10.0);
                                }

                                if let Some(pct) = provider.model_percent {
                                    let name = provider.model_name.as_deref().unwrap_or("Model");
                                    draw_usage_bar(ui, name, pct, None);
                                    ui.add_space(10.0);
                                }

                                // Pace info
                                if let Some(pace) = provider.pace_percent {
                                    draw_pace_info(ui, pace, provider.pace_lasts_to_reset);
                                }

                                // Credits section (Codex)
                                if let Some(credits_pct) = provider.credits_percent {
                                    ui.add_space(12.0);
                                    draw_credits_bar(ui, credits_pct, provider.credits_remaining);
                                }

                                // Cost section
                                if let Some(ref cost_used) = provider.cost_used {
                                    ui.add_space(12.0);
                                    draw_cost_section(ui, cost_used, provider.cost_limit.as_deref(), provider.cost_period.as_deref());
                                }
                            }
                        });
                } else if providers.is_empty() {
                    egui::Frame::none()
                        .fill(Theme::CARD_BG)
                        .rounding(Rounding::same(14.0))
                        .inner_margin(24.0)
                        .stroke(Stroke::new(1.0, Theme::CARD_BORDER))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.spinner();
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new("Loading providers...")
                                        .size(13.0)
                                        .color(Theme::TEXT_MUTED),
                                );
                            });
                        });
                }

                ui.add_space(12.0);

                // ════════════════════════════════════════════════════════════
                // MENU BUTTONS
                // ════════════════════════════════════════════════════════════

                egui::Frame::none()
                    .fill(Theme::CARD_BG)
                    .rounding(Rounding::same(14.0))
                    .inner_margin(8.0)
                    .stroke(Stroke::new(1.0, Theme::CARD_BORDER))
                    .show(ui, |ui| {
                        if menu_button(ui, "🔄", "Refresh") {
                            self.refresh_providers();
                        }

                        if menu_button(ui, "📊", "Usage Dashboard") {
                            if let Some(p) = providers.get(self.selected_provider) {
                                if let Some(url) = &p.dashboard_url {
                                    let _ = open::that(url);
                                }
                            }
                        }

                        if menu_button(ui, "📈", "Status Page") {
                            // Use provider-specific status page URL
                            if let Some(p) = providers.get(self.selected_provider) {
                                if let Some(url) = get_status_page_url(&p.name) {
                                    let _ = open::that(url);
                                }
                            }
                        }

                        if menu_button(ui, "⚙", "Settings...") {
                            self.preferences_window.open();
                        }

                        if menu_button(ui, "ℹ", "About CodexBar") {
                            self.preferences_window.active_tab = super::preferences::PreferencesTab::About;
                            self.preferences_window.open();
                        }

                        // Separator
                        ui.add_space(4.0);
                        let sep_rect = ui.available_rect_before_wrap();
                        ui.painter().hline(
                            sep_rect.x_range(),
                            sep_rect.top(),
                            Stroke::new(1.0, Theme::SEPARATOR),
                        );
                        ui.add_space(4.0);

                        if menu_button(ui, "✕", "Quit") {
                            std::process::exit(0);
                        }
                    });
            });

        // Show preferences window if open
        self.preferences_window.show(ctx);

        // Sync settings if preferences changed
        if self.preferences_window.settings_changed {
            self.settings = self.preferences_window.settings.clone();
        }
    }
}

/// Draw a usage bar with label and percentage (thin macOS-style)
fn draw_usage_bar(ui: &mut egui::Ui, label: &str, percent: f64, reset: Option<&str>) {
    let color = Theme::usage_color(percent);

    // Label row
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(14.0).color(Theme::TEXT_PRIMARY));
    });

    ui.add_space(2.0);

    // Thin progress bar (macOS style)
    let bar_height = 4.0;
    let available_width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(available_width, bar_height), egui::Sense::hover());

    ui.painter().rect_filled(rect, Rounding::same(2.0), Theme::PROGRESS_TRACK);

    let fill_w = rect.width() * (percent as f32 / 100.0);
    if fill_w > 0.0 {
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, bar_height));
        ui.painter().rect_filled(fill_rect, Rounding::same(2.0), color);
    }

    ui.add_space(4.0);

    // Stats row
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{}% used", percent as i32)).size(11.0).color(Theme::TEXT_MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(r) = reset {
                ui.label(RichText::new(format!("Resets in {}", r)).size(11.0).color(Theme::TEXT_MUTED));
            }
        });
    });
}

/// Draw pace info showing if user is ahead or behind expected usage
fn draw_pace_info(ui: &mut egui::Ui, pace_percent: f64, lasts_to_reset: bool) {
    let (pace_label, pace_color) = if pace_percent <= -5.0 {
        // Significantly behind (using less than expected) - good!
        ("Behind", Theme::USAGE_GREEN)
    } else if pace_percent >= 5.0 {
        // Significantly ahead (using more than expected) - warning
        ("Ahead", Theme::USAGE_ORANGE)
    } else {
        // On pace
        ("On pace", Theme::TEXT_MUTED)
    };

    let pace_text = if pace_percent.abs() >= 1.0 {
        format!("Pace: {} ({:+.0}%)", pace_label, pace_percent)
    } else {
        format!("Pace: {}", pace_label)
    };

    let lasts_text = if lasts_to_reset {
        " · Lasts to reset"
    } else {
        ""
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(pace_text).size(11.0).color(pace_color));
        if !lasts_text.is_empty() {
            ui.label(RichText::new(lasts_text).size(11.0).color(Theme::USAGE_GREEN));
        }
    });
}

/// Draw credits bar section (for Codex)
fn draw_credits_bar(ui: &mut egui::Ui, percent_remaining: f64, credits_remaining: Option<f64>) {
    // Separator line
    let sep_rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        sep_rect.x_range(),
        sep_rect.top(),
        Stroke::new(1.0, Theme::SEPARATOR),
    );
    ui.add_space(8.0);

    // Credits header
    ui.horizontal(|ui| {
        ui.label(RichText::new("🎫").size(12.0));
        ui.label(
            RichText::new("Codex Credits")
                .size(12.0)
                .color(Theme::TEXT_PRIMARY)
                .strong(),
        );
    });

    ui.add_space(4.0);

    // Credits bar (shows remaining, so we use a cyan/blue color)
    let bar_height = 4.0;
    let available_width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(available_width, bar_height), egui::Sense::hover());

    // Track (gray)
    ui.painter().rect_filled(rect, Rounding::same(2.0), Theme::PROGRESS_TRACK);

    // Fill (cyan for remaining credits)
    let fill_w = rect.width() * (percent_remaining as f32 / 100.0);
    if fill_w > 0.0 {
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, bar_height));
        // Use cyan for credits remaining (similar to loading animation)
        let credits_color = Color32::from_rgb(64, 196, 255);
        ui.painter().rect_filled(fill_rect, Rounding::same(2.0), credits_color);
    }

    ui.add_space(4.0);

    // Credits details
    ui.horizontal(|ui| {
        if let Some(remaining) = credits_remaining {
            ui.label(RichText::new(format!("{:.1} left", remaining)).size(11.0).color(Theme::TEXT_MUTED));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(format!("{}% remaining", percent_remaining as i32)).size(11.0).color(Theme::TEXT_MUTED));
        });
    });
}

/// Draw cost/credits section
fn draw_cost_section(ui: &mut egui::Ui, used: &str, limit: Option<&str>, period: Option<&str>) {
    // Separator line
    let sep_rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        sep_rect.x_range(),
        sep_rect.top(),
        Stroke::new(1.0, Theme::SEPARATOR),
    );
    ui.add_space(8.0);

    // Cost header
    ui.horizontal(|ui| {
        ui.label(RichText::new("💰").size(12.0));
        ui.label(
            RichText::new(period.unwrap_or("Cost"))
                .size(12.0)
                .color(Theme::TEXT_PRIMARY)
                .strong(),
        );
    });

    ui.add_space(4.0);

    // Cost details
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} used", used)).size(11.0).color(Theme::TEXT_MUTED));
        if let Some(lim) = limit {
            ui.label(RichText::new(format!(" / {}", lim)).size(11.0).color(Theme::TEXT_MUTED));
        }
    });
}

/// Draw a menu button, returns true if clicked
fn menu_button(ui: &mut egui::Ui, icon: &str, label: &str) -> bool {
    let btn = egui::Button::new(
        RichText::new(format!("{}   {}", icon, label))
            .size(13.0)
            .color(Theme::TEXT_PRIMARY),
    )
    .fill(Color32::TRANSPARENT)
    .stroke(Stroke::NONE)
    .rounding(Rounding::same(6.0))
    .min_size(Vec2::new(ui.available_width(), 30.0));

    ui.add(btn).clicked()
}

/// Run the application
pub fn run() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 520.0])
            .with_min_inner_size([420.0, 520.0])
            .with_max_inner_size([420.0, 520.0])
            .with_resizable(false)
            .with_decorations(true)
            .with_transparent(false)
            .with_always_on_top()
            .with_title("CodexBar"),
        ..Default::default()
    };

    eframe::run_native(
        "CodexBar",
        options,
        Box::new(|cc| Ok(Box::new(CodexBarApp::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    Ok(())
}
