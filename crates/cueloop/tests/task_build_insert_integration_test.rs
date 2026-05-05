//! Integration coverage for task-builder queue insertion.
//!
//! Purpose:
//! - Verify `cueloop task build` lets builder agents use `cueloop task insert`.
//!
//! Responsibilities:
//! - Exercise the CLI path with a fake runner that invokes the real insert command.
//! - Guard against parent/child queue-lock contention regressions.
//!
//! Not handled here:
//! - Prompt quality or runner-provider behavior.
//!
//! Usage:
//! - Run through Cargo integration tests.
//!
//! Invariants/assumptions:
//! - The fake Codex runner can call the just-built `cueloop` binary through PATH.

use anyhow::{Context, Result};

mod test_support;

#[test]
fn task_build_runner_can_insert_tasks_atomically() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::cueloop_init(dir.path()).context("init cueloop repo")?;

    let runner_path = test_support::create_fake_runner(
        dir.path(),
        "codex",
        r#"#!/bin/sh
set -e
cat >/dev/null
mkdir -p .cueloop/cache
cat > .cueloop/cache/task-build-insert-request.json <<'JSON'
{
  "version": 1,
  "tasks": [
    {
      "key": "builder-insert",
      "title": "Created through task insert",
      "status": "todo",
      "priority": "medium",
      "tags": ["builder"],
      "scope": ["crates/cueloop/src/commands/task/build.rs"],
      "evidence": ["regression: task build runner uses task insert"],
      "plan": ["Insert the task atomically.", "Validate the queue."],
      "request": "Build one task through task insert"
    }
  ]
}
JSON
cueloop task insert --format json --input .cueloop/cache/task-build-insert-request.json >/dev/null
echo '{"type":"item.completed","item":{"type":"agent_message","text":"created task through insert"}}'
"#,
    )?;
    test_support::configure_runner(dir.path(), "codex", "gpt-5.3-codex", Some(&runner_path))?;

    let (status, stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &["task", "build", "Build one task through task insert"],
    );
    anyhow::ensure!(
        status.success(),
        "task build should allow nested task insert\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Task builder added 1 task(s):"),
        "expected task builder to report the inserted task\nstderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("Queue lock already held"),
        "nested task insert should not hit queue lock contention\nstderr:\n{stderr}"
    );

    let (validate_status, validate_stdout, validate_stderr) =
        test_support::run_in_dir(dir.path(), &["queue", "validate"]);
    anyhow::ensure!(
        validate_status.success(),
        "queue validate failed after task build\nstdout:\n{validate_stdout}\nstderr:\n{validate_stderr}"
    );

    let queue_path = dir.path().join(".cueloop/queue.jsonc");
    let queue: cueloop::contracts::QueueFile =
        serde_json::from_str(&std::fs::read_to_string(queue_path)?)?;
    assert_eq!(queue.tasks.len(), 1);
    assert_eq!(queue.tasks[0].id, "RQ-0001");
    assert_eq!(queue.tasks[0].title, "Created through task insert");
    Ok(())
}
