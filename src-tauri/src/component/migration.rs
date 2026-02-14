use std::fs;

use crate::paths::{get_component_dir, get_data_dir};

/// Migrate legacy `python/` and `compat_python/` directories to the new
/// `components/python312` and `components/python310` layout.
///
/// Migration errors are logged but never crash the app.
pub fn migrate_legacy_python_dirs() {
    let data_dir = get_data_dir();

    migrate_dir(
        &data_dir.join("python"),
        &get_component_dir("python312"),
        "python/ -> components/python312",
    );

    migrate_dir(
        &data_dir.join("compat_python"),
        &get_component_dir("python310"),
        "compat_python/ -> components/python310",
    );
}

fn migrate_dir(src: &std::path::Path, dst: &std::path::Path, label: &str) {
    if !src.exists() {
        return;
    }

    if dst.exists() {
        // Destination already exists — remove the legacy source to clean up.
        log::info!("Migration {}: destination already exists, removing legacy dir", label);
        if let Err(e) = fs::remove_dir_all(src) {
            log::warn!("Migration {}: failed to remove legacy dir: {}", label, e);
        }
        return;
    }

    // Ensure parent of dst exists
    if let Some(parent) = dst.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            log::warn!("Migration {}: failed to create parent dir: {}", label, e);
            return;
        }
    }

    log::info!("Migration {}: renaming", label);
    if let Err(e) = fs::rename(src, dst) {
        log::warn!("Migration {}: rename failed: {}", label, e);
    }
}
