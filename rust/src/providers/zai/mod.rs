//! Zed AI (z.ai) provider implementation
//!
//! Fetches usage data from Zed's AI service
//! Zed stores credentials and usage info locally

pub mod mcp_details;

// Re-exports for MCP details menu
#[allow(unused_imports)]
pub use mcp_details::{McpDetailsMenu, ZaiLimitEntry, ZaiLimitType, ZaiLimitUnit, ZaiUsageDetail, ZaiUsageSnapshot};

use async_trait::async_trait;
use std::path::PathBuf;

use crate::core::{
    FetchContext, Provider, ProviderId, ProviderError, ProviderFetchResult,
    ProviderMetadata, RateWindow, SourceMode, UsageSnapshot,
};

/// Zed AI provider
pub struct ZaiProvider {
    metadata: ProviderMetadata,
}

impl ZaiProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Zai,
                display_name: "Zed AI",
                session_label: "Session",
                weekly_label: "Monthly",
                supports_opus: false,
                supports_credits: true,
                default_enabled: false,
                is_primary: false,
                dashboard_url: Some("https://zed.dev/account"),
                status_page_url: Some("https://status.zed.dev"),
            },
        }
    }

    /// Get Zed config directory
    fn get_zed_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::config_dir().map(|p| p.join("Zed"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            dirs::config_dir().map(|p| p.join("zed"))
        }
    }

    /// Find the Zed CLI binary
    fn which_zed() -> Option<PathBuf> {
        let possible_paths = [
            which::which("zed").ok(),
            #[cfg(target_os = "windows")]
            dirs::data_local_dir().map(|p| p.join("Programs").join("Zed").join("zed.exe")),
            #[cfg(target_os = "windows")]
            Some(PathBuf::from("C:\\Program Files\\Zed\\zed.exe")),
            #[cfg(not(target_os = "windows"))]
            None,
        ];

        possible_paths.into_iter().flatten().find(|p| p.exists())
    }

    /// Read credentials from Zed config
    async fn read_credentials(&self) -> Result<String, ProviderError> {
        let config_path = Self::get_zed_config_path()
            .ok_or_else(|| ProviderError::NotInstalled("Zed config directory not found".to_string()))?;

        // Zed stores credentials in db/
        let creds_file = config_path.join("db").join("zed_credentials");
        if creds_file.exists() {
            let content = tokio::fs::read_to_string(&creds_file).await
                .map_err(|e| ProviderError::Other(e.to_string()))?;
            return Ok(content.trim().to_string());
        }

        // Also check settings.json for access_token
        let settings_file = config_path.join("settings.json");
        if settings_file.exists() {
            let content = tokio::fs::read_to_string(&settings_file).await
                .map_err(|e| ProviderError::Other(e.to_string()))?;
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(token) = json.get("access_token").and_then(|v| v.as_str()) {
                    return Ok(token.to_string());
                }
            }
        }

        Err(ProviderError::AuthRequired)
    }

    /// Fetch usage via Zed API
    async fn fetch_via_web(&self) -> Result<UsageSnapshot, ProviderError> {
        let token = self.read_credentials().await?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let resp = client
            .get("https://api.zed.dev/user/usage")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ProviderError::AuthRequired);
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        self.parse_usage_response(&json)
    }

    fn parse_usage_response(&self, json: &serde_json::Value) -> Result<UsageSnapshot, ProviderError> {
        let used_credits = json.get("used_credits")
            .or_else(|| json.get("usage"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let credit_limit = json.get("credit_limit")
            .or_else(|| json.get("limit"))
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0);

        let used_percent = if credit_limit > 0.0 {
            (used_credits / credit_limit) * 100.0
        } else {
            0.0
        };

        let email = json.get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let plan = json.get("plan")
            .or_else(|| json.get("subscription"))
            .and_then(|v| v.as_str())
            .unwrap_or("Zed AI");

        let mut usage = UsageSnapshot::new(RateWindow::new(used_percent))
            .with_login_method(plan);

        if let Some(email) = email {
            usage = usage.with_email(email);
        }

        Ok(usage)
    }

    /// Probe CLI for basic detection
    async fn probe_cli(&self) -> Result<UsageSnapshot, ProviderError> {
        let zed_path = Self::which_zed().ok_or_else(|| {
            ProviderError::NotInstalled("Zed not found. Install from https://zed.dev".to_string())
        })?;

        if zed_path.exists() {
            let usage = UsageSnapshot::new(RateWindow::new(0.0))
                .with_login_method("Zed (installed)");
            Ok(usage)
        } else {
            Err(ProviderError::NotInstalled("Zed not found".to_string()))
        }
    }
}

impl Default for ZaiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for ZaiProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Zai
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching Zed AI usage");

        match ctx.source_mode {
            SourceMode::Auto => {
                if let Ok(usage) = self.fetch_via_web().await {
                    return Ok(ProviderFetchResult::new(usage, "web"));
                }
                let usage = self.probe_cli().await?;
                Ok(ProviderFetchResult::new(usage, "cli"))
            }
            SourceMode::Web => {
                let usage = self.fetch_via_web().await?;
                Ok(ProviderFetchResult::new(usage, "web"))
            }
            SourceMode::Cli => {
                let usage = self.probe_cli().await?;
                Ok(ProviderFetchResult::new(usage, "cli"))
            }
            SourceMode::OAuth => {
                Err(ProviderError::UnsupportedSource(SourceMode::OAuth))
            }
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![SourceMode::Auto, SourceMode::Web, SourceMode::Cli]
    }

    fn supports_web(&self) -> bool {
        true
    }

    fn supports_cli(&self) -> bool {
        true
    }
}
