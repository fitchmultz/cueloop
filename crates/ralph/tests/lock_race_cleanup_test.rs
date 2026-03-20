//! Purpose: thin integration-test hub for lock cleanup race coverage.
//!
//! Responsibilities:
//! - Re-export shared imports and the suite-local helper for lock cleanup race companions.
//! - Preserve the exact pre-split root-level test names required by existing cargo filtering and reporting.
//! - Delegate behavior bodies to focused companion modules grouped by cleanup scenario.
//!
//! Scope:
//! - Suite wiring, shared helper, and thin root-level test wrappers only.
//!
//! Usage:
//! - Companion modules use `use super::*;` to access shared imports, `test_support`, and `get_task_owner_files`.
//! - Run `cargo test --test lock_race_cleanup_test` to execute the full suite while keeping original test names stable.
//!
//! Invariants/Assumptions:
//! - Test names, assertions, concurrency semantics, constants, and coverage remain unchanged from the pre-split monolith.
//! - `get_task_owner_files` behavior remains byte-for-byte equivalent to the previous inline helper.
//! - Root-level wrappers are intentionally retained because stock Rust test naming would otherwise prefix module paths and break the unchanged-name requirement.

mod test_support;

#[path = "lock_race_cleanup_test/concurrent_orphan_cleanup.rs"]
mod concurrent_orphan_cleanup;
#[path = "lock_race_cleanup_test/force_cleanup.rs"]
mod force_cleanup;
#[path = "lock_race_cleanup_test/task_sidecar_cleanup.rs"]
mod task_sidecar_cleanup;

pub(crate) use ralph::lock;
pub(crate) use std::fs;
pub(crate) use std::sync::atomic::{AtomicUsize, Ordering};
pub(crate) use std::sync::{Arc, Barrier};
pub(crate) use std::thread;
pub(crate) use std::time::Duration;
pub(crate) use tempfile::TempDir;

pub(crate) fn get_task_owner_files(lock_dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    if !lock_dir.exists() {
        return vec![];
    }
    fs::read_dir(lock_dir)
        .expect("read lock dir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|name| name.starts_with("owner_task_"))
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect()
}

#[test]
fn test_concurrent_lock_cleanup_no_orphans() {
    concurrent_orphan_cleanup::test_concurrent_lock_cleanup_no_orphans();
}

#[test]
fn test_drop_retry_handles_race_condition() {
    concurrent_orphan_cleanup::test_drop_retry_handles_race_condition();
}

#[test]
fn test_force_cleanup_removes_orphaned_directory() {
    force_cleanup::test_force_cleanup_removes_orphaned_directory();
}

#[test]
fn test_shared_task_lock_cleanup() {
    task_sidecar_cleanup::test_shared_task_lock_cleanup();
}

#[test]
fn test_rapid_acquire_release_no_leak() {
    concurrent_orphan_cleanup::test_rapid_acquire_release_no_leak();
}

#[test]
fn test_multiple_task_sidecars_cleanup() {
    task_sidecar_cleanup::test_multiple_task_sidecars_cleanup();
}

#[test]
fn test_task_cleanup_with_other_files() {
    task_sidecar_cleanup::test_task_cleanup_with_other_files();
}

#[test]
fn test_rapid_task_lock_unique_names() {
    task_sidecar_cleanup::test_rapid_task_lock_unique_names();
}
