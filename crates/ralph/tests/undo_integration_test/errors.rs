//! Purpose: error-handling integration tests for `ralph undo`.
//!
//! Responsibilities:
//! - Verify helpful failure output when undo is requested without any available snapshots.
//!
//! Scope:
//! - Error-path behavior only.
//!
//! Usage:
//! - Run via the root `undo_integration_test` integration suite.
//!
//! Invariants/assumptions callers must respect:
//! - The test preserves the original non-zero exit expectation and stderr matching.
//! - No snapshot-creating mutation is performed before invoking `ralph undo`.

use super::*;

/// Test that `ralph undo` fails with a clear error when no snapshots exist.
#[test]
fn undo_no_snapshots_error() -> Result<()> {
    let dir = setup_undo_repo()?;

    let task = make_test_task("RQ-0001", "Test task", TaskStatus::Todo);
    write_queue(dir.path(), &[task])?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["undo"]);

    anyhow::ensure!(
        !status.success(),
        "undo should fail when no snapshots exist"
    );

    anyhow::ensure!(
        stderr.contains("No continuation checkpoints are available")
            || stderr.contains("No undo snapshots available")
            || stderr.contains("no continuation checkpoints")
            || stderr.contains("No snapshots"),
        "expected continuation checkpoint error, got stderr:\n{stderr}"
    );

    Ok(())
}
