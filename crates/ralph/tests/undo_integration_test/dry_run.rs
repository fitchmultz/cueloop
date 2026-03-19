//! Purpose: dry-run integration tests for `ralph undo`.
//!
//! Responsibilities:
//! - Verify `ralph undo --dry-run` previews changes without mutating queue files.
//!
//! Scope:
//! - Dry-run restore behavior only.
//!
//! Usage:
//! - Run via the root `undo_integration_test` integration suite.
//!
//! Invariants/assumptions callers must respect:
//! - The test preserves the original fixture flow and compares file contents before and after dry-run.
//! - CLI coverage remains end-to-end through shared `test_support` helpers.

use super::*;

/// Test that `ralph undo --dry-run` previews without modifying files.
#[test]
fn undo_dry_run_does_not_modify_files() -> Result<()> {
    let dir = setup_undo_repo()?;

    let task = make_test_task("RQ-0001", "Test task", TaskStatus::Todo);
    write_queue(dir.path(), &[task])?;
    write_done(dir.path(), &[])?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["task", "done", "RQ-0001"]);
    anyhow::ensure!(status.success(), "task done failed\nstderr:\n{stderr}");

    let queue_before = read_queue(dir.path())?;
    let done_before = read_done(dir.path())?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["undo", "--dry-run"]);
    anyhow::ensure!(status.success(), "undo --dry-run failed\nstderr:\n{stderr}");

    anyhow::ensure!(
        stdout.contains("Dry run") || stdout.contains("dry run"),
        "expected 'Dry run' in output, got:\n{stdout}"
    );

    let queue_after = read_queue(dir.path())?;
    let done_after = read_done(dir.path())?;

    anyhow::ensure!(
        queue_before.tasks.len() == queue_after.tasks.len(),
        "queue.json was modified during dry run"
    );
    anyhow::ensure!(
        done_before.tasks.len() == done_after.tasks.len(),
        "done.json was modified during dry run"
    );

    anyhow::ensure!(
        queue_after.tasks.is_empty(),
        "queue.json should still be empty after dry run"
    );
    anyhow::ensure!(
        done_after.tasks.len() == 1,
        "done.json should still have the task after dry run"
    );

    Ok(())
}
