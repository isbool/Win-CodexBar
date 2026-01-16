//! Preferences window for CodexBar
//!
//! A tabbed settings window similar to the macOS version

use eframe::egui::{self, Color32, RichText, Rounding, Stroke, Vec2};

use super::theme::Theme;
use crate::settings::{ManualCookies, Settings};
use crate::core::ProviderId;

/// Which preferences tab is active
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PreferencesTab {
    #[default]
    General,
    Providers,
    Cookies,
    Advanced,
    About,
}

impl PreferencesTab {
    fn label(&self) -> &'static str {
        match self {
            PreferencesTab::General => "General",
            PreferencesTab::Providers => "Providers",
            PreferencesTab::Cookies => "Cookies",
            PreferencesTab::Advanced => "Advanced",
            PreferencesTab::About => "About",
        }
    }
}

/// Preferences window state
pub struct PreferencesWindow {
    pub is_open: bool,
    pub active_tab: PreferencesTab,
    pub settings: Settings,
    pub settings_changed: bool,
    // Cookie management state
    cookies: ManualCookies,
    new_cookie_provider: String,
    new_cookie_value: String,
    cookie_status_msg: Option<(String, bool)>, // (message, is_error)
}

impl Default for PreferencesWindow {
    fn default() -> Self {
        Self {
            is_open: false,
            active_tab: PreferencesTab::General,
            settings: Settings::load(),
            settings_changed: false,
            cookies: ManualCookies::load(),
            new_cookie_provider: String::new(),
            new_cookie_value: String::new(),
            cookie_status_msg: None,
        }
    }
}

impl PreferencesWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.settings = Settings::load();
        self.cookies = ManualCookies::load();
        self.settings_changed = false;
        self.cookie_status_msg = None;
    }

    pub fn close(&mut self) {
        if self.settings_changed {
            let _ = self.settings.save();
        }
        self.is_open = false;
    }

    /// Show the preferences window
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }

        egui::Window::new("Preferences")
            .collapsible(false)
            .resizable(false)
            .default_size(Vec2::new(450.0, 400.0))
            .show(ctx, |ui| {
                // Tab bar
                ui.horizontal(|ui| {
                    for tab in [
                        PreferencesTab::General,
                        PreferencesTab::Providers,
                        PreferencesTab::Cookies,
                        PreferencesTab::Advanced,
                        PreferencesTab::About,
                    ] {
                        let is_selected = self.active_tab == tab;
                        let btn = egui::Button::new(
                            RichText::new(tab.label())
                                .size(13.0)
                                .color(if is_selected {
                                    Color32::WHITE
                                } else {
                                    Theme::TEXT_PRIMARY
                                }),
                        )
                        .fill(if is_selected {
                            Theme::TAB_ACTIVE
                        } else {
                            Theme::TAB_INACTIVE
                        })
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(80.0, 28.0));

                        if ui.add(btn).clicked() {
                            self.active_tab = tab;
                        }
                    }
                });

                ui.add_space(16.0);

                // Separator
                let sep_rect = ui.available_rect_before_wrap();
                ui.painter().hline(
                    sep_rect.x_range(),
                    sep_rect.top(),
                    Stroke::new(1.0, Theme::SEPARATOR),
                );
                ui.add_space(16.0);

                // Tab content
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.active_tab {
                        PreferencesTab::General => self.show_general_tab(ui),
                        PreferencesTab::Providers => self.show_providers_tab(ui),
                        PreferencesTab::Cookies => self.show_cookies_tab(ui),
                        PreferencesTab::Advanced => self.show_advanced_tab(ui),
                        PreferencesTab::About => self.show_about_tab(ui),
                    }
                });

                ui.add_space(16.0);

                // Close button
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.close();
                        }
                    });
                });
            });
    }

    fn show_general_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("General");
        ui.add_space(12.0);

        // Start at Login
        let mut start_at_login = self.settings.start_at_login;
        if ui.checkbox(&mut start_at_login, "Start at login").changed() {
            if let Err(e) = self.settings.set_start_at_login(start_at_login) {
                tracing::error!("Failed to set start at login: {}", e);
            } else {
                self.settings_changed = true;
            }
        }
        ui.label(
            RichText::new("Automatically start CodexBar when you log in")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.add_space(12.0);

        // Start minimized
        let mut start_minimized = self.settings.start_minimized;
        if ui.checkbox(&mut start_minimized, "Start minimized").changed() {
            self.settings.start_minimized = start_minimized;
            self.settings_changed = true;
        }
        ui.label(
            RichText::new("Start CodexBar in the system tray")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.add_space(12.0);

        // Show notifications
        let mut show_notifications = self.settings.show_notifications;
        if ui
            .checkbox(&mut show_notifications, "Show notifications")
            .changed()
        {
            self.settings.show_notifications = show_notifications;
            self.settings_changed = true;
        }
        ui.label(
            RichText::new("Get notified when usage thresholds are reached")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.add_space(12.0);

        // High usage threshold
        ui.horizontal(|ui| {
            ui.label("High usage warning:");
            let mut threshold = self.settings.high_usage_threshold as i32;
            if ui
                .add(egui::Slider::new(&mut threshold, 50..=95).suffix("%"))
                .changed()
            {
                self.settings.high_usage_threshold = threshold as f64;
                self.settings_changed = true;
            }
        });

        // Critical usage threshold
        ui.horizontal(|ui| {
            ui.label("Critical usage alert:");
            let mut threshold = self.settings.critical_usage_threshold as i32;
            if ui
                .add(egui::Slider::new(&mut threshold, 80..=100).suffix("%"))
                .changed()
            {
                self.settings.critical_usage_threshold = threshold as f64;
                self.settings_changed = true;
            }
        });
    }

    fn show_providers_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Providers");
        ui.add_space(12.0);

        ui.label(
            RichText::new("Enable or disable AI providers to track")
                .size(12.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.add_space(12.0);

        // List all providers with checkboxes
        for provider_id in ProviderId::all() {
            let provider_name = provider_id.cli_name();
            let display_name = provider_id.display_name();
            let is_enabled = self.settings.enabled_providers.contains(provider_name);

            let mut enabled = is_enabled;
            if ui.checkbox(&mut enabled, display_name).changed() {
                if enabled {
                    self.settings.enabled_providers.insert(provider_name.to_string());
                } else {
                    self.settings.enabled_providers.remove(provider_name);
                }
                self.settings_changed = true;
            }
        }
    }

    fn show_cookies_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Browser Cookies");
        ui.add_space(8.0);

        ui.label(
            RichText::new("CodexBar automatically extracts cookies from Chrome, Edge, Brave, and Firefox.")
                .size(12.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("You can also add manual cookies for providers that need web authentication.")
                .size(12.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.add_space(16.0);

        // Status message
        if let Some((msg, is_error)) = &self.cookie_status_msg {
            let color = if *is_error { Theme::RED } else { Theme::USAGE_GREEN };
            ui.label(RichText::new(msg).size(11.0).color(color));
            ui.add_space(8.0);
        }

        // --- Saved Manual Cookies Section ---
        ui.label(
            RichText::new("Saved Manual Cookies")
                .size(14.0)
                .color(Theme::TEXT_PRIMARY)
                .strong(),
        );
        ui.add_space(8.0);

        let saved_cookies = self.cookies.get_all_for_display();
        if saved_cookies.is_empty() {
            ui.label(
                RichText::new("No manual cookies saved.")
                    .size(11.0)
                    .color(Theme::TEXT_MUTED),
            );
        } else {
            let mut to_remove: Option<String> = None;
            for cookie_info in &saved_cookies {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&cookie_info.provider)
                            .size(12.0)
                            .color(Theme::TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("(saved {})", &cookie_info.saved_at))
                            .size(10.0)
                            .color(Theme::TEXT_MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Remove").clicked() {
                            to_remove = Some(cookie_info.provider_id.clone());
                        }
                    });
                });
            }
            if let Some(provider_id) = to_remove {
                self.cookies.remove(&provider_id);
                let _ = self.cookies.save();
                self.cookie_status_msg = Some((format!("Removed cookie for {}", provider_id), false));
            }
        }

        ui.add_space(16.0);

        // --- Add Manual Cookie Section ---
        ui.label(
            RichText::new("Add Manual Cookie")
                .size(14.0)
                .color(Theme::TEXT_PRIMARY)
                .strong(),
        );
        ui.add_space(8.0);

        // Provider dropdown
        ui.horizontal(|ui| {
            ui.label("Provider:");
            egui::ComboBox::from_id_salt("cookie_provider")
                .selected_text(if self.new_cookie_provider.is_empty() {
                    "Select provider..."
                } else {
                    &self.new_cookie_provider
                })
                .show_ui(ui, |ui| {
                    // Show providers that support web auth
                    let web_providers = ["claude", "cursor", "kimi"];
                    for provider_name in web_providers {
                        if let Some(id) = ProviderId::from_cli_name(provider_name) {
                            if ui.selectable_label(
                                self.new_cookie_provider == provider_name,
                                id.display_name(),
                            ).clicked() {
                                self.new_cookie_provider = provider_name.to_string();
                            }
                        }
                    }
                });
        });

        ui.add_space(8.0);

        // Cookie value text area
        ui.label("Cookie header value:");
        ui.add_space(4.0);
        let text_edit = egui::TextEdit::multiline(&mut self.new_cookie_value)
            .desired_width(ui.available_width())
            .desired_rows(3)
            .hint_text("Paste cookie header from browser dev tools (e.g., 'session=abc123; token=xyz')");
        ui.add(text_edit);

        ui.add_space(8.0);

        // Save button
        ui.horizontal(|ui| {
            let can_save = !self.new_cookie_provider.is_empty() && !self.new_cookie_value.is_empty();
            if ui.add_enabled(can_save, egui::Button::new("Save Cookie")).clicked() {
                self.cookies.set(&self.new_cookie_provider, &self.new_cookie_value);
                if let Err(e) = self.cookies.save() {
                    self.cookie_status_msg = Some((format!("Failed to save: {}", e), true));
                } else {
                    let provider_name = ProviderId::from_cli_name(&self.new_cookie_provider)
                        .map(|id| id.display_name().to_string())
                        .unwrap_or_else(|| self.new_cookie_provider.clone());
                    self.cookie_status_msg = Some((format!("Cookie saved for {}", provider_name), false));
                    self.new_cookie_provider.clear();
                    self.new_cookie_value.clear();
                }
            }
        });

        ui.add_space(16.0);

        // Help text
        ui.separator();
        ui.add_space(8.0);
        ui.label(
            RichText::new("How to get cookies:")
                .size(12.0)
                .color(Theme::TEXT_PRIMARY)
                .strong(),
        );
        ui.label(
            RichText::new("1. Open the provider's website and log in")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.label(
            RichText::new("2. Open browser DevTools (F12) → Network tab")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.label(
            RichText::new("3. Make any request, find Cookie header in Request Headers")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.label(
            RichText::new("4. Copy the entire Cookie value and paste above")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
    }

    fn show_advanced_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Advanced");
        ui.add_space(12.0);

        // Refresh interval
        ui.horizontal(|ui| {
            ui.label("Refresh interval:");
            let intervals = [
                (0, "Manual"),     // 0 = manual refresh only
                (60, "1 min"),
                (120, "2 min"),
                (300, "5 min"),
                (600, "10 min"),
                (900, "15 min"),
            ];
            egui::ComboBox::from_id_salt("refresh_interval")
                .selected_text(
                    intervals
                        .iter()
                        .find(|(secs, _)| *secs == self.settings.refresh_interval_secs)
                        .map(|(_, label)| *label)
                        .unwrap_or("5 min"),
                )
                .show_ui(ui, |ui| {
                    for (secs, label) in intervals {
                        if ui
                            .selectable_value(
                                &mut self.settings.refresh_interval_secs,
                                secs,
                                label,
                            )
                            .changed()
                        {
                            self.settings_changed = true;
                        }
                    }
                });
        });
        ui.label(
            RichText::new("Set to Manual for refresh only on demand")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.add_space(12.0);

        // Merge tray icons
        let mut merge_icons = self.settings.merge_tray_icons;
        if ui
            .checkbox(&mut merge_icons, "Merge tray icons")
            .changed()
        {
            self.settings.merge_tray_icons = merge_icons;
            self.settings_changed = true;
        }
        ui.label(
            RichText::new("Show all providers in a single tray icon")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.add_space(12.0);

        // Show usage as used vs remaining
        let mut show_as_used = self.settings.show_as_used;
        if ui
            .checkbox(&mut show_as_used, "Show usage as used")
            .changed()
        {
            self.settings.show_as_used = show_as_used;
            self.settings_changed = true;
        }
        ui.label(
            RichText::new("When off, bars show remaining quota instead")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
        ui.add_space(12.0);

        // Surprise Me animations
        let mut surprise = self.settings.surprise_animations;
        if ui
            .checkbox(&mut surprise, "Surprise me")
            .changed()
        {
            self.settings.surprise_animations = surprise;
            self.settings_changed = true;
        }
        ui.label(
            RichText::new("Random blink and wiggle animations on tray icon")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
    }

    fn show_about_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("About CodexBar");
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("C").size(32.0).color(Theme::TAB_ACTIVE).strong());
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("CodexBar for Windows").size(16.0).strong());
                ui.label(
                    RichText::new("Version 0.1.0")
                        .size(12.0)
                        .color(Theme::TEXT_MUTED),
                );
            });
        });

        ui.add_space(20.0);

        ui.label("A Windows port of the macOS CodexBar app.");
        ui.label("Track your AI provider usage from the system tray.");

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.link("GitHub Repository").clicked() {
                let _ = open::that("https://github.com/steipete/CodexBar");
            }
        });

        ui.add_space(20.0);

        ui.label(
            RichText::new("Built with Rust + egui")
                .size(11.0)
                .color(Theme::TEXT_MUTED),
        );
    }
}
