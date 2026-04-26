//! Run-surface contract coverage for `ralph machine`.
//!
//! Purpose:
//! - Verify successful machine run contracts that rely on canonical CLI task selection.
//!
//! Responsibilities:
//! - Assert no-ID `machine run one --resume` emits `run_started` without a task ID.
//! - Verify `task_selected` and the final summary expose the actual CLI-selected task.
//! - Keep run-selection assertions isolated from queue, recovery, and parallel suites.
//!
//! Non-scope:
//! - Runner output formatting details beyond the machine contract markers needed by RalphMac.
//! - Parallel execution or operator-recovery state coverage.
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions callers must respect:
//! - The fake runner completes deterministically and does not require network access.
//! - Queue fixtures intentionally make the CLI-selected task unambiguous.

use super::machine_contract_test_support::{
    configure_ci_gate, configure_runner, create_fake_runner, git_add_all_commit, run_in_dir,
    setup_ralph_repo,
};
use anyhow::{Context, Result};
use serde_json::Value;

#[test]
fn machine_run_one_without_id_reports_selected_task_via_events_and_summary() -> Result<()> {
    let dir = setup_ralph_repo()?;

    let queue = serde_json::json!({
        "version": 1,
        "tasks": [
            {
                "id": "RQ-0001",
                "status": "todo",
                "title": "Canonical next task",
                "priority": "high",
                "created_at": "2026-03-10T00:00:00Z",
                "updated_at": "2026-03-10T00:00:00Z"
            }
        ]
    });
    std::fs::write(
        dir.path().join(".ralph/queue.jsonc"),
        serde_json::to_string_pretty(&queue)?,
    )
    .context("write queue fixture")?;

    let runner_path = create_fake_runner(
        dir.path(),
        "codex",
        r#"#!/bin/sh
printf '{"type":"assistant","message":{"content":[{"type":"output_text","text":"runner output"}]}}\n'
"#,
    )?;
    configure_runner(dir.path(), "codex", "gpt-5.3-codex", Some(&runner_path))?;
    configure_ci_gate(dir.path(), None, Some(false))?;
    std::fs::write(dir.path().join("Makefile"), "ci:\n\t@echo CI passed\n")?;
    git_add_all_commit(dir.path(), "setup")?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["machine", "run", "one", "--resume"]);
    assert!(
        status.success(),
        "machine run one --resume failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let lines: Vec<Value> = stdout
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .context("parse machine run output")?;

    let run_started = lines.first().context("expected run_started event")?;
    assert_eq!(run_started["kind"], "run_started");
    assert!(run_started["task_id"].is_null());

    let task_selected = lines
        .iter()
        .find(|line| line["kind"] == "task_selected")
        .context("expected task_selected event")?;
    assert_eq!(task_selected["task_id"], "RQ-0001");

    let summary = lines.last().context("expected machine run summary")?;
    assert_eq!(summary["version"], 2);
    assert_eq!(summary["task_id"], "RQ-0001");

    Ok(())
}
