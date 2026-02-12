use std::fs;
use std::io::Write as _;
use std::path::Path;

use futures_util::StreamExt as _;
use reqwest::Client;

use crate::config::{with_config_mut, InstalledVersion};
use crate::error::{AppError, Result};
use crate::github::{get_source_archive_url, GitHubRelease};
use crate::paths::get_versions_dir;
use crate::validation::resolve_version_zip_path;

pub async fn download_file(client: &Client, url: &str, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(e.to_string()))?;
    }

    let resp = client
        .get(url)
        .header("User-Agent", "astrbot-launcher")
        .send()
        .await
        .map_err(|e| AppError::network(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(AppError::network(resp.status().to_string()));
    }

    let mut file = fs::File::create(dest).map_err(|e| AppError::io(e.to_string()))?;

    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::network(e.to_string()))?;
        file.write_all(&chunk)
            .map_err(|e| AppError::io(e.to_string()))?;
    }

    Ok(())
}

/// Download and register an AstrBot version archive.
pub async fn download_version(client: &Client, release: &GitHubRelease) -> Result<()> {
    let version = &release.tag_name;
    let versions_dir = get_versions_dir();
    let zip_path = resolve_version_zip_path(version)?;

    std::fs::create_dir_all(&versions_dir)
        .map_err(|e| AppError::io(format!("Failed to create versions dir: {}", e)))?;

    if zip_path.exists() {
        if let Err(e) = std::fs::remove_file(&zip_path) {
            log::warn!("Failed to remove old zip {:?}: {}", zip_path, e);
        }
    }

    let core_archive_url = get_source_archive_url(version);
    download_file(client, &core_archive_url, &zip_path).await?;

    let installed = InstalledVersion {
        version: version.to_string(),
        zip_path: zip_path.to_str().unwrap_or("").to_string(),
    };

    let version_owned = version.to_string();
    with_config_mut(move |config| {
        config
            .installed_versions
            .retain(|v| v.version != version_owned.as_str());
        config.installed_versions.push(installed);
        Ok(())
    })?;

    Ok(())
}

/// Unregister and remove an AstrBot version archive.
pub fn remove_version(version: &str) -> Result<()> {
    let zip_path = resolve_version_zip_path(version)?;

    let version_owned = version.to_string();
    with_config_mut(|config| {
        for inst in config.instances.values() {
            if inst.version == version_owned.as_str() {
                return Err(AppError::version_in_use(&version_owned, &inst.name));
            }
        }

        config
            .installed_versions
            .retain(|v| v.version != version_owned.as_str());
        Ok(())
    })?;

    if zip_path.exists() {
        if let Err(e) = std::fs::remove_file(&zip_path) {
            log::warn!("Failed to remove zip {:?}: {}", zip_path, e);
        }
    }

    Ok(())
}
