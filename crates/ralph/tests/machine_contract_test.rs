//! Integration tests for the `ralph machine` contract surface.
//!
//! Responsibilities:
//! - Verify the app-facing machine commands return stable JSON documents.
//! - Exercise queue read, task create, and task mutate through the public machine API.
//! - Keep machine-surface regressions isolated from human CLI snapshot tests.
//!
//! Does not handle:
//! - Exhaustive coverage for every human CLI command.
//! - macOS app decoding behavior.
//!
//! Invariants/assumptions callers must respect:
//! - Tests execute against the built `ralph` binary from Cargo.
//! - Fixtures use disposable Ralph repos initialized through the public CLI.

mod test_support;

use anyhow::Result;
use serde_json::Value;
use tempfile::tempdir;
use test_support::{git_init, ralph_init, run_in_dir};

use ralph::contracts::{TaskPriority, TaskStatus};

#[test]
fn machine_queue_read_returns_versioned_snapshot() -> Result<()> {
    let dir = tempdir()?;
    git_init(dir.path())?;
    ralph_init(dir.path())?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["machine", "queue", "read"]);
    assert!(
        status.success(),
        "machine queue read failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let document: Value = serde_json::from_str(&stdout)?;
    assert_eq!(document["version"], 1);
    assert!(document["paths"]["queue_path"].is_string());
    assert!(document["active"]["tasks"].is_array());
    assert!(document["done"]["tasks"].is_array());
    Ok(())
}

#[test]
fn machine_task_create_and_mutate_round_trip() -> Result<()> {
    let dir = tempdir()?;
    git_init(dir.path())?;
    ralph_init(dir.path())?;

    let create_request = serde_json::json!({
        "version": 1,
        "title": "Machine-created task",
        "description": "Created through ralph machine task create",
        "priority": TaskPriority::High.as_str(),
        "tags": ["machine", "app"],
        "scope": ["crates/ralph"],
        "template": null,
        "target": null
    });
    let create_path = dir.path().join("create-request.json");
    std::fs::write(&create_path, serde_json::to_string_pretty(&create_request)?)?;

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

    let created: Value = serde_json::from_str(&create_stdout)?;
    let task_id = created["task"]["id"]
        .as_str()
        .expect("created task id should be present")
        .to_string();

    let mutate_request = serde_json::json!({
        "version": 1,
        "atomic": true,
        "tasks": [
            {
                "task_id": task_id,
                "edits": [
                    { "field": "status", "value": TaskStatus::Doing.as_str() },
                    { "field": "priority", "value": TaskPriority::Critical.as_str() }
                ]
            }
        ]
    });
    let mutate_path = dir.path().join("mutate-request.json");
    std::fs::write(&mutate_path, serde_json::to_string_pretty(&mutate_request)?)?;

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

    let mutate_document: Value = serde_json::from_str(&mutate_stdout)?;
    assert_eq!(mutate_document["version"], 2);
    assert_eq!(mutate_document["report"]["tasks"][0]["applied_edits"], 2);
    assert_eq!(mutate_document["blocking"], Value::Null);
    assert_eq!(
        mutate_document["continuation"]["headline"],
        "Task mutation has been applied."
    );

    let (read_status, read_stdout, read_stderr) =
        run_in_dir(dir.path(), &["machine", "queue", "read"]);
    assert!(
        read_status.success(),
        "machine queue read failed\nstdout:\n{read_stdout}\nstderr:\n{read_stderr}"
    );
    let read_document: Value = serde_json::from_str(&read_stdout)?;
    let tasks = read_document["active"]["tasks"]
        .as_array()
        .expect("queue read tasks array");
    let updated_task = tasks
        .iter()
        .find(|task| task["id"].as_str() == Some(&task_id))
        .expect("updated task should remain in queue");
    assert_eq!(updated_task["status"], TaskStatus::Doing.as_str());
    assert_eq!(updated_task["priority"], TaskPriority::Critical.as_str());

    Ok(())
}

#[test]
fn task_mutate_json_uses_shared_continuation_document() -> Result<()> {
    let dir = tempdir()?;
    git_init(dir.path())?;
    ralph_init(dir.path())?;

    let create_request = serde_json::json!({
        "version": 1,
        "title": "Human task mutation seed",
        "description": null,
        "priority": TaskPriority::Medium.as_str(),
        "tags": [],
        "scope": [],
        "template": null,
        "target": null
    });
    let create_path = dir.path().join("task-mutate-create.json");
    std::fs::write(&create_path, serde_json::to_string_pretty(&create_request)?)?;
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
            "edits": [{ "field": "title", "value": "Clarified human title" }]
        }]
    });
    let mutate_path = dir.path().join("task-mutate-request.json");
    std::fs::write(&mutate_path, serde_json::to_string_pretty(&mutate_request)?)?;

    let (status, stdout, stderr) = run_in_dir(
        dir.path(),
        &[
            "task",
            "mutate",
            "--dry-run",
            "--format",
            "json",
            "--input",
            mutate_path.to_str().expect("utf-8 mutate request path"),
        ],
    );
    assert!(
        status.success(),
        "task mutate failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let document: Value = serde_json::from_str(&stdout)?;
    assert_eq!(document["version"], 2);
    assert_eq!(document["blocking"], Value::Null);
    assert_eq!(document["report"]["tasks"][0]["applied_edits"], 1);
    assert_eq!(
        document["continuation"]["headline"],
        "Mutation continuation is ready."
    );
    assert_eq!(
        document["continuation"]["next_steps"][0]["command"],
        "ralph task mutate --input <PATH>"
    );

    Ok(())
}

#[test]
fn machine_queue_recovery_documents_are_versioned() -> Result<()> {
    let dir = tempdir()?;
    git_init(dir.path())?;
    ralph_init(dir.path())?;

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
    let create_path = dir.path().join("undo-seed-create.json");
    std::fs::write(&create_path, serde_json::to_string_pretty(&create_request)?)?;
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
    let mutate_path = dir.path().join("undo-seed-request.json");
    std::fs::write(&mutate_path, serde_json::to_string_pretty(&mutate_request)?)?;
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

#[test]
fn machine_system_info_reports_cli_version() -> Result<()> {
    let dir = tempdir()?;
    git_init(dir.path())?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["machine", "system", "info"]);
    assert!(
        status.success(),
        "machine system info failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let document: Value = serde_json::from_str(&stdout)?;
    assert_eq!(document["version"], 1);
    assert!(document["cli_version"].as_str().is_some());
    Ok(())
}

#[test]
fn machine_doctor_report_returns_versioned_blocking_document() -> Result<()> {
    let dir = tempdir()?;
    git_init(dir.path())?;
    ralph_init(dir.path())?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["machine", "doctor", "report"]);
    assert!(
        status.success(),
        "machine doctor report failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let document: Value = serde_json::from_str(&stdout)?;
    assert_eq!(document["version"], 2);
    assert!(document["blocking"].is_object());
    assert_eq!(document["blocking"], document["report"]["blocking"]);
    assert!(document["report"]["checks"].is_array());
    Ok(())
}
