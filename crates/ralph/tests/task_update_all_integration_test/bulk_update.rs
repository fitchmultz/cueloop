//! Purpose: bulk update integration coverage for `ralph task update` without a task ID.
//!
//! Responsibilities:
//! - Verify update-all success when multiple tasks exist in the queue.
//! - Verify the empty-queue failure path for update-all invocation.
//!
//! Scope:
//! - End-to-end CLI coverage for multi-task update-all and empty-queue error handling.
//!
//! Usage:
//! - Imported by `task_update_all_integration_test.rs`; relies on `use super::*;` for shared helpers.
//!
//! Invariants/assumptions callers must respect:
//! - Test bodies and assertions intentionally preserve the pre-split coverage exactly.
//! - Queue fixture setup continues to use suite-local helpers that write the original JSON fixtures.

use super::*;

#[test]
fn task_update_without_id_updates_all_tasks() -> Result<()> {
    let dir = temp_dir_outside_repo();

    let (status, stdout, stderr) =
        run_in_dir(dir.path(), &["init", "--force", "--non-interactive"]);
    anyhow::ensure!(
        status.success(),
        "ralph init failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    write_queue_with_two_tasks(dir.path())?;

    let script = "#!/bin/sh\ncat >/dev/null\necho run >> .ralph/runner_calls.txt\nexit 0\n";
    let runner_path = create_fake_runner(dir.path(), "codex", script)?;
    configure_runner(dir.path(), "codex", "gpt-5.3-codex", Some(&runner_path))?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["task", "update"]);
    anyhow::ensure!(
        status.success(),
        "expected task update to succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let calls_path = dir.path().join(".ralph/runner_calls.txt");
    let calls = std::fs::read_to_string(&calls_path).context("read runner calls")?;
    let call_count = calls.lines().count();
    anyhow::ensure!(
        call_count == 2,
        "expected runner to be invoked for each task, got {call_count}"
    );

    Ok(())
}

#[test]
fn task_update_without_id_fails_on_empty_queue() -> Result<()> {
    let dir = temp_dir_outside_repo();

    let (status, stdout, stderr) =
        run_in_dir(dir.path(), &["init", "--force", "--non-interactive"]);
    anyhow::ensure!(
        status.success(),
        "ralph init failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    write_empty_queue(dir.path())?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["task", "update"]);
    anyhow::ensure!(!status.success(), "expected task update to fail");
    anyhow::ensure!(
        stderr.contains("No tasks in queue to update"),
        "expected empty-queue error, got:\n{stderr}"
    );

    Ok(())
}
