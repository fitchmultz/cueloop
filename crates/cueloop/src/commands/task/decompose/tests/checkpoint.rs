//! Preview checkpoint regression tests for task decomposition.
//!
//! Purpose:
//! - Cover exact replay checkpoint persistence for decomposition previews.
//!
//! Responsibilities:
//! - Assert checkpoint round trips preserve planned trees and statuses.
//! - Assert checkpoint replay writes the saved preview without planner reruns.
//! - Assert unsafe checkpoint IDs are rejected before path access.
//!
//! Not handled here:
//! - Planner invocation or CLI rendering.
//!
//! Usage:
//! - Included by the parent decomposition test hub.
//!
//! Invariants/assumptions:
//! - Plan-file checkpoint replay uses the normalized plan and does not persist source-file content.

use super::{planned_node, planned_node_with_plan_and_scope, test_resolved};
use crate::commands::task::{
    DecompositionChildPolicy, DecompositionPlan, DecompositionPreview, DecompositionSource,
    DependencyEdgePreview, write_task_decomposition,
};
use crate::contracts::{QueueFile, TaskStatus};
use crate::queue;
use anyhow::Result;

#[test]
fn preview_checkpoint_round_trip_preserves_exact_plan_without_plan_file_content() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    let preview = DecompositionPreview {
        source: DecompositionSource::PlanFile {
            path: "docs/plans/auth.md".to_string(),
            content: "large source plan should not be persisted".to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node(
                "root",
                "Ship auth",
                vec![],
                vec![planned_node_with_plan_and_scope(
                    "leaf",
                    "Wire login",
                    vec!["apps/CueLoopMac"],
                    vec!["Implement UI", "Add tests"],
                    vec![],
                    vec![],
                )],
            ),
            warnings: vec!["check OAuth scope".to_string()],
            total_nodes: 2,
            leaf_nodes: 1,
            dependency_edges: vec![],
        },
        write_blockers: vec![],
        child_status: TaskStatus::Draft,
        parent_status: TaskStatus::Draft,
        leaf_status: TaskStatus::Todo,
        child_policy: DecompositionChildPolicy::Fail,
        with_dependencies: false,
    };

    let checkpoint = super::super::save_decomposition_preview_checkpoint(&resolved, &preview)?;
    let checkpoint_path = resolved.repo_root.join(&checkpoint.path);
    let raw = std::fs::read_to_string(&checkpoint_path)?;
    assert!(!raw.contains("large source plan should not be persisted"));

    let (loaded, loaded_ref) =
        super::super::load_decomposition_preview_checkpoint(&resolved, &checkpoint.id)?;
    assert_eq!(loaded_ref.id, checkpoint.id);
    assert_eq!(loaded.plan.total_nodes, preview.plan.total_nodes);
    assert_eq!(loaded.plan.leaf_nodes, preview.plan.leaf_nodes);
    assert_eq!(loaded.plan.root.planner_key, "root");
    assert_eq!(loaded.plan.root.children[0].title, "Wire login");
    assert_eq!(
        loaded.plan.root.children[0].plan,
        vec!["Implement UI", "Add tests"]
    );
    assert_eq!(loaded.child_status, TaskStatus::Draft);
    assert_eq!(loaded.leaf_status, TaskStatus::Todo);
    assert_eq!(loaded.child_policy, DecompositionChildPolicy::Fail);
    assert!(matches!(
        loaded.source,
        DecompositionSource::PlanFile { content, .. } if content.is_empty()
    ));
    Ok(())
}

#[test]
fn write_from_loaded_preview_checkpoint_matches_original_preview() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    queue::save_queue(&resolved.queue_path, &QueueFile::default())?;
    let preview = DecompositionPreview {
        source: DecompositionSource::Freeform {
            request: "Ship billing".to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node(
                "billing",
                "Ship billing",
                vec![],
                vec![
                    planned_node("api", "Billing API", vec![], vec![]),
                    planned_node("ui", "Billing UI", vec!["api".to_string()], vec![]),
                ],
            ),
            warnings: vec![],
            total_nodes: 3,
            leaf_nodes: 2,
            dependency_edges: vec![DependencyEdgePreview {
                task_title: "Billing UI".to_string(),
                depends_on_title: "Billing API".to_string(),
            }],
        },
        write_blockers: vec![],
        child_status: TaskStatus::Draft,
        parent_status: TaskStatus::Draft,
        leaf_status: TaskStatus::Draft,
        child_policy: DecompositionChildPolicy::Fail,
        with_dependencies: true,
    };

    let checkpoint = super::super::save_decomposition_preview_checkpoint(&resolved, &preview)?;
    let (loaded, _) =
        super::super::load_decomposition_preview_checkpoint(&resolved, &checkpoint.id)?;
    let result = write_task_decomposition(&resolved, &loaded, false)?;
    assert_eq!(result.created_ids.len(), preview.plan.total_nodes);

    let queue_file = queue::load_queue(&resolved.queue_path)?;
    let titles = queue_file
        .tasks
        .iter()
        .map(|task| task.title.as_str())
        .collect::<Vec<_>>();
    assert_eq!(titles, vec!["Ship billing", "Billing API", "Billing UI"]);
    let ui = queue_file
        .tasks
        .iter()
        .find(|task| task.title == "Billing UI")
        .expect("ui task");
    assert_eq!(ui.depends_on.len(), 1);
    Ok(())
}

#[test]
fn preview_checkpoint_rejects_path_traversal_ids() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    let err =
        super::super::load_decomposition_preview_checkpoint(&resolved, "../queue").unwrap_err();
    assert!(
        err.to_string()
            .contains("Invalid decomposition preview checkpoint id")
    );
    Ok(())
}
