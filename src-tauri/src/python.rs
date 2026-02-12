use std::path::PathBuf;

use reqwest::Client;

use crate::archive::extract_tar_gz;
use crate::config::load_config;
use crate::download::download_file;
use crate::error::{AppError, Result};
use crate::github::fetch_python_releases;
use crate::paths::{get_compat_python_dir, get_python_dir, get_python_exe_path};
use crate::platform::find_python_asset_for_version;

/// Check if Python is installed by checking both directories.
pub fn is_python_installed() -> bool {
    let python_exe = get_python_exe_path(&get_python_dir());
    let compat_exe = get_python_exe_path(&get_compat_python_dir());
    python_exe.exists() && compat_exe.exists()
}

/// Check if an AstrBot version requires Python 3.10 (v4.14.6 and earlier).
/// Returns true if the version is <= v4.14.6, false otherwise.
fn requires_python310(version: &str) -> bool {
    // Strip 'v' prefix if present
    let version = version.strip_prefix('v').unwrap_or(version);

    // Parse version parts
    let parts: Vec<u32> = version.split('.').filter_map(|s| s.parse().ok()).collect();

    // Compare with v4.14.6
    match parts.as_slice() {
        [major, minor, patch, ..] => (*major, *minor, *patch) <= (4, 14, 6),
        [major, minor] => (*major, *minor) <= (4, 14),
        [major] => *major <= 4,
        _ => false, // Default to Python 3.12 for unparseable versions
    }
}

/// Get the appropriate Python executable for a given AstrBot version.
/// - v4.14.6 and earlier: Python 3.10 (compat_python)
/// - v4.14.7 and later: Python 3.12 (python)
pub fn get_python_for_version(version: &str) -> Result<PathBuf> {
    let python_dir = if requires_python310(version) {
        get_compat_python_dir()
    } else {
        get_python_dir()
    };

    let exe = get_python_exe_path(&python_dir);

    if exe.exists() {
        Ok(exe)
    } else {
        Err(AppError::python_not_installed())
    }
}

fn resolve_python_runtime_target(major_version: &str) -> Result<PathBuf> {
    match major_version {
        "3.12" => Ok(get_python_dir()),
        "3.10" => Ok(get_compat_python_dir()),
        _ => Err(AppError::python(format!(
            "Unsupported Python major version: {}",
            major_version
        ))),
    }
}

/// Download and install a specific Python version to the given directory.
async fn install_python_version(
    client: &Client,
    major_version: &str,
    target_dir: &PathBuf,
) -> Result<String> {
    // If target directory exists but runtime is missing/corrupted, clean it first.
    if target_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(target_dir) {
            log::warn!("Failed to clean python dir {:?}: {}", target_dir, e);
        }
    }

    // Fetch releases with assets
    let releases = fetch_python_releases(client).await?;

    // Find a release that has the Python version we need
    let mut download_url = None;
    let mut python_version = String::new();

    for release in &releases {
        if let Ok((url, version)) = find_python_asset_for_version(&release.assets, major_version) {
            download_url = Some(url);
            python_version = version;
            break;
        }
    }

    let mut url = download_url.ok_or_else(|| AppError::python(major_version.to_string()))?;

    // Apply GitHub proxy to the download URL if configured
    if let Ok(config) = load_config() {
        if !config.github_proxy.is_empty() {
            let base = config.github_proxy.trim_end_matches('/');
            url = format!("{}/{}", base, url);
        }
    }

    let archive_path = target_dir.join("python.tar.gz");

    // Ensure directory exists
    std::fs::create_dir_all(target_dir)
        .map_err(|e| AppError::io(format!("Failed to create python dir: {}", e)))?;

    download_file(client, &url, &archive_path).await?;

    // Extract to target_dir, top-level directory in archive will be stripped
    extract_tar_gz(&archive_path, target_dir)?;

    // Clean up archive
    if let Err(e) = std::fs::remove_file(&archive_path) {
        log::warn!("Failed to remove archive {:?}: {}", archive_path, e);
    }

    Ok(python_version)
}

/// Install missing Python runtimes only:
/// - Python 3.12 -> `python/`
/// - Python 3.10 -> `compat_python/`
pub async fn install_python(client: &Client) -> Result<String> {
    let python_dir = get_python_dir();
    let compat_python_dir = get_compat_python_dir();

    let python_312_exe = get_python_exe_path(&python_dir);
    let python_310_exe = get_python_exe_path(&compat_python_dir);

    let mut installed: Vec<String> = Vec::new();
    let mut skipped: Vec<&str> = Vec::new();

    // Install Python 3.12 if missing
    if python_312_exe.exists() {
        skipped.push("3.12");
    } else {
        let version_312 = install_python_version(client, "3.12", &python_dir).await?;
        installed.push(version_312);
    }

    // Install Python 3.10 if missing
    if python_310_exe.exists() {
        skipped.push("3.10");
    } else {
        let version_310 = install_python_version(client, "3.10", &compat_python_dir).await?;
        installed.push(version_310);
    }

    if installed.is_empty() {
        return Ok("Python runtimes already installed".to_string());
    }

    if skipped.is_empty() {
        Ok(format!(
            "Installed Python runtimes: {}",
            installed.join(", ")
        ))
    } else {
        Ok(format!(
            "Installed Python runtimes: {}; already present: {}",
            installed.join(", "),
            skipped.join(", ")
        ))
    }
}

/// Reinstall one specific Python runtime:
/// - "3.12" -> `python/`
/// - "3.10" -> `compat_python/`
///
/// This always removes the existing runtime directory and installs it again.
pub async fn reinstall_python(client: &Client, major_version: &str) -> Result<String> {
    let target_dir = resolve_python_runtime_target(major_version)?;
    let installed_version = install_python_version(client, major_version, &target_dir).await?;

    Ok(format!(
        "Reinstalled Python {} runtime: {}",
        major_version, installed_version
    ))
}
