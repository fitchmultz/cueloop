//! Integration coverage for the `cueloop agent` ledger command surface.
//!
//! Purpose:
//! - Verify agent-ledger command behavior at the CLI boundary.
//!
//! Responsibilities:
//! - Exercise task lookup and active-task mutations through the public agent CLI.
//! - Keep regression coverage separate from machine-contract tests.
//!
//! Non-scope:
//! - Runner execution, machine documents, or broad queue validation.
//!
//! Invariants/assumptions:
//! - Task ID lookup follows the canonical queue-operation contract: trim surrounding whitespace and preserve case.

mod test_support;

use anyhow::Result;
use cueloop::contracts::TaskStatus;
use serde_json::Value;
use test_support::{
    make_test_task, read_done, read_queue, run_in_dir, seed_git_repo_with_cueloop, write_queue,
};

#[test]
fn agent_show_and_mutations_accept_padded_task_ids() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    seed_git_repo_with_cueloop(dir.path())?;
    write_queue(
        dir.path(),
        &[make_test_task("CL-0001", "Agent task", TaskStatus::Todo)],
    )?;

    let (show_status, show_stdout, show_stderr) = run_in_dir(
        dir.path(),
        &["agent", "show", " CL-0001 ", "--format", "json"],
    );
    assert!(
        show_status.success(),
        "agent show failed\nstdout:\n{show_stdout}\nstderr:\n{show_stderr}"
    );
    let shown: Value = serde_json::from_str(&show_stdout)?;
    assert_eq!(shown["task_id"], "CL-0001");
    assert_eq!(shown["location"], "active");

    let (start_status, start_stdout, start_stderr) = run_in_dir(
        dir.path(),
        &[
            "agent",
            "start",
            " CL-0001 ",
            "--note",
            "Started through agent ledger",
            "--format",
            "json",
        ],
    );
    assert!(
        start_status.success(),
        "agent start failed\nstdout:\n{start_stdout}\nstderr:\n{start_stderr}"
    );
    let started: Value = serde_json::from_str(&start_stdout)?;
    assert_eq!(started["task_id"], "CL-0001");
    assert_eq!(started["task"]["status"], TaskStatus::Doing.as_str());
    assert_eq!(read_queue(dir.path())?.tasks[0].status, TaskStatus::Doing);

    let (complete_status, complete_stdout, complete_stderr) = run_in_dir(
        dir.path(),
        &[
            "agent",
            "complete",
            " CL-0001 ",
            "--evidence",
            "agent ledger integration passed",
            "--format",
            "json",
        ],
    );
    assert!(
        complete_status.success(),
        "agent complete failed\nstdout:\n{complete_stdout}\nstderr:\n{complete_stderr}"
    );
    let completed: Value = serde_json::from_str(&complete_stdout)?;
    assert_eq!(completed["task_id"], "CL-0001");
    assert_eq!(completed["archived"], true);
    assert!(read_queue(dir.path())?.tasks.is_empty());
    assert_eq!(read_done(dir.path())?.tasks[0].id, "CL-0001");
    Ok(())
}
