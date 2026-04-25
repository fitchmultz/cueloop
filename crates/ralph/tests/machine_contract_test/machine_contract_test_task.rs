//! Task creation and mutation contract coverage for `ralph machine`.
//!
//! Purpose:
//! - Verify machine task create and mutate documents stay stable for app consumers.
//!
//! Responsibilities:
//! - Exercise machine task creation plus machine task mutation round trips.
//! - Assert JSON continuations shared between human and machine mutation surfaces.
//! - Confirm queue state reflects the applied task edits after contract calls.
//!
//! Non-scope:
//! - Queue recovery documents.
//! - Parallel runtime status documents.
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions callers must respect:
//! - Input payload shapes and assertions match the legacy suite exactly.
//! - Repo setup flows through the shared public CLI bootstrap helpers.

use super::machine_contract_test_support::{run_in_dir, setup_ralph_repo, write_json_file};
use anyhow::Result;
use ralph::contracts::{TaskPriority, TaskStatus};
use serde_json::Value;

#[test]
fn machine_task_create_and_mutate_round_trip() -> Result<()> {
    let dir = setup_ralph_repo()?;

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
    let create_path = write_json_file(dir.path(), "create-request.json", &create_request)?;

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
    let mutate_path = write_json_file(dir.path(), "mutate-request.json", &mutate_request)?;

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
    let dir = setup_ralph_repo()?;

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
    let create_path = write_json_file(dir.path(), "task-mutate-create.json", &create_request)?;
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
    let mutate_path = write_json_file(dir.path(), "task-mutate-request.json", &mutate_request)?;

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
        "ralph machine task mutate --input <PATH>"
    );

    Ok(())
}
