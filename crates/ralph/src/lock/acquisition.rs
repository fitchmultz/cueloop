//! Lock acquisition and shared-lock semantics.
//!
//! Purpose:
//! - Lock acquisition and shared-lock semantics.
//!
//! Responsibilities:
//! - Create lock directories and owner files.
//! - Apply stale-lock auto-cleanup and shared supervisor/task lock rules.
//! - Detect supervising-process ownership for callers that should avoid re-locking.
//!
//! Not handled here:
//! - PID liveness implementation details.
//! - Lock cleanup retries after drop.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - A `task` lock may coexist only with a supervising `owner` file.
//! - Task owner sidecars must be unique per acquisition attempt.

use super::{
    DirLock,
    owner::{
        LockOwner, OWNER_FILE_NAME, TASK_OWNER_PREFIX, command_line, is_supervising_label,
        parse_lock_owner, read_lock_owner, write_lock_owner,
    },
    stale::{format_lock_error, inspect_existing_lock},
};
use crate::timeutil;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TASK_OWNER_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn queue_lock_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".ralph").join("lock")
}

pub fn is_supervising_process(lock_dir: &Path) -> Result<bool> {
    let owner_path = lock_dir.join(OWNER_FILE_NAME);
    let raw = match fs::read_to_string(&owner_path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(anyhow!(err))
                .with_context(|| format!("read lock owner {}", owner_path.display()));
        }
    };

    let owner = match parse_lock_owner(&raw) {
        Some(owner) => owner,
        None => return Ok(false),
    };
    Ok(is_supervising_label(&owner.label))
}

pub fn acquire_dir_lock(lock_dir: &Path, label: &str, _force: bool) -> Result<DirLock> {
    log::debug!(
        "acquiring dir lock: {} (label: {})",
        lock_dir.display(),
        label
    );
    if let Some(parent) = lock_dir.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create lock parent {}", parent.display()))?;
    }

    let trimmed_label = label.trim();
    let is_task_label = trimmed_label == "task";

    match fs::create_dir(lock_dir) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = inspect_existing_lock(lock_dir, read_lock_owner);
            let owner_label = existing
                .owner
                .as_ref()
                .map(|owner| owner.label.clone())
                .unwrap_or_else(|| "none".to_string());
            let owner_pid = existing.owner.as_ref().map(|owner| owner.pid);
            let shared_task_mode_allowed = is_task_label
                && existing
                    .owner
                    .as_ref()
                    .is_some_and(|owner| is_supervising_label(&owner.label));

            // #region agent log
            append_debug_log(
                "H1",
                "crates/ralph/src/lock/acquisition.rs:acquire_dir_lock",
                "lock directory already exists during acquisition",
                serde_json::json!({
                    "lockDir": lock_dir.display().to_string(),
                    "requestedLabel": trimmed_label,
                    "existingOwnerLabel": owner_label,
                    "existingOwnerPid": owner_pid,
                    "existingIsStale": existing.is_stale,
                    "existingOwnerUnreadable": existing.owner_unreadable,
                    "sharedTaskModeAllowed": shared_task_mode_allowed,
                }),
            );
            // #endregion

            if existing.is_stale {
                match fs::remove_dir_all(lock_dir) {
                    Ok(()) => {
                        log::info!(
                            "Auto-cleared stale lock directory {} before acquiring {} lock",
                            lock_dir.display(),
                            label
                        );
                        return acquire_dir_lock(lock_dir, label, false);
                    }
                    Err(remove_error) => {
                        log::debug!(
                            "Failed to auto-clear stale lock directory {}: {}",
                            lock_dir.display(),
                            remove_error
                        );
                    }
                }
            }

            if !shared_task_mode_allowed {
                let formatted_error = format_lock_error(
                    lock_dir,
                    existing.owner.as_ref(),
                    existing.is_stale,
                    existing.owner_unreadable,
                    existing.staleness,
                );
                // #region agent log
                append_debug_log(
                    "H1",
                    "crates/ralph/src/lock/acquisition.rs:acquire_dir_lock",
                    "lock acquisition rejected because shared mode not permitted",
                    serde_json::json!({
                        "lockDir": lock_dir.display().to_string(),
                        "requestedLabel": trimmed_label,
                        "sharedTaskModeAllowed": false,
                        "error": formatted_error,
                    }),
                );
                // #endregion
                return Err(anyhow!(formatted_error));
            }
        }
        Err(error) => {
            return Err(anyhow!(error))
                .with_context(|| format!("create lock dir {}", lock_dir.display()));
        }
    }

    let effective_label = if trimmed_label.is_empty() {
        "unspecified"
    } else {
        trimmed_label
    };
    let owner = LockOwner {
        pid: std::process::id(),
        started_at: timeutil::now_utc_rfc3339()?,
        command: command_line(),
        label: effective_label.to_string(),
    };

    let owner_path = if is_task_label && lock_dir.exists() {
        let counter = TASK_OWNER_COUNTER.fetch_add(1, Ordering::SeqCst);
        lock_dir.join(format!(
            "{}{}_{}",
            TASK_OWNER_PREFIX,
            std::process::id(),
            counter
        ))
    } else {
        lock_dir.join(OWNER_FILE_NAME)
    };

    if let Err(error) = write_lock_owner(&owner_path, &owner) {
        if let Err(remove_error) = fs::remove_file(&owner_path) {
            log::debug!(
                "Failed to remove owner file {}: {}",
                owner_path.display(),
                remove_error
            );
        }
        if let Err(remove_error) = fs::remove_dir(lock_dir) {
            log::debug!(
                "Failed to remove lock directory {}: {}",
                lock_dir.display(),
                remove_error
            );
        }
        return Err(error);
    }

    Ok(DirLock {
        lock_dir: lock_dir.to_path_buf(),
        owner_path,
    })
}

fn append_debug_log(hypothesis_id: &str, location: &str, message: &str, data: serde_json::Value) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    let payload = serde_json::json!({
        "sessionId": "f05fb4",
        "runId": "pre-fix",
        "hypothesisId": hypothesis_id,
        "location": location,
        "message": message,
        "data": data,
        "timestamp": timestamp,
    });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/Users/mitchfultz/Projects/AI/ralph/.cursor/debug-f05fb4.log")
        && let Ok(line) = serde_json::to_string(&payload)
    {
        let _ = writeln!(file, "{line}");
    }
}
