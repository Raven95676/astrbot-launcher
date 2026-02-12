//! Instance deployment functionality.

use std::path::Path;

use tauri::{AppHandle, Emitter as _};
use tokio::process::Command;

use super::types::DeployProgress;
use crate::archive::extract_zip;
use crate::config::load_config;
use crate::error::{AppError, Result};
use crate::paths::{get_instance_core_dir, get_instance_venv_dir, get_venv_python};
use crate::python::get_python_for_version;
use crate::validation::validate_instance_id;

/// Emit deployment progress event.
pub fn emit_progress(
    app_handle: &AppHandle,
    instance_id: &str,
    step: &str,
    message: &str,
    progress: u8,
) {
    let _ = app_handle.emit(
        "deploy-progress",
        DeployProgress {
            instance_id: instance_id.to_string(),
            step: step.to_string(),
            message: message.to_string(),
            progress,
        },
    );
}

/// Deploy an instance by extracting the version zip and setting up venv.
pub async fn deploy_instance(instance_id: &str, app_handle: &AppHandle) -> Result<()> {
    validate_instance_id(instance_id)?;

    let config = load_config()?;
    let instance = config
        .instances
        .get(instance_id)
        .ok_or_else(|| AppError::instance_not_found(instance_id))?;

    let installed = config
        .installed_versions
        .iter()
        .find(|v| v.version == instance.version)
        .ok_or_else(|| AppError::version_not_found(&instance.version))?;

    let zip_path = std::path::PathBuf::from(&installed.zip_path);
    if !zip_path.exists() {
        return Err(AppError::io(format!(
            "Version zip file not found: {:?}",
            zip_path
        )));
    }

    let core_dir = get_instance_core_dir(instance_id);
    let venv_dir = get_instance_venv_dir(instance_id);

    // Extract zip (skip if code already exists)
    let main_py = core_dir.join("main.py");
    if main_py.exists() {
        log::info!(
            "Instance {} code already exists, skipping extraction",
            instance_id
        );
        emit_progress(
            app_handle,
            instance_id,
            "extract",
            "代码已存在，跳过解压",
            30,
        );
    } else {
        emit_progress(app_handle, instance_id, "extract", "正在解压代码...", 10);

        if core_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&core_dir) {
                log::warn!("Failed to remove old core directory {:?}: {}", core_dir, e);
            }
        }
        std::fs::create_dir_all(&core_dir)
            .map_err(|e| AppError::io(format!("Failed to create core dir: {}", e)))?;

        extract_zip(&zip_path, &core_dir)?;
        emit_progress(app_handle, instance_id, "extract", "代码解压完成", 30);
    }

    // Create venv
    emit_progress(app_handle, instance_id, "venv", "正在创建虚拟环境...", 40);
    create_venv(&venv_dir, &instance.version).await?;
    emit_progress(app_handle, instance_id, "venv", "虚拟环境创建完成", 50);

    // Install requirements
    emit_progress(app_handle, instance_id, "deps", "正在安装依赖...", 60);
    let venv_python = get_venv_python(&venv_dir);
    install_requirements(&venv_python, &core_dir).await?;
    emit_progress(app_handle, instance_id, "deps", "依赖安装完成", 90);

    // Note: "done" is emitted by start_instance after the instance is truly running

    Ok(())
}

/// Create a virtual environment using the appropriate Python for the version.
async fn create_venv(venv_dir: &Path, version: &str) -> Result<()> {
    let python_exe = get_python_for_version(version)?;

    if venv_dir.exists() {
        let venv_python = get_venv_python(venv_dir);
        if venv_python.exists() {
            return Ok(());
        }
        // Venv directory exists but Python executable is missing or corrupted, remove and recreate
        log::warn!(
            "Venv at {:?} is corrupted (python not found), recreating",
            venv_dir
        );
        if let Err(e) = std::fs::remove_dir_all(venv_dir) {
            return Err(AppError::python(format!(
                "Failed to remove corrupted venv: {}",
                e
            )));
        }
    }

    let output = Command::new(&python_exe)
        .args(["-m", "venv", venv_dir.to_str().unwrap_or("")])
        .output()
        .await
        .map_err(|e| AppError::python(format!("Failed to create venv: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::python(format!(
            "Failed to create venv: {}",
            stderr
        )));
    }

    Ok(())
}

/// Install requirements into an instance's venv.
async fn install_requirements(venv_python: &Path, core_path: &Path) -> Result<()> {
    let requirements_path = core_path.join("requirements.txt");

    if !requirements_path.exists() {
        return Ok(());
    }

    let mut args = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "-r".to_string(),
        requirements_path
            .to_str()
            .ok_or_else(|| AppError::io("requirements.txt path is not valid UTF-8"))?
            .to_string(),
    ];

    // Apply PyPI mirror if configured
    if let Ok(config) = load_config() {
        let mirror = config.pypi_mirror.as_str();
        if !mirror.is_empty() {
            args.push("-i".to_string());
            args.push(mirror.to_string());
        }
    }

    let output = Command::new(venv_python)
        .args(&args)
        .output()
        .await
        .map_err(|e| AppError::python(format!("Failed to install requirements: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::python(format!(
            "Failed to install requirements: {}",
            stderr
        )));
    }

    Ok(())
}
