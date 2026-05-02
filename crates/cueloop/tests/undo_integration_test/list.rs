//! Purpose: list-focused integration tests for `cueloop undo`.
//!
//! Responsibilities:
//! - Verify helpful output when no snapshots exist.
//! - Verify snapshots appear in `undo --list` output after a mutating operation.
//!
//! Scope:
//! - `cueloop undo --list` behavior only.
//!
//! Usage:
//! - Run via the root `undo_integration_test` integration suite.
//!
//! Invariants/assumptions callers must respect:
//! - Companion modules import shared helpers from the hub with `use super::*;`.
//! - Assertions and CLI coverage remain identical to the original monolithic test file.

use super::*;

/// Test that `cueloop undo --list` shows a helpful message when no snapshots exist.
#[test]
fn undo_list_empty_shows_helpful_message() -> Result<()> {
    let dir = setup_undo_repo()?;

    let task = make_test_task("RQ-0001", "Test task", TaskStatus::Todo);
    write_queue(dir.path(), &[task])?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["undo", "--list"]);
    anyhow::ensure!(
        status.success(),
        "undo --list should succeed even with no snapshots\nstderr:\n{stderr}"
    );

    anyhow::ensure!(
        stdout.contains("No continuation checkpoints are available"),
        "expected continuation checkpoint message, got:\n{stdout}"
    );

    anyhow::ensure!(
        stdout.contains("cueloop task mutate --dry-run")
            || stdout.contains("checkpoint")
            || stdout.contains("queue writes"),
        "expected helpful continuation guidance, got:\n{stdout}"
    );

    Ok(())
}

/// Test that `cueloop task done` creates a snapshot that appears in `--list` output.
#[test]
fn undo_list_shows_snapshots_after_task_done() -> Result<()> {
    let dir = setup_undo_repo()?;

    let task = make_test_task("RQ-0001", "Test task", TaskStatus::Todo);
    write_queue(dir.path(), &[task])?;
    write_done(dir.path(), &[])?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["task", "done", "RQ-0001"]);
    anyhow::ensure!(status.success(), "task done failed\nstderr:\n{stderr}");

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["undo", "--list"]);
    anyhow::ensure!(status.success(), "undo --list failed\nstderr:\n{stderr}");

    anyhow::ensure!(
        stdout.contains("Available continuation checkpoints"),
        "expected continuation checkpoint header, got:\n{stdout}"
    );

    anyhow::ensure!(
        stdout.contains("complete_task") || stdout.contains("RQ-0001"),
        "expected operation description in output, got:\n{stdout}"
    );

    Ok(())
}
