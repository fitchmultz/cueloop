//! Actionability-focused task decomposition tests.
//!
//! Purpose:
//! - Actionability-focused task decomposition tests.
//!
//! Responsibilities:
//! - Verify generated parent/leaf status materialization.
//! - Verify machine continuation guidance for draft and runnable decomposition writes.
//!
//! Not handled here:
//! - Planner normalization coverage.
//! - Plan-file ordering and source provenance coverage.
//!
//! Usage:
//! - Included by the parent decompose test module.
//!
//! Invariants/assumptions:
//! - First actionable leaf IDs come from write result metadata, not text parsing.
//! - `todo` leaf status is the runnable activation mode.

use super::{planned_node, test_resolved};
use crate::commands::task::decompose::types::{
    DecompositionChildPolicy, DecompositionPlan, DecompositionPreview, DecompositionSource,
    TaskDecomposeWriteResult,
};
use crate::commands::task::decompose::write_task_decomposition;
use crate::contracts::{QueueFile, TaskKind, TaskStatus};
use crate::queue;
use anyhow::Result;

#[test]
fn write_task_decomposition_applies_parent_and_leaf_statuses_by_kind() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    queue::save_queue(&resolved.queue_path, &QueueFile::default())?;
    let preview = DecompositionPreview {
        source: DecompositionSource::Freeform {
            request: "Ship OAuth".to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node(
                "root",
                "Ship OAuth",
                vec![],
                vec![planned_node("schema", "Schema", vec![], vec![])],
            ),
            warnings: vec![],
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

    let result = write_task_decomposition(&resolved, &preview, false)?;
    assert_eq!(result.root_group_task_id.as_deref(), Some("RQ-0001"));
    assert_eq!(
        result.first_actionable_leaf_task_id.as_deref(),
        Some("RQ-0002")
    );
    let queue_file = queue::load_queue(&resolved.queue_path)?;
    assert_eq!(queue_file.tasks[0].kind, TaskKind::Group);
    assert_eq!(queue_file.tasks[0].status, TaskStatus::Draft);
    assert_eq!(queue_file.tasks[1].kind, TaskKind::WorkItem);
    assert_eq!(queue_file.tasks[1].status, TaskStatus::Todo);
    Ok(())
}

#[test]
fn write_task_decomposition_single_node_uses_leaf_status() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    queue::save_queue(&resolved.queue_path, &QueueFile::default())?;
    let preview = DecompositionPreview {
        source: DecompositionSource::Freeform {
            request: "Ship OAuth".to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node("root", "Ship OAuth", vec![], vec![]),
            warnings: vec![],
            total_nodes: 1,
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

    let result = write_task_decomposition(&resolved, &preview, false)?;
    assert_eq!(result.root_group_task_id.as_deref(), Some("RQ-0001"));
    assert_eq!(
        result.first_actionable_leaf_task_id.as_deref(),
        Some("RQ-0001")
    );
    let queue_file = queue::load_queue(&resolved.queue_path)?;
    assert_eq!(queue_file.tasks[0].kind, TaskKind::WorkItem);
    assert_eq!(queue_file.tasks[0].status, TaskStatus::Todo);
    Ok(())
}

#[test]
fn decompose_document_all_draft_write_guides_first_leaf_promotion() {
    let preview = DecompositionPreview {
        source: DecompositionSource::Freeform {
            request: "Ship OAuth".to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node(
                "root",
                "Ship OAuth",
                vec![],
                vec![planned_node("schema", "Schema", vec![], vec![])],
            ),
            warnings: vec![],
            total_nodes: 2,
            leaf_nodes: 1,
            dependency_edges: vec![],
        },
        write_blockers: vec![],
        child_status: TaskStatus::Draft,
        parent_status: TaskStatus::Draft,
        leaf_status: TaskStatus::Draft,
        child_policy: DecompositionChildPolicy::Fail,
        with_dependencies: false,
    };
    let write = TaskDecomposeWriteResult {
        root_task_id: Some("RQ-0001".to_string()),
        root_group_task_id: Some("RQ-0001".to_string()),
        first_actionable_leaf_task_id: Some("RQ-0002".to_string()),
        parent_task_id: None,
        created_ids: vec!["RQ-0001".to_string(), "RQ-0002".to_string()],
        replaced_ids: vec![],
        parent_annotated: false,
    };

    let document = crate::cli::machine::build_task_decompose_document(&preview, Some(&write));

    assert_eq!(
        document.continuation.headline,
        "Decomposition has been written as draft."
    );
    assert_eq!(
        document.continuation.next_steps[0].command,
        "ralph task ready RQ-0002"
    );
    assert!(
        document
            .continuation
            .detail
            .contains("Promote first actionable leaf RQ-0002")
    );
}

#[test]
fn decompose_document_single_leaf_draft_write_ignores_parent_status_for_activation_guidance() {
    let preview = DecompositionPreview {
        source: DecompositionSource::Freeform {
            request: "Ship OAuth".to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node("root", "Ship OAuth", vec![], vec![]),
            warnings: vec![],
            total_nodes: 1,
            leaf_nodes: 1,
            dependency_edges: vec![],
        },
        write_blockers: vec![],
        child_status: TaskStatus::Draft,
        parent_status: TaskStatus::Todo,
        leaf_status: TaskStatus::Draft,
        child_policy: DecompositionChildPolicy::Fail,
        with_dependencies: false,
    };
    let write = TaskDecomposeWriteResult {
        root_task_id: Some("RQ-0001".to_string()),
        root_group_task_id: Some("RQ-0001".to_string()),
        first_actionable_leaf_task_id: Some("RQ-0001".to_string()),
        parent_task_id: None,
        created_ids: vec!["RQ-0001".to_string()],
        replaced_ids: vec![],
        parent_annotated: false,
    };

    let document = crate::cli::machine::build_task_decompose_document(&preview, Some(&write));

    assert_eq!(
        document.continuation.headline,
        "Decomposition has been written as draft."
    );
    assert_eq!(
        document.continuation.next_steps[0].command,
        "ralph task ready RQ-0001"
    );
}

#[test]
fn decompose_document_runnable_leaf_write_guides_run_without_activation() {
    let preview = DecompositionPreview {
        source: DecompositionSource::Freeform {
            request: "Ship OAuth".to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node(
                "root",
                "Ship OAuth",
                vec![],
                vec![planned_node("schema", "Schema", vec![], vec![])],
            ),
            warnings: vec![],
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
    let write = TaskDecomposeWriteResult {
        root_task_id: Some("RQ-0001".to_string()),
        root_group_task_id: Some("RQ-0001".to_string()),
        first_actionable_leaf_task_id: Some("RQ-0002".to_string()),
        parent_task_id: None,
        created_ids: vec!["RQ-0001".to_string(), "RQ-0002".to_string()],
        replaced_ids: vec![],
        parent_annotated: false,
    };

    let document = crate::cli::machine::build_task_decompose_document(&preview, Some(&write));
    let commands = document
        .continuation
        .next_steps
        .iter()
        .map(|step| step.command.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        document.continuation.headline,
        "Decomposition has been written with runnable leaves."
    );
    assert!(
        !commands
            .iter()
            .any(|command| command.starts_with("ralph task ready "))
    );
    assert!(commands.contains(&"ralph machine run one --resume"));
}
