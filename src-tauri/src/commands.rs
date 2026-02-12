use std::cmp::Ordering;
use std::sync::Arc;

use reqwest::Client;
use tauri::{AppHandle, State};

use crate::backup;
use crate::config::{load_config, with_config_mut, AppConfig, BackupInfo, InstalledVersion};
use crate::download;
use crate::error::{AppError, Result};
use crate::github::{self, GitHubRelease};
use crate::instance::{self, InstanceStatus, ProcessManager};
use crate::paths;
use crate::python;

fn sort_installed_versions_semver(versions: &mut [InstalledVersion]) {
    versions.sort_by(|a, b| {
        let av = semver::Version::parse(a.version.trim_start_matches('v')).ok();
        let bv = semver::Version::parse(b.version.trim_start_matches('v')).ok();

        match (av, bv) {
            (Some(va), Some(vb)) => vb.cmp(&va),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => b.version.cmp(&a.version),
        }
    });
}

pub struct AppState {
    pub client: Client,
    pub process_manager: Arc<ProcessManager>,
}

pub async fn build_app_snapshot(process_manager: &ProcessManager) -> Result<AppSnapshot> {
    let config = load_config()?;
    let instances = instance::list_instances(process_manager).await?;
    let backups = backup::list_backups()?;
    let mut config_for_snapshot = (*config).clone();
    sort_installed_versions_semver(&mut config_for_snapshot.installed_versions);

    Ok(AppSnapshot {
        instances,
        versions: config_for_snapshot.installed_versions.clone(),
        backups,
        python_installed: python::is_python_installed(),
        config: config_for_snapshot,
    })
}

#[tauri::command]
pub async fn get_app_snapshot(state: State<'_, AppState>) -> Result<AppSnapshot> {
    build_app_snapshot(&state.process_manager).await
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AppSnapshot {
    pub instances: Vec<InstanceStatus>,
    pub versions: Vec<InstalledVersion>,
    pub backups: Vec<BackupInfo>,
    pub python_installed: bool,
    pub config: AppConfig,
}

// === Config ===

#[tauri::command]
pub async fn save_github_proxy(github_proxy: String, state: State<'_, AppState>) -> Result<()> {
    // Test connectivity first
    let url = github::build_api_url(&github_proxy);
    let resp = state
        .client
        .get(&url)
        .header("User-Agent", "astrbot-launcher")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| AppError::network_with_url(&url, e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::network_with_url(&url, resp.status().to_string()));
    }
    // Test passed, save
    with_config_mut(move |config| {
        config.github_proxy = github_proxy;
        Ok(())
    })
}

#[tauri::command]
pub async fn save_pypi_mirror(pypi_mirror: String, state: State<'_, AppState>) -> Result<()> {
    // Test connectivity first
    let base = if pypi_mirror.is_empty() {
        "https://pypi.org".to_string()
    } else {
        pypi_mirror.trim_end_matches('/').to_string()
    };
    let url = format!("{}/simple/", base);
    let resp = state
        .client
        .get(&url)
        .header("User-Agent", "astrbot-launcher")
        .send()
        .await
        .map_err(|e| AppError::network_with_url(&url, e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::network_with_url(&url, resp.status().to_string()));
    }
    // Test passed, save
    with_config_mut(move |config| {
        config.pypi_mirror = pypi_mirror;
        Ok(())
    })
}

#[tauri::command]
pub async fn save_close_to_tray(close_to_tray: bool) -> Result<()> {
    with_config_mut(move |config| {
        config.close_to_tray = close_to_tray;
        Ok(())
    })
}

#[tauri::command]
pub fn compare_versions(a: String, b: String) -> i32 {
    match (
        semver::Version::parse(a.trim_start_matches('v')),
        semver::Version::parse(b.trim_start_matches('v')),
    ) {
        (Ok(va), Ok(vb)) => va.cmp(&vb) as i32,
        _ => 0,
    }
}

#[tauri::command]
pub async fn save_check_instance_update(check_instance_update: bool) -> Result<()> {
    with_config_mut(move |config| {
        config.check_instance_update = check_instance_update;
        Ok(())
    })
}

#[tauri::command]
pub async fn save_persist_instance_state(persist_instance_state: bool) -> Result<()> {
    with_config_mut(move |config| {
        config.persist_instance_state = persist_instance_state;
        Ok(())
    })
}

// === Python ===

#[tauri::command]
pub fn is_instance_deployed(instance_id: &str) -> bool {
    paths::is_instance_deployed(instance_id)
}

#[tauri::command]
pub async fn install_python(state: State<'_, AppState>) -> Result<String> {
    python::install_python(&state.client).await
}

#[tauri::command]
pub async fn reinstall_python(state: State<'_, AppState>, major_version: String) -> Result<String> {
    python::reinstall_python(&state.client, &major_version).await
}

// === GitHub ===

#[tauri::command]
pub async fn fetch_releases(state: State<'_, AppState>) -> Result<Vec<GitHubRelease>> {
    github::fetch_releases(&state.client).await
}

// === Version Management ===

#[tauri::command]
pub async fn install_version(state: State<'_, AppState>, release: GitHubRelease) -> Result<()> {
    download::download_version(&state.client, &release).await
}

#[tauri::command]
pub async fn uninstall_version(version: String) -> Result<()> {
    download::remove_version(&version)
}

// === Troubleshooting ===

#[tauri::command]
pub async fn clear_instance_data(instance_id: String, state: State<'_, AppState>) -> Result<()> {
    if state.process_manager.is_running(&instance_id).await {
        return Err(AppError::instance_running());
    }
    instance::clear_instance_data(&instance_id)
}

#[tauri::command]
pub async fn clear_instance_venv(instance_id: String, state: State<'_, AppState>) -> Result<()> {
    if state.process_manager.is_running(&instance_id).await {
        return Err(AppError::instance_running());
    }
    instance::clear_instance_venv(&instance_id)
}

#[tauri::command]
pub async fn clear_pycache(instance_id: String, state: State<'_, AppState>) -> Result<()> {
    if state.process_manager.is_running(&instance_id).await {
        return Err(AppError::instance_running());
    }
    instance::clear_pycache(&instance_id)
}

// === Instance Management ===

#[tauri::command]
pub async fn create_instance(name: String, version: String, port: u16) -> Result<()> {
    instance::create_instance(&name, &version, port)
}

#[tauri::command]
pub async fn delete_instance(instance_id: String, state: State<'_, AppState>) -> Result<()> {
    instance::delete_instance(&instance_id, Arc::clone(&state.process_manager)).await
}

#[tauri::command]
pub async fn update_instance(
    app_handle: AppHandle,
    instance_id: String,
    name: Option<String>,
    version: Option<String>,
    port: Option<u16>,
    state: State<'_, AppState>,
) -> Result<()> {
    if state.process_manager.is_running(&instance_id).await {
        return Err(AppError::instance_running());
    }

    instance::update_instance(
        &instance_id,
        name.as_deref(),
        version.as_deref(),
        port,
        &app_handle,
    )
    .await
}

#[tauri::command]
pub async fn start_instance(
    app_handle: AppHandle,
    instance_id: String,
    state: State<'_, AppState>,
) -> Result<u16> {
    instance::start_instance(
        &instance_id,
        &app_handle,
        Arc::clone(&state.process_manager),
    )
    .await
}

#[tauri::command]
pub async fn stop_instance(instance_id: String, state: State<'_, AppState>) -> Result<()> {
    instance::stop_instance(&instance_id, Arc::clone(&state.process_manager)).await
}

#[tauri::command]
pub async fn restart_instance(
    app_handle: AppHandle,
    instance_id: String,
    state: State<'_, AppState>,
) -> Result<u16> {
    instance::restart_instance(
        &instance_id,
        &app_handle,
        Arc::clone(&state.process_manager),
    )
    .await
}

#[tauri::command]
pub async fn get_instance_port(instance_id: String, state: State<'_, AppState>) -> Result<u16> {
    state
        .process_manager
        .get_port(&instance_id)
        .ok_or_else(AppError::instance_not_running)
}

// === Backup ===

#[tauri::command]
pub async fn create_backup(
    instance_id: String,
    include_venv: bool,
    state: State<'_, AppState>,
) -> Result<String> {
    if state.process_manager.is_running(&instance_id).await {
        return Err(AppError::instance_running());
    }
    backup::create_backup(&instance_id, include_venv)
}

#[tauri::command]
pub async fn restore_backup(backup_path: String) -> Result<()> {
    backup::restore_backup(&backup_path)
}

#[tauri::command]
pub async fn delete_backup(backup_path: String) -> Result<()> {
    backup::delete_backup(&backup_path)
}
