//! Directory lock helpers for queue/task coordination.
//!
//! Responsibilities:
//! - Provide directory-based locks for queue/task operations.
//! - Record lock ownership metadata (PID, timestamp, command, label).
//! - Detect supervising processes and stale lock holders.
//! - Support shared task locks when a supervising process owns the lock.
//!
//! Not handled here:
//! - Atomic writes or temp file cleanup (see `crate::fsutil`).
//! - Cross-machine locking or distributed coordination.
//! - Guaranteed PID liveness on non-Unix platforms.
//! - Lock timeouts/backoff beyond the current retry/force logic.
//!
//! Invariants/assumptions:
//! - Callers hold `DirLock` for the entire critical section.
//! - The lock directory path is stable for the resource being protected.
//! - The "task" label is reserved for shared lock semantics.
//! - Labels are informational and should be trimmed before evaluation.

use crate::fsutil::sync_dir_best_effort;
use crate::timeutil;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct DirLock {
    lock_dir: PathBuf,
    owner_path: PathBuf,
}

impl Drop for DirLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.owner_path);

        // Best-effort: remove the lock directory if it's empty.
        // - For standard locks, removing the owner file above should leave the directory empty.
        // - For shared "task" locks under supervision, the directory still contains the supervisor's
        //   `owner` file, so this removal fails and the supervisor cleans up when it exits.
        let _ = fs::remove_dir(&self.lock_dir);
    }
}

struct LockOwner {
    pid: u32,
    started_at: String,
    command: String,
    label: String,
}

impl LockOwner {
    fn render(&self) -> String {
        format!(
            "pid: {}\nstarted_at: {}\ncommand: {}\nlabel: {}\n",
            self.pid, self.started_at, self.command, self.label
        )
    }
}

pub fn queue_lock_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".ralph").join("lock")
}

fn is_supervising_label(label: &str) -> bool {
    matches!(label, "run one" | "run loop" | "tui")
}

/// Check if the queue lock is currently held by a supervising process
/// (run one or run loop), which means the caller is running under
/// ralph's supervision and should not attempt to acquire the lock.
pub fn is_supervising_process(lock_dir: &Path) -> Result<bool> {
    let owner_path = lock_dir.join("owner");

    let raw = match fs::read_to_string(&owner_path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(anyhow!(err))
                .with_context(|| format!("read lock owner {}", owner_path.display()))
        }
    };

    let owner = match parse_lock_owner(&raw) {
        Some(owner) => owner,
        None => return Ok(false),
    };

    Ok(is_supervising_label(&owner.label))
}

pub fn acquire_dir_lock(lock_dir: &Path, label: &str, force: bool) -> Result<DirLock> {
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
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            let mut owner_unreadable = false;
            let owner = match read_lock_owner(lock_dir) {
                Ok(owner) => owner,
                Err(_) => {
                    owner_unreadable = true;
                    None
                }
            };

            let is_stale = owner
                .as_ref()
                .is_some_and(|o| pid_is_running(o.pid) == Some(false));

            if force && is_stale {
                let _ = fs::remove_dir_all(lock_dir);
                // Retry once
                return acquire_dir_lock(lock_dir, label, false);
            }

            // Shared lock mode: "task" label can coexist with supervising lock
            if is_task_label
                && owner
                    .as_ref()
                    .is_some_and(|o| is_supervising_label(&o.label))
            {
                // Proceed to create sidecar owner file below
            } else {
                let msg = format_lock_error(lock_dir, owner.as_ref(), is_stale, owner_unreadable);
                return Err(anyhow!(msg));
            }
        }
        Err(err) => {
            return Err(anyhow!(err))
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

    // For "task" label in shared lock mode, create sidecar owner file
    let owner_path = if is_task_label && lock_dir.exists() {
        lock_dir.join(format!("owner_task_{}", std::process::id()))
    } else {
        lock_dir.join("owner")
    };

    if let Err(err) = write_lock_owner(&owner_path, &owner) {
        let _ = fs::remove_file(&owner_path);

        // Best-effort cleanup: if the lock directory is empty, remove it.
        // This prevents task lock attempts from leaving an empty `.ralph/lock` behind on errors.
        let _ = fs::remove_dir(lock_dir);

        return Err(err);
    }

    Ok(DirLock {
        lock_dir: lock_dir.to_path_buf(),
        owner_path,
    })
}

fn format_lock_error(
    lock_dir: &Path,
    owner: Option<&LockOwner>,
    is_stale: bool,
    owner_unreadable: bool,
) -> String {
    let mut msg = format!("Queue lock already held at: {}", lock_dir.display());
    if is_stale {
        msg.push_str(" (STALE PID)");
    }
    if owner_unreadable {
        msg.push_str(" (owner metadata unreadable)");
    }

    msg.push_str("\n\nLock Holder:");
    if let Some(owner) = owner {
        msg.push_str(&format!(
            "\n  PID: {}{}\n  Label: {}\n  Started At: {}\n  Command: {}",
            owner.pid,
            if is_stale { " (not running)" } else { "" },
            owner.label,
            owner.started_at,
            owner.command
        ));
    } else {
        msg.push_str("\n  (owner metadata missing)");
    }

    msg.push_str("\n\nSuggested Action:");
    if is_stale {
        msg.push_str(&format!(
            "\n  The process that held this lock is no longer running.\n  Use --force to automatically clear it, or remove the directory manually:\n  rm -rf {}",
            lock_dir.display()
        ));
    } else {
        msg.push_str(&format!(
            "\n  If you are sure no other ralph process is running, remove the lock directory:\n  rm -rf {}",
            lock_dir.display()
        ));
    }
    msg
}

fn write_lock_owner(owner_path: &Path, owner: &LockOwner) -> Result<()> {
    log::debug!("writing lock owner: {}", owner_path.display());
    let mut file = fs::File::create(owner_path)
        .with_context(|| format!("create lock owner {}", owner_path.display()))?;
    file.write_all(owner.render().as_bytes())
        .context("write lock owner")?;
    file.flush().context("flush lock owner")?;
    file.sync_all().context("sync lock owner")?;
    if let Some(parent) = owner_path.parent() {
        sync_dir_best_effort(parent);
    }
    Ok(())
}

fn read_lock_owner(lock_dir: &Path) -> Result<Option<LockOwner>> {
    let owner_path = lock_dir.join("owner");
    let raw = match fs::read_to_string(&owner_path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(anyhow!(err))
                .with_context(|| format!("read lock owner {}", owner_path.display()))
        }
    };
    Ok(parse_lock_owner(&raw))
}

fn parse_lock_owner(raw: &str) -> Option<LockOwner> {
    let mut pid = None;
    let mut started_at = None;
    let mut command = None;
    let mut label = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            let value = value.trim().to_string();
            match key.trim() {
                "pid" => pid = value.parse::<u32>().ok(),
                "started_at" => started_at = Some(value),
                "command" => command = Some(value),
                "label" => label = Some(value),
                _ => {}
            }
        }
    }

    let pid = pid?;
    Some(LockOwner {
        pid,
        started_at: started_at.unwrap_or_else(|| "unknown".to_string()),
        command: command.unwrap_or_else(|| "unknown".to_string()),
        label: label.unwrap_or_else(|| "unknown".to_string()),
    })
}

fn pid_is_running(pid: u32) -> Option<bool> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as i32, 0) };
        if result == 0 {
            return Some(true);
        }
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            return Some(false);
        }
        None
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
        None
    }
}

fn command_line() -> String {
    let args: Vec<String> = std::env::args().collect();
    let joined = args.join(" ");
    let trimmed = joined.trim();
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}
