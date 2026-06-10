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
fn agent_next_prefers_existing_doing_task() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::seed_cueloop_dir(dir.path())?;

    let insert = serde_json::json!({
        "version": 1,
        "tasks": [
            {
                "key": "todo",
                "title": "Later todo task",
                "status": "todo",
                "priority": "medium"
            },
            {
                "key": "doing",
                "title": "Continue doing task",
                "status": "doing",
                "priority": "medium"
            }
        ]
    });
    let insert_path = dir.path().join("agent-next-doing-insert.json");
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
    let doing_id = inserted["tasks"][1]["task"]["id"].as_str().unwrap();

    let (next_status, next_stdout, next_stderr) =
        test_support::run_in_dir(dir.path(), &["agent", "next", "--format", "json"]);
    assert!(
        next_status.success(),
        "agent next failed\nstdout:\n{next_stdout}\nstderr:\n{next_stderr}"
    );
    let next: Value = serde_json::from_str(&next_stdout)?;
    assert_eq!(next["task"]["id"], doing_id);

    let (overview_status, overview_stdout, overview_stderr) =
        test_support::run_in_dir(dir.path(), &["agent", "overview", "--format", "json"]);
    assert!(
        overview_status.success(),
        "agent overview failed\nstdout:\n{overview_stdout}\nstderr:\n{overview_stderr}"
    );
    let overview: Value = serde_json::from_str(&overview_stdout)?;
    assert_eq!(overview["next_runnable_task_id"], doing_id);

    Ok(())
}

#[test]
fn agent_claim_rejects_different_active_owner_without_force() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::seed_cueloop_dir(dir.path())?;

    let insert = serde_json::json!({
        "version": 1,
        "tasks": [{
            "key": "claim-conflict",
            "title": "Claim conflict smoke task",
            "status": "todo",
            "priority": "medium"
        }]
    });
    let insert_path = dir.path().join("claim-conflict-insert.json");
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

    let (first_status, first_stdout, first_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent", "claim", task_id, "--owner", "one", "--format", "json",
        ],
    );
    assert!(
        first_status.success(),
        "first claim failed\nstdout:\n{first_stdout}\nstderr:\n{first_stderr}"
    );

    let (conflict_status, conflict_stdout, conflict_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent", "claim", task_id, "--owner", "two", "--format", "json",
        ],
    );
    assert!(
        !conflict_status.success(),
        "second owner should conflict\nstdout:\n{conflict_stdout}"
    );
    assert!(
        conflict_stderr.contains("already claimed by 'one'"),
        "unexpected conflict stderr: {conflict_stderr}"
    );

    let (force_status, force_stdout, force_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent", "--force", "claim", task_id, "--owner", "two", "--format", "json",
        ],
    );
    assert!(
        force_status.success(),
        "force claim failed\nstdout:\n{force_stdout}\nstderr:\n{force_stderr}"
    );
    let forced: Value = serde_json::from_str(&force_stdout)?;
    assert_eq!(forced["task"]["custom_fields"]["agent_claim_owner"], "two");

    Ok(())
}

#[test]
fn agent_claim_without_ttl_clears_stale_expiration() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::seed_cueloop_dir(dir.path())?;

    let insert = serde_json::json!({
        "version": 1,
        "tasks": [{
            "key": "claim-expiration",
            "title": "Claim expiration smoke task",
            "status": "todo",
            "priority": "medium"
        }]
    });
    let insert_path = dir.path().join("claim-expiration-insert.json");
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

    let (first_status, first_stdout, first_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent",
            "claim",
            task_id,
            "--owner",
            "one",
            "--ttl-minutes",
            "60",
            "--format",
            "json",
        ],
    );
    assert!(
        first_status.success(),
        "first claim failed\nstdout:\n{first_stdout}\nstderr:\n{first_stderr}"
    );

    let (refresh_status, refresh_stdout, refresh_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent", "claim", task_id, "--owner", "one", "--format", "json",
        ],
    );
    assert!(
        refresh_status.success(),
        "same-owner refresh failed\nstdout:\n{refresh_stdout}\nstderr:\n{refresh_stderr}"
    );
    let refreshed: Value = serde_json::from_str(&refresh_stdout)?;
    assert_eq!(
        refreshed["task"]["custom_fields"]["agent_claim_owner"],
        "one"
    );
    assert!(
        refreshed["task"]["custom_fields"]["agent_claim_expires_at"].is_null(),
        "same-owner no-ttl refresh should clear expiration: {refresh_stdout}"
    );

    let (conflict_status, conflict_stdout, conflict_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent", "claim", task_id, "--owner", "two", "--format", "json",
        ],
    );
    assert!(
        !conflict_status.success(),
        "cleared expiration should leave active owner protected\nstdout:\n{conflict_stdout}"
    );
    assert!(
        conflict_stderr.contains("already claimed by 'one'"),
        "unexpected conflict stderr: {conflict_stderr}"
    );

    let (expired_status, expired_stdout, expired_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent",
            "--force",
            "claim",
            task_id,
            "--owner",
            "expired-owner",
            "--ttl-minutes",
            "0",
            "--format",
            "json",
        ],
    );
    assert!(
        expired_status.success(),
        "expired seed claim failed\nstdout:\n{expired_stdout}\nstderr:\n{expired_stderr}"
    );

    let (replace_status, replace_stdout, replace_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent", "claim", task_id, "--owner", "two", "--format", "json",
        ],
    );
    assert!(
        replace_status.success(),
        "expired claim replacement failed\nstdout:\n{replace_stdout}\nstderr:\n{replace_stderr}"
    );
    let replaced: Value = serde_json::from_str(&replace_stdout)?;
    assert_eq!(
        replaced["task"]["custom_fields"]["agent_claim_owner"],
        "two"
    );
    assert!(
        replaced["task"]["custom_fields"]["agent_claim_expires_at"].is_null(),
        "expired replacement without ttl should clear stale expiration: {replace_stdout}"
    );

    let (third_status, third_stdout, third_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "agent", "claim", task_id, "--owner", "three", "--format", "json",
        ],
    );
    assert!(
        !third_status.success(),
        "new owner should not replace ttl-less owner\nstdout:\n{third_stdout}"
    );
    assert!(
        third_stderr.contains("already claimed by 'two'"),
        "unexpected third-owner stderr: {third_stderr}"
    );

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
