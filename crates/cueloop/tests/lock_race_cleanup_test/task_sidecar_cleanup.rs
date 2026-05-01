//! Purpose: task-sidecar ownership and cleanup coverage for `lock_race_cleanup_test`.
//!
//! Responsibilities:
//! - Verify task sidecar owner files coexist with supervising locks.
//! - Preserve cleanup behavior for multiple task sidecars and unrelated files.
//! - Validate rapid task-sidecar creation keeps unique owner-file names.
//!
//! Scope:
//! - Task-lock sidecar behavior only; not general contention or forced orphan cleanup.
//!
//! Usage:
//! - Invoked by thin root-level wrappers in `lock_race_cleanup_test.rs`.
//! - Imports shared helpers and the `get_task_owner_files` helper through `use super::*;`.
//!
//! Invariants/Assumptions:
//! - Task owner-file counting and cleanup assertions remain unchanged from the pre-split suite.
//! - Supervisor/task ordering, waits, and cleanup semantics are preserved exactly.
//! - No test names are declared here; root wrappers preserve the original top-level names.

use super::*;

pub(super) fn test_shared_task_lock_cleanup() {
    let dir = TempDir::new().expect("create temp dir");
    let lock_dir = dir.path().join("lock");

    let supervisor_lock = lock::acquire_dir_lock(&lock_dir, "run one", false).unwrap();
    assert!(lock_dir.exists());

    let task_lock_dir = lock_dir.clone();
    let task_handle = thread::spawn(move || {
        let task_lock = lock::acquire_dir_lock(&task_lock_dir, "task", false).unwrap();
        let task_files: Vec<_> = get_task_owner_files(&task_lock_dir);
        assert_eq!(task_files.len(), 1, "Expected one task owner file");
        task_lock
    });

    let task_lock = task_handle.join().unwrap();

    assert!(lock_dir.exists());
    assert!(lock_dir.join("owner").exists());
    assert_eq!(get_task_owner_files(&lock_dir).len(), 1);

    drop(task_lock);

    assert!(lock_dir.exists());
    assert!(lock_dir.join("owner").exists());
    assert!(
        get_task_owner_files(&lock_dir).is_empty(),
        "Task owner files should be cleaned up"
    );

    drop(supervisor_lock);

    assert!(
        test_support::wait_until(Duration::from_secs(5), Duration::from_millis(25), || {
            !lock_dir.exists()
        }),
        "lock directory should be cleaned up after dropping supervisor lock"
    );
}

pub(super) fn test_multiple_task_sidecars_cleanup() {
    let dir = TempDir::new().expect("create temp dir");
    let lock_dir = dir.path().join("lock");

    let supervisor_lock = lock::acquire_dir_lock(&lock_dir, "run one", false).unwrap();
    assert!(lock_dir.join("owner").exists());

    let task_lock1 = lock::acquire_dir_lock(&lock_dir, "task", false).unwrap();
    let task_lock2 = lock::acquire_dir_lock(&lock_dir, "task", false).unwrap();
    let task_lock3 = lock::acquire_dir_lock(&lock_dir, "task", false).unwrap();

    assert!(lock_dir.join("owner").exists());
    let task_files = get_task_owner_files(&lock_dir);
    assert_eq!(task_files.len(), 3, "Expected three task owner files");

    drop(task_lock2);

    assert!(lock_dir.exists(), "Lock directory should still exist");
    assert!(
        lock_dir.join("owner").exists(),
        "Supervisor owner should still exist"
    );
    let remaining_files = get_task_owner_files(&lock_dir);
    assert_eq!(
        remaining_files.len(),
        2,
        "Expected two task owner files remaining, found: {:?}",
        remaining_files
    );

    drop(task_lock1);

    assert!(lock_dir.exists(), "Lock directory should still exist");
    assert!(
        lock_dir.join("owner").exists(),
        "Supervisor owner should still exist"
    );
    let remaining_files = get_task_owner_files(&lock_dir);
    assert_eq!(
        remaining_files.len(),
        1,
        "Expected one task owner file remaining, found: {:?}",
        remaining_files
    );

    drop(task_lock3);

    assert!(lock_dir.exists(), "Lock directory should still exist");
    assert!(
        lock_dir.join("owner").exists(),
        "Supervisor owner should still exist"
    );
    assert!(
        get_task_owner_files(&lock_dir).is_empty(),
        "All task owner files should be cleaned up"
    );

    drop(supervisor_lock);
    assert!(
        test_support::wait_until(Duration::from_secs(5), Duration::from_millis(25), || {
            !lock_dir.exists()
        }),
        "Lock directory should be removed"
    );
}

pub(super) fn test_task_cleanup_with_other_files() {
    let dir = TempDir::new().expect("create temp dir");
    let lock_dir = dir.path().join("lock");

    let supervisor_lock = lock::acquire_dir_lock(&lock_dir, "run loop", false).unwrap();

    let task_lock = lock::acquire_dir_lock(&lock_dir, "task", false).unwrap();

    fs::write(lock_dir.join("debug.log"), "some debug info").unwrap();

    assert!(lock_dir.join("owner").exists());
    assert_eq!(get_task_owner_files(&lock_dir).len(), 1);
    assert!(lock_dir.join("debug.log").exists());

    drop(task_lock);

    assert!(lock_dir.exists(), "Lock directory should still exist");
    assert!(
        lock_dir.join("owner").exists(),
        "Supervisor owner should still exist"
    );
    assert!(
        lock_dir.join("debug.log").exists(),
        "Extra file should still exist"
    );
    assert!(
        get_task_owner_files(&lock_dir).is_empty(),
        "Task owner file should be cleaned up"
    );

    drop(supervisor_lock);
    assert!(
        test_support::wait_until(Duration::from_secs(5), Duration::from_millis(25), || {
            !lock_dir.exists() || get_task_owner_files(&lock_dir).is_empty()
        }),
        "Task owner files should be cleaned up even if directory remains"
    );
    if lock_dir.exists() {
        let _ = fs::remove_dir_all(&lock_dir);
    }
}

pub(super) fn test_rapid_task_lock_unique_names() {
    let dir = TempDir::new().expect("create temp dir");
    let lock_dir = dir.path().join("lock");

    let supervisor_lock = lock::acquire_dir_lock(&lock_dir, "run one", false).unwrap();

    const LOCKS: usize = 10;
    let mut locks = Vec::with_capacity(LOCKS);

    for _ in 0..LOCKS {
        locks.push(lock::acquire_dir_lock(&lock_dir, "task", false).unwrap());
    }

    let task_files = get_task_owner_files(&lock_dir);
    assert_eq!(
        task_files.len(),
        LOCKS,
        "Expected {} unique task owner files, found: {:?}",
        LOCKS,
        task_files
    );

    drop(locks);

    assert!(
        get_task_owner_files(&lock_dir).is_empty(),
        "All task owner files should be cleaned up"
    );

    drop(supervisor_lock);
    assert!(
        test_support::wait_until(Duration::from_secs(5), Duration::from_millis(25), || {
            !lock_dir.exists()
        }),
        "Lock directory should be removed after supervisor drops"
    );
}
