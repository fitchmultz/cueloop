//! Purpose: CueLoop temp-root, temp-file, and stale-cleanup helpers.
//!
//! Responsibilities:
//! - Resolve CueLoop's temp root directory.
//! - Create CueLoop-scoped temp directories and files.
//! - Remove stale temp entries by prefix and retention window.
//!
//! Scope:
//! - Temp-path creation and cleanup only; atomic writes and safeguard dump gating live elsewhere.
//!
//! Usage:
//! - Used by cleanup flows, runner prompts, plugin IO, issue publishing, and safeguard dump persistence.
//!
//! Invariants/Assumptions:
//! - CueLoop temp artifacts currently live under the legacy-compatible
//!   `std::env::temp_dir()/cueloop` namespace.
//! - Cleanup is prefix-based and best-effort on per-entry metadata or deletion failures.
//! - CueLoop-created temp files currently use the legacy-compatible `cueloop_` prefix so
//!   cleanup can discover them.

use crate::constants::paths::{CUELOOP_TEMP_DIR_NAME, CUELOOP_TEMP_PREFIX, LEGACY_PROMPT_PREFIX};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub fn cueloop_temp_root() -> PathBuf {
    std::env::temp_dir().join(CUELOOP_TEMP_DIR_NAME)
}

pub fn cleanup_stale_temp_entries(
    base: &Path,
    prefixes: &[&str],
    retention: Duration,
) -> Result<usize> {
    if !base.exists() {
        return Ok(0);
    }

    let now = SystemTime::now();
    let mut removed = 0usize;

    for entry in fs::read_dir(base).with_context(|| format!("read temp dir {}", base.display()))? {
        let entry = entry.with_context(|| format!("read temp dir entry in {}", base.display()))?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if !prefixes.iter().any(|prefix| name.starts_with(prefix)) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                log::warn!(
                    "unable to read temp metadata for {}: {}",
                    path.display(),
                    err
                );
                continue;
            }
        };

        let modified = match metadata.modified() {
            Ok(time) => time,
            Err(err) => {
                log::warn!(
                    "unable to read temp modified time for {}: {}",
                    path.display(),
                    err
                );
                continue;
            }
        };

        let age = match now.duration_since(modified) {
            Ok(age) => age,
            Err(_) => continue,
        };

        if age < retention {
            continue;
        }

        if metadata.is_dir() {
            if fs::remove_dir_all(&path).is_ok() {
                removed += 1;
            } else {
                log::warn!("failed to remove temp dir {}", path.display());
            }
        } else if fs::remove_file(&path).is_ok() {
            removed += 1;
        } else {
            log::warn!("failed to remove temp file {}", path.display());
        }
    }

    Ok(removed)
}

pub fn cleanup_stale_temp_dirs(base: &Path, retention: Duration) -> Result<usize> {
    cleanup_stale_temp_entries(base, &[CUELOOP_TEMP_PREFIX], retention)
}

pub fn cleanup_default_temp_dirs(retention: Duration) -> Result<usize> {
    let mut removed = 0usize;
    removed += cleanup_stale_temp_dirs(&cueloop_temp_root(), retention)?;
    removed +=
        cleanup_stale_temp_entries(&std::env::temp_dir(), &[LEGACY_PROMPT_PREFIX], retention)?;
    Ok(removed)
}

pub fn create_cueloop_temp_dir(label: &str) -> Result<tempfile::TempDir> {
    let base = cueloop_temp_root();
    fs::create_dir_all(&base).with_context(|| format!("create temp dir {}", base.display()))?;
    let prefix = format!(
        "{prefix}{label}_",
        prefix = CUELOOP_TEMP_PREFIX,
        label = label.trim()
    );
    let dir = tempfile::Builder::new()
        .prefix(&prefix)
        .tempdir_in(&base)
        .with_context(|| format!("create temp dir in {}", base.display()))?;
    Ok(dir)
}

/// Creates a NamedTempFile in the CueLoop temp directory with the managed cleanup prefix.
/// This ensures the file will be caught by cleanup_default_temp_dirs().
pub fn create_cueloop_temp_file(label: &str) -> Result<tempfile::NamedTempFile> {
    let base = cueloop_temp_root();
    fs::create_dir_all(&base).with_context(|| format!("create temp dir {}", base.display()))?;
    let prefix = format!(
        "{prefix}{label}_",
        prefix = CUELOOP_TEMP_PREFIX,
        label = label.trim()
    );
    tempfile::Builder::new()
        .prefix(&prefix)
        .suffix(".tmp")
        .tempfile_in(&base)
        .with_context(|| format!("create temp file in {}", base.display()))
}
