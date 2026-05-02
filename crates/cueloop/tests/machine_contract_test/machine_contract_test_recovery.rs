//! Recovery and continuation contract coverage for `cueloop machine`.
//!
//! Purpose:
//! - Verify versioned recovery documents and undo/repair continuations for machine clients.
//!
//! Responsibilities:
//! - Assert queue validate, repair, and undo documents remain versioned and aligned.
//! - Seed mutation history needed for undo contract coverage.
//! - Keep recovery document regressions isolated from task and parallel execution tests.
//!
//! Non-scope:
//! - Parallel worker status behavior.
//! - System info and doctor report coverage.
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions callers must respect:
//! - Recovery assertions intentionally mirror the historical flat suite.
//! - Undo coverage depends on first creating and mutating a task through the public CLI.

use super::machine_contract_test_support::{run_in_dir, setup_cueloop_repo, write_json_file};
use anyhow::Result;
use cueloop::contracts::TaskPriority;
use serde_json::Value;

#[test]
fn machine_queue_recovery_documents_are_versioned() -> Result<()> {
    let dir = setup_cueloop_repo()?;

    let (validate_status, validate_stdout, validate_stderr) =
        run_in_dir(dir.path(), &["machine", "queue", "validate"]);
    assert!(
        validate_status.success(),
        "machine queue validate failed\nstdout:\n{validate_stdout}\nstderr:\n{validate_stderr}"
    );
    let validate_document: Value = serde_json::from_str(&validate_stdout)?;
    assert_eq!(validate_document["version"], 1);
    assert!(validate_document["continuation"]["headline"].is_string());

    let (repair_status, repair_stdout, repair_stderr) =
        run_in_dir(dir.path(), &["machine", "queue", "repair", "--dry-run"]);
    assert!(
        repair_status.success(),
        "machine queue repair failed\nstdout:\n{repair_stdout}\nstderr:\n{repair_stderr}"
    );
    let repair_document: Value = serde_json::from_str(&repair_stdout)?;
    assert_eq!(repair_document["version"], 1);
    assert_eq!(repair_document["dry_run"], true);
    assert_eq!(
        repair_document["blocking"],
        repair_document["continuation"]["blocking"]
    );
    assert!(repair_document["continuation"]["headline"].is_string());

    let create_request = serde_json::json!({
        "version": 1,
        "title": "Undo seed task",
        "description": null,
        "priority": TaskPriority::Medium.as_str(),
        "tags": [],
        "scope": [],
        "template": null,
        "target": null
    });
    let create_path = write_json_file(dir.path(), "undo-seed-create.json", &create_request)?;
    let (create_status, create_stdout, create_stderr) = run_in_dir(
        dir.path(),
        &[
            "machine",
            "task",
            "create",
            "--input",
            create_path.to_str().expect("utf-8 create request path"),
        ],
    );
    assert!(
        create_status.success(),
        "machine task create failed\nstdout:\n{create_stdout}\nstderr:\n{create_stderr}"
    );
    let created_document: Value = serde_json::from_str(&create_stdout)?;
    let task_id = created_document["task"]["id"]
        .as_str()
        .expect("created task id should be present")
        .to_string();

    let mutate_request = serde_json::json!({
        "version": 1,
        "atomic": true,
        "tasks": [{
            "task_id": task_id,
            "edits": [{ "field": "title", "value": "Changed title" }]
        }]
    });
    let mutate_path = write_json_file(dir.path(), "undo-seed-request.json", &mutate_request)?;
    let (mutate_status, mutate_stdout, mutate_stderr) = run_in_dir(
        dir.path(),
        &[
            "machine",
            "task",
            "mutate",
            "--input",
            mutate_path.to_str().expect("utf-8 mutate request path"),
        ],
    );
    assert!(
        mutate_status.success(),
        "machine task mutate failed\nstdout:\n{mutate_stdout}\nstderr:\n{mutate_stderr}"
    );

    let (undo_status, undo_stdout, undo_stderr) =
        run_in_dir(dir.path(), &["machine", "queue", "undo", "--dry-run"]);
    assert!(
        undo_status.success(),
        "machine queue undo failed\nstdout:\n{undo_stdout}\nstderr:\n{undo_stderr}"
    );
    let undo_document: Value = serde_json::from_str(&undo_stdout)?;
    assert_eq!(undo_document["version"], 1);
    assert_eq!(undo_document["dry_run"], true);
    assert_eq!(undo_document["restored"], false);
    assert_eq!(
        undo_document["blocking"],
        undo_document["continuation"]["blocking"]
    );
    assert!(undo_document["result"].is_object());
    assert!(undo_document["continuation"]["headline"].is_string());
    Ok(())
}
