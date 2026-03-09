//! Stale-lock policy.
//!
//! Responsibilities:
//! - Inspect existing lock directories for owner readability and stale ownership.
//! - Format actionable lock contention error messages.
//!
//! Not handled here:
//! - Lock directory cleanup or owner-file writes.
//! - PID liveness implementation details.
//!
//! Invariants/assumptions:
//! - A lock is stale only when owner metadata exists and the PID is definitively dead.

use super::owner::LockOwner;
use super::pid::pid_liveness;
use anyhow::Result;
use std::path::Path;

pub(crate) struct ExistingLock {
    pub(crate) owner: Option<LockOwner>,
    pub(crate) owner_unreadable: bool,
    pub(crate) is_stale: bool,
}

pub(crate) fn inspect_existing_lock(
    lock_dir: &Path,
    read_owner: impl FnOnce(&Path) -> Result<Option<LockOwner>>,
) -> ExistingLock {
    match read_owner(lock_dir) {
        Ok(owner) => {
            let is_stale = owner
                .as_ref()
                .is_some_and(|owner| pid_liveness(owner.pid).is_definitely_not_running());
            ExistingLock {
                owner,
                owner_unreadable: false,
                is_stale,
            }
        }
        Err(_) => ExistingLock {
            owner: None,
            owner_unreadable: true,
            is_stale: false,
        },
    }
}

pub(crate) fn format_lock_error(
    lock_dir: &Path,
    owner: Option<&LockOwner>,
    is_stale: bool,
    owner_unreadable: bool,
) -> String {
    let mut message = format!("Queue lock already held at: {}", lock_dir.display());
    if is_stale {
        message.push_str(" (STALE PID)");
    }
    if owner_unreadable {
        message.push_str(" (owner metadata unreadable)");
    }

    message.push_str("\n\nLock Holder:");
    if let Some(owner) = owner {
        message.push_str(&format!(
            "\n  PID: {}\n  Label: {}\n  Started At: {}\n  Command: {}",
            owner.pid, owner.label, owner.started_at, owner.command
        ));
    } else {
        message.push_str("\n  (owner metadata missing)");
    }

    message.push_str("\n\nSuggested Action:");
    if is_stale {
        message.push_str(&format!(
            "\n  The process that held this lock is no longer running.\n  Use --force to automatically clear it, or use the built-in unlock command (unsafe if another ralph is running):\n  ralph queue unlock\n  Or remove the directory manually:\n  rm -rf {}",
            lock_dir.display()
        ));
    } else {
        message.push_str(&format!(
            "\n  If you are sure no other ralph process is running, use the built-in unlock command:\n  ralph queue unlock\n  Or remove the lock directory manually:\n  rm -rf {}",
            lock_dir.display()
        ));
    }

    message
}
