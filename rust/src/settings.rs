//! Settings management for CodexBar
//!
//! Handles persistent configuration including:
//! - Enabled/disabled providers
//! - Refresh interval
//! - Manual cookies
//! - Other user preferences

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::core::ProviderId;

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Enabled provider IDs (by CLI name)
    pub enabled_providers: HashSet<String>,

    /// Refresh interval in seconds (0 = manual only)
    pub refresh_interval_secs: u64,

    /// Whether to start minimized
    pub start_minimized: bool,

    /// Whether to start at login
    pub start_at_login: bool,

    /// Whether to show notifications
    pub show_notifications: bool,

    /// High usage threshold for warnings (percentage)
    pub high_usage_threshold: f64,

    /// Critical usage threshold for alerts (percentage)
    pub critical_usage_threshold: f64,

    /// Merge mode: show all enabled providers in a single tray icon
    pub merge_tray_icons: bool,

    /// Show usage bars as "used" (true) or "remaining" (false)
    pub show_as_used: bool,

    /// Enable random "surprise" animations (blinks, wiggles)
    pub surprise_animations: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let mut enabled = HashSet::new();
        // Default enabled providers
        enabled.insert("claude".to_string());
        enabled.insert("codex".to_string());

        Self {
            enabled_providers: enabled,
            refresh_interval_secs: 300, // 5 minutes
            start_minimized: false,
            start_at_login: false,
            show_notifications: true,
            high_usage_threshold: 70.0,
            critical_usage_threshold: 90.0,
            merge_tray_icons: false, // Show single provider by default
            show_as_used: true,      // Show as "used" by default
            surprise_animations: false, // Off by default
        }
    }
}

impl Settings {
    /// Get the settings file path
    pub fn settings_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("CodexBar").join("settings.json"))
    }

    /// Load settings from disk
    pub fn load() -> Self {
        if let Some(path) = Self::settings_path() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(settings) = serde_json::from_str(&content) {
                        return settings;
                    }
                }
            }
        }
        Self::default()
    }

    /// Save settings to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::settings_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine settings path"))?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;

        Ok(())
    }

    /// Set start at login (updates Windows registry)
    pub fn set_start_at_login(&mut self, enabled: bool) -> anyhow::Result<()> {
        self.start_at_login = enabled;

        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;

            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let run_key = hkcu.open_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\Run",
                KEY_READ | KEY_WRITE,
            )?;

            if enabled {
                // Get the current executable path
                let exe_path = std::env::current_exe()?;
                let exe_str = exe_path.to_string_lossy();
                // Add --minimized flag when starting at login
                let cmd = format!("\"{}\" menubar", exe_str);
                run_key.set_value("CodexBar", &cmd)?;
            } else {
                // Remove the registry entry (ignore if it doesn't exist)
                let _ = run_key.delete_value("CodexBar");
            }
        }

        Ok(())
    }

    /// Check if start at login is actually enabled in registry
    #[cfg(target_os = "windows")]
    pub fn is_start_at_login_enabled() -> bool {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(run_key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
            run_key.get_value::<String, _>("CodexBar").is_ok()
        } else {
            false
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn is_start_at_login_enabled() -> bool {
        false
    }

    /// Check if a provider is enabled
    pub fn is_provider_enabled(&self, id: ProviderId) -> bool {
        self.enabled_providers.contains(id.cli_name())
    }

    /// Enable a provider
    pub fn enable_provider(&mut self, id: ProviderId) {
        self.enabled_providers.insert(id.cli_name().to_string());
    }

    /// Disable a provider
    pub fn disable_provider(&mut self, id: ProviderId) {
        self.enabled_providers.remove(id.cli_name());
    }

    /// Toggle a provider's enabled state
    pub fn toggle_provider(&mut self, id: ProviderId) -> bool {
        let name = id.cli_name().to_string();
        if self.enabled_providers.contains(&name) {
            self.enabled_providers.remove(&name);
            false
        } else {
            self.enabled_providers.insert(name);
            true
        }
    }

    /// Get list of enabled provider IDs
    pub fn get_enabled_provider_ids(&self) -> Vec<ProviderId> {
        ProviderId::all()
            .iter()
            .filter(|id| self.is_provider_enabled(**id))
            .copied()
            .collect()
    }

    /// Get all available providers with their enabled status
    pub fn get_all_providers_status(&self) -> Vec<ProviderStatus> {
        ProviderId::all()
            .iter()
            .map(|id| ProviderStatus {
                id: id.cli_name().to_string(),
                name: id.display_name().to_string(),
                enabled: self.is_provider_enabled(*id),
            })
            .collect()
    }
}

/// Provider status for settings UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}

/// Refresh interval options
#[derive(Debug, Clone, Serialize)]
pub struct RefreshIntervalOption {
    pub value: u64,
    pub label: String,
}

/// Get available refresh interval options
pub fn get_refresh_interval_options() -> Vec<RefreshIntervalOption> {
    vec![
        RefreshIntervalOption { value: 60, label: "1 minute".to_string() },
        RefreshIntervalOption { value: 120, label: "2 minutes".to_string() },
        RefreshIntervalOption { value: 300, label: "5 minutes".to_string() },
        RefreshIntervalOption { value: 600, label: "10 minutes".to_string() },
        RefreshIntervalOption { value: 900, label: "15 minutes".to_string() },
        RefreshIntervalOption { value: 1800, label: "30 minutes".to_string() },
    ]
}

/// Manual cookie storage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManualCookies {
    /// Provider ID -> cookie header mapping
    pub cookies: HashMap<String, ManualCookieEntry>,
}

/// A single manual cookie entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualCookieEntry {
    pub cookie_header: String,
    pub saved_at: String,
}

impl ManualCookies {
    /// Get the cookies file path
    pub fn cookies_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("CodexBar").join("manual_cookies.json"))
    }

    /// Load manual cookies from disk
    pub fn load() -> Self {
        if let Some(path) = Self::cookies_path() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(cookies) = serde_json::from_str(&content) {
                        return cookies;
                    }
                }
            }
        }
        Self::default()
    }

    /// Save manual cookies to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::cookies_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine cookies path"))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;

        Ok(())
    }

    /// Get cookie for a provider
    pub fn get(&self, provider_id: &str) -> Option<&str> {
        self.cookies.get(provider_id).map(|e| e.cookie_header.as_str())
    }

    /// Set cookie for a provider
    pub fn set(&mut self, provider_id: &str, cookie_header: &str) {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
        self.cookies.insert(
            provider_id.to_string(),
            ManualCookieEntry {
                cookie_header: cookie_header.to_string(),
                saved_at: now,
            },
        );
    }

    /// Remove cookie for a provider
    pub fn remove(&mut self, provider_id: &str) {
        self.cookies.remove(provider_id);
    }

    /// Get all saved cookies for UI display
    pub fn get_all_for_display(&self) -> Vec<SavedCookieInfo> {
        self.cookies
            .iter()
            .map(|(id, entry)| {
                let provider_name = ProviderId::from_cli_name(id)
                    .map(|p| p.display_name().to_string())
                    .unwrap_or_else(|| id.clone());

                SavedCookieInfo {
                    provider_id: id.clone(),
                    provider: provider_name,
                    saved_at: entry.saved_at.clone(),
                }
            })
            .collect()
    }
}

/// Info about a saved cookie for UI display
#[derive(Debug, Clone, Serialize)]
pub struct SavedCookieInfo {
    pub provider_id: String,
    pub provider: String,
    pub saved_at: String,
}

/// API key storage for providers that need tokens
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeys {
    /// Provider ID -> API key mapping
    pub keys: HashMap<String, ApiKeyEntry>,
}

/// A single API key entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    pub api_key: String,
    pub saved_at: String,
    /// Optional label for the key (e.g., "Personal", "Work")
    #[serde(default)]
    pub label: Option<String>,
}

impl ApiKeys {
    /// Get the API keys file path
    pub fn keys_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("CodexBar").join("api_keys.json"))
    }

    /// Load API keys from disk
    pub fn load() -> Self {
        if let Some(path) = Self::keys_path() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(keys) = serde_json::from_str(&content) {
                        return keys;
                    }
                }
            }
        }
        Self::default()
    }

    /// Save API keys to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::keys_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine API keys path"))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;

        Ok(())
    }

    /// Get API key for a provider
    pub fn get(&self, provider_id: &str) -> Option<&str> {
        self.keys.get(provider_id).map(|e| e.api_key.as_str())
    }

    /// Set API key for a provider
    pub fn set(&mut self, provider_id: &str, api_key: &str, label: Option<&str>) {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
        self.keys.insert(
            provider_id.to_string(),
            ApiKeyEntry {
                api_key: api_key.to_string(),
                saved_at: now,
                label: label.map(|s| s.to_string()),
            },
        );
    }

    /// Remove API key for a provider
    pub fn remove(&mut self, provider_id: &str) {
        self.keys.remove(provider_id);
    }

    /// Check if a provider has an API key configured
    pub fn has_key(&self, provider_id: &str) -> bool {
        self.keys.get(provider_id).map(|e| !e.api_key.is_empty()).unwrap_or(false)
    }

    /// Get all saved API keys for UI display (with masked values)
    pub fn get_all_for_display(&self) -> Vec<SavedApiKeyInfo> {
        self.keys
            .iter()
            .map(|(id, entry)| {
                let provider_name = ProviderId::from_cli_name(id)
                    .map(|p| p.display_name().to_string())
                    .unwrap_or_else(|| id.clone());

                // Mask the key for display (show first 4 and last 4 chars)
                let masked = if entry.api_key.len() > 12 {
                    format!(
                        "{}...{}",
                        &entry.api_key[..4],
                        &entry.api_key[entry.api_key.len() - 4..]
                    )
                } else if entry.api_key.len() > 4 {
                    format!("{}...", &entry.api_key[..4])
                } else {
                    "****".to_string()
                };

                SavedApiKeyInfo {
                    provider_id: id.clone(),
                    provider: provider_name,
                    masked_key: masked,
                    saved_at: entry.saved_at.clone(),
                    label: entry.label.clone(),
                }
            })
            .collect()
    }
}

/// Info about a saved API key for UI display
#[derive(Debug, Clone, Serialize)]
pub struct SavedApiKeyInfo {
    pub provider_id: String,
    pub provider: String,
    pub masked_key: String,
    pub saved_at: String,
    pub label: Option<String>,
}

/// Provider configuration info
#[derive(Debug, Clone)]
pub struct ProviderConfigInfo {
    pub id: ProviderId,
    pub name: &'static str,
    pub requires_api_key: bool,
    pub api_key_env_var: Option<&'static str>,
    pub api_key_help: Option<&'static str>,
    pub config_file_path: Option<&'static str>,
    pub dashboard_url: Option<&'static str>,
}

/// Get configuration info for providers that need API keys
pub fn get_api_key_providers() -> Vec<ProviderConfigInfo> {
    vec![
        ProviderConfigInfo {
            id: ProviderId::Amp,
            name: "Amp (Sourcegraph)",
            requires_api_key: true,
            api_key_env_var: Some("SRC_ACCESS_TOKEN"),
            api_key_help: Some("Get your token from Sourcegraph → Settings → Access Tokens"),
            config_file_path: Some("~/.amp/config.json"),
            dashboard_url: Some("https://sourcegraph.com/cody/manage"),
        },
        ProviderConfigInfo {
            id: ProviderId::Synthetic,
            name: "Synthetic",
            requires_api_key: true,
            api_key_env_var: Some("SYNTHETIC_API_KEY"),
            api_key_help: Some("Get your API key from Synthetic → Account → API Keys"),
            config_file_path: Some("~/.synthetic/config.json"),
            dashboard_url: Some("https://synthetic.computer/account"),
        },
        ProviderConfigInfo {
            id: ProviderId::Copilot,
            name: "GitHub Copilot",
            requires_api_key: true,
            api_key_env_var: Some("GITHUB_TOKEN"),
            api_key_help: Some("GitHub Personal Access Token with copilot scope"),
            config_file_path: None,
            dashboard_url: Some("https://github.com/settings/copilot"),
        },
        ProviderConfigInfo {
            id: ProviderId::Zai,
            name: "Zai",
            requires_api_key: true,
            api_key_env_var: Some("ZED_API_KEY"),
            api_key_help: Some("Get your API key from Zed → Settings → API"),
            config_file_path: None,
            dashboard_url: Some("https://zed.dev/account"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert!(settings.enabled_providers.contains("claude"));
        assert!(settings.enabled_providers.contains("codex"));
        assert_eq!(settings.refresh_interval_secs, 300);
        assert!(settings.show_notifications);
        assert_eq!(settings.high_usage_threshold, 70.0);
        assert_eq!(settings.critical_usage_threshold, 90.0);
    }

    #[test]
    fn test_settings_provider_enabled() {
        let settings = Settings::default();
        assert!(settings.is_provider_enabled(ProviderId::Claude));
        assert!(settings.is_provider_enabled(ProviderId::Codex));
        assert!(!settings.is_provider_enabled(ProviderId::Gemini));
    }

    #[test]
    fn test_settings_toggle_provider() {
        let mut settings = Settings::default();

        // Claude starts enabled
        assert!(settings.is_provider_enabled(ProviderId::Claude));

        // Toggle off
        let enabled = settings.toggle_provider(ProviderId::Claude);
        assert!(!enabled);
        assert!(!settings.is_provider_enabled(ProviderId::Claude));

        // Toggle back on
        let enabled = settings.toggle_provider(ProviderId::Claude);
        assert!(enabled);
        assert!(settings.is_provider_enabled(ProviderId::Claude));
    }

    #[test]
    fn test_settings_get_enabled_provider_ids() {
        let settings = Settings::default();
        let enabled = settings.get_enabled_provider_ids();
        assert!(enabled.contains(&ProviderId::Claude));
        assert!(enabled.contains(&ProviderId::Codex));
    }

    #[test]
    fn test_settings_get_all_providers_status() {
        let settings = Settings::default();
        let status = settings.get_all_providers_status();
        assert_eq!(status.len(), 17); // All 17 providers

        let claude_status = status.iter().find(|s| s.id == "claude").unwrap();
        assert_eq!(claude_status.name, "Claude");
        assert!(claude_status.enabled);

        let gemini_status = status.iter().find(|s| s.id == "gemini").unwrap();
        assert!(!gemini_status.enabled);
    }

    #[test]
    fn test_refresh_interval_options() {
        let options = get_refresh_interval_options();
        assert!(!options.is_empty());
        assert!(options.iter().any(|o| o.value == 60));
        assert!(options.iter().any(|o| o.value == 300));
    }

    #[test]
    fn test_manual_cookies_default() {
        let cookies = ManualCookies::default();
        assert!(cookies.cookies.is_empty());
    }

    #[test]
    fn test_manual_cookies_set_get_remove() {
        let mut cookies = ManualCookies::default();

        // Set a cookie
        cookies.set("claude", "session=abc123");
        assert_eq!(cookies.get("claude"), Some("session=abc123"));

        // Remove it
        cookies.remove("claude");
        assert_eq!(cookies.get("claude"), None);
    }
}
