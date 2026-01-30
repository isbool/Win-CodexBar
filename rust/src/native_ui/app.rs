//! Main egui application - Modern refined menubar popup
//! Clean, spacious design with rich visual hierarchy

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Rect, RichText, Rounding, Stroke, Vec2};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::charts::{ChartPoint, CostHistoryChart};
use super::preferences::PreferencesWindow;
use super::provider_icons::ProviderIconCache;
use super::theme::{provider_color, provider_icon, FontSize, Radius, Spacing, Theme};
use crate::core::{FetchContext, Provider, ProviderId, ProviderFetchResult, RateWindow};
use crate::cost_scanner::get_daily_cost_history;
use crate::login::{LoginOutcome, LoginPhase};
use crate::providers::*;
use crate::settings::{ManualCookies, Settings};
use crate::shortcuts::ShortcutManager;
use crate::status::{fetch_provider_status, get_status_page_url, StatusLevel};
use crate::tray::{IconOverlay, LoadingPattern, ProviderUsage, SurpriseAnimation, TrayManager, TrayMenuAction};
use crate::updater::{self, UpdateInfo};

#[derive(Clone, Debug)]
pub struct ProviderData {
    pub name: String,
    pub display_name: String,
    pub account: Option<String>,  // Account email for display
    pub session_percent: Option<f64>,
    pub session_reset: Option<String>,
    pub weekly_percent: Option<f64>,
    pub weekly_reset: Option<String>,
    pub model_percent: Option<f64>,
    pub model_name: Option<String>,
    pub plan: Option<String>,
    pub error: Option<String>,
    pub dashboard_url: Option<String>,
    pub pace_percent: Option<f64>,
    pub pace_lasts_to_reset: bool,
    pub cost_used: Option<String>,
    pub cost_limit: Option<String>,
    pub cost_period: Option<String>,
    pub credits_remaining: Option<f64>,
    pub credits_percent: Option<f64>,
    pub status_level: StatusLevel,
    pub status_description: Option<String>,
    pub cost_history: Vec<(String, f64)>,
}

impl ProviderData {
    fn from_result(id: ProviderId, result: &ProviderFetchResult, metadata: &crate::core::ProviderMetadata) -> Self {
        let snapshot = &result.usage;
        let (pace_percent, pace_lasts) = calculate_pace(&snapshot.primary);

        let (cost_used, cost_limit, cost_period, credits_remaining, credits_percent) = if let Some(ref cost) = result.cost {
            if cost.period == "Credits" {
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
            account: snapshot.account_email.clone(),  // Account email if available
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
            status_level: StatusLevel::Unknown,
            status_description: None,
            cost_history: Vec::new(),
        }
    }

    fn from_error(id: ProviderId, error: String) -> Self {
        Self {
            name: id.cli_name().to_string(),
            display_name: id.display_name().to_string(),
            account: None,
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

    let expected_percent = (elapsed_minutes / total_minutes) * 100.0;
    let actual_percent = rate_window.used_percent;
    let pace = actual_percent - expected_percent;
    let lasts_to_reset = actual_percent <= expected_percent;

    (Some(pace), lasts_to_reset)
}

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
    surprise_animation: Option<SurpriseAnimation>,
    surprise_frame: u32,
    next_surprise_time: Instant,
    update_available: Option<UpdateInfo>,
    update_checked: bool,
    update_dismissed: bool,
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
    icon_cache: ProviderIconCache,
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

        let placeholders: Vec<ProviderData> = enabled_ids
            .iter()
            .map(|&id| ProviderData {
                name: id.cli_name().to_string(),
                display_name: id.display_name().to_string(),
                account: None,
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
            last_refresh: Instant::now() - Duration::from_secs(999),
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
            icon_cache: ProviderIconCache::new(),
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
                let handles: Vec<_> = enabled_ids
                    .iter()
                    .enumerate()
                    .map(|(idx, &id)| {
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

                            let mut result = match usage_result {
                                Ok(Ok(result)) => ProviderData::from_result(id, &result, &metadata),
                                Ok(Err(e)) => ProviderData::from_error(id, e.to_string()),
                                Err(_) => ProviderData::from_error(id, "Timeout".to_string()),
                            };

                            if let Ok(Some(status)) = status_result {
                                result.status_level = status.level;
                                result.status_description = Some(status.description);
                            }

                            let provider_name_lower = provider_name.to_lowercase();
                            if provider_name_lower == "codex" || provider_name_lower == "claude" {
                                result.cost_history = get_daily_cost_history(&provider_name_lower, 30);
                            }

                            if let Ok(mut s) = state.lock() {
                                if idx < s.providers.len() {
                                    s.providers[idx] = result;
                                }
                            }
                        })
                    })
                    .collect();

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

    /// Get an animated value that smoothly transitions to the target over 300ms.
    ///
    /// This helper provides consistent animation behavior for progress bar fills
    /// and other numeric value transitions.
    ///
    /// # Arguments
    /// * `ctx` - The egui context for animation state
    /// * `id` - A unique identifier for tracking this animation
    /// * `target` - The target value to animate towards
    ///
    /// # Returns
    /// The current animated value, which will smoothly approach the target
    #[allow(dead_code)]
    fn get_animated_value(ctx: &egui::Context, id: egui::Id, target: f32) -> f32 {
        ctx.animate_value_with_time(id, target, 0.3)
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
        ProviderId::JetBrains => Box::new(JetBrainsProvider::new()),
    }
}

impl eframe::App for CodexBarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check keyboard shortcuts
        if let Some(ref shortcut_mgr) = self.shortcut_manager {
            while shortcut_mgr.check_events() {
                tracing::info!("Keyboard shortcut triggered - focusing window");
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        // Auto-refresh check
        let should_refresh = {
            if self.settings.refresh_interval_secs == 0 {
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
                if state.is_refreshing {
                    state.loading_phase += 0.05;
                    if state.loading_phase > 1.0 {
                        state.loading_phase -= 1.0;
                    }
                }

                let surprise = if self.settings.surprise_animations && !state.is_refreshing {
                    if let Some(anim) = state.surprise_animation {
                        state.surprise_frame += 1;
                        if state.surprise_frame >= anim.duration_frames() {
                            state.surprise_animation = None;
                            state.surprise_frame = 0;
                            state.next_surprise_time = Instant::now() + random_surprise_delay();
                            None
                        } else {
                            Some((anim, state.surprise_frame))
                        }
                    } else if Instant::now() >= state.next_surprise_time {
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

                let update = if state.update_dismissed {
                    None
                } else {
                    state.update_available.clone()
                };

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

        ctx.request_repaint_after(if is_refreshing || surprise_state.is_some() || is_logging_in {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(200)
        });

        // Update tray icon
        if let Some(ref tray) = self.tray_manager {
            if is_refreshing {
                tray.show_loading(loading_pattern, loading_phase);
            } else if let Some((anim, frame)) = surprise_state {
                if let Some(provider) = providers.get(self.selected_provider) {
                    let session = provider.session_percent.unwrap_or(0.0);
                    let weekly = provider.weekly_percent.unwrap_or(session);
                    tray.show_surprise(anim, frame, session, weekly);
                }
            } else if self.settings.merge_tray_icons {
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
                if let Some(provider) = providers.get(self.selected_provider) {
                    let session = provider.session_percent.unwrap_or(0.0);
                    let weekly = provider.weekly_percent.unwrap_or(session);
                    let weekly_exhausted = weekly >= 99.0;
                    let has_credits = provider.credits_percent.is_some() && provider.credits_percent.unwrap_or(0.0) > 0.0;

                    if weekly_exhausted && has_credits {
                        tray.update_credits_mode(provider.credits_percent.unwrap_or(0.0), &provider.display_name);
                    } else {
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

            if let Some(action) = TrayManager::check_events() {
                match action {
                    TrayMenuAction::Quit => std::process::exit(0),
                    TrayMenuAction::Open => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                }
            }
        }

        // Apply refined style
        let mut style = (*ctx.style()).clone();
        style.visuals.window_fill = Theme::BG_PRIMARY;
        style.visuals.panel_fill = Theme::BG_PRIMARY;
        style.visuals.widgets.noninteractive.bg_fill = Theme::BG_SECONDARY;
        style.visuals.widgets.inactive.bg_fill = Theme::CARD_BG;
        style.visuals.widgets.hovered.bg_fill = Theme::CARD_BG_HOVER;
        style.visuals.widgets.active.bg_fill = Theme::ACCENT_PRIMARY;
        style.visuals.selection.bg_fill = Theme::selection_overlay();
        style.visuals.selection.stroke = Stroke::new(1.0, Theme::ACCENT_PRIMARY);
        ctx.set_style(style);

        // Handle keyboard shortcuts
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Comma) {
                self.preferences_window.open();
            }
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Theme::BG_PRIMARY).inner_margin(Spacing::SM))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // ════════════════════════════════════════════════════════════
                    // UPDATE BANNER
                    // ════════════════════════════════════════════════════════════
                    if let Some(ref update) = update_info {
                        egui::Frame::none()
                            .fill(Theme::ACCENT_PRIMARY)
                            .rounding(Rounding::same(Radius::LG))
                            .inner_margin(Spacing::MD)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("🎉").size(FontSize::MD));
                                    ui.add_space(Spacing::XS);
                                    ui.label(
                                        RichText::new(format!("Update available: {}", update.version))
                                            .size(FontSize::BASE)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    );

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.add(
                                            egui::Button::new(RichText::new("✕").size(FontSize::SM).color(Color32::WHITE))
                                                .fill(Color32::TRANSPARENT)
                                                .stroke(Stroke::NONE)
                                        ).clicked() {
                                            if let Ok(mut s) = self.state.lock() {
                                                s.update_dismissed = true;
                                            }
                                        }

                                        let download_url = update.download_url.clone();
                                        if ui.add(
                                            egui::Button::new(RichText::new("Download").size(FontSize::SM).color(Theme::ACCENT_PRIMARY))
                                                .fill(Color32::WHITE)
                                                .rounding(Rounding::same(Radius::SM))
                                        ).clicked() {
                                            let _ = open::that(&download_url);
                                        }
                                    });
                                });
                            });
                        ui.add_space(Spacing::MD);
                    }

                    // ════════════════════════════════════════════════════════════
                    // PROVIDER SWITCHER - Grid layout (macOS style)
                    // ════════════════════════════════════════════════════════════
                    const MAX_PROVIDERS_PER_ROW: usize = 4;

                    egui::Frame::none()
                        .fill(Theme::BG_SECONDARY)
                        .rounding(Rounding::same(Radius::LG))
                        .inner_margin(Spacing::SM)
                        .show(ui, |ui| {
                            let spacing = Spacing::XS;
                            ui.spacing_mut().item_spacing = Vec2::new(spacing, spacing);

                            // Calculate button width to fill available space evenly
                            let available_width = ui.available_width();
                            let btn_width = (available_width - spacing * (MAX_PROVIDERS_PER_ROW as f32 - 1.0)) / MAX_PROVIDERS_PER_ROW as f32;
                            let btn_height = 58.0;

                            // Split providers into rows of MAX_PROVIDERS_PER_ROW
                            for row_providers in providers.chunks(MAX_PROVIDERS_PER_ROW) {
                                ui.horizontal(|ui| {
                                    for provider in row_providers.iter() {
                                        let idx = providers.iter().position(|p| p.name == provider.name).unwrap_or(0);
                                        let is_selected = idx == self.selected_provider;
                                        let fallback_icon = provider_icon(&provider.name);
                                        let brand_color = provider_color(&provider.name);

                                        // Provider button
                                        let (rect, response) = ui.allocate_exact_size(
                                            Vec2::new(btn_width, btn_height),
                                            egui::Sense::click(),
                                        );

                                        let is_hovered = response.hovered();

                                        // Background - pill shape for selected
                                        let bg_color = if is_selected {
                                            Theme::ACCENT_PRIMARY
                                        } else if is_hovered {
                                            Theme::hover_overlay()
                                        } else {
                                            Color32::TRANSPARENT
                                        };

                                        ui.painter().rect_filled(rect, Rounding::same(Radius::MD), bg_color);

                                        // Icon - try SVG first, fallback to text symbol
                                        let icon_size = 20u32;
                                        let icon_pos = egui::pos2(rect.center().x, rect.min.y + 18.0);

                                        if let Some(texture) = self.icon_cache.get_icon(ui.ctx(), &provider.name, icon_size) {
                                            // Render SVG icon
                                            let icon_rect = Rect::from_center_size(
                                                icon_pos,
                                                Vec2::splat(icon_size as f32),
                                            );

                                            // Apply tint based on selection state
                                            let tint = if is_selected {
                                                Color32::WHITE
                                            } else if is_hovered {
                                                brand_color
                                            } else {
                                                Color32::from_gray(180)
                                            };

                                            ui.painter().image(
                                                texture.id(),
                                                icon_rect,
                                                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                                tint,
                                            );
                                        } else {
                                            // Fallback to text symbol
                                            let icon_color = if is_selected {
                                                Color32::WHITE
                                            } else if is_hovered {
                                                brand_color
                                            } else {
                                                Theme::TEXT_SECONDARY
                                            };

                                            ui.painter().text(
                                                icon_pos,
                                                egui::Align2::CENTER_CENTER,
                                                fallback_icon,
                                                egui::FontId::proportional(18.0),
                                                icon_color,
                                            );
                                        }

                                        // Label - truncate long names
                                        let label = if provider.display_name.len() > 8 {
                                            format!("{}...", &provider.display_name[..6])
                                        } else {
                                            provider.display_name.clone()
                                        };

                                        let text_color = if is_selected {
                                            Color32::WHITE
                                        } else {
                                            Theme::TEXT_MUTED
                                        };

                                        let label_pos = egui::pos2(rect.center().x, rect.max.y - 12.0);
                                        ui.painter().text(
                                            label_pos,
                                            egui::Align2::CENTER_CENTER,
                                            &label,
                                            egui::FontId::proportional(FontSize::XS),
                                            text_color,
                                        );

                                        if response.clicked() {
                                            self.selected_provider = idx;
                                        }
                                    }
                                });
                            }
                        });

                    ui.add_space(Spacing::SM);

                    // ════════════════════════════════════════════════════════════
                    // PROVIDER DETAIL CARD (macOS style)
                    // ════════════════════════════════════════════════════════════
                    if let Some(provider) = providers.get(self.selected_provider) {
                        egui::Frame::none()
                            .fill(Theme::CARD_BG)
                            .rounding(Rounding::same(Radius::MD))
                            .inner_margin(Spacing::MD)
                            .stroke(Stroke::new(0.5, Theme::CARD_BORDER))
                            .show(ui, |ui| {
                                // Header with name and badge
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        // Provider name - medium size
                                        ui.label(
                                            RichText::new(&provider.display_name)
                                                .size(FontSize::LG)
                                                .color(Theme::TEXT_PRIMARY)
                                                .strong(),
                                        );
                                        // Updated timestamp
                                        ui.label(
                                            RichText::new("Updated just now")
                                                .size(FontSize::XS)
                                                .color(Theme::TEXT_MUTED),
                                        );
                                    });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                        // Plan badge (compact)
                                        if let Some(plan) = &provider.plan {
                                            ui.label(
                                                RichText::new(plan)
                                                    .size(FontSize::SM)
                                                    .color(Theme::TEXT_SECONDARY),
                                            );
                                        }
                                    });
                                });

                                ui.add_space(Spacing::LG);

                                // Usage sections
                                if provider.error.is_some() {
                                    ui.label(
                                        RichText::new("Unable to fetch usage data")
                                            .size(FontSize::SM)
                                            .color(Theme::TEXT_MUTED),
                                    );
                                } else {
                                    // Session Section
                                    if let Some(pct) = provider.session_percent {
                                        draw_section_header(ui, "Session");
                                        ui.add_space(Spacing::XS);
                                        let bar_id = Some(ui.id().with(&provider.name).with("session"));
                                        draw_usage_bar(ui, pct, provider.session_reset.as_deref(), Theme::BLUE, bar_id);
                                        ui.add_space(Spacing::MD);
                                    }

                                    // Weekly Section
                                    if let Some(pct) = provider.weekly_percent {
                                        draw_section_header(ui, "Weekly");
                                        ui.add_space(Spacing::XS);
                                        let bar_id = Some(ui.id().with(&provider.name).with("weekly"));
                                        draw_usage_bar(ui, pct, provider.weekly_reset.as_deref(), Theme::BLUE, bar_id);
                                        ui.add_space(Spacing::MD);
                                    }

                                    // Model/Code Review Section with Pace Indicator
                                    if let Some(pct) = provider.model_percent {
                                        let name = provider.model_name.as_deref().unwrap_or("Code review");
                                        draw_section_header(ui, name);
                                        ui.add_space(Spacing::XS);

                                        // Pace indicator (dot) before bar
                                        ui.horizontal(|ui| {
                                            if let Some(pace) = provider.pace_percent {
                                                let dot_color = if pace <= -5.0 {
                                                    Theme::GREEN
                                                } else if pace >= 5.0 {
                                                    Theme::ORANGE
                                                } else {
                                                    Theme::TEXT_MUTED
                                                };

                                                let (rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
                                                ui.painter().circle_filled(rect.center(), 3.0, dot_color);
                                            }

                                            let bar_id = Some(ui.id().with(&provider.name).with("model"));
                                            draw_usage_bar_content(ui, pct, Theme::BLUE, bar_id);
                                        });

                                        // Pace text
                                        if let Some(pace) = provider.pace_percent {
                                            ui.add_space(2.0);
                                            draw_pace_indicator(ui, pace, provider.pace_lasts_to_reset);
                                        }
                                        ui.add_space(Spacing::MD);
                                    }

                                    // Credits Section
                                    if let Some(credits_pct) = provider.credits_percent {
                                        draw_section_header(ui, "Credits");
                                        ui.add_space(Spacing::XS);
                                        draw_credits_section(ui, credits_pct, provider.credits_remaining, None);
                                        ui.add_space(Spacing::SM);

                                        // Buy Credits button
                                        if ui.add(
                                            egui::Button::new(RichText::new("⊕ Buy Credits...").size(FontSize::SM).color(Theme::TEXT_PRIMARY))
                                                .fill(Color32::TRANSPARENT)
                                                .frame(false)
                                        ).clicked() {
                                            if let Some(url) = &provider.dashboard_url {
                                                let _ = open::that(url);
                                            }
                                        }
                                        ui.add_space(Spacing::MD);
                                    }

                                    // Cost Section
                                    if let Some(ref cost_used) = provider.cost_used {
                                        draw_section_header(ui, "Cost");
                                        ui.add_space(Spacing::XS);
                                        draw_cost_display(
                                            ui,
                                            Some(cost_used.as_str()),
                                            None, // today tokens
                                            provider.cost_limit.as_deref(),  // Use limit as monthly cost
                                            None, // monthly tokens
                                        );
                                    }

                                    // Chart toggle
                                    if !provider.cost_history.is_empty() || provider.cost_used.is_some() {
                                        ui.add_space(Spacing::MD);
                                        let chart_label = if self.show_chart { "▼ Hide Chart" } else { "▶ Show Chart" };
                                        if ui.add(
                                            egui::Button::new(RichText::new(chart_label).size(FontSize::XS).color(Theme::TEXT_SECONDARY))
                                                .fill(Color32::TRANSPARENT)
                                        ).clicked() {
                                            self.show_chart = !self.show_chart;
                                        }

                                        if self.show_chart && !provider.cost_history.is_empty() {
                                            ui.add_space(Spacing::SM);
                                            let points: Vec<ChartPoint> = provider.cost_history.iter()
                                                .map(|(date, cost)| ChartPoint {
                                                    date: date.clone(),
                                                    value: *cost,
                                                    tokens: None,
                                                    model_breakdowns: None,
                                                })
                                                .collect();
                                            let mut chart = CostHistoryChart::new(points, Theme::ACCENT_PRIMARY);
                                            chart.show(ui);
                                        }
                                    }
                                }
                            });
                    } else if providers.is_empty() {
                        // Loading state
                        egui::Frame::none()
                            .fill(Theme::CARD_BG)
                            .rounding(Rounding::same(Radius::LG))
                            .inner_margin(Spacing::XXL)
                            .stroke(Stroke::new(1.0, Theme::CARD_BORDER))
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.spinner();
                                    ui.add_space(Spacing::SM);
                                    ui.label(
                                        RichText::new("Loading providers...")
                                            .size(FontSize::BASE)
                                            .color(Theme::TEXT_MUTED),
                                    );
                                });
                            });
                    }

                    ui.add_space(Spacing::SM);

                    // ════════════════════════════════════════════════════════════
                    // MENU ITEMS
                    // ════════════════════════════════════════════════════════════
                    egui::Frame::none()
                        .fill(Theme::CARD_BG)
                        .rounding(Rounding::same(Radius::MD))
                        .inner_margin(Spacing::XS)
                        .stroke(Stroke::new(1.0, Theme::CARD_BORDER))
                        .show(ui, |ui| {
                            if draw_menu_item(ui, "↻", "Refresh") {
                                self.refresh_providers();
                            }

                            if draw_menu_item(ui, "📊", "Usage Dashboard") {
                                if let Some(p) = providers.get(self.selected_provider) {
                                    if let Some(url) = &p.dashboard_url {
                                        let _ = open::that(url);
                                    }
                                }
                            }

                            if draw_menu_item(ui, "📈", "Status Page") {
                                if let Some(p) = providers.get(self.selected_provider) {
                                    if let Some(url) = get_status_page_url(&p.name) {
                                        let _ = open::that(url);
                                    }
                                }
                            }

                            // Login handling
                            if let Some(p) = providers.get(self.selected_provider) {
                                let supports_cli_login = matches!(p.name.as_str(), "claude" | "codex" | "gemini" | "copilot");
                                let needs_api_key = matches!(p.name.as_str(), "zai" | "z.ai" | "amp" | "synthetic" | "copilot" | "jetbrains");

                                let web_login_url = match p.name.as_str() {
                                    "cursor" => Some("https://cursor.com/settings"),
                                    "droid" | "factory" => Some("https://app.factory.ai"),
                                    "kiro" => Some("https://kiro.dev"),
                                    "vertexai" | "vertex ai" => Some("https://console.cloud.google.com/vertex-ai"),
                                    "augment" => Some("https://app.augmentcode.com"),
                                    "minimax" => Some("https://www.minimax.chat"),
                                    "opencode" => Some("https://opencode.ai"),
                                    "kimi" | "kimik2" | "kimi k2" => Some("https://kimi.moonshot.cn"),
                                    _ => None,
                                };

                                let is_local_app_login = matches!(p.name.as_str(), "antigravity");

                                if supports_cli_login {
                                    if is_logging_in && login_provider.as_ref() == Some(&p.name) {
                                        ui.add_space(Spacing::XS);
                                        egui::Frame::none()
                                            .fill(Theme::BG_SECONDARY)
                                            .rounding(Rounding::same(Radius::MD))
                                            .inner_margin(Spacing::MD)
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.spinner();
                                                    ui.add_space(Spacing::XS);
                                                    let phase_icon = match login_phase {
                                                        LoginPhase::Idle => "⚪",
                                                        LoginPhase::Requesting => "🔄",
                                                        LoginPhase::WaitingBrowser => "🌐",
                                                        LoginPhase::Complete => "✅",
                                                    };
                                                    ui.label(
                                                        RichText::new(format!("{} {}", phase_icon, login_message.as_deref().unwrap_or("")))
                                                            .size(FontSize::BASE)
                                                            .color(Theme::TEXT_PRIMARY),
                                                    );
                                                });

                                                if ui.add(
                                                    egui::Button::new(RichText::new("Cancel").size(FontSize::SM))
                                                        .fill(Theme::CARD_BG)
                                                ).clicked() {
                                                    if let Ok(mut s) = self.state.lock() {
                                                        s.login_provider = None;
                                                        s.login_phase = LoginPhase::Idle;
                                                        s.login_message = None;
                                                        s.login_auth_url = None;
                                                    }
                                                }
                                            });
                                        ui.add_space(Spacing::XS);
                                    } else if draw_menu_item(ui, "🔑", "Login...") {
                                        self.start_login(&p.name);
                                    }
                                } else if needs_api_key {
                                    if draw_menu_item(ui, "🔑", "Configure API Key...") {
                                        self.preferences_window.active_tab = super::preferences::PreferencesTab::ApiKeys;
                                        self.preferences_window.open();
                                    }
                                } else if let Some(url) = web_login_url {
                                    if draw_menu_item(ui, "🔑", "Login (Web)...") {
                                        let _ = open::that(url);
                                    }
                                } else if is_local_app_login {
                                    ui.add_space(Spacing::XXS);
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("ℹ").size(FontSize::SM).color(Theme::ACCENT_PRIMARY));
                                        ui.label(
                                            RichText::new("Login is managed in the Antigravity app")
                                                .size(FontSize::SM)
                                                .color(Theme::TEXT_MUTED),
                                        );
                                    });
                                    ui.add_space(Spacing::XXS);
                                }
                            }

                            if draw_menu_item(ui, "⚙", "Settings...") {
                                self.preferences_window.open();
                            }

                            if draw_menu_item(ui, "ℹ", "About CodexBar") {
                                self.preferences_window.active_tab = super::preferences::PreferencesTab::About;
                                self.preferences_window.open();
                            }

                            // Separator
                            ui.add_space(Spacing::XS);
                            let sep_rect = ui.available_rect_before_wrap();
                            ui.painter().hline(
                                sep_rect.x_range(),
                                sep_rect.top(),
                                Stroke::new(1.0, Theme::SEPARATOR),
                            );
                            ui.add_space(Spacing::XS);

                            // Keyboard hint
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Tip: Press Ctrl+Shift+U to open")
                                        .size(FontSize::XS)
                                        .color(Theme::TEXT_DIM),
                                );
                            });

                            ui.add_space(Spacing::XS);

                            if draw_menu_item(ui, "✕", "Quit") {
                                std::process::exit(0);
                            }
                        });
                });
            });

        // Show preferences window
        self.preferences_window.show(ctx);

        // Sync settings
        if self.preferences_window.settings_changed {
            self.settings = self.preferences_window.settings.clone();
        }
    }
}

/// Draw section header with chevron - macOS style
fn draw_section_header(ui: &mut egui::Ui, title: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(FontSize::MD).color(Theme::TEXT_PRIMARY).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("›").size(FontSize::LG).color(Theme::TEXT_MUTED));
        });
    });
}

/// Draw a usage bar with label, percentage, and capsule progress - macOS style
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `percent` - The target percentage value (0-100)
/// * `reset` - Optional reset time string to display
/// * `bar_color` - The fill color for the progress bar
/// * `bar_id` - A unique identifier for animation state tracking
fn draw_usage_bar(ui: &mut egui::Ui, percent: f64, reset: Option<&str>, bar_color: Color32, bar_id: Option<egui::Id>) {
    // Label row with reset time on right
    ui.horizontal(|ui| {
        let pct_str = format!("{}% used", percent as i32);
        ui.label(RichText::new(pct_str).size(FontSize::SM).color(Theme::TEXT_SECONDARY));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(r) = reset {
                ui.label(RichText::new(format!("Resets in {}", r)).size(FontSize::SM).color(Theme::TEXT_SECONDARY));
            }
        });
    });

    ui.add_space(4.0);

    draw_usage_bar_content(ui, percent, bar_color, bar_id);
}

/// Draw a pulsing glow effect behind high-usage bars (>= 80%)
fn draw_usage_bar_glow(ui: &mut egui::Ui, rect: Rect, percent: f64) {
    if percent < 80.0 {
        return;
    }

    // Calculate pulse intensity based on usage level:
    // 80-89%: subtle (0.3), 90-94%: medium (0.6), 95%+: strong (1.0)
    let base_intensity = if percent >= 95.0 {
        1.0
    } else if percent >= 90.0 {
        0.6
    } else {
        0.3
    };

    // Sine-wave pulse animation
    let time = ui.input(|i| i.time) as f32;
    let pulse_phase = (time * std::f32::consts::PI).sin() * 0.5 + 0.5;

    // Combine base intensity with pulse (range 0.5 to 1.0 of base)
    let intensity = base_intensity * (0.5 + 0.5 * pulse_phase);

    // Get glow color from theme
    let glow_color = Theme::usage_glow_color(percent);

    // Apply intensity to alpha
    let alpha = (glow_color.a() as f32 * intensity) as u8;
    let final_glow = egui::Color32::from_rgba_unmultiplied(
        glow_color.r(),
        glow_color.g(),
        glow_color.b(),
        alpha,
    );

    // Draw expanded glow rectangle behind the bar
    let glow_expand = 3.0;
    let glow_rect = rect.expand(glow_expand);
    ui.painter().rect_filled(glow_rect, Rounding::same(Radius::PILL + glow_expand), final_glow);
}

/// Draw just the progress bar part with optional animation
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `percent` - The target percentage value (0-100)
/// * `color` - The fill color for the progress bar
/// * `bar_id` - A unique identifier for animation state tracking. When provided,
///              the bar will smoothly animate to the target value over 300ms.
fn draw_usage_bar_content(ui: &mut egui::Ui, percent: f64, color: Color32, bar_id: Option<egui::Id>) {
    // Progress bar - macOS style
    let bar_height = 6.0;
    let available_width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(available_width, bar_height), egui::Sense::hover());

    // Calculate animated or static fill percentage
    let animated_percent = if let Some(id) = bar_id {
        // Animate to target value over 300ms
        ui.ctx().animate_value_with_time(id, percent as f32, 0.3) as f64
    } else {
        percent
    };

    // Draw pulsing glow for high-usage bars (>= 80%) - use animated value for smooth glow transition
    draw_usage_bar_glow(ui, rect, animated_percent);

    // Track
    ui.painter().rect_filled(rect, Rounding::same(Radius::PILL), Theme::PROGRESS_TRACK);

    // Fill - use animated percentage for smooth transitions
    let fill_w = rect.width() * (animated_percent as f32 / 100.0);
    if fill_w > 0.0 {
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, bar_height));
        ui.painter().rect_filled(fill_rect, Rounding::same(Radius::PILL), color);
    }

    // Request continuous repaint when pulsing bars are visible or animation is in progress
    if animated_percent >= 80.0 || (bar_id.is_some() && (animated_percent - percent).abs() > 0.1) {
        ui.ctx().request_repaint();
    }
}

/// Draw pace indicator - compact
fn draw_pace_indicator(ui: &mut egui::Ui, pace_percent: f64, lasts_to_reset: bool) {
    let (pace_label, pace_color) = if pace_percent <= -5.0 {
        ("Behind", Theme::GREEN)
    } else if pace_percent >= 5.0 {
        ("Ahead", Theme::ORANGE)
    } else {
        ("On pace", Theme::TEXT_MUTED)
    };

    let pace_text = if pace_percent.abs() >= 1.0 {
        format!("Pace: {} ({:+.0}%)", pace_label, pace_percent)
    } else {
        format!("Pace: {}", pace_label)
    };

    let lasts_text = if lasts_to_reset { " · Lasts to reset" } else { "" };

    ui.horizontal(|ui| {
        ui.label(RichText::new(pace_text).size(FontSize::XS).color(pace_color));
        if !lasts_text.is_empty() {
            ui.label(RichText::new(lasts_text).size(FontSize::XS).color(Theme::GREEN));
        }
    });
}

/// Draw credits section - macOS style compact
fn draw_credits_section(ui: &mut egui::Ui, percent_remaining: f64, credits_remaining: Option<f64>, tokens_remaining: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Credits").size(FontSize::MD).color(Theme::TEXT_PRIMARY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Show exact value if available
            if let Some(remaining) = credits_remaining {
                let tokens_str = tokens_remaining.unwrap_or("");
                if tokens_str.is_empty() {
                    ui.label(RichText::new(format!("{:.1} left", remaining))
                        .size(FontSize::SM)
                        .color(Theme::TEXT_SECONDARY));
                } else {
                    ui.label(RichText::new(format!("{:.1} left · {}", remaining, tokens_str))
                        .size(FontSize::SM)
                        .color(Theme::TEXT_SECONDARY));
                }
            } else {
                ui.label(RichText::new(format!("{}%", percent_remaining as i32))
                    .size(FontSize::SM)
                    .color(Theme::TEXT_SECONDARY));
            }
        });
    });

    ui.add_space(2.0);

    // Credits bar - thin macOS style
    let bar_height = 4.0;
    let available_width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(available_width, bar_height), egui::Sense::hover());

    ui.painter().rect_filled(rect, Rounding::same(Radius::PILL), Theme::PROGRESS_TRACK);

    let fill_w = rect.width() * (percent_remaining as f32 / 100.0);
    if fill_w > 0.0 {
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, bar_height));
        ui.painter().rect_filled(fill_rect, Rounding::same(Radius::PILL), Theme::CYAN);
    }
}

/// Draw cost display - macOS style compact
fn draw_cost_display(ui: &mut egui::Ui, today_cost: Option<&str>, today_tokens: Option<&str>, monthly_cost: Option<&str>, monthly_tokens: Option<&str>) {
    ui.vertical(|ui| {
        // Today row
        ui.horizontal(|ui| {
            ui.label(RichText::new("Today").size(FontSize::SM).color(Theme::TEXT_SECONDARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let cost = today_cost.unwrap_or("$0.00");
                let tokens = today_tokens.unwrap_or("");
                if tokens.is_empty() {
                    ui.label(RichText::new(cost).size(FontSize::SM).color(Theme::TEXT_PRIMARY));
                } else {
                    ui.label(RichText::new(format!("{} · {}", cost, tokens)).size(FontSize::SM).color(Theme::TEXT_PRIMARY));
                }
            });
        });

        ui.add_space(2.0);

        // Last 30 days row
        ui.horizontal(|ui| {
            ui.label(RichText::new("Last 30 days").size(FontSize::SM).color(Theme::TEXT_SECONDARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let cost = monthly_cost.unwrap_or("$0.00");
                let tokens = monthly_tokens.unwrap_or("");
                if tokens.is_empty() {
                    ui.label(RichText::new(cost).size(FontSize::SM).color(Theme::TEXT_PRIMARY));
                } else {
                    ui.label(RichText::new(format!("{} · {}", cost, tokens)).size(FontSize::SM).color(Theme::TEXT_PRIMARY));
                }
            });
        });
    });
}

/// Draw a menu item button - macOS style compact
fn draw_menu_item(ui: &mut egui::Ui, icon: &str, label: &str) -> bool {
    let available_width = ui.available_width();

    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(available_width, 32.0),  // Slightly larger height
        egui::Sense::click(),
    );

    let is_hovered = response.hovered();

    if is_hovered {
        ui.painter().rect_filled(rect, Rounding::same(Radius::SM), Theme::menu_hover());
    }

    let text_color = if is_hovered {
        Theme::TEXT_PRIMARY
    } else {
        Theme::TEXT_SECONDARY
    };

    // Icon
    let icon_pos = egui::pos2(rect.min.x + Spacing::SM, rect.center().y);
    ui.painter().text(
        icon_pos,
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(FontSize::MD),
        text_color,
    );

    // Label
    let label_pos = egui::pos2(rect.min.x + Spacing::SM + 22.0, rect.center().y);
    ui.painter().text(
        label_pos,
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(FontSize::SM),
        text_color,
    );

    response.clicked()
}

/// Run the application
pub fn run() -> anyhow::Result<()> {
    // Delete any corrupted window state
    if let Some(data_dir) = dirs::data_dir() {
        let state_file = data_dir.join("CodexBar").join("data").join("app.ron");
        if state_file.exists() {
            let _ = std::fs::remove_file(&state_file);
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 720.0])
            .with_min_inner_size([380.0, 500.0])
            .with_resizable(true)
            .with_decorations(true)
            .with_transparent(false)
            .with_always_on_top()
            .with_title("CodexBar"),
        persist_window: false,  // Don't persist window state
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
