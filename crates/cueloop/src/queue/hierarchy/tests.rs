//! Purpose: Preserve queue-hierarchy indexing, cycle detection, and rendering behavior during refactors.
//!
//! Responsibilities:
//! - Verify deterministic hierarchy indexing and ordering.
//! - Verify cycle-detection semantics.
//! - Verify rendering behavior for orphans, depth limits, and cycles.
//!
//! Scope:
//! - Unit coverage for `queue::hierarchy` only.
//!
//! Usage:
//! - Compiled only under `#[cfg(test)]` from the hierarchy facade.
//!
//! Invariants/Assumptions:
//! - Tests should continue exercising the facade re-exports rather than sibling internals directly.
//! - Expected behavior here is contract coverage, not an invitation to change semantics during file splits.

use super::*;
use crate::contracts::{QueueFile, Task, TaskStatus};

fn make_task(id: &str, parent_id: Option<&str>) -> Task {
    Task {
        id: id.to_string(),
        title: format!("Task {id}"),
        description: None,
        status: TaskStatus::Todo,
        kind: Default::default(),
        parent_id: parent_id.map(str::to_string),
        created_at: Some("2026-01-01T00:00:00Z".to_string()),
        updated_at: Some("2026-01-01T00:00:00Z".to_string()),
        ..Default::default()
    }
}

fn make_active_queue(tasks: Vec<Task>) -> QueueFile {
    QueueFile { version: 1, tasks }
}

#[test]
fn hierarchy_index_builds_correctly() {
    let tasks = vec![
        make_task("RQ-0001", None),
        make_task("RQ-0002", Some("RQ-0001")),
        make_task("RQ-0003", Some("RQ-0001")),
    ];
    let active = make_active_queue(tasks);

    let idx = HierarchyIndex::build(&active, None);

    assert_eq!(idx.children_of("RQ-0001").len(), 2);
    assert!(idx.children_of("RQ-0002").is_empty());
    assert!(idx.get("RQ-0001").is_some());
    assert!(idx.get("RQ-0002").is_some());
}

#[test]
fn children_preserve_file_order() {
    let tasks = vec![
        make_task("RQ-0001", None),
        make_task("RQ-0003", Some("RQ-0001")),
        make_task("RQ-0002", Some("RQ-0001")),
    ];
    let active = make_active_queue(tasks);

    let idx = HierarchyIndex::build(&active, None);
    let children = idx.children_of("RQ-0001");

    assert_eq!(children[0].task.id, "RQ-0003");
    assert_eq!(children[1].task.id, "RQ-0002");
}

#[test]
fn orphan_detection_works() {
    let tasks = vec![
        make_task("RQ-0001", None),
        make_task("RQ-0002", Some("RQ-9999")),
    ];
    let active = make_active_queue(tasks);

    let idx = HierarchyIndex::build(&active, None);

    let roots: Vec<&str> = idx
        .roots()
        .iter()
        .map(|root| root.task.id.as_str())
        .collect();
    assert!(roots.contains(&"RQ-0002"));

    let output = render_tree(
        &idx,
        &["RQ-0002"],
        10,
        true,
        |_task, _depth, _is_cycle, orphan_parent| {
            format!("orphan_parent={}", orphan_parent.unwrap_or("<none>"))
        },
    );
    assert!(output.contains("orphan_parent=RQ-9999"));
}

#[test]
fn empty_parent_id_treated_as_unset() {
    let mut task = make_task("RQ-0001", None);
    task.parent_id = Some("   ".to_string());

    let active = make_active_queue(vec![task]);
    let idx = HierarchyIndex::build(&active, None);

    assert_eq!(idx.roots().len(), 1);
}

#[test]
fn cycle_detection_finds_simple_cycle() {
    let tasks = [
        Task {
            id: "RQ-0001".to_string(),
            parent_id: Some("RQ-0002".to_string()),
            title: "Task 1".to_string(),
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            updated_at: Some("2026-01-01T00:00:00Z".to_string()),
            ..Default::default()
        },
        Task {
            id: "RQ-0002".to_string(),
            parent_id: Some("RQ-0001".to_string()),
            title: "Task 2".to_string(),
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            updated_at: Some("2026-01-01T00:00:00Z".to_string()),
            ..Default::default()
        },
    ];

    let task_refs: Vec<&Task> = tasks.iter().collect();
    let cycles = detect_parent_cycles(&task_refs);

    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].len(), 2);
}

#[test]
fn cycle_detection_finds_self_cycle() {
    let tasks = [Task {
        id: "RQ-0001".to_string(),
        parent_id: Some("RQ-0001".to_string()),
        title: "Task 1".to_string(),
        created_at: Some("2026-01-01T00:00:00Z".to_string()),
        updated_at: Some("2026-01-01T00:00:00Z".to_string()),
        ..Default::default()
    }];

    let task_refs: Vec<&Task> = tasks.iter().collect();
    let cycles = detect_parent_cycles(&task_refs);

    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0], vec!["RQ-0001"]);
}

#[test]
fn roots_includes_orphans() {
    let tasks = vec![
        make_task("RQ-0001", None),
        make_task("RQ-0002", Some("RQ-9999")),
    ];
    let active = make_active_queue(tasks);

    let idx = HierarchyIndex::build(&active, None);
    let roots = idx.roots();

    assert_eq!(roots.len(), 2);
}

#[test]
fn active_done_combined_ordering() {
    let active = make_active_queue(vec![make_task("RQ-0001", None)]);
    let done = QueueFile {
        version: 1,
        tasks: vec![make_task("RQ-0002", Some("RQ-0001"))],
    };

    let idx = HierarchyIndex::build(&active, Some(&done));

    assert!(idx.get("RQ-0001").is_some());
    assert!(idx.get("RQ-0002").is_some());

    let root = idx.get("RQ-0001").unwrap();
    let child = idx.get("RQ-0002").unwrap();
    assert!(root.order < child.order);
}

#[test]
fn tree_rendering_produces_output() {
    let tasks = vec![
        make_task("RQ-0001", None),
        make_task("RQ-0002", Some("RQ-0001")),
    ];
    let active = make_active_queue(tasks);
    let idx = HierarchyIndex::build(&active, None);

    let output = render_tree(
        &idx,
        &["RQ-0001"],
        10,
        true,
        |task, depth, _is_cycle, _orphan| format!("{}{}", "  ".repeat(depth), task.id),
    );

    assert!(output.contains("RQ-0001"));
    assert!(output.contains("  RQ-0002"));
}

#[test]
fn tree_rendering_respects_max_depth() {
    let tasks = vec![
        make_task("RQ-0001", None),
        make_task("RQ-0002", Some("RQ-0001")),
        make_task("RQ-0003", Some("RQ-0002")),
    ];
    let active = make_active_queue(tasks);
    let idx = HierarchyIndex::build(&active, None);

    let output = render_tree(
        &idx,
        &["RQ-0001"],
        1,
        true,
        |task, depth, _is_cycle, _orphan| format!("{}{}", "  ".repeat(depth), task.id),
    );

    assert!(output.contains("RQ-0001"));
    assert!(output.contains("RQ-0002"));
    assert!(!output.contains("RQ-0003"));
}

#[test]
fn tree_rendering_handles_cycles() {
    let tasks = vec![
        Task {
            id: "RQ-0001".to_string(),
            parent_id: Some("RQ-0002".to_string()),
            title: "Task 1".to_string(),
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            updated_at: Some("2026-01-01T00:00:00Z".to_string()),
            ..Default::default()
        },
        Task {
            id: "RQ-0002".to_string(),
            parent_id: Some("RQ-0001".to_string()),
            title: "Task 2".to_string(),
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            updated_at: Some("2026-01-01T00:00:00Z".to_string()),
            ..Default::default()
        },
    ];
    let active = make_active_queue(tasks);
    let idx = HierarchyIndex::build(&active, None);

    let output = render_tree(
        &idx,
        &["RQ-0001"],
        10,
        true,
        |task, depth, is_cycle, _orphan| {
            let marker = if is_cycle { " (cycle)" } else { "" };
            format!("{}{}{}", "  ".repeat(depth), task.id, marker)
        },
    );

    assert!(output.contains("RQ-0001"));
    assert!(output.contains("(cycle)"));
}
