//! Centralized path utilities for the application.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};

/// Get the root data directory for the application (~/.astrbot_launcher).
#[allow(clippy::expect_used)]
pub fn get_data_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".astrbot_launcher")
}

/// Get the path to the config file.
pub fn config_path() -> PathBuf {
    get_data_dir().join("config.toml")
}

/// Ensure all required data directories exist.
pub fn ensure_data_dirs() -> Result<()> {
    let base = get_data_dir();
    fs::create_dir_all(&base).map_err(|e| AppError::io(e.to_string()))?;

    let dirs = [
        base.join("python"),
        base.join("compat_python"),
        base.join("versions"),
        base.join("instances"),
        base.join("backups"),
    ];
    for dir in &dirs {
        fs::create_dir_all(dir).map_err(|e| AppError::io(e.to_string()))?;
    }
    Ok(())
}

/// Get the root directory for an instance.
pub fn get_instance_dir(instance_id: &str) -> PathBuf {
    get_data_dir().join("instances").join(instance_id)
}

/// Get the core directory for an instance.
pub fn get_instance_core_dir(instance_id: &str) -> PathBuf {
    get_instance_dir(instance_id).join("core")
}

/// Get the virtual environment directory for an instance.
pub fn get_instance_venv_dir(instance_id: &str) -> PathBuf {
    get_instance_dir(instance_id).join("venv")
}

/// Check if an instance is fully deployed
pub fn is_instance_deployed(instance_id: &str) -> bool {
    let core_dir = get_instance_core_dir(instance_id);
    let venv_dir = get_instance_venv_dir(instance_id);
    let venv_python = get_venv_python(&venv_dir);
    core_dir.join("main.py").exists() && venv_python.exists()
}

/// Get the versions directory.
pub fn get_versions_dir() -> PathBuf {
    get_data_dir().join("versions")
}

/// Get the zip file path for a specific version (e.g., versions/v4.14.8.zip).
pub fn get_version_zip_path(version: &str) -> PathBuf {
    get_versions_dir().join(format!("{}.zip", version))
}

/// Get the backups directory.
pub fn get_backups_dir() -> PathBuf {
    get_data_dir().join("backups")
}

/// Python 3.12 directory (main Python).
pub fn get_python_dir() -> PathBuf {
    get_data_dir().join("python")
}

/// Python 3.10 directory (compatibility Python).
pub fn get_compat_python_dir() -> PathBuf {
    get_data_dir().join("compat_python")
}

/// Get the path to the Python executable for a standalone Python directory.
pub fn get_python_exe_path(python_dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        python_dir.join("python.exe")
    }

    #[cfg(not(target_os = "windows"))]
    {
        python_dir.join("bin").join("python3")
    }
}

/// Get the Python executable path within a virtual environment.
pub fn get_venv_python(venv_dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        venv_dir.join("Scripts").join("python.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        venv_dir.join("bin").join("python")
    }
}
