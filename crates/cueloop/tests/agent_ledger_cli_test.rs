//! Agent-ledger CLI integration coverage.
//!
//! Purpose:
//! - Verify `cueloop agent ...` supports durable task tracking without runner dispatch.
//!
//! Responsibilities:
//! - Exercise compact overview, claim, progress, handoff, completion, and validation commands.
//! - Prove agent ledger state is persisted in the canonical queue/done files.
//!
//! Non-scope:
//! - Runner execution, phase supervision, or macOS UI behavior.
//!
//! Invariants/assumptions:
//! - Tests use disposable initialized repositories and the public CLI binary.

use anyhow::Result;
use serde_json::Value;

#[path = "test_support.rs"]
mod test_support;

#[test]
fn agent_ledger_flow_tracks_claim_progress_evidence_handoff_and_completion() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::seed_cueloop_dir(dir.path())?;

    let insert = serde_json::json!({
        "version": 1,
        "tasks": [{
            "key": "agent-ledger",
            "title": "Agent ledger smoke task",
            "status": "todo",
            "priority": "medium",
            "plan": ["Claim", "Add evidence", "Complete"]
        }]
    });
    let insert_path = dir.path().join("insert.json");
    std::fs::write(&insert_path, serde_json::to_string_pretty(&insert)?)?;
    let (insert_status, insert_stdout, insert_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "machine",
            "task",
            "insert",
            "--input",
            insert_path.to_str().unwrap(),
        ],
    );
    assert!(
        insert_status.success(),
        "insert failed\nstdout:\n{insert_stdout}\nstderr:\n{insert_stderr}"
    );
    let inserted: Value = serde_json::from_str(&insert_stdout)?;
    let task_id = inserted["tasks"][0]["task"]["id"].as_str().unwrap();

    let (overview_status, overview_stdout, overview_stderr) =
        test_support::run_in_dir(dir.path(), &["agent", "overview", "--format", "json"]);
    assert!(
        overview_status.success(),
        "overview failed\nstdout:\n{overview_stdout}\nstderr:\n{overview_stderr}"
    );
    let overview: Value = serde_json::from_str(&overview_stdout)?;
    assert_eq!(overview["next_runnable_task_id"], task_id);

    let (claim_status, claim_stdout, claim_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent",
            "claim",
            task_id,
            "--owner",
            "pi-session-test",
            "--ttl-minutes",
            "60",
            "--format",
            "json",
        ],
    );
    assert!(
        claim_status.success(),
        "claim failed\nstdout:\n{claim_stdout}\nstderr:\n{claim_stderr}"
    );
    let claimed: Value = serde_json::from_str(&claim_stdout)?;
    assert_eq!(
        claimed["task"]["custom_fields"]["agent_claim_owner"],
        "pi-session-test"
    );

    let (start_status, start_stdout, start_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent",
            "start",
            task_id,
            "--note",
            "Started by existing agent",
            "--evidence",
            "Reproduced baseline",
            "--format",
            "json",
        ],
    );
    assert!(
        start_status.success(),
        "start failed\nstdout:\n{start_stdout}\nstderr:\n{start_stderr}"
    );

    let (note_status, note_stdout, note_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent",
            "note",
            task_id,
            "Implementation complete",
            "--format",
            "json",
        ],
    );
    assert!(
        note_status.success(),
        "note failed\nstdout:\n{note_stdout}\nstderr:\n{note_stderr}"
    );

    let (evidence_status, evidence_stdout, evidence_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent",
            "evidence",
            task_id,
            "cargo test passed",
            "--format",
            "json",
        ],
    );
    assert!(
        evidence_status.success(),
        "evidence failed\nstdout:\n{evidence_stdout}\nstderr:\n{evidence_stderr}"
    );

    let (handoff_status, handoff_stdout, handoff_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent",
            "handoff",
            task_id,
            "--next",
            "Complete after final gate",
            "--format",
            "json",
        ],
    );
    assert!(
        handoff_status.success(),
        "handoff failed\nstdout:\n{handoff_stdout}\nstderr:\n{handoff_stderr}"
    );
    let handoff: Value = serde_json::from_str(&handoff_stdout)?;
    assert_eq!(handoff["next_step"], "Complete after final gate");
    assert!(
        handoff["recent_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| {
                note.as_str()
                    .is_some_and(|note| note.contains("Implementation complete"))
            })
    );

    let (complete_status, complete_stdout, complete_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent",
            "complete",
            task_id,
            "--evidence",
            "make agent-ci passed",
            "--format",
            "json",
        ],
    );
    assert!(
        complete_status.success(),
        "complete failed\nstdout:\n{complete_stdout}\nstderr:\n{complete_stderr}"
    );
    let completed: Value = serde_json::from_str(&complete_stdout)?;
    assert_eq!(completed["archived"], true);
    assert_eq!(completed["task"]["status"], "done");
    assert!(
        completed["task"]["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item.as_str()
                    .is_some_and(|item| item == "make agent-ci passed")
            })
    );

    let (validate_status, validate_stdout, validate_stderr) =
        test_support::run_in_dir(dir.path(), &["agent", "validate", "--format", "json"]);
    assert!(
        validate_status.success(),
        "validate failed\nstdout:\n{validate_stdout}\nstderr:\n{validate_stderr}"
    );
    let validated: Value = serde_json::from_str(&validate_stdout)?;
    assert_eq!(validated["valid"], true);

    Ok(())
}

#[test]
fn agent_complete_requires_explicit_evidence() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::seed_cueloop_dir(dir.path())?;

    let (status, stdout, stderr) =
        test_support::run_in_dir(dir.path(), &["agent", "complete", "RQ-0001"]);
    assert!(!status.success(), "missing evidence should fail\n{stdout}");
    assert!(
        stderr.contains("required") || stderr.contains("--evidence"),
        "unexpected stderr: {stderr}"
    );
    Ok(())
}
