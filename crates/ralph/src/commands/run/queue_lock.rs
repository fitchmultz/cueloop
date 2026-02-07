//! Queue lock helpers for run command.
//!
//! Responsibilities:
//! - Clear stale queue locks when resuming a session.
//! - Detect queue lock contention errors that should not be retried.
//!
//! Not handled here:
//! - Lock acquisition logic (see `crate::queue`).
//! - Session management (see `crate::session`).
//!
//! Invariants/assumptions:
//! - `clear_stale_queue_lock_for_resume` uses `force=true` which only clears
//!   the lock if the owning PID is confirmed dead.
//! - Queue lock errors are non-retriable to prevent infinite loops.

use anyhow::Result;
use std::path::Path;

const QUEUE_LOCK_ALREADY_HELD_PREFIX: &str = "Queue lock already held at:";

/// Clear stale queue lock when resuming a session.
///
/// This helper is called during resume to preemptively clean up stale locks
/// left behind by a crashed or killed ralph process. It uses `force=true`
/// which only clears the lock if the owning PID is confirmed dead (see lock.rs).
///
/// Returns Ok(()) if no lock exists, lock was cleared, or lock is held by
/// a live process/unreadable metadata (those cases are handled later during
/// normal acquisition with a single actionable error).
pub fn clear_stale_queue_lock_for_resume(repo_root: &Path) -> Result<()> {
    let lock_dir = crate::lock::queue_lock_dir(repo_root);
    if !lock_dir.exists() {
        return Ok(());
    }

    // `force=true` only clears when the PID is confirmed stale (see lock.rs).
    // We acquire+drop immediately: this performs stale cleanup without holding the lock.
    let lock = match crate::queue::acquire_queue_lock(repo_root, "run loop resume", true) {
        Ok(lock) => lock,
        Err(err) => {
            // If the lock is held by a live process, or the owner metadata is missing/unreadable,
            // we cannot safely clear it here. Let normal acquisition report the actionable error.
            if is_queue_lock_already_held_error(&err) {
                return Ok(());
            }
            return Err(err);
        }
    };
    drop(lock);
    Ok(())
}

/// Check if an error is a "Queue lock already held" error.
///
/// This is used to detect lock contention errors that should not be retried
/// in the run loop, preventing the 50-failure abort loop.
pub fn is_queue_lock_already_held_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .to_string()
            .starts_with(QUEUE_LOCK_ALREADY_HELD_PREFIX)
    })
}
