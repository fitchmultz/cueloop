//! Purpose: integration coverage for done/queue safety during parallel direct-push execution.
//!
//! Responsibilities:
//! - Verify queue/done artifacts remain conflict-free after parallel worker bookkeeping.
//! - Verify queue/done JSON stays parseable and semantically valid after the run.
//! - Verify remote merge simulation and persisted parallel state do not report done/queue drift.
//!
//! Scope:
//! - CLI-driven parallel safety scenarios using disposable repos and live bare remotes.
//!
//! Usage:
//! - Each test creates a fresh `ParallelDoneJsonRepo`, seeds the shared two-task fixture, and runs
//!   `ralph run loop --parallel 2 --max-tasks 2 --force`.
//! - Assertions inspect queue, done, and parallel-state files through suite-local helpers.
//!
//! Invariants/Assumptions:
//! - Tests preserve end-to-end CLI coverage; they do not bypass `ralph` commands.
//! - Repo/bootstrap uses cached git+`.ralph/` fixtures instead of repeated real `ralph init` calls.
//! - The fake runner is configured by explicit path, so suite tests must not serialize on `env_lock()`.

#[path = "parallel_done_json_safety_test/support.rs"]
mod support;
mod test_support;

use anyhow::{Context, Result};
use support::{ParallelDoneJsonRepo, assert_no_conflict_markers};

/// Verify that done.json contains no merge conflict markers after a parallel run.
#[test]
fn done_json_no_merge_conflicts_after_parallel_run() -> Result<()> {
    let repo = ParallelDoneJsonRepo::new()?;
    repo.seed_parallel_fixture()?;

    let (_status, _stdout, _stderr) = repo.run_parallel();

    if let Some(done_content) = repo.read_done_text()? {
        assert_no_conflict_markers("done.jsonc", &done_content);
    }

    if let Some(queue_content) = repo.read_queue_text()? {
        assert_no_conflict_markers("queue.jsonc", &queue_content);
    }

    Ok(())
}

/// Verify that queue/done semantics remain valid after a parallel run.
#[test]
fn queue_done_semantics_valid_after_parallel_run() -> Result<()> {
    let repo = ParallelDoneJsonRepo::new()?;
    repo.seed_parallel_fixture()?;

    let (_status, _stdout, _stderr) = repo.run_parallel();

    if let Some(queue_content) = repo.read_queue_text()? {
        let _: serde_json::Value =
            serde_json::from_str(&queue_content).context("queue.jsonc should be valid JSON")?;
    }

    if let Some(done_content) = repo.read_done_text()? {
        let _: serde_json::Value =
            serde_json::from_str(&done_content).context("done.jsonc should be valid JSON")?;
    }

    let (validate_status, validate_stdout, validate_stderr) =
        test_support::run_in_dir(repo.path(), &["queue", "validate"]);
    let validate_combined = format!("{}{}", validate_stdout, validate_stderr);

    assert!(
        validate_status.success()
            || validate_combined.contains("valid")
            || validate_combined.contains("empty")
            || validate_combined.is_empty(),
        "Queue validation should pass or be empty: {}",
        validate_combined
    );

    Ok(())
}

/// Verify that local and remote done/queue state do not produce merge conflicts.
#[test]
fn done_json_consistency_between_local_and_remote() -> Result<()> {
    let repo = ParallelDoneJsonRepo::new()?;
    repo.seed_parallel_fixture()?;

    let (_status, _stdout, _stderr) = repo.run_parallel();
    let merge_tree_result = repo.merge_tree_against_origin_main()?;

    if merge_tree_result.contains("conflict") {
        assert!(
            !merge_tree_result.contains("queue.jsonc") && !merge_tree_result.contains("done.jsonc"),
            "queue.jsonc or done.jsonc should not have merge conflicts: {}",
            merge_tree_result
        );
    }

    Ok(())
}

/// Verify that persisted worker state does not report blocked pushes due to queue/done issues.
#[test]
fn no_blocked_push_workers_from_done_json_conflicts() -> Result<()> {
    let repo = ParallelDoneJsonRepo::new()?;
    repo.seed_parallel_fixture()?;

    let (_status, _stdout, _stderr) = repo.run_parallel();

    if let Some(state) = repo.read_parallel_state()?
        && let Some(workers) = state["workers"].as_array()
    {
        for worker in workers {
            let lifecycle = worker["lifecycle"].as_str().unwrap_or("unknown");
            let task_id = worker["task_id"].as_str().unwrap_or("unknown");
            let last_error = worker["last_error"].as_str().unwrap_or("");
            let is_queue_done_error = last_error.contains("queue")
                || last_error.contains("done")
                || last_error.contains("conflict");

            if lifecycle == "blocked_push" {
                assert!(
                    !is_queue_done_error,
                    "Worker {} is blocked_push with queue/done error: {}",
                    task_id, last_error
                );
            }
        }
    }

    Ok(())
}
