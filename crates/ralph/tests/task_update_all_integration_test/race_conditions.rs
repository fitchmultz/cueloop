//! Purpose: race-condition integration coverage for single-task `ralph task update` flows.
//!
//! Responsibilities:
//! - Verify behavior when a runner moves the target task into `done.jsonc` during update.
//! - Verify behavior when a runner removes the target task from the queue during update.
//! - Verify behavior when a runner moves the task to done with only terminal-field changes.
//!
//! Scope:
//! - End-to-end CLI coverage for queue mutation races observed during task update execution.
//!
//! Usage:
//! - Imported by `task_update_all_integration_test.rs`; relies on `use super::*;` for shared helpers.
//!
//! Invariants/assumptions callers must respect:
//! - Test names, scripts, and assertions intentionally preserve the pre-split suite behavior.
//! - Queue fixtures remain the exact one-task JSON used before the split.

use super::*;

#[test]
fn task_update_single_task_moved_to_done_during_update() -> Result<()> {
    let dir = temp_dir_outside_repo();

    let (status, stdout, stderr) =
        run_in_dir(dir.path(), &["init", "--force", "--non-interactive"]);
    anyhow::ensure!(
        status.success(),
        "ralph init failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    write_queue_with_one_task(dir.path())?;

    let script = r#"#!/bin/sh
cat >/dev/null
# Move task from queue.jsonc to done.jsonc
mv .cueloop/queue.jsonc .cueloop/queue.jsonc.bak
cat > .cueloop/queue.jsonc << 'QUEUEEOF'
{
  "version": 1,
  "tasks": []
}
QUEUEEOF
cat > .cueloop/done.jsonc << 'DONEEOF'
{
  "version": 1,
  "tasks": [
    {
      "id": "RQ-0001",
      "status": "done",
      "title": "First task - completed",
      "tags": ["test", "completed"],
      "scope": ["crates/ralph"],
      "evidence": ["integration test"],
      "plan": ["step one"],
      "notes": [],
      "request": "first request",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T12:00:00Z",
      "completed_at": "2026-01-18T12:00:00Z"
    }
  ]
}
DONEEOF
rm .cueloop/queue.jsonc.bak
exit 0
"#;
    let runner_path = create_fake_runner(dir.path(), "codex", script)?;
    configure_runner(dir.path(), "codex", "gpt-5.3-codex", Some(&runner_path))?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["task", "update", "RQ-0001"]);
    anyhow::ensure!(
        status.success(),
        "expected task update to succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    anyhow::ensure!(
        stderr.contains("moved to done.jsonc") || stdout.contains("moved to done.jsonc"),
        "expected 'moved to done.jsonc' message, got stdout:\n{stdout}\nstderr:\n{stderr}"
    );

    anyhow::ensure!(
        stderr.contains("Changed fields") || stdout.contains("Changed fields"),
        "expected 'Changed fields' message, got stdout:\n{stdout}\nstderr:\n{stderr}"
    );

    Ok(())
}

#[test]
fn task_update_single_task_removed_during_update() -> Result<()> {
    let dir = temp_dir_outside_repo();

    let (status, stdout, stderr) =
        run_in_dir(dir.path(), &["init", "--force", "--non-interactive"]);
    anyhow::ensure!(
        status.success(),
        "ralph init failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    write_queue_with_one_task(dir.path())?;

    let script = r#"#!/bin/sh
cat >/dev/null
# Remove task from queue.jsonc (empty queue)
cat > .cueloop/queue.jsonc << 'QUEUEEOF'
{
  "version": 1,
  "tasks": []
}
QUEUEEOF
exit 0
"#;
    let runner_path = create_fake_runner(dir.path(), "codex", script)?;
    configure_runner(dir.path(), "codex", "gpt-5.3-codex", Some(&runner_path))?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["task", "update", "RQ-0001"]);
    anyhow::ensure!(
        status.success(),
        "expected task update to succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    anyhow::ensure!(
        stderr.contains("removed during update") || stdout.contains("removed during update"),
        "expected 'removed during update' warning, got stdout:\n{stdout}\nstderr:\n{stderr}"
    );

    Ok(())
}

#[test]
fn task_update_single_task_moved_to_done_no_changes() -> Result<()> {
    let dir = temp_dir_outside_repo();

    let (status, stdout, stderr) =
        run_in_dir(dir.path(), &["init", "--force", "--non-interactive"]);
    anyhow::ensure!(
        status.success(),
        "ralph init failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    write_queue_with_one_task(dir.path())?;

    let script = r#"#!/bin/sh
cat >/dev/null
# Move task from queue.jsonc to done.jsonc without changes
mv .cueloop/queue.jsonc .cueloop/queue.jsonc.bak
cat > .cueloop/queue.jsonc << 'QUEUEEOF'
{
  "version": 1,
  "tasks": []
}
QUEUEEOF
cat > .cueloop/done.jsonc << 'DONEEOF'
{
  "version": 1,
  "tasks": [
    {
      "id": "RQ-0001",
      "status": "done",
      "title": "First task",
      "tags": ["test"],
      "scope": ["crates/ralph"],
      "evidence": ["integration test"],
      "plan": ["step one"],
      "notes": [],
      "request": "first request",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z",
      "completed_at": "2026-01-18T12:00:00Z"
    }
  ]
}
DONEEOF
rm .cueloop/queue.jsonc.bak
exit 0
"#;
    let runner_path = create_fake_runner(dir.path(), "codex", script)?;
    configure_runner(dir.path(), "codex", "gpt-5.3-codex", Some(&runner_path))?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["task", "update", "RQ-0001"]);
    anyhow::ensure!(
        status.success(),
        "expected task update to succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    anyhow::ensure!(
        stderr.contains("moved to done.jsonc") || stdout.contains("moved to done.jsonc"),
        "expected 'moved to done.jsonc' message, got stdout:\n{stdout}\nstderr:\n{stderr}"
    );

    anyhow::ensure!(
        stderr.contains("Changed fields") || stdout.contains("Changed fields"),
        "expected 'Changed fields' message, got stdout:\n{stdout}\nstderr:\n{stderr}"
    );

    Ok(())
}
