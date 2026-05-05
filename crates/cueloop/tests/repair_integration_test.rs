//! Purpose: Exercise persisted `cueloop queue repair` behavior.
//!
//! Responsibilities:
//! - Verify CLI repair rewrites queue and done files safely.
//! - Cover regressions that require full on-disk repair and validation flows.
//!
//! Scope:
//! - Integration coverage for the `cueloop queue repair` command.
//! - Unit-level repair helper behavior belongs in `crates/cueloop/src/queue/repair.rs`.
//!
//! Usage:
//! - Run through Cargo integration tests for the `cueloop` crate.
//!
//! Invariants/Assumptions:
//! - Test workspaces are created outside the repository to avoid nested repo detection.
//! - Each scenario initializes its own CueLoop workspace before replacing fixtures.

use anyhow::{Context, Result};
use cueloop::config::project_runtime_dir;
use std::path::Path;
use std::process::ExitStatus;

mod test_support;

fn run_in_dir(dir: &Path, args: &[&str]) -> (ExitStatus, String, String) {
    test_support::run_in_dir(dir, args)
}

fn repair_report_from_stdout(stdout: &str) -> Result<serde_json::Value> {
    let report_and_next = stdout
        .split_once("Repair report:\n")
        .map(|(_, tail)| tail)
        .context("repair stdout should include a report section")?;
    let report_json = report_and_next
        .split_once("\n\nNext:")
        .map(|(report, _)| report)
        .unwrap_or(report_and_next)
        .trim();
    serde_json::from_str(report_json).context("repair report should be JSON")
}

#[test]
fn repair_queue_fixes_missing_fields_and_duplicates() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();

    let (status, stdout, stderr) =
        run_in_dir(dir.path(), &["init", "--force", "--non-interactive"]);
    anyhow::ensure!(
        status.success(),
        "cueloop init failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Create broken queue.json
    // - RQ-0001: Missing request, missing created_at/updated_at, empty title
    // - RQ-0001: Duplicate ID
    let broken_queue = r#"{ 
  "version": 1,
  "tasks": [
    {
      "id": "RQ-0001",
      "status": "todo",
      "title": "",
      "tags": [],
      "scope": [],
      "evidence": [],
      "plan": [],
      "notes": [],
      "depends_on": [],
      "custom_fields": {}
    },
    {
      "id": "RQ-0001",
      "status": "todo",
      "title": "Duplicate task",
      "tags": ["rust"],
      "scope": ["crates/cueloop"],
      "evidence": ["none"],
      "plan": ["none"],
      "request": "Some request",
      "created_at": "2026-01-18T00:00:00.000000000Z",
      "updated_at": "2026-01-18T00:00:00.000000000Z",
      "completed_at": null,
      "notes": [],
      "depends_on": [],
      "custom_fields": {}
    }
  ]
}"#;

    // Create broken done.json
    // - RQ-0002: Valid
    // - RQ-0001: Duplicate from queue
    let broken_done = r#"{ 
  "version": 1,
  "tasks": [
    {
      "id": "RQ-0002",
      "status": "done",
      "title": "Valid done task",
      "tags": [],
      "scope": [],
      "evidence": ["ok"],
      "plan": ["ok"],
      "request": "done",
      "created_at": "2026-01-18T00:00:00.000000000Z",
      "updated_at": "2026-01-18T00:00:00.000000000Z",
      "completed_at": "2026-01-18T00:00:00.000000000Z",
      "notes": [],
      "depends_on": [],
      "custom_fields": {}
    },
    {
      "id": "RQ-0001",
      "status": "done",
      "title": "Duplicate done task",
      "tags": [],
      "scope": [],
      "evidence": ["ok"],
      "plan": ["ok"],
      "request": "done",
      "created_at": "2026-01-18T00:00:00.000000000Z",
      "updated_at": "2026-01-18T00:00:00.000000000Z",
      "completed_at": "2026-01-18T00:00:00.000000000Z",
      "notes": [],
      "depends_on": [],
      "custom_fields": {}
    }
  ]
}"#;

    let runtime_dir = project_runtime_dir(dir.path());
    std::fs::write(runtime_dir.join("queue.jsonc"), broken_queue)?;
    std::fs::write(runtime_dir.join("done.jsonc"), broken_done)?;

    // Run repair
    let (status, stdout, stderr) = run_in_dir(dir.path(), &["queue", "repair"]);
    anyhow::ensure!(
        status.success(),
        "cueloop queue repair failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Queue repair now narrates continuation guidance on stdout.
    assert!(stdout.contains("Queue continuation has been normalized."));
    assert!(stdout.contains("cueloop machine queue validate"));
    let report = repair_report_from_stdout(&stdout)?;
    assert_eq!(report["fixed_tasks"].as_u64(), Some(3));
    assert_eq!(report["fixed_timestamps"].as_u64(), Some(2));
    assert_eq!(report["remapped_ids"].as_array().map(Vec::len), Some(2));
    assert_repair_undo_snapshot_created(dir.path())?;

    // Verify file content
    let runtime_dir = project_runtime_dir(dir.path());
    let queue_path = runtime_dir.join("queue.jsonc");
    let done_path = runtime_dir.join("done.jsonc");

    let queue_file = cueloop::queue::load_queue(&queue_path)?;
    let done_file = cueloop::queue::load_queue(&done_path)?;

    let queue_ids = queue_file
        .tasks
        .iter()
        .map(|task| task.id.as_str())
        .collect::<Vec<_>>();
    let done_ids = done_file
        .tasks
        .iter()
        .map(|task| task.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(queue_ids, vec!["RQ-0001", "RQ-0003"]);
    assert_eq!(done_ids, vec!["RQ-0002", "RQ-0004"]);

    let repaired_missing_fields = &queue_file.tasks[0];
    assert_eq!(repaired_missing_fields.title, "Untitled");
    assert_eq!(
        repaired_missing_fields.request.as_deref(),
        Some("Imported task")
    );
    assert!(repaired_missing_fields.created_at.is_some());
    assert!(repaired_missing_fields.updated_at.is_some());

    let repaired_duplicate = &queue_file.tasks[1];
    assert_eq!(repaired_duplicate.id, "RQ-0003");
    assert_eq!(repaired_duplicate.title, "Duplicate task");
    Ok(())
}

#[test]
fn repair_remaps_all_relationship_fields_for_invalid_ids() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();

    let (status, stdout, stderr) =
        run_in_dir(dir.path(), &["init", "--force", "--non-interactive"]);
    anyhow::ensure!(
        status.success(),
        "cueloop init failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Create broken queue.json:
    // - INVALID-1: Invalid ID format.
    // - RQ-0002: References INVALID-1 through every task-ID relationship field.
    let broken_queue = r#"{
  "version": 1,
  "tasks": [
    {
      "id": "INVALID-1",
      "status": "todo",
      "title": "Invalid ID task",
      "tags": ["test"],
      "scope": ["crates/cueloop"],
      "evidence": ["none"],
      "plan": ["none"],
      "request": "Test request",
      "created_at": "2026-01-18T00:00:00.000000000Z",
      "updated_at": "2026-01-18T00:00:00.000000000Z",
      "completed_at": null,
      "notes": [],
      "depends_on": [],
      "blocks": [],
      "relates_to": [],
      "custom_fields": {}
    },
    {
      "id": "RQ-0002",
      "status": "draft",
      "title": "Relationship task",
      "tags": ["test"],
      "scope": ["crates/cueloop"],
      "evidence": ["none"],
      "plan": ["none"],
      "request": "Test request",
      "created_at": "2026-01-18T00:00:00.000000000Z",
      "updated_at": "2026-01-18T00:00:00.000000000Z",
      "completed_at": null,
      "notes": [],
      "depends_on": ["INVALID-1"],
      "blocks": ["INVALID-1"],
      "relates_to": ["INVALID-1"],
      "duplicates": "INVALID-1",
      "custom_fields": {},
      "parent_id": "INVALID-1"
    }
  ]
}"#;

    std::fs::write(
        project_runtime_dir(dir.path()).join("queue.jsonc"),
        broken_queue,
    )?;

    // Run repair
    let (status, stdout, stderr) = run_in_dir(dir.path(), &["queue", "repair"]);
    anyhow::ensure!(
        status.success(),
        "cueloop queue repair failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let queue_str = std::fs::read_to_string(project_runtime_dir(dir.path()).join("queue.jsonc"))?;

    // Verify that INVALID-1 is gone and replaced by a valid generated ID.
    assert!(
        !queue_str.contains("INVALID-1"),
        "INVALID-1 should be remapped"
    );

    // Find the new ID for the first task
    let queue: serde_json::Value = serde_json::from_str(&queue_str)?;
    let tasks = queue["tasks"].as_array().expect("tasks array");

    let task1 = tasks
        .iter()
        .find(|t| t["title"] == "Invalid ID task")
        .expect("Task 1 found");
    let new_id = task1["id"].as_str().expect("id string");

    assert!(new_id.starts_with("RQ-"), "New ID should start with RQ-");

    // Verify the referencing task points to the remapped ID everywhere.
    let task2 = tasks
        .iter()
        .find(|t| t["title"] == "Relationship task")
        .expect("Task 2 found");
    assert_single_id(task2, "depends_on", new_id);
    assert_single_id(task2, "blocks", new_id);
    assert_single_id(task2, "relates_to", new_id);
    assert_eq!(task2["duplicates"].as_str(), Some(new_id));
    assert_eq!(task2["parent_id"].as_str(), Some(new_id));

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["queue", "validate"]);
    anyhow::ensure!(
        status.success(),
        "cueloop queue validate failed after repair\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    Ok(())
}

fn assert_single_id(task: &serde_json::Value, field: &str, expected_id: &str) {
    let values = task[field].as_array().unwrap_or_else(|| {
        panic!("{field} should be an array");
    });

    assert_eq!(values.len(), 1, "{field} should have 1 ID");
    assert_eq!(
        values[0].as_str(),
        Some(expected_id),
        "{field} should be updated to the remapped ID"
    );
}

fn assert_repair_undo_snapshot_created(repo_root: &Path) -> Result<()> {
    let undo_dir = project_runtime_dir(repo_root).join("cache/undo");
    let snapshots = std::fs::read_dir(&undo_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("undo-") && name.ends_with(".json"))
        })
        .collect::<Vec<_>>();

    anyhow::ensure!(
        snapshots.len() == 1,
        "expected exactly one undo snapshot after repair in {}, found {}",
        undo_dir.display(),
        snapshots.len()
    );

    let snapshot: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&snapshots[0])?)?;
    assert_eq!(
        snapshot["operation"].as_str(),
        Some("queue repair continuation")
    );
    assert_eq!(
        snapshot["queue_json"]["tasks"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(
        snapshot["done_json"]["tasks"].as_array().map(Vec::len),
        Some(2)
    );

    Ok(())
}
