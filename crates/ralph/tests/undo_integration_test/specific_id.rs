//! Purpose: snapshot-selection integration tests for `ralph undo`.
//!
//! Responsibilities:
//! - Verify `ralph undo --id <id>` restores the requested snapshot rather than the latest by default.
//!
//! Scope:
//! - Specific snapshot ID restore behavior only.
//!
//! Usage:
//! - Run via the root `undo_integration_test` integration suite.
//!
//! Invariants/assumptions callers must respect:
//! - Snapshot IDs are parsed using the same `ID:` extraction logic as the original monolithic test.
//! - Test names, assertions, and CLI command coverage remain unchanged.

use super::*;

/// Test that `ralph undo --id <id>` restores the specified snapshot.
///
/// This test verifies that:
/// 1. First snapshot is created before RQ-0001 is marked done (capturing initial state)
/// 2. Second snapshot is created before RQ-0002 is marked done (capturing state with RQ-0001 done)
/// 3. Restoring from the second snapshot brings back the state where RQ-0001 is done and RQ-0002 is in queue
#[test]
fn undo_with_specific_id_restores_correct_snapshot() -> Result<()> {
    let dir = setup_undo_repo()?;

    let task1 = make_test_task("RQ-0001", "First task", TaskStatus::Todo);
    let task2 = make_test_task("RQ-0002", "Second task", TaskStatus::Todo);
    write_queue(dir.path(), &[task1, task2])?;
    write_done(dir.path(), &[])?;

    let (status, _, stderr) = run_in_dir(dir.path(), &["task", "done", "RQ-0001"]);
    anyhow::ensure!(
        status.success(),
        "task done RQ-0001 failed\nstderr:\n{stderr}"
    );

    let (status, _, stderr) = run_in_dir(dir.path(), &["task", "done", "RQ-0002"]);
    anyhow::ensure!(
        status.success(),
        "task done RQ-0002 failed\nstderr:\n{stderr}"
    );

    let queue_after_both = read_queue(dir.path())?;
    let done_after_both = read_done(dir.path())?;
    anyhow::ensure!(
        queue_after_both.tasks.is_empty(),
        "expected empty queue after both tasks done"
    );
    anyhow::ensure!(
        done_after_both.tasks.len() == 2,
        "expected 2 tasks in done after both done"
    );

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["undo", "--list"]);
    anyhow::ensure!(status.success(), "undo --list failed\nstderr:\n{stderr}");

    let snapshot_ids = snapshot_ids_from_list_output(&stdout);

    anyhow::ensure!(
        snapshot_ids.len() == 2,
        "expected 2 snapshots, found {}\noutput:\n{stdout}",
        snapshot_ids.len()
    );

    let second_snapshot_id = &snapshot_ids[0];

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["undo", "--id", second_snapshot_id]);
    anyhow::ensure!(
        status.success(),
        "undo --id {second_snapshot_id} failed\nstderr:\n{stderr}"
    );

    let restored_queue = read_queue(dir.path())?;
    let restored_done = read_done(dir.path())?;

    anyhow::ensure!(
        restored_queue.tasks.len() == 1,
        "expected 1 task in queue after restoring second snapshot, got {} tasks",
        restored_queue.tasks.len()
    );
    anyhow::ensure!(
        restored_queue.tasks[0].id == "RQ-0002",
        "expected RQ-0002 in queue after restore, got {:?}",
        restored_queue.tasks[0].id
    );

    anyhow::ensure!(
        restored_done.tasks.len() == 1,
        "expected 1 task in done after restoring second snapshot, got {} tasks",
        restored_done.tasks.len()
    );
    anyhow::ensure!(
        restored_done.tasks[0].id == "RQ-0001",
        "expected RQ-0001 in done after restore, got {:?}",
        restored_done.tasks[0].id
    );

    Ok(())
}
