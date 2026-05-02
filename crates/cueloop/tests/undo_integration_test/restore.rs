//! Purpose: restore-focused integration tests for `cueloop undo`.
//!
//! Responsibilities:
//! - Verify `cueloop undo` restores queue and done files atomically.
//! - Verify successful restore consumes the snapshot file.
//!
//! Scope:
//! - Restore-path undo behavior only.
//!
//! Usage:
//! - Run via the root `undo_integration_test` integration suite.
//!
//! Invariants/assumptions callers must respect:
//! - Tests keep the original queue/done fixture setup and CLI invocations unchanged.
//! - Snapshot file inspection remains filesystem-based, matching the original suite.

use super::*;

/// Test that `cueloop undo` atomically restores both queue.json and done.json.
#[test]
fn undo_restores_queue_after_task_done() -> Result<()> {
    let dir = setup_undo_repo()?;

    let task = make_test_task("RQ-0001", "Test task", TaskStatus::Todo);
    write_queue(dir.path(), &[task])?;
    write_done(dir.path(), &[])?;

    let initial_queue = read_queue(dir.path())?;
    let initial_done = read_done(dir.path())?;
    anyhow::ensure!(
        initial_queue.tasks.len() == 1,
        "expected 1 task in queue initially"
    );
    anyhow::ensure!(
        initial_done.tasks.is_empty(),
        "expected empty done initially"
    );

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["task", "done", "RQ-0001"]);
    anyhow::ensure!(status.success(), "task done failed\nstderr:\n{stderr}");

    let queue_after_done = read_queue(dir.path())?;
    let done_after_done = read_done(dir.path())?;
    anyhow::ensure!(
        queue_after_done.tasks.is_empty(),
        "expected empty queue after done"
    );
    anyhow::ensure!(
        done_after_done.tasks.len() == 1,
        "expected 1 task in done after done"
    );
    anyhow::ensure!(
        done_after_done.tasks[0].id == "RQ-0001",
        "expected RQ-0001 in done.json"
    );

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["undo"]);
    anyhow::ensure!(status.success(), "undo failed\nstderr:\n{stderr}");

    anyhow::ensure!(
        stdout.contains("Continuation has been restored"),
        "expected restore continuation message in output, got:\n{stdout}"
    );

    let restored_queue = read_queue(dir.path())?;
    let restored_done = read_done(dir.path())?;

    anyhow::ensure!(
        restored_queue.tasks.len() == 1,
        "expected 1 task in queue after restore, got {} tasks",
        restored_queue.tasks.len()
    );
    anyhow::ensure!(
        restored_queue.tasks[0].id == "RQ-0001",
        "expected RQ-0001 in queue after restore, got {:?}",
        restored_queue.tasks[0].id
    );
    anyhow::ensure!(
        restored_queue.tasks[0].status == TaskStatus::Todo,
        "expected status Todo after restore, got {:?}",
        restored_queue.tasks[0].status
    );

    anyhow::ensure!(
        restored_done.tasks.is_empty(),
        "expected empty done.json after restore, got {} tasks",
        restored_done.tasks.len()
    );

    Ok(())
}

/// Test that the snapshot file is deleted after successful restore.
#[test]
fn undo_removes_used_snapshot() -> Result<()> {
    let dir = setup_undo_repo()?;

    let task = make_test_task("RQ-0001", "Test task", TaskStatus::Todo);
    write_queue(dir.path(), &[task])?;
    write_done(dir.path(), &[])?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["task", "done", "RQ-0001"]);
    anyhow::ensure!(status.success(), "task done failed\nstderr:\n{stderr}");

    let undo_dir = dir.path().join(".ralph/cache/undo");
    anyhow::ensure!(
        undo_dir.exists(),
        "undo directory should exist after mutation"
    );

    let snapshots_before: Vec<_> = fs::read_dir(&undo_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();
    anyhow::ensure!(
        !snapshots_before.is_empty(),
        "expected at least one snapshot after mutation"
    );

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["undo"]);
    anyhow::ensure!(status.success(), "undo failed\nstderr:\n{stderr}");

    let snapshots_after: Vec<_> = fs::read_dir(&undo_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();

    anyhow::ensure!(
        snapshots_after.is_empty(),
        "expected no snapshots after undo, found {}",
        snapshots_after.len()
    );

    Ok(())
}
