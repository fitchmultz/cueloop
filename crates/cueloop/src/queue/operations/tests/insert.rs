//! Tests for atomic task insert requests.
//!
//! Purpose:
//! - Verify rich insert requests allocate IDs under the shared materializer.
//!
//! Responsibilities:
//! - Cover local dependency remapping, external relationships, and dry-run
//!   atomicity.

use super::*;
use crate::contracts::{
    TASK_INSERT_VERSION, TaskInsertRequest, TaskInsertSpec, TaskPriority, TaskStatus,
};

fn request(tasks: Vec<TaskInsertSpec>) -> TaskInsertRequest {
    TaskInsertRequest {
        version: TASK_INSERT_VERSION,
        tasks,
    }
}

fn spec(key: &str, title: &str) -> TaskInsertSpec {
    TaskInsertSpec {
        key: key.to_string(),
        title: title.to_string(),
        description: Some(format!("{title} description")),
        priority: TaskPriority::Medium,
        status: TaskStatus::Todo,
        kind: TaskKind::WorkItem,
        tags: vec!["scan".to_string()],
        scope: vec!["docs/queue-and-tasks.md".to_string()],
        evidence: vec!["path: docs/queue-and-tasks.md :: task insert".to_string()],
        plan: vec![
            "Insert atomically".to_string(),
            "Validate queue".to_string(),
        ],
        notes: vec![],
        request: Some("scan: queue safety".to_string()),
        depends_on_keys: vec![],
        depends_on: vec![],
        blocks: vec![],
        relates_to: vec![],
        duplicates: None,
        parent_key: None,
        parent_id: None,
        custom_fields: HashMap::new(),
        estimated_minutes: None,
        agent: None,
    }
}

#[test]
fn task_insert_request_maps_local_and_external_relationships() -> anyhow::Result<()> {
    let mut existing = task("RQ-0001");
    existing.status = TaskStatus::Doing;
    let mut active = QueueFile {
        version: 1,
        tasks: vec![existing.clone()],
    };

    let mut alpha = spec("alpha", "Alpha");
    alpha
        .custom_fields
        .insert("scan_agent".into(), "scan-general".into());
    alpha.blocks = vec!["RQ-0001".to_string()];
    alpha.relates_to = vec!["RQ-0001".to_string()];

    let mut beta = spec("beta", "Beta");
    beta.depends_on_keys = vec!["alpha".to_string()];
    beta.depends_on = vec!["RQ-0001".to_string()];
    beta.parent_key = Some("alpha".to_string());

    let document = apply_task_insert_request(
        &mut active,
        None,
        &request(vec![alpha, beta]),
        "2026-05-03T20:00:00Z",
        "RQ",
        4,
        10,
        false,
    )?;

    assert_eq!(document.created_count, 2);
    assert_eq!(document.tasks[0].task.id, "RQ-0002");
    assert_eq!(document.tasks[1].task.id, "RQ-0003");
    assert_eq!(
        document.tasks[1].task.depends_on,
        vec!["RQ-0002", "RQ-0001"]
    );
    assert_eq!(document.tasks[1].task.parent_id.as_deref(), Some("RQ-0002"));
    assert_eq!(document.tasks[0].task.blocks, vec!["RQ-0001"]);
    assert_eq!(document.tasks[0].task.relates_to, vec!["RQ-0001"]);
    assert_eq!(
        document.tasks[0]
            .task
            .custom_fields
            .get("scan_agent")
            .map(String::as_str),
        Some("scan-general")
    );
    assert_eq!(active.tasks[1].id, "RQ-0002");
    Ok(())
}

#[test]
fn task_insert_request_dry_run_leaves_queue_unchanged() -> anyhow::Result<()> {
    let mut active = QueueFile {
        version: 1,
        tasks: vec![task("RQ-0001")],
    };
    let before = serde_json::to_value(&active)?;

    let document = apply_task_insert_request(
        &mut active,
        None,
        &request(vec![spec("alpha", "Alpha")]),
        "2026-05-03T20:05:00Z",
        "RQ",
        4,
        10,
        true,
    )?;

    assert!(document.dry_run);
    assert_eq!(document.tasks[0].task.id, "RQ-0002");
    assert_eq!(serde_json::to_value(&active)?, before);
    Ok(())
}

#[test]
fn task_insert_request_rejects_terminal_statuses() {
    let mut active = QueueFile {
        version: 1,
        tasks: vec![task("RQ-0001")],
    };
    let before = serde_json::to_value(&active).expect("queue snapshot");
    let mut done_task = spec("alpha", "Alpha");
    done_task.status = TaskStatus::Done;

    let err = apply_task_insert_request(
        &mut active,
        None,
        &request(vec![done_task]),
        "2026-05-03T20:10:00Z",
        "RQ",
        4,
        10,
        false,
    )
    .unwrap_err();

    assert!(format!("{err:#}").contains("cannot use terminal status done"));
    assert_eq!(
        serde_json::to_value(&active).expect("queue snapshot"),
        before
    );
}
