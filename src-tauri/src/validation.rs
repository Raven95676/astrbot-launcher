use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};
use crate::paths::{get_data_dir, get_version_zip_path, get_versions_dir};

pub fn validate_instance_id(instance_id: &str) -> Result<()> {
    if uuid::Uuid::parse_str(instance_id).is_err() {
        return Err(AppError::other("Invalid instance id"));
    }
    Ok(())
}

pub fn resolve_backup_path(backup_path: &str, require_exists: bool) -> Result<PathBuf> {
    let backups_dir = get_data_dir().join("backups");
    let backups_dir_canonical = ensure_and_canonicalize_dir(&backups_dir, "backups")?;

    let file_name = Path::new(backup_path)
        .file_name()
        .ok_or_else(|| AppError::backup("Invalid backup path"))?;

    let candidate = backups_dir.join(file_name);

    if !is_backup_filename(&candidate) {
        return Err(AppError::backup("Invalid backup filename"));
    }

    if !candidate.exists() {
        if require_exists {
            return Err(AppError::backup(""));
        }
        return Ok(candidate);
    }

    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|e| AppError::backup(format!("Failed to resolve backup path: {}", e)))?;

    if !canonical_candidate.starts_with(&backups_dir_canonical) {
        return Err(AppError::backup("Backup path is outside backups directory"));
    }

    Ok(canonical_candidate)
}

pub fn resolve_version_zip_path(version: &str) -> Result<PathBuf> {
    validate_version_tag(version)?;

    let versions_dir = get_versions_dir();
    let versions_dir_canonical = ensure_and_canonicalize_dir(&versions_dir, "versions")?;
    let zip_path = get_version_zip_path(version);

    let zip_name = zip_path
        .file_name()
        .ok_or_else(|| AppError::io("Invalid version zip path"))?;

    let zip_path = versions_dir.join(Path::new(zip_name));

    if zip_path.exists() {
        let canonical = zip_path
            .canonicalize()
            .map_err(|e| AppError::io(format!("Failed to resolve version zip path: {}", e)))?;
        if !canonical.starts_with(&versions_dir_canonical) {
            return Err(AppError::io(
                "Version zip path is outside versions directory",
            ));
        }
        return Ok(canonical);
    }

    if !zip_path.starts_with(&versions_dir) {
        return Err(AppError::io(
            "Version zip path is outside versions directory",
        ));
    }

    Ok(zip_path)
}

fn validate_version_tag(version: &str) -> Result<()> {
    let is_safe = !version.is_empty()
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'));

    if !is_safe {
        return Err(AppError::version_not_found(version));
    }

    Ok(())
}

fn ensure_and_canonicalize_dir(path: &Path, label: &str) -> Result<PathBuf> {
    fs::create_dir_all(path)
        .map_err(|e| AppError::io(format!("Failed to create {} dir: {}", label, e)))?;
    path.canonicalize()
        .map_err(|e| AppError::io(format!("Failed to resolve {} dir: {}", label, e)))
}

fn is_backup_filename(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase().ends_with(".tar.gz"))
        .unwrap_or(false)
}
