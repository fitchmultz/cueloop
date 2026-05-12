//! Task creation and mutation contract coverage for `cueloop machine`.
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

use super::machine_contract_test_support::{run_in_dir, setup_cueloop_repo, write_json_file};
use anyhow::Result;
use cueloop::contracts::{TaskPriority, TaskStatus};
use serde_json::Value;

#[test]
fn machine_task_create_and_mutate_round_trip() -> Result<()> {
    let dir = setup_cueloop_repo()?;

    let create_request = serde_json::json!({
        "version": 1,
        "title": "Machine-created task",
        "description": "Created through cueloop machine task create",
        "priority": TaskPriority::High.as_str(),
        "tags": ["machine", "app"],
        "scope": ["crates/cueloop"],
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
fn machine_task_insert_creates_full_tasks_atomically() -> Result<()> {
    let dir = setup_cueloop_repo()?;

    let insert_request = serde_json::json!({
        "version": 1,
        "tasks": [
            {
                "key": "alpha",
                "title": "Machine-inserted task",
                "description": "Created through cueloop machine task insert",
                "priority": TaskPriority::High.as_str(),
                "status": TaskStatus::Todo.as_str(),
                "tags": ["machine", "queue"],
                "scope": ["crates/cueloop"],
                "evidence": ["path: crates/cueloop :: machine task insert"],
                "plan": ["Insert atomically", "Validate queue"],
                "request": "scan: queue safety",
                "custom_fields": {"scan_agent": "scan-general"}
            }
        ]
    });
    let insert_path = write_json_file(dir.path(), "insert-request.json", &insert_request)?;

    let (status, stdout, stderr) = run_in_dir(
        dir.path(),
        &[
            "machine",
            "task",
            "insert",
            "--input",
            insert_path.to_str().expect("utf-8 insert request path"),
        ],
    );
    assert!(
        status.success(),
        "machine task insert failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let document: Value = serde_json::from_str(&stdout)?;
    assert_eq!(document["version"], 1);
    assert_eq!(document["created_count"], 1);
    assert_eq!(document["tasks"][0]["key"], "alpha");
    assert_eq!(document["tasks"][0]["task"]["id"], "RQ-0001");
    assert_eq!(
        document["tasks"][0]["task"]["custom_fields"]["scan_agent"],
        "scan-general"
    );

    let (read_status, read_stdout, read_stderr) =
        run_in_dir(dir.path(), &["machine", "queue", "read"]);
    assert!(
        read_status.success(),
        "machine queue read failed\nstdout:\n{read_stdout}\nstderr:\n{read_stderr}"
    );
    let read_document: Value = serde_json::from_str(&read_stdout)?;
    assert_eq!(read_document["active"]["tasks"][0]["id"], "RQ-0001");
    Ok(())
}

#[test]
fn machine_task_show_and_lifecycle_round_trip() -> Result<()> {
    let dir = setup_cueloop_repo()?;

    let insert_request = serde_json::json!({
        "version": 1,
        "tasks": [{
            "key": "alpha",
            "title": "Lifecycle task",
            "status": TaskStatus::Todo.as_str(),
            "priority": TaskPriority::Medium.as_str(),
            "plan": ["Start it", "Complete it"]
        }]
    });
    let insert_path = write_json_file(dir.path(), "lifecycle-insert.json", &insert_request)?;
    let (insert_status, insert_stdout, insert_stderr) = run_in_dir(
        dir.path(),
        &[
            "machine",
            "task",
            "insert",
            "--input",
            insert_path.to_str().expect("utf-8 insert request path"),
        ],
    );
    assert!(
        insert_status.success(),
        "machine task insert failed\nstdout:\n{insert_stdout}\nstderr:\n{insert_stderr}"
    );
    let inserted: Value = serde_json::from_str(&insert_stdout)?;
    let task_id = inserted["tasks"][0]["task"]["id"]
        .as_str()
        .expect("inserted task id")
        .to_string();

    let (show_status, show_stdout, show_stderr) =
        run_in_dir(dir.path(), &["machine", "task", "show", &task_id]);
    assert!(
        show_status.success(),
        "machine task show failed\nstdout:\n{show_stdout}\nstderr:\n{show_stderr}"
    );
    let shown: Value = serde_json::from_str(&show_stdout)?;
    assert_eq!(shown["version"], 1);
    assert_eq!(shown["location"], "active");
    assert_eq!(shown["task"]["title"], "Lifecycle task");

    let (start_status, start_stdout, start_stderr) = run_in_dir(
        dir.path(),
        &[
            "machine",
            "task",
            "start",
            &task_id,
            "--note",
            "Started by machine API",
        ],
    );
    assert!(
        start_status.success(),
        "machine task start failed\nstdout:\n{start_stdout}\nstderr:\n{start_stderr}"
    );
    let started: Value = serde_json::from_str(&start_stdout)?;
    assert_eq!(started["version"], 1);
    assert_eq!(started["status"], TaskStatus::Doing.as_str());
    assert_eq!(started["archived"], false);
    assert_eq!(started["task"]["status"], TaskStatus::Doing.as_str());

    let (done_status, done_stdout, done_stderr) = run_in_dir(
        dir.path(),
        &[
            "machine",
            "task",
            "done",
            &task_id,
            "--note",
            "Verified by machine API",
        ],
    );
    assert!(
        done_status.success(),
        "machine task done failed\nstdout:\n{done_stdout}\nstderr:\n{done_stderr}"
    );
    let done: Value = serde_json::from_str(&done_stdout)?;
    assert_eq!(done["status"], TaskStatus::Done.as_str());
    assert_eq!(done["archived"], true);
    assert_eq!(done["task"]["status"], TaskStatus::Done.as_str());

    let (show_done_status, show_done_stdout, show_done_stderr) =
        run_in_dir(dir.path(), &["machine", "task", "show", &task_id]);
    assert!(
        show_done_status.success(),
        "machine task show done failed\nstdout:\n{show_done_stdout}\nstderr:\n{show_done_stderr}"
    );
    let shown_done: Value = serde_json::from_str(&show_done_stdout)?;
    assert_eq!(shown_done["location"], "done");

    Ok(())
}

#[test]
fn machine_task_followups_apply_materializes_agent_proposal() -> Result<()> {
    let dir = setup_cueloop_repo()?;

    let insert_request = serde_json::json!({
        "version": 1,
        "tasks": [{
            "key": "source",
            "title": "Source task",
            "status": TaskStatus::Todo.as_str(),
            "priority": TaskPriority::Medium.as_str(),
            "request": "source request"
        }]
    });
    let insert_path = write_json_file(dir.path(), "followup-source-insert.json", &insert_request)?;
    let (insert_status, insert_stdout, insert_stderr) = run_in_dir(
        dir.path(),
        &[
            "machine",
            "task",
            "insert",
            "--input",
            insert_path.to_str().expect("utf-8 insert request path"),
        ],
    );
    assert!(
        insert_status.success(),
        "machine task insert failed\nstdout:\n{insert_stdout}\nstderr:\n{insert_stderr}"
    );
    let inserted: Value = serde_json::from_str(&insert_stdout)?;
    let source_id = inserted["tasks"][0]["task"]["id"]
        .as_str()
        .expect("source task id")
        .to_string();

    let proposal_dir = dir.path().join(".cueloop/cache/followups");
    std::fs::create_dir_all(&proposal_dir)?;
    let proposal_path = proposal_dir.join(format!("{source_id}.json"));
    let proposal = serde_json::json!({
        "version": 1,
        "source_task_id": source_id,
        "tasks": [{
            "key": "agent-followup",
            "title": "Agent follow-up task",
            "description": "Created from an agent follow-up proposal.",
            "priority": TaskPriority::Medium.as_str(),
            "tags": ["agent"],
            "scope": ["docs/guides/agent-usage.md"],
            "evidence": ["Agent found a durable follow-up."],
            "plan": ["Implement follow-up"],
            "depends_on_keys": [],
            "independence_rationale": "Separate durable work."
        }]
    });
    std::fs::write(&proposal_path, serde_json::to_string_pretty(&proposal)?)?;

    let (dry_status, dry_stdout, dry_stderr) = run_in_dir(
        dir.path(),
        &[
            "machine",
            "task",
            "followups",
            "apply",
            "--task",
            &source_id,
            "--dry-run",
        ],
    );
    assert!(
        dry_status.success(),
        "machine task followups dry-run failed\nstdout:\n{dry_stdout}\nstderr:\n{dry_stderr}"
    );
    let dry_doc: Value = serde_json::from_str(&dry_stdout)?;
    assert_eq!(dry_doc["version"], 1);
    assert_eq!(dry_doc["dry_run"], true);
    assert_eq!(
        dry_doc["report"]["created_tasks"][0]["key"],
        "agent-followup"
    );

    let (apply_status, apply_stdout, apply_stderr) = run_in_dir(
        dir.path(),
        &[
            "machine",
            "task",
            "followups",
            "apply",
            "--task",
            &source_id,
        ],
    );
    assert!(
        apply_status.success(),
        "machine task followups apply failed\nstdout:\n{apply_stdout}\nstderr:\n{apply_stderr}"
    );
    let apply_doc: Value = serde_json::from_str(&apply_stdout)?;
    assert_eq!(apply_doc["dry_run"], false);
    assert_eq!(
        apply_doc["report"]["created_tasks"][0]["title"],
        "Agent follow-up task"
    );
    assert!(
        !proposal_path.exists(),
        "applied default proposal should be removed"
    );

    Ok(())
}

#[test]
fn task_mutate_json_uses_shared_continuation_document() -> Result<()> {
    let dir = setup_cueloop_repo()?;

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
        "cueloop machine task mutate --input <PATH>"
    );

    Ok(())
}
