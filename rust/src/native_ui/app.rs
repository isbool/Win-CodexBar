//! Main egui application - macOS-style menubar popup
//! Clean, working implementation with proper layout

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Rect, RichText, Rounding, Stroke, Vec2};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::charts::{ChartPoint, CostHistoryChart};
use super::preferences::PreferencesWindow;
use super::theme::{provider_icon, Theme};
use crate::core::{FetchContext, Provider, ProviderId, ProviderFetchResult, RateWindow};
use crate::cost_scanner::get_daily_cost_history;
use crate::login::{LoginOutcome, LoginPhase};
use crate::providers::*;
use crate::settings::{ManualCookies, Settings};
use crate::shortcuts::ShortcutManager;
use crate::status::{fetch_provider_status, get_status_page_url, StatusLevel};
use crate::tray::{IconOverlay, LoadingPattern, ProviderUsage, SurpriseAnimation, TrayManager, TrayMenuAction, WeeklyIndicatorColors};
use crate::updater::{self, UpdateInfo};

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
    // Cost history for charts (date -> cost USD)
    pub cost_history: Vec<(String, f64)>,
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
            cost_history: Vec::new(), // TODO: Populate from cost scanner
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
            cost_history: Vec::new(),
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
    // Update check state
    update_available: Option<UpdateInfo>,
    update_checked: bool,
    update_dismissed: bool,
    // Login state
    login_provider: Option<String>,
    login_phase: LoginPhase,
    login_message: Option<String>,
    login_auth_url: Option<String>,
}

pub struct CodexBarApp {
    state: Arc<Mutex<SharedState>>,
    selected_provider: usize,
    settings: Settings,
    tray_manager: Option<TrayManager>,
    preferences_window: PreferencesWindow,
    shortcut_manager: Option<ShortcutManager>,
    show_chart: bool,
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
                cost_history: Vec::new(),
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
            update_available: None,
            update_checked: false,
            update_dismissed: false,
            login_provider: None,
            login_phase: LoginPhase::Idle,
            login_message: None,
            login_auth_url: None,
        }));

        // Initialize system tray
        let tray_manager = match TrayManager::new() {
            Ok(tm) => Some(tm),
            Err(e) => {
                tracing::warn!("Failed to create tray manager: {}", e);
                None
            }
        };

        // Check for updates in background
        {
            let state = Arc::clone(&state);
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Some(update) = updater::check_for_updates().await {
                        if let Ok(mut s) = state.lock() {
                            s.update_available = Some(update);
                            s.update_checked = true;
                        }
                    } else if let Ok(mut s) = state.lock() {
                        s.update_checked = true;
                    }
                });
            });
        }

        // Initialize keyboard shortcuts
        let shortcut_manager = match ShortcutManager::new() {
            Ok(sm) => {
                tracing::info!("Keyboard shortcut registered: Ctrl+Shift+U");
                Some(sm)
            }
            Err(e) => {
                tracing::warn!("Failed to register keyboard shortcuts: {}", e);
                None
            }
        };

        Self {
            state,
            selected_provider: 0,
            settings,
            tray_manager,
            preferences_window: PreferencesWindow::new(),
            shortcut_manager,
            show_chart: false,
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

                            // Populate cost history for providers that support it
                            let provider_name_lower = provider_name.to_lowercase();
                            if provider_name_lower == "codex" || provider_name_lower == "claude" {
                                result.cost_history = get_daily_cost_history(&provider_name_lower, 30);
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

    fn start_login(&self, provider_name: &str) {
        let state = Arc::clone(&self.state);
        let provider = provider_name.to_string();

        // Set initial login state
        if let Ok(mut s) = state.lock() {
            s.login_provider = Some(provider.clone());
            s.login_phase = LoginPhase::Requesting;
            s.login_message = Some(format!("Starting {} login...", provider));
            s.login_auth_url = None;
        }

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let state_for_phase = Arc::clone(&state);
                let on_phase = move |phase: LoginPhase| {
                    if let Ok(mut s) = state_for_phase.lock() {
                        s.login_phase = phase;
                        s.login_message = Some(match phase {
                            LoginPhase::Idle => "Ready".to_string(),
                            LoginPhase::Requesting => "Requesting login...".to_string(),
                            LoginPhase::WaitingBrowser => "Complete login in browser...".to_string(),
                            LoginPhase::Complete => "Login complete!".to_string(),
                        });
                    }
                };

                let result = match provider.as_str() {
                    "claude" => crate::login::run_claude_login(120, on_phase).await,
                    "codex" => crate::login::run_codex_login(120, on_phase).await,
                    "gemini" => crate::login::run_gemini_login(120, on_phase).await,
                    "copilot" => crate::login::run_copilot_login(120, on_phase).await,
                    _ => return,
                };

                // Update state with result
                if let Ok(mut s) = state.lock() {
                    s.login_auth_url = result.auth_link.clone();
                    match result.outcome {
                        LoginOutcome::Success => {
                            s.login_phase = LoginPhase::Complete;
                            s.login_message = Some("Login successful!".to_string());
                        }
                        LoginOutcome::TimedOut => {
                            s.login_message = Some("Login timed out. Please try again.".to_string());
                        }
                        LoginOutcome::MissingBinary => {
                            s.login_message = Some(format!("{} CLI not found in PATH", provider));
                        }
                        LoginOutcome::Failed { status } => {
                            s.login_message = Some(format!("Login failed (exit code {})", status));
                        }
                        LoginOutcome::LaunchFailed(ref err) => {
                            s.login_message = Some(format!("Failed to start login: {}", err));
                        }
                    }
                }

                // Auto-clear after a delay on success
                if matches!(result.outcome, LoginOutcome::Success) {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    if let Ok(mut s) = state.lock() {
                        s.login_provider = None;
                        s.login_phase = LoginPhase::Idle;
                        s.login_message = None;
                        s.login_auth_url = None;
                    }
                }
            });
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
        ProviderId::Amp => Box::new(AmpProvider::new()),
        ProviderId::Synthetic => Box::new(SyntheticProvider::new()),
    }
}

impl eframe::App for CodexBarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check keyboard shortcuts - process ALL pending events
        if let Some(ref shortcut_mgr) = self.shortcut_manager {
            while shortcut_mgr.check_events() {
                // Shortcut pressed - bring window to front and make visible
                tracing::info!("Keyboard shortcut triggered - focusing window");
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

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
        let (providers, is_refreshing, loading_pattern, loading_phase, surprise_state, update_info, login_state) = {
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

                // Get update info if not dismissed
                let update = if state.update_dismissed {
                    None
                } else {
                    state.update_available.clone()
                };

                // Get login state
                let login_state = (
                    state.login_provider.clone(),
                    state.login_phase,
                    state.login_message.clone(),
                );

                (state.providers.clone(), state.is_refreshing, state.loading_pattern, state.loading_phase, surprise, update, login_state)
            } else {
                (Vec::new(), false, LoadingPattern::default(), 0.0, None, None, (None, LoginPhase::Idle, None))
            }
        };

        let (login_provider, login_phase, login_message) = login_state;
        let is_logging_in = login_provider.is_some() && login_phase != LoginPhase::Idle;

        // Request repaint - faster during loading, animation, login, or to catch hotkeys
        ctx.request_repaint_after(if is_refreshing || surprise_state.is_some() || is_logging_in {
            Duration::from_millis(50) // ~20fps for smooth animation
        } else {
            Duration::from_millis(200) // Check for hotkeys frequently
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
                // Wrap everything in a scroll area
                egui::ScrollArea::vertical().show(ui, |ui| {
                // ════════════════════════════════════════════════════════════
                // UPDATE BANNER (if available)
                // ════════════════════════════════════════════════════════════

                if let Some(ref update) = update_info {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(45, 140, 255)) // Blue banner
                        .rounding(Rounding::same(10.0))
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("🎉")
                                        .size(14.0),
                                );
                                ui.label(
                                    RichText::new(format!("Update available: {}", update.version))
                                        .size(12.0)
                                        .color(Color32::WHITE)
                                        .strong(),
                                );

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    // Dismiss button
                                    if ui.add(
                                        egui::Button::new(RichText::new("✕").size(12.0).color(Color32::WHITE))
                                            .fill(Color32::TRANSPARENT)
                                            .stroke(Stroke::NONE)
                                    ).clicked() {
                                        if let Ok(mut s) = self.state.lock() {
                                            s.update_dismissed = true;
                                        }
                                    }

                                    // Download button
                                    let download_url = update.download_url.clone();
                                    if ui.add(
                                        egui::Button::new(RichText::new("Download").size(11.0).color(Color32::from_rgb(45, 140, 255)))
                                            .fill(Color32::WHITE)
                                            .rounding(Rounding::same(4.0))
                                    ).clicked() {
                                        let _ = open::that(&download_url);
                                    }
                                });
                            });
                        });
                    ui.add_space(8.0);
                }

                // ════════════════════════════════════════════════════════════
                // PROVIDERS CARD - Refined tab bar design
                // ════════════════════════════════════════════════════════════

                egui::Frame::none()
                    .fill(Theme::CARD_BG)
                    .rounding(Rounding::same(16.0))
                    .inner_margin(16.0)
                    .stroke(Stroke::new(1.0, Theme::CARD_BORDER))
                    .show(ui, |ui| {
                        // Main tab bar - horizontal scrollable area
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                            let main_count = providers.len().min(MAIN_PROVIDER_COUNT);
                            for idx in 0..main_count {
                                let provider = &providers[idx];
                                let is_selected = idx == self.selected_provider;
                                let icon = provider_icon(&provider.name);

                                // Custom tab rendering
                                let tab_width = 56.0;
                                let tab_height = 52.0;
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(tab_width, tab_height),
                                    egui::Sense::click(),
                                );

                                let is_hovered = response.hovered();

                                // Background
                                let bg_color = if is_selected {
                                    Theme::ACCENT_PRIMARY
                                } else if is_hovered {
                                    Theme::SURFACE_ELEVATED
                                } else {
                                    Color32::TRANSPARENT
                                };

                                let rounding = Rounding::same(12.0);
                                ui.painter().rect_filled(rect, rounding, bg_color);

                                // Glow effect for selected
                                if is_selected {
                                    let glow_rect = rect.expand(2.0);
                                    let glow_color = Color32::from_rgba_unmultiplied(0, 212, 255, 25);
                                    ui.painter().rect_filled(glow_rect, Rounding::same(14.0), glow_color);
                                    ui.painter().rect_filled(rect, rounding, bg_color);
                                }

                                // Icon
                                let icon_color = if is_selected {
                                    Color32::WHITE
                                } else if is_hovered {
                                    Theme::ACCENT_PRIMARY
                                } else {
                                    Theme::TEXT_MUTED
                                };

                                let icon_pos = egui::pos2(rect.center().x, rect.min.y + 18.0);
                                ui.painter().text(
                                    icon_pos,
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    egui::FontId::proportional(16.0),
                                    icon_color,
                                );

                                // Label
                                let text_color = if is_selected {
                                    Color32::WHITE
                                } else if is_hovered {
                                    Theme::TEXT_PRIMARY
                                } else {
                                    Theme::TEXT_MUTED
                                };

                                let label_pos = egui::pos2(rect.center().x, rect.max.y - 12.0);
                                ui.painter().text(
                                    label_pos,
                                    egui::Align2::CENTER_CENTER,
                                    &provider.display_name,
                                    egui::FontId::proportional(10.0),
                                    text_color,
                                );

                                // Weekly indicator bar (4px at bottom) - only for non-selected tabs with weekly data
                                if !is_selected {
                                    if let Some(weekly) = provider.weekly_percent {
                                        let indicator_height = 4.0;
                                        let indicator_padding = 6.0;
                                        let indicator_y = rect.max.y - indicator_height - 2.0;
                                        let indicator_width = rect.width() - (indicator_padding * 2.0);

                                        // Track (background)
                                        let track_rect = Rect::from_min_size(
                                            egui::pos2(rect.min.x + indicator_padding, indicator_y),
                                            Vec2::new(indicator_width, indicator_height),
                                        );
                                        let track_color = Color32::from_rgba_unmultiplied(128, 128, 128, 56);
                                        ui.painter().rect_filled(track_rect, Rounding::same(2.0), track_color);

                                        // Fill (remaining = 100 - used)
                                        let remaining = (100.0 - weekly).clamp(0.0, 100.0);
                                        let fill_width = indicator_width * (remaining as f32 / 100.0);
                                        if fill_width > 0.0 {
                                            let fill_rect = Rect::from_min_size(
                                                egui::pos2(rect.min.x + indicator_padding, indicator_y),
                                                Vec2::new(fill_width, indicator_height),
                                            );
                                            // Get provider color
                                            if let Some(pid) = ProviderId::from_cli_name(&provider.name) {
                                                let colors = WeeklyIndicatorColors::for_provider(pid);
                                                let fill_color = Color32::from_rgba_unmultiplied(
                                                    colors.fill.0, colors.fill.1, colors.fill.2, colors.fill.3
                                                );
                                                ui.painter().rect_filled(fill_rect, Rounding::same(2.0), fill_color);
                                            }
                                        }
                                    }
                                }

                                if response.clicked() {
                                    self.selected_provider = idx;
                                }
                            }

                            if is_refreshing {
                                ui.add_space(8.0);
                                ui.spinner();
                            }
                        });

                        // More Providers - compact pill style
                        if providers.len() > MAIN_PROVIDER_COUNT {
                            ui.add_space(12.0);

                            // Separator line
                            let sep_rect = ui.available_rect_before_wrap();
                            ui.painter().hline(
                                sep_rect.x_range(),
                                sep_rect.top(),
                                Stroke::new(1.0, Theme::SEPARATOR),
                            );
                            ui.add_space(12.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);

                                for idx in MAIN_PROVIDER_COUNT..providers.len() {
                                    let provider = &providers[idx];
                                    let is_selected = idx == self.selected_provider;
                                    let icon = provider_icon(&provider.name);

                                    // Pill-style button
                                    let pill_height = 26.0;
                                    let text = format!("{} {}", icon, provider.display_name);
                                    let text_width = ui.fonts(|f| {
                                        f.glyph_width(&egui::FontId::proportional(10.0), ' ') * text.len() as f32 * 0.6
                                    }).max(50.0);

                                    let (rect, response) = ui.allocate_exact_size(
                                        Vec2::new(text_width + 16.0, pill_height),
                                        egui::Sense::click(),
                                    );

                                    let is_hovered = response.hovered();

                                    let bg_color = if is_selected {
                                        Theme::ACCENT_PRIMARY
                                    } else if is_hovered {
                                        Theme::SURFACE_ELEVATED
                                    } else {
                                        Theme::TAB_CONTAINER
                                    };

                                    let text_color = if is_selected {
                                        Color32::WHITE
                                    } else if is_hovered {
                                        Theme::TEXT_PRIMARY
                                    } else {
                                        Theme::TEXT_MUTED
                                    };

                                    // Border for unselected
                                    if !is_selected {
                                        ui.painter().rect_stroke(
                                            rect,
                                            Rounding::same(13.0),
                                            Stroke::new(1.0, Theme::SEPARATOR),
                                        );
                                    }

                                    ui.painter().rect_filled(rect, Rounding::same(13.0), bg_color);

                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        &text,
                                        egui::FontId::proportional(10.0),
                                        text_color,
                                    );

                                    if response.clicked() {
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

                                // Cost history chart (toggle button)
                                if !provider.cost_history.is_empty() || provider.cost_used.is_some() {
                                    ui.add_space(8.0);
                                    if ui.small_button(if self.show_chart { "▼ Hide Chart" } else { "▶ Show Chart" }).clicked() {
                                        self.show_chart = !self.show_chart;
                                    }

                                    if self.show_chart && !provider.cost_history.is_empty() {
                                        ui.add_space(8.0);
                                        let points: Vec<ChartPoint> = provider.cost_history.iter()
                                            .map(|(date, cost)| ChartPoint {
                                                date: date.clone(),
                                                value: *cost,
                                                tokens: None,
                                                model_breakdowns: None,
                                            })
                                            .collect();
                                        let bar_color = Theme::TAB_ACTIVE;
                                        let mut chart = CostHistoryChart::new(points, bar_color);
                                        chart.show(ui);
                                    }
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

                        // Login button for all providers
                        if let Some(p) = providers.get(self.selected_provider) {
                            let supports_cli_login = matches!(
                                p.name.as_str(),
                                "claude" | "codex" | "gemini" | "copilot"
                            );

                            // Get web login URL for providers without CLI login
                            let web_login_url = match p.name.as_str() {
                                "cursor" => Some("https://cursor.com/settings"),
                                "windsurf" | "factory" => Some("https://codeium.com/account"),
                                "zed" | "zed ai" => Some("https://zed.dev/account"),
                                "kiro" => Some("https://kiro.dev"),
                                "vertexai" | "vertex ai" => Some("https://console.cloud.google.com/vertex-ai"),
                                "augment" => Some("https://app.augmentcode.com"),
                                "minimax" => Some("https://www.minimax.chat"),
                                "opencode" => Some("https://opencode.ai"),
                                "kimi" | "kimik2" | "kimi k2" => Some("https://kimi.moonshot.cn"),
                                _ => None,
                            };

                            // Check if this is a local-only provider (login managed in their app)
                            let is_local_app_login = matches!(p.name.as_str(), "antigravity");

                            if supports_cli_login {
                                // Show login status if logging in
                                if is_logging_in && login_provider.as_ref() == Some(&p.name) {
                                    ui.add_space(4.0);
                                    egui::Frame::none()
                                        .fill(Theme::BG_SECONDARY)
                                        .rounding(Rounding::same(8.0))
                                        .inner_margin(12.0)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.add_space(8.0);
                                                let phase_icon = match login_phase {
                                                    LoginPhase::Idle => "⚪",
                                                    LoginPhase::Requesting => "🔄",
                                                    LoginPhase::WaitingBrowser => "🌐",
                                                    LoginPhase::Complete => "✅",
                                                };
                                                ui.label(
                                                    RichText::new(format!("{} {}", phase_icon, login_message.as_deref().unwrap_or("")))
                                                        .size(12.0)
                                                        .color(Theme::TEXT_PRIMARY),
                                                );
                                            });

                                            // Cancel button
                                            if ui.add(egui::Button::new(
                                                RichText::new("Cancel")
                                                    .size(11.0)
                                            ).fill(Theme::CARD_BG)).clicked() {
                                                if let Ok(mut s) = self.state.lock() {
                                                    s.login_provider = None;
                                                    s.login_phase = LoginPhase::Idle;
                                                    s.login_message = None;
                                                    s.login_auth_url = None;
                                                }
                                            }
                                        });
                                    ui.add_space(4.0);
                                } else if menu_button(ui, "🔑", "Login...") {
                                    // Start in-app CLI login
                                    self.start_login(&p.name);
                                }
                            } else if let Some(url) = web_login_url {
                                // Web-based login for providers without CLI
                                if menu_button(ui, "🔑", "Login (Web)...") {
                                    let _ = open::that(url);
                                }
                            } else if is_local_app_login {
                                // Show info for local-app-managed login
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("ℹ")
                                            .size(12.0)
                                            .color(Theme::ACCENT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new("Login is managed in the Antigravity app")
                                            .size(11.0)
                                            .color(Theme::TEXT_MUTED),
                                    );
                                });
                                ui.add_space(4.0);
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

                        // Keyboard shortcut hint
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Tip: Press Ctrl+Shift+U to open")
                                    .size(10.0)
                                    .color(Theme::TEXT_MUTED),
                            );
                        });

                        ui.add_space(4.0);

                        if menu_button(ui, "✕", "Quit") {
                            std::process::exit(0);
                        }
                    });
                }); // End ScrollArea
            });

        // Show preferences window if open
        self.preferences_window.show(ctx);

        // Sync settings if preferences changed
        if self.preferences_window.settings_changed {
            self.settings = self.preferences_window.settings.clone();
        }
    }
}

/// Draw a usage bar with label and percentage - glowing neon style
fn draw_usage_bar(ui: &mut egui::Ui, label: &str, percent: f64, reset: Option<&str>) {
    let color = Theme::usage_color(percent);
    let glow_color = Theme::usage_glow_color(percent);

    // Label row with percentage badge
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.0).color(Theme::TEXT_PRIMARY).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Percentage badge
            let badge_color = if percent >= 90.0 {
                Theme::RED
            } else if percent >= 75.0 {
                Theme::ORANGE
            } else {
                Theme::TEXT_DIM
            };
            ui.label(RichText::new(format!("{}%", percent as i32)).size(11.0).color(badge_color));
        });
    });

    ui.add_space(6.0);

    // Glowing progress bar
    let bar_height = 6.0;
    let glow_height = 10.0;
    let available_width = ui.available_width();

    // Allocate space for glow + bar
    let (rect, _) = ui.allocate_exact_size(Vec2::new(available_width, glow_height), egui::Sense::hover());
    let bar_rect = Rect::from_min_size(
        egui::pos2(rect.min.x, rect.min.y + 2.0),
        Vec2::new(available_width, bar_height),
    );

    // Track with subtle inner glow
    ui.painter().rect_filled(bar_rect, Rounding::same(3.0), Theme::PROGRESS_TRACK);

    let fill_w = bar_rect.width() * (percent as f32 / 100.0);
    if fill_w > 0.0 {
        let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_height));

        // Glow effect behind the bar
        let glow_rect = Rect::from_min_size(
            egui::pos2(fill_rect.min.x, rect.min.y),
            Vec2::new(fill_w, glow_height),
        );
        ui.painter().rect_filled(glow_rect, Rounding::same(5.0), glow_color);

        // Main fill
        ui.painter().rect_filled(fill_rect, Rounding::same(3.0), color);
    }

    ui.add_space(6.0);

    // Stats row - cleaner layout
    ui.horizontal(|ui| {
        if let Some(r) = reset {
            ui.label(RichText::new(format!("⟳ {}", r)).size(10.0).color(Theme::TEXT_MUTED));
        }
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

/// Draw a menu button with hover glow, returns true if clicked
fn menu_button(ui: &mut egui::Ui, icon: &str, label: &str) -> bool {
    let available_width = ui.available_width();

    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(available_width, 36.0),
        egui::Sense::click(),
    );

    let is_hovered = response.hovered();
    let bg_color = if is_hovered {
        Theme::CARD_BG_HOVER
    } else {
        Color32::TRANSPARENT
    };

    // Background with hover effect
    ui.painter().rect_filled(rect, Rounding::same(8.0), bg_color);

    // Accent line on hover
    if is_hovered {
        let accent_rect = Rect::from_min_size(
            rect.min,
            Vec2::new(3.0, rect.height()),
        );
        ui.painter().rect_filled(accent_rect, Rounding::same(2.0), Theme::ACCENT_PRIMARY);
    }

    // Icon and text
    let text_color = if is_hovered {
        Theme::TEXT_PRIMARY
    } else {
        Theme::TEXT_MUTED
    };

    let icon_color = if is_hovered {
        Theme::ACCENT_PRIMARY
    } else {
        Theme::TEXT_DIM
    };
    let _ = icon_color; // Suppress warning - reserved for future use

    let text_pos = egui::pos2(rect.min.x + 16.0, rect.center().y);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        format!("{}  {}", icon, label),
        egui::FontId::proportional(13.0),
        text_color,
    );

    response.clicked()
}

/// Run the application
pub fn run() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 720.0])
            .with_min_inner_size([420.0, 600.0])
            .with_max_inner_size([420.0, 900.0])
            .with_resizable(true)
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
