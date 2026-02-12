use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::load_config;
use crate::error::{AppError, Result};

const ASTRBOT_REPO: &str = "AstrBotDevs/AstrBot";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub published_at: String,
    pub prerelease: bool,
    pub assets: Vec<GitHubAsset>,
    pub html_url: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// Build the API URL, optionally using a GitHub proxy.
/// If proxy is empty, uses the official GitHub API.
/// Proxy wraps the full original URL, e.g. `https://cdn.gh-proxy.org/https://api.github.com/...`.
pub fn build_api_url(proxy: &str) -> String {
    let raw = format!(
        "https://api.github.com/repos/{}/releases?per_page=30",
        ASTRBOT_REPO
    );
    wrap_with_proxy(proxy, &raw)
}

/// Wrap a URL with the GitHub proxy prefix.
/// If proxy is empty, returns the original URL unchanged.
fn wrap_with_proxy(proxy: &str, url: &str) -> String {
    if proxy.is_empty() {
        url.to_string()
    } else {
        let base = proxy.trim_end_matches('/');
        format!("{}/{}", base, url)
    }
}

/// Build a raw download URL, optionally using a GitHub proxy.
pub fn build_download_url(proxy: &str, tag: &str) -> String {
    let raw = format!("https://github.com/{}/archive/{}.zip", ASTRBOT_REPO, tag);
    wrap_with_proxy(proxy, &raw)
}

pub async fn fetch_releases(client: &Client) -> Result<Vec<GitHubRelease>> {
    let config = load_config()?;
    let url = build_api_url(&config.github_proxy);
    let resp = client
        .get(&url)
        .header("User-Agent", "astrbot-launcher")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| AppError::github(format!("Failed to fetch releases: {}", e)))?;

    if !resp.status().is_success() {
        return Err(AppError::github(format!(
            "GitHub API returned status: {}",
            resp.status()
        )));
    }

    resp.json::<Vec<GitHubRelease>>()
        .await
        .map_err(|e| AppError::github(format!("Failed to parse releases: {}", e)))
}

/// Fetch python-build-standalone releases with full asset information.
pub async fn fetch_python_releases(client: &Client) -> Result<Vec<GitHubRelease>> {
    let config = load_config()?;
    let url = wrap_with_proxy(
        &config.github_proxy,
        "https://api.github.com/repos/astral-sh/python-build-standalone/releases?per_page=10",
    );
    let resp = client
        .get(&url)
        .header("User-Agent", "astrbot-launcher")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| {
            AppError::github(format!(
                "Failed to fetch python-build-standalone releases: {}",
                e
            ))
        })?;

    if !resp.status().is_success() {
        return Err(AppError::github(format!(
            "GitHub API returned status: {}",
            resp.status()
        )));
    }

    resp.json::<Vec<GitHubRelease>>()
        .await
        .map_err(|e| AppError::github(format!("Failed to parse releases: {}", e)))
}

/// Get the source archive URL for a given tag, optionally using proxy.
pub fn get_source_archive_url(tag: &str) -> String {
    match load_config() {
        Ok(config) => build_download_url(&config.github_proxy, tag),
        Err(_) => format!("https://github.com/{}/archive/{}.zip", ASTRBOT_REPO, tag),
    }
}
