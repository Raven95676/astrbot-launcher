use std::io;
use std::path::{Component, Path, PathBuf};

use crate::error::{AppError, Result};

pub(super) fn normalize_entry_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}

fn first_segment(path: &str) -> Option<&str> {
    path.split('/').next()
}

/// Convert an archive entry path to a relative PathBuf, rejecting empty or traversal paths.
pub(crate) fn parse_entry_rel_path(raw: &str) -> Option<PathBuf> {
    let normalized = normalize_entry_path(raw);
    let first = first_segment(&normalized)?;
    if first.is_empty() || has_windows_drive_prefix(&normalized) {
        return None;
    }

    let mut relative = PathBuf::new();

    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            _ => relative.push(part),
        }
    }

    if relative.as_os_str().is_empty() {
        return None;
    }

    Some(relative)
}

/// Canonicalize the longest existing prefix of a path, appending any remaining components.
fn canonicalize_longest_prefix(path: &Path) -> Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::Normal(segment) => normalized.push(segment),
            Component::ParentDir => {
                if !normalized.pop() && !normalized.has_root() {
                    return Err(AppError::io(format!(
                        "failed to normalize path {path:?}: parent traversal escapes root",
                    )));
                }
            }
        }
    }

    let mut current = normalized.clone();
    let mut suffix_parts: Vec<std::ffi::OsString> = Vec::new();

    loop {
        match current.canonicalize() {
            Ok(canonical) => {
                let mut result = canonical;
                for part in suffix_parts.into_iter().rev() {
                    result.push(part);
                }
                return Ok(result);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => match current.file_name() {
                Some(name) => {
                    suffix_parts.push(name.to_owned());
                    if !current.pop() {
                        return Err(AppError::io(format!(
                            "failed to canonicalize path {normalized:?}: reached filesystem root",
                        )));
                    }
                }
                None => {
                    return Err(AppError::io(format!(
                        "failed to canonicalize path {normalized:?}: reached filesystem root",
                    )));
                }
            },
            Err(error) => {
                return Err(AppError::io(format!(
                    "failed to canonicalize path {current:?}: {error}",
                )));
            }
        }
    }
}

/// Verify that `path` resolves to a location within `base_dir`, returning the canonical path.
pub(super) fn resolve_within_dir(base_dir: &Path, path: &Path) -> Result<PathBuf> {
    let canonical_base = base_dir
        .canonicalize()
        .map_err(|e| AppError::io(format!("failed to canonicalize base dir: {e}")))?;
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        canonical_base.join(path)
    };
    let canonical_candidate = canonicalize_longest_prefix(&candidate)?;

    if !canonical_candidate.starts_with(&canonical_base) {
        return Err(AppError::io(
            "archive contains path escaping destination — not a legitimate archive",
        ));
    }

    Ok(canonical_candidate)
}

pub(super) fn has_windows_path_prefix(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::Prefix(_)))
        || path.to_str().map(has_windows_drive_prefix).unwrap_or(false)
}

pub(super) fn validate_rel_link_target(target: &Path, kind: &str) -> Result<()> {
    if target.as_os_str().is_empty() {
        return Err(AppError::io(format!("{kind} target path is empty")));
    }
    if target.is_absolute() {
        return Err(AppError::io(format!(
            "absolute {kind} targets are not allowed in archives",
        )));
    }
    if has_windows_path_prefix(target) {
        return Err(AppError::io(format!(
            "{kind} target uses unsupported Windows path prefix",
        )));
    }
    Ok(())
}

pub(super) fn scan_common_top_dir(
    path: &str,
    candidate: &mut Option<String>,
    saw_nested: &mut bool,
    valid: &mut bool,
) {
    if !*valid {
        return;
    }

    let normalized = normalize_entry_path(path);

    let Some(first) = first_segment(&normalized) else {
        *valid = false;
        return;
    };

    if first.is_empty() {
        *valid = false;
        return;
    }

    if normalized.contains('/') {
        *saw_nested = true;
    }

    match candidate.as_deref() {
        None => *candidate = Some(first.to_string()),
        Some(existing) if existing == first => {}
        Some(_) => *valid = false,
    }
}

pub(super) fn finalize_common_top_dir(
    candidate: Option<String>,
    saw_nested: bool,
    valid: bool,
) -> Option<String> {
    if valid && saw_nested {
        candidate
    } else {
        None
    }
}

pub(super) fn detect_common_top_dir<'a, I>(paths: I) -> Option<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut top_dir_candidate = None;
    let mut top_dir_saw_nested = false;
    let mut top_dir_valid = true;

    for path in paths {
        if let Some(relative) = parse_entry_rel_path(path) {
            if let Some(s) = relative.to_str() {
                scan_common_top_dir(
                    s,
                    &mut top_dir_candidate,
                    &mut top_dir_saw_nested,
                    &mut top_dir_valid,
                );
            }
        }
    }

    finalize_common_top_dir(top_dir_candidate, top_dir_saw_nested, top_dir_valid)
}

/// Strip the common top-level directory from a relative path, if present.
pub(super) fn strip_common_top_dir(relative: &Path, top_dir: Option<&str>) -> Option<PathBuf> {
    let Some(top) = top_dir else {
        return Some(relative.to_path_buf());
    };

    let stripped = relative.strip_prefix(top).ok()?;
    if stripped.as_os_str().is_empty() {
        None
    } else {
        Some(stripped.to_path_buf())
    }
}
