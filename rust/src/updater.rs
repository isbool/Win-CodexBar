//! Auto-update checker for CodexBar
//! Checks GitHub releases for new versions

use serde::Deserialize;
use crate::settings::UpdateChannel;

const GITHUB_REPO: &str = "Finesssee/Win-CodexBar";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    #[allow(dead_code)]
    pub release_url: String,
    #[allow(dead_code)]
    pub release_notes: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    assets: Vec<GitHubAsset>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    #[allow(dead_code)]
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Check for updates from GitHub releases
///
/// When `channel` is `UpdateChannel::Beta`, includes pre-release versions.
/// When `channel` is `UpdateChannel::Stable`, only considers stable releases.
#[allow(dead_code)]
pub async fn check_for_updates() -> Option<UpdateInfo> {
    check_for_updates_with_channel(UpdateChannel::Stable).await
}

/// Check for updates from GitHub releases with a specific channel
///
/// When `channel` is `UpdateChannel::Beta`, includes pre-release versions.
/// When `channel` is `UpdateChannel::Stable`, only considers stable releases.
pub async fn check_for_updates_with_channel(channel: UpdateChannel) -> Option<UpdateInfo> {
    let url = match channel {
        UpdateChannel::Beta => {
            // For beta, we need to check all releases and find the latest (including pre-releases)
            format!(
                "https://api.github.com/repos/{}/releases",
                GITHUB_REPO
            )
        }
        UpdateChannel::Stable => {
            // For stable, use the /latest endpoint which excludes pre-releases
            format!(
                "https://api.github.com/repos/{}/releases/latest",
                GITHUB_REPO
            )
        }
    };

    let client = reqwest::Client::builder()
        .user_agent("CodexBar")
        .build()
        .ok()?;

    let response = client.get(&url).send().await.ok()?;

    if !response.status().is_success() {
        tracing::debug!("GitHub API returned status: {}", response.status());
        return None;
    }

    // Parse response based on channel
    let release: GitHubRelease = match channel {
        UpdateChannel::Beta => {
            // For beta, we get an array of releases - take the first non-draft one
            let releases: Vec<GitHubRelease> = response.json().await.ok()?;
            releases.into_iter().find(|r| !r.draft)?
        }
        UpdateChannel::Stable => {
            // For stable, we get a single release object
            response.json().await.ok()?
        }
    };

    // Parse version from tag (remove 'v' prefix and '-windows' suffix if present)
    let remote_version = release
        .tag_name
        .trim_start_matches('v')
        .split('-')
        .next()
        .unwrap_or(&release.tag_name);

    // Compare versions
    if is_newer_version(remote_version, CURRENT_VERSION) {
        // Find the installer or exe asset
        let download_url = release
            .assets
            .iter()
            .find(|a| a.name.ends_with("-Setup.exe"))
            .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".exe")))
            .map(|a| a.browser_download_url.clone())
            .unwrap_or_else(|| release.html_url.clone());

        Some(UpdateInfo {
            version: release.tag_name,
            download_url,
            release_url: release.html_url,
            release_notes: release.body.unwrap_or_default(),
        })
    } else {
        None
    }
}

/// Compare semantic versions, returns true if remote is newer
fn is_newer_version(remote: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };

    let remote_v = parse_version(remote);
    let current_v = parse_version(current);

    remote_v > current_v
}

/// Get the current version
#[allow(dead_code)]
pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("1.0.1", "1.0.0"));
        assert!(is_newer_version("1.1.0", "1.0.0"));
        assert!(is_newer_version("2.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("0.9.0", "1.0.0"));
        assert!(is_newer_version("1.0.0", "0.1.0"));
    }
}
