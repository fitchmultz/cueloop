//! Purpose: concurrent and retry-focused lock cleanup coverage for `lock_race_cleanup_test`.
//!
//! Responsibilities:
//! - Verify concurrent acquisition/release does not leave orphaned lock directories.
//! - Preserve the drop-cleanup retry regression coverage.
//! - Validate repeated rapid acquire/release cycles do not leak directories.
//!
//! Scope:
//! - General lock cleanup under contention and repeated reuse only.
//!
//! Usage:
//! - Invoked by thin root-level wrappers in `lock_race_cleanup_test.rs`.
//! - Imports shared fixtures and types through `use super::*;`.
//!
//! Invariants/Assumptions:
//! - Thread counts, iteration counts, wait bounds, and assertions remain identical to the pre-split suite.
//! - No test names are declared here; root wrappers preserve the original top-level names.

use super::*;

pub(super) fn test_concurrent_lock_cleanup_no_orphans() {
    let dir = TempDir::new().expect("create temp dir");
    let lock_dir = dir.path().join("lock");

    const NUM_THREADS: usize = 10;
    const ITERATIONS_PER_THREAD: usize = 20;

    let success_count = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(NUM_THREADS));

    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|thread_id| {
            let lock_dir = lock_dir.clone();
            let success_count = success_count.clone();
            let barrier = barrier.clone();

            thread::spawn(move || {
                barrier.wait();

                for i in 0..ITERATIONS_PER_THREAD {
                    let label = format!("thread_{}_iter_{}", thread_id, i);

                    match lock::acquire_dir_lock(&lock_dir, &label, false) {
                        Ok(_lock) => {
                            thread::yield_now();
                            success_count.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(_) => {
                            thread::yield_now();
                        }
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("thread should not panic");
    }

    assert!(
        test_support::wait_until(Duration::from_secs(5), Duration::from_millis(25), || {
            !lock_dir.exists()
                || fs::read_dir(&lock_dir)
                    .map(|mut entries| entries.next().is_none())
                    .unwrap_or(true)
        }),
        "lock directory cleanup timed out"
    );

    if lock_dir.exists() {
        let entries: Vec<_> = fs::read_dir(&lock_dir)
            .expect("read lock dir")
            .filter_map(|e| e.ok())
            .collect();

        assert!(
            entries.is_empty(),
            "Lock directory should be empty after concurrent test, but found: {:?}",
            entries
        );

        let _ = fs::remove_dir(&lock_dir);
    }

    let total_successes = success_count.load(Ordering::SeqCst);
    assert!(
        total_successes > 0,
        "At least some lock acquisitions should have succeeded"
    );
}

pub(super) fn test_drop_retry_handles_race_condition() {
    let dir = TempDir::new().expect("create temp dir");
    let lock_dir = dir.path().join("lock");

    {
        let _lock = lock::acquire_dir_lock(&lock_dir, "first", false).unwrap();
        assert!(lock_dir.exists());
    }

    assert!(
        !lock_dir.exists(),
        "Lock directory should be removed after Drop"
    );
}

pub(super) fn test_rapid_acquire_release_no_leak() {
    let dir = TempDir::new().expect("create temp dir");
    let base_lock_dir = dir.path().join("locks");

    const CYCLES: usize = 50;

    for i in 0..CYCLES {
        let lock_dir = base_lock_dir.join(format!("lock_{}", i % 5));

        let lock = lock::acquire_dir_lock(&lock_dir, &format!("cycle_{}", i), false).unwrap();
        drop(lock);
    }

    assert!(
        test_support::wait_until(Duration::from_secs(5), Duration::from_millis(25), || {
            !base_lock_dir.exists()
                || fs::read_dir(&base_lock_dir)
                    .map(|entries| entries.filter_map(|e| e.ok()).all(|e| !e.path().is_dir()))
                    .unwrap_or(true)
        }),
        "lock directories were not cleaned up after rapid acquire/release cycles"
    );

    if base_lock_dir.exists() {
        let remaining: Vec<_> = fs::read_dir(&base_lock_dir)
            .expect("read base lock dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        assert!(
            remaining.is_empty(),
            "Expected no remaining lock directories, found: {:?}",
            remaining
        );
    }
}
