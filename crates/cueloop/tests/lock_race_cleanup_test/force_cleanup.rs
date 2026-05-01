//! Purpose: forced orphan-cleanup coverage for `lock_race_cleanup_test`.
//!
//! Responsibilities:
//! - Verify force acquisition removes orphaned lock directories that contain leftover files.
//!
//! Scope:
//! - Forced cleanup of stale/orphaned directories only.
//!
//! Usage:
//! - Invoked by the thin root-level wrapper in `lock_race_cleanup_test.rs`.
//! - Imports shared filesystem and lock helpers through `use super::*;`.
//!
//! Invariants/Assumptions:
//! - Fixture contents, force-acquisition semantics, and assertions remain unchanged from the pre-split suite.
//! - No test names are declared here; the root wrapper preserves the original top-level name.

use super::*;

pub(super) fn test_force_cleanup_removes_orphaned_directory() {
    let dir = TempDir::new().expect("create temp dir");
    let lock_dir = dir.path().join("lock");

    fs::create_dir_all(&lock_dir).unwrap();
    fs::write(
        lock_dir.join("owner"),
        "pid: 99999\nstarted_at: 2025-01-01T00:00:00Z\ncommand: test\nlabel: stale\n",
    )
    .unwrap();
    fs::write(lock_dir.join("extra_file.txt"), "orphaned content").unwrap();

    assert!(lock_dir.exists());

    let lock = lock::acquire_dir_lock(&lock_dir, "new_lock", true).unwrap();

    assert!(lock_dir.exists());
    let owner_content = fs::read_to_string(lock_dir.join("owner")).unwrap();
    assert!(owner_content.contains("label: new_lock"));

    drop(lock);
}
