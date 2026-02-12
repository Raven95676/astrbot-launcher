use std::fs::{self, File};
use std::io::Read as _;
use std::path::Path;

use flate2::read::GzDecoder;
use tar::Archive;

use crate::archive::{
    append_dir_to_tar, build_output_path, create_tar_gz_archive, extract_tar_gz_with,
};
use crate::config::{load_config, with_config_mut, BackupInfo, BackupMetadata, InstanceConfig};
use crate::error::{AppError, Result};
use crate::paths::{
    get_backups_dir, get_instance_core_dir, get_instance_dir, get_instance_venv_dir,
};
use crate::platform::get_arch_target;
use crate::validation::{resolve_backup_path, validate_instance_id};

/// Common backup creation logic.
fn create_backup_archive(
    instance: &InstanceConfig,
    instance_id: &str,
    include_venv: bool,
    filename_suffix: Option<&str>,
) -> Result<String> {
    let backups_dir = get_backups_dir();
    fs::create_dir_all(&backups_dir)
        .map_err(|e| AppError::backup(format!("Failed to create backups dir: {}", e)))?;

    let core_dir = get_instance_core_dir(instance_id);
    let data_dir = core_dir.join("data");
    let venv_dir = get_instance_venv_dir(instance_id);

    // Generate backup filename
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = match filename_suffix {
        Some(suffix) => format!("{}-{}-{}.tar.gz", instance_id, timestamp, suffix),
        None => format!("{}-{}.tar.gz", instance_id, timestamp),
    };
    let backup_path = backups_dir.join(&filename);

    // Create metadata
    let arch_target = get_arch_target()
        .map_err(|e| AppError::backup(format!("Failed to detect platform: {}", e)))?;
    let metadata = BackupMetadata {
        created_at: chrono::Utc::now().to_rfc3339(),
        instance_name: instance.name.clone(),
        instance_id: instance_id.to_string(),
        version: instance.version.clone(),
        includes_venv: include_venv,
        includes_data: true,
        arch_target: arch_target.to_string(),
    };

    create_tar_gz_archive(&backup_path, |builder| {
        // Write metadata as backup.toml
        let metadata_toml = toml::to_string_pretty(&metadata)
            .map_err(|e| AppError::backup(format!("Failed to serialize metadata: {}", e)))?;
        let metadata_bytes = metadata_toml.as_bytes();
        let mut header = tar::Header::new_gnu();
        header
            .set_path("backup.toml")
            .map_err(|e| AppError::backup(e.to_string()))?;
        header.set_size(metadata_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append(&header, metadata_bytes)
            .map_err(|e| AppError::backup(format!("Failed to add metadata: {}", e)))?;

        // Add data directory
        if data_dir.exists() {
            append_dir_to_tar(builder, &data_dir, "data")
                .map_err(|e| AppError::backup(format!("Failed to add data dir: {}", e)))?;
        }

        // Add venv directory if requested
        if include_venv {
            if !venv_dir.exists() {
                return Err(AppError::backup(""));
            }
            append_dir_to_tar(builder, &venv_dir, "venv")
                .map_err(|e| AppError::backup(format!("Failed to add venv dir: {}", e)))?;
        }

        Ok(())
    })
    .map_err(|e| AppError::backup(format!("Failed to create backup archive: {}", e)))?;

    Ok(backup_path
        .to_str()
        .ok_or_else(|| AppError::io("backup path is not valid UTF-8"))?
        .to_string())
}

/// Create a backup of an instance
pub fn create_backup(instance_id: &str, include_venv: bool) -> Result<String> {
    let config = load_config()?;
    let instance = config
        .instances
        .get(instance_id)
        .ok_or_else(|| AppError::instance_not_found(instance_id))?;

    create_backup_archive(instance, instance_id, include_venv, None)
}

/// Create an automatic backup for version upgrade/downgrade.
pub fn create_auto_backup(instance_id: &str, task: &str) -> Result<String> {
    let config = load_config()?;
    let instance = config
        .instances
        .get(instance_id)
        .ok_or_else(|| AppError::instance_not_found(instance_id))?;

    create_backup_archive(
        instance,
        instance_id,
        false,
        Some(&format!("auto-{}", task)),
    )
}

/// List all backups
pub fn list_backups() -> Result<Vec<BackupInfo>> {
    let backups_dir = get_backups_dir();
    if !backups_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(&backups_dir)
        .map_err(|e| AppError::backup(format!("Failed to read backups dir: {}", e)))?
    {
        let entry = entry.map_err(|e| AppError::backup(e.to_string()))?;
        let path = entry.path();

        if path.extension().map(|e| e == "gz").unwrap_or(false) {
            // Skip auto-backups created during upgrade/downgrade
            let fname = match path.file_name().and_then(|n| n.to_str()) {
                Some(s) => s.to_string(),
                None => {
                    log::warn!("Skipping backup with non-UTF-8 filename: {:?}", path);
                    continue;
                }
            };
            if fname.contains("-auto-") {
                continue;
            }

            let path_str = match path.to_str() {
                Some(s) => s.to_string(),
                None => {
                    log::warn!("Skipping backup with non-UTF-8 path: {:?}", path);
                    continue;
                }
            };

            if let Ok(metadata) = read_backup_metadata(&path) {
                backups.push(BackupInfo {
                    filename: fname,
                    path: path_str,
                    metadata,
                });
            }
        }
    }

    // Sort by created_at descending
    backups.sort_by(|a, b| b.metadata.created_at.cmp(&a.metadata.created_at));

    Ok(backups)
}

fn read_backup_metadata(backup_path: &Path) -> Result<BackupMetadata> {
    let file = File::open(backup_path)
        .map_err(|e| AppError::backup(format!("Failed to open backup: {}", e)))?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|e| AppError::backup(e.to_string()))?
    {
        let mut entry = entry.map_err(|e| AppError::backup(e.to_string()))?;
        let path = entry.path().map_err(|e| AppError::backup(e.to_string()))?;

        if path.to_str() == Some("backup.toml") {
            let mut content = String::new();
            entry
                .read_to_string(&mut content)
                .map_err(|e| AppError::backup(e.to_string()))?;
            return toml::from_str(&content)
                .map_err(|e| AppError::backup(format!("Failed to parse metadata: {}", e)));
        }
    }

    Err(AppError::backup(""))
}

/// Restore a backup to its original instance
pub fn restore_backup(backup_path: &str) -> Result<()> {
    let backup_path = resolve_backup_path(backup_path, true)?;

    // Read metadata
    let metadata = read_backup_metadata(&backup_path)?;

    // Check architecture compatibility for venv backups
    if metadata.includes_venv {
        let current_arch = get_arch_target()
            .map_err(|e| AppError::backup(format!("Failed to detect platform: {}", e)))?;
        if metadata.arch_target != current_arch {
            return Err(AppError::backup_arch_mismatch(
                &metadata.arch_target,
                current_arch,
            ));
        }
    }

    // Check if version is installed
    let config = load_config()?;
    if !config
        .installed_versions
        .iter()
        .any(|v| v.version == metadata.version)
    {
        return Err(AppError::version_not_found(&metadata.version));
    }

    // Validate original instance still exists
    let instance_id = &metadata.instance_id;
    if !config.instances.contains_key(instance_id) {
        return Err(AppError::instance_not_found(instance_id));
    }

    let instance_dir = get_instance_dir(instance_id);
    let core_dir = get_instance_core_dir(instance_id);

    // Extract backup to existing instance
    extract_backup_to_instance(&backup_path, &instance_dir, &core_dir)?;

    // Update instance version if different
    with_config_mut(|config| {
        if let Some(instance) = config.instances.get_mut(instance_id) {
            instance.version = metadata.version.clone();
        }
        Ok(())
    })?;

    Ok(())
}

/// Extract backup archive to instance directories.
fn extract_backup_to_instance(
    backup_path: &Path,
    instance_dir: &Path,
    core_dir: &Path,
) -> Result<()> {
    extract_tar_gz_with(backup_path, |components| {
        if components.len() == 1 && components[0] == "backup.toml" {
            return None;
        }

        if components.first().map(|component| component == "data") == Some(true) {
            build_output_path(core_dir, components)
        } else {
            build_output_path(instance_dir, components)
        }
    })
    .map_err(|e| AppError::backup(format!("Failed to extract backup: {}", e)))
}

/// Delete a backup
pub fn delete_backup(backup_path: &str) -> Result<()> {
    let path = resolve_backup_path(backup_path, false)?;
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| AppError::backup(format!("Failed to delete backup: {}", e)))?;
    }
    Ok(())
}

/// Restore only data from a backup to an existing instance.
pub fn restore_data_to_instance(backup_path: &str, instance_id: &str) -> Result<()> {
    validate_instance_id(instance_id)?;

    let backup_path = resolve_backup_path(backup_path, true)?;

    let core_dir = get_instance_core_dir(instance_id);

    extract_tar_gz_with(&backup_path, |components| {
        if components.first().map(|component| component == "data") != Some(true) {
            return None;
        }

        build_output_path(&core_dir, components)
    })
    .map_err(|e| AppError::backup(format!("Failed to restore backup data: {}", e)))
}
