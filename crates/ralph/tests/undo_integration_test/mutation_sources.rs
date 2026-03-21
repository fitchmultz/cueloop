//! Purpose: mutation-source integration tests for `ralph undo` snapshot creation.
//!
//! Responsibilities:
//! - Verify non-`task done` mutations also create undo snapshots.
//! - Verify snapshots created by those mutations can be restored successfully.
//!
//! Scope:
//! - Snapshot creation coverage for `task reject` and `queue archive` only.
//!
//! Usage:
//! - Run via the root `undo_integration_test` integration suite.
//!
//! Invariants/assumptions callers must respect:
//! - Tests preserve the original CLI commands and post-undo assertions exactly.
//! - Queue/archive coverage remains separate from import-specific undo tests.

use super::*;

/// Test that `ralph task reject` also creates snapshots.
#[test]
fn undo_creates_snapshot_on_task_reject() -> Result<()> {
    let dir = setup_undo_repo()?;

    let task = make_test_task("RQ-0001", "Test task", TaskStatus::Todo);
    write_queue(dir.path(), &[task])?;
    write_done(dir.path(), &[])?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["task", "reject", "RQ-0001"]);
    anyhow::ensure!(status.success(), "task reject failed\nstderr:\n{stderr}");

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["undo", "--list"]);
    anyhow::ensure!(status.success(), "undo --list failed\nstderr:\n{stderr}");

    anyhow::ensure!(
        stdout.contains("Available continuation checkpoints"),
        "expected snapshot after task reject, got:\n{stdout}"
    );

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["undo"]);
    anyhow::ensure!(
        status.success(),
        "undo after reject failed\nstderr:\n{stderr}"
    );

    let queue = read_queue(dir.path())?;
    anyhow::ensure!(
        queue.tasks.len() == 1,
        "expected 1 task in queue after undo"
    );
    anyhow::ensure!(
        queue.tasks[0].status == TaskStatus::Todo,
        "expected status Todo after undo, got {:?}",
        queue.tasks[0].status
    );

    Ok(())
}

/// Test that `ralph queue archive` creates a snapshot.
#[test]
fn undo_creates_snapshot_on_queue_archive() -> Result<()> {
    let dir = setup_undo_repo()?;

    let mut task = make_test_task("RQ-0001", "Done task", TaskStatus::Done);
    task.completed_at = Some("2026-01-20T00:00:00Z".to_string());
    write_queue(dir.path(), &[task])?;
    write_done(dir.path(), &[])?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["queue", "archive"]);
    anyhow::ensure!(status.success(), "queue archive failed\nstderr:\n{stderr}");

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["undo", "--list"]);
    anyhow::ensure!(status.success(), "undo --list failed\nstderr:\n{stderr}");

    anyhow::ensure!(
        stdout.contains("Available continuation checkpoints"),
        "expected snapshot after queue archive, got:\n{stdout}"
    );

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["undo"]);
    anyhow::ensure!(
        status.success(),
        "undo after archive failed\nstderr:\n{stderr}"
    );

    let queue = read_queue(dir.path())?;
    let done = read_done(dir.path())?;
    anyhow::ensure!(
        queue.tasks.len() == 1,
        "expected 1 task in queue after undo archive"
    );
    anyhow::ensure!(
        done.tasks.is_empty(),
        "expected empty done.json after undo archive"
    );

    Ok(())
}
