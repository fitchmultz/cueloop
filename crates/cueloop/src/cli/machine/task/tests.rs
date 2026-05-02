//! Regression coverage for machine task command parsing helpers.
//!
//! Purpose:
//! - Regression coverage for machine task command parsing helpers.
//!
//! Responsibilities:
//! - Validate supported status parsing.
//! - Validate supported child-policy parsing.
//!
//! Not handled here:
//! - Machine task write workflows.
//! - Queue mutation/decomposition integration.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Parsing remains case-insensitive for supported values.
//! - Unsupported values continue to fail fast.

use super::{build_decompose_document, parse_child_policy, parse_task_status};
use crate::commands::task::{
    DecompositionChildPolicy, DecompositionPlan, DecompositionPreview,
    DecompositionPreviewCheckpointRef, DecompositionSource, PlannedNode,
};
use crate::contracts::{BlockingReason, TaskStatus};

#[test]
fn parse_task_status_accepts_supported_values_case_insensitively() {
    assert_eq!(
        parse_task_status("ToDo").expect("todo status"),
        TaskStatus::Todo
    );
    assert_eq!(
        parse_task_status("done").expect("done status"),
        TaskStatus::Done
    );
}

#[test]
fn parse_task_status_rejects_unknown_values() {
    assert!(parse_task_status("later").is_err());
}

#[test]
fn parse_child_policy_accepts_supported_values_case_insensitively() {
    assert_eq!(
        parse_child_policy("Append").expect("append child policy"),
        DecompositionChildPolicy::Append
    );
}

#[test]
fn parse_child_policy_rejects_unknown_values() {
    assert!(parse_child_policy("merge").is_err());
}

#[test]
fn preview_continuation_uses_exact_checkpoint_command_without_ellipsis() {
    let preview = test_preview(Vec::new());
    let checkpoint = test_checkpoint();
    let document = build_decompose_document(&preview, None, Some(&checkpoint));
    let commands = document
        .continuation
        .next_steps
        .iter()
        .map(|step| step.command.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        commands,
        vec!["cueloop machine task decompose --write --from-preview dp-test-123"]
    );
    assert!(commands.iter().all(|command| !command.contains("...")));
}

#[test]
fn blocked_preview_continuation_has_no_placeholder_ellipsis() {
    let preview = test_preview(vec!["Parent already has children".to_string()]);
    let checkpoint = test_checkpoint();
    let document = build_decompose_document(&preview, None, Some(&checkpoint));
    let commands = document
        .continuation
        .next_steps
        .iter()
        .map(|step| step.command.as_str())
        .collect::<Vec<_>>();
    assert!(
        commands.contains(&"cueloop machine task decompose --write --from-preview dp-test-123")
    );
    assert!(commands.iter().all(|command| !command.contains("...")));
    let suggested = match &document.blocking.as_ref().expect("blocking state").reason {
        BlockingReason::OperatorRecovery {
            suggested_command, ..
        } => suggested_command.as_deref().expect("suggested command"),
        other => panic!("unexpected blocking reason: {other:?}"),
    };
    assert!(!suggested.contains("..."));
}

fn test_checkpoint() -> DecompositionPreviewCheckpointRef {
    DecompositionPreviewCheckpointRef {
        id: "dp-test-123".to_string(),
        path: ".cueloop/cache/decompose-previews/dp-test-123.json".to_string(),
        created_at: "2026-04-30T00:00:00Z".to_string(),
        expires_at: "2026-05-07T00:00:00Z".to_string(),
    }
}

fn test_preview(write_blockers: Vec<String>) -> DecompositionPreview {
    DecompositionPreview {
        source: DecompositionSource::Freeform {
            request: "Ship auth".to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: PlannedNode {
                planner_key: "root".to_string(),
                title: "Ship auth".to_string(),
                description: None,
                plan: vec![],
                tags: vec![],
                scope: vec![],
                depends_on_keys: vec![],
                children: vec![PlannedNode {
                    planner_key: "leaf".to_string(),
                    title: "Wire login".to_string(),
                    description: None,
                    plan: vec![],
                    tags: vec![],
                    scope: vec![],
                    depends_on_keys: vec![],
                    children: vec![],
                    dependency_refs: vec![],
                }],
                dependency_refs: vec![],
            },
            warnings: vec![],
            total_nodes: 2,
            leaf_nodes: 1,
            dependency_edges: vec![],
        },
        write_blockers,
        child_status: TaskStatus::Draft,
        parent_status: TaskStatus::Draft,
        leaf_status: TaskStatus::Draft,
        child_policy: DecompositionChildPolicy::Fail,
        with_dependencies: false,
    }
}
