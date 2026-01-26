//! Filesystem helpers for temp directories, atomic writes, and safeguard dumps.
//!
//! Responsibilities:
//! - Create and clean Ralph temp directories.
//! - Write files atomically and sync parent directories best-effort.
//! - Persist safeguard dumps for troubleshooting output.
//!
//! Not handled here:
//! - Directory locks or lock ownership metadata (see `crate::lock`).
//! - Cross-device file moves or distributed filesystem semantics.
//! - Retry/backoff behavior beyond the current best-effort operations.
//!
//! Invariants/assumptions:
//! - Callers provide valid paths; `write_atomic` requires a parent directory.
//! - Temp cleanup is best-effort and may skip entries on IO errors.

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const RALPH_TEMP_DIR_NAME: &str = "ralph";
const LEGACY_PROMPT_PREFIX: &str = "ralph_prompt_";
pub const RALPH_TEMP_PREFIX: &str = "ralph_";

pub fn ralph_temp_root() -> PathBuf {
    std::env::temp_dir().join(RALPH_TEMP_DIR_NAME)
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
    cleanup_stale_temp_entries(base, &[RALPH_TEMP_PREFIX], retention)
}

pub fn cleanup_default_temp_dirs(retention: Duration) -> Result<usize> {
    let mut removed = 0usize;
    removed += cleanup_stale_temp_dirs(&ralph_temp_root(), retention)?;
    removed +=
        cleanup_stale_temp_entries(&std::env::temp_dir(), &[LEGACY_PROMPT_PREFIX], retention)?;
    Ok(removed)
}

pub fn create_ralph_temp_dir(label: &str) -> Result<tempfile::TempDir> {
    let base = ralph_temp_root();
    fs::create_dir_all(&base).with_context(|| format!("create temp dir {}", base.display()))?;
    let prefix = format!(
        "{prefix}{label}_",
        prefix = RALPH_TEMP_PREFIX,
        label = label.trim()
    );
    let dir = tempfile::Builder::new()
        .prefix(&prefix)
        .tempdir_in(&base)
        .with_context(|| format!("create temp dir in {}", base.display()))?;
    Ok(dir)
}

pub fn safeguard_text_dump(label: &str, content: &str) -> Result<PathBuf> {
    let temp_dir = create_ralph_temp_dir(label)?;
    let output_path = temp_dir.path().join("output.txt");
    fs::write(&output_path, content)
        .with_context(|| format!("write safeguard dump to {}", output_path.display()))?;

    // Persist the temp dir so it's not deleted when the TempDir object is dropped.
    let dir_path = temp_dir.keep();
    Ok(dir_path.join("output.txt"))
}

pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    log::debug!("atomic write: {}", path.display());
    let dir = path
        .parent()
        .context("atomic write requires a parent directory")?;
    fs::create_dir_all(dir).with_context(|| format!("create directory {}", dir.display()))?;

    let mut tmp = tempfile::NamedTempFile::new_in(dir)
        .with_context(|| format!("create temp file in {}", dir.display()))?;
    tmp.write_all(contents).context("write temp file")?;
    tmp.flush().context("flush temp file")?;
    tmp.as_file().sync_all().context("sync temp file")?;

    tmp.persist(path)
        .map_err(|err| err.error)
        .with_context(|| format!("persist {}", path.display()))?;

    sync_dir_best_effort(dir);
    Ok(())
}

pub(crate) fn sync_dir_best_effort(dir: &Path) {
    #[cfg(unix)]
    {
        log::debug!("syncing directory: {}", dir.display());
        if let Ok(file) = fs::File::open(dir) {
            let _ = file.sync_all();
        }
    }

    #[cfg(not(unix))]
    {
        let _ = dir;
    }
}
