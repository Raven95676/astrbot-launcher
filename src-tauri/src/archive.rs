//! Shared archive extraction utilities.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::error::{AppError, Result};

/// Normalize an archive entry path into safe components.
///
/// Returns `None` if the path contains traversal (`..`) or absolute components.
pub fn normalize_archive_components(raw_path: &str) -> Option<Vec<String>> {
    let normalized = raw_path.replace('\\', "/");
    let mut components = Vec::new();

    for part in normalized.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return None;
        }
        if Path::new(part).is_absolute() {
            return None;
        }
        components.push(part.to_string());
    }

    Some(components)
}

/// Build an output path from a base directory and path components.
///
/// Returns `None` if any component is absolute, includes a `..` parent-dir component,
/// or if the resolved path escapes `dest_dir`.
pub fn build_output_path(dest_dir: &Path, components: &[String]) -> Option<PathBuf> {
    let mut out_path = dest_dir.to_path_buf();
    for component in components {
        let component_path = Path::new(component);
        if component_path.is_absolute()
            || component_path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return None;
        }
        out_path.push(component);
    }
    if !out_path.starts_with(dest_dir) {
        return None;
    }
    Some(out_path)
}

/// Detect a common top-level directory shared by all archive entries.
///
/// Used to strip the root folder when extracting (e.g. `project-v1.0/src/...` -> `src/...`).
pub fn detect_common_top_dir(paths: &[Vec<String>]) -> Option<String> {
    let candidate = paths.first()?.first()?;

    if !paths.iter().all(|path| path.first() == Some(candidate)) {
        return None;
    }

    if paths.iter().any(|path| path.len() > 1) {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn resolve_output_path(raw_path: &str, dest_dir: &Path, top_dir: Option<&str>) -> Option<PathBuf> {
    let mut components = normalize_archive_components(raw_path)?;

    if let Some(top) = top_dir {
        if components.first().map(String::as_str) == Some(top) {
            components.remove(0);
        }
    }

    if components.is_empty() {
        return None;
    }

    build_output_path(dest_dir, &components)
}

fn create_output_dir(path: &Path) {
    if let Err(error) = fs::create_dir_all(path) {
        log::warn!("Failed to create directory {path:?}: {error}");
    }
}

#[cfg(unix)]
fn set_unix_permissions(path: &Path, mode: Option<u32>) {
    use std::os::unix::fs::PermissionsExt as _;

    if let Some(mode) = mode {
        if let Err(error) = fs::set_permissions(path, fs::Permissions::from_mode(mode)) {
            log::warn!("Failed to set permissions on {path:?}: {error}");
        }
    }
}

#[cfg(not(unix))]
fn set_unix_permissions(_path: &Path, _mode: Option<u32>) {}

/// Common logic for extracting an archive entry to the output path.
/// Creates parent directories if needed, writes the file content via `write_fn`,
/// and optionally sets Unix permissions via `set_perms_fn`.
fn extract_file_entry<W, P>(out_path: &Path, write_fn: W, set_perms_fn: P) -> Result<()>
where
    W: FnOnce(&mut fs::File) -> Result<()>,
    P: FnOnce(&Path),
{
    if let Some(parent) = out_path.parent() {
        create_output_dir(parent);
    }

    let mut outfile =
        fs::File::create(out_path).map_err(|error| AppError::io(error.to_string()))?;
    write_fn(&mut outfile)?;
    set_perms_fn(out_path);
    Ok(())
}

fn extract_entry<R>(
    out_path: &Path,
    is_dir: bool,
    reader: &mut R,
    unix_mode: Option<u32>,
) -> Result<()>
where
    R: io::Read,
{
    if is_dir {
        create_output_dir(out_path);
        return Ok(());
    }

    extract_file_entry(
        out_path,
        |outfile| {
            io::copy(reader, outfile).map_err(|error| AppError::io(error.to_string()))?;
            Ok(())
        },
        |path| set_unix_permissions(path, unix_mode),
    )
}

/// Append a directory recursively to a tar archive under the provided prefix.
pub fn append_dir_to_tar<W: io::Write>(
    builder: &mut tar::Builder<W>,
    dir: &Path,
    prefix: &str,
) -> Result<()> {
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(|error| AppError::io(error.to_string()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(dir)
            .map_err(|error| AppError::io(error.to_string()))?;
        let archive_path = format!("{prefix}/{}", relative.display());

        if path.is_file() {
            builder
                .append_path_with_name(path, &archive_path)
                .map_err(|error| AppError::io(error.to_string()))?;
        } else if path.is_dir() && path != dir {
            let mut header = tar::Header::new_gnu();
            header
                .set_path(&archive_path)
                .map_err(|error| AppError::io(error.to_string()))?;
            header.set_size(0);
            header.set_mode(0o755);
            header.set_entry_type(tar::EntryType::Directory);
            header.set_cksum();
            builder
                .append(&header, &[] as &[u8])
                .map_err(|error| AppError::io(error.to_string()))?;
        }
    }

    Ok(())
}

/// Create a tar.gz archive at `archive_path` and let the caller append entries.
pub fn create_tar_gz_archive<F>(archive_path: &Path, fill_entries: F) -> Result<()>
where
    F: FnOnce(&mut tar::Builder<flate2::write::GzEncoder<fs::File>>) -> Result<()>,
{
    let file = fs::File::create(archive_path).map_err(|error| AppError::io(error.to_string()))?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);

    fill_entries(&mut builder)?;

    builder
        .finish()
        .map_err(|error| AppError::io(error.to_string()))?;
    Ok(())
}

/// Extract tar.gz entries using a caller-provided destination resolver.
///
/// Returning `None` from `destination_for` skips the entry.
pub fn extract_tar_gz_with<F>(archive_path: &Path, mut destination_for: F) -> Result<()>
where
    F: FnMut(&[String]) -> Option<PathBuf>,
{
    let file = fs::File::open(archive_path).map_err(|error| AppError::io(error.to_string()))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|error| AppError::io(error.to_string()))?
    {
        let mut entry = entry.map_err(|error| AppError::io(error.to_string()))?;
        let entry_path = entry
            .path()
            .map_err(|error| AppError::io(error.to_string()))?;
        let entry_path_str = entry_path.as_ref().to_str().ok_or_else(|| {
            AppError::io(format!(
                "archive entry path is not valid UTF-8: {:?}",
                entry_path
            ))
        })?;

        let Some(components) = normalize_archive_components(entry_path_str)
            .filter(|components| !components.is_empty())
        else {
            log::warn!("Skipping unsafe archive path: {:?}", entry_path);
            continue;
        };

        let entry_type = entry.header().entry_type();
        if !entry_type.is_dir() && !entry_type.is_file() {
            log::warn!(
                "Skipping unsupported archive entry type at {:?}",
                entry_path
            );
            continue;
        }

        let Some(out_path) = destination_for(&components) else {
            continue;
        };

        let unix_mode = entry.header().mode().ok();
        extract_entry(&out_path, entry_type.is_dir(), &mut entry, unix_mode)?;
    }

    Ok(())
}

/// Extract tar.gz archive to dest_dir, stripping the top-level directory from the archive.
pub fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    // First pass: find the common top-level directory.
    let file = fs::File::open(archive_path).map_err(|error| AppError::io(error.to_string()))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    let mut all_paths = Vec::new();
    for entry in archive
        .entries()
        .map_err(|error| AppError::io(error.to_string()))?
    {
        let entry = entry.map_err(|error| AppError::io(error.to_string()))?;
        let entry_path = entry
            .path()
            .map_err(|error| AppError::io(error.to_string()))?;
        let entry_path_str = entry_path.as_ref().to_str().ok_or_else(|| {
            AppError::io(format!(
                "archive entry path is not valid UTF-8: {:?}",
                entry_path
            ))
        })?;
        if let Some(components) =
            normalize_archive_components(entry_path_str).filter(|components| !components.is_empty())
        {
            all_paths.push(components);
        }
    }

    let top_dir = detect_common_top_dir(&all_paths);

    extract_tar_gz_with(archive_path, |components| {
        let mut components = components.to_vec();

        if let Some(top) = top_dir.as_deref() {
            if components.first().map(String::as_str) == Some(top) {
                components.remove(0);
            }
        }

        if components.is_empty() {
            return None;
        }

        build_output_path(dest_dir, &components)
    })
}

/// Extract zip archive to dest_dir, stripping the top-level directory from the archive.
pub fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = fs::File::open(archive_path).map_err(|error| AppError::io(error.to_string()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| AppError::io(error.to_string()))?;

    // Find the common top-level directory.
    let all_paths: Vec<Vec<String>> = archive
        .file_names()
        .filter_map(|path| {
            normalize_archive_components(path).filter(|components| !components.is_empty())
        })
        .collect();
    let top_dir = detect_common_top_dir(&all_paths);

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| AppError::io(error.to_string()))?;

        let mangled = entry.mangled_name();
        let mangled_str = mangled.to_str().ok_or_else(|| {
            AppError::io(format!("zip entry path is not valid UTF-8: {mangled:?}"))
        })?;

        let Some(out_path) = resolve_output_path(mangled_str, dest_dir, top_dir.as_deref()) else {
            continue;
        };

        let is_dir = entry.is_dir();
        let unix_mode = entry.unix_mode();
        extract_entry(&out_path, is_dir, &mut entry, unix_mode)?;
    }

    Ok(())
}
