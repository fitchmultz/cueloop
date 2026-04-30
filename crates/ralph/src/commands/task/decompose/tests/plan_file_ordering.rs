//! Ordered plan-file decomposition regression tests.
//!
//! Purpose:
//! - Prove complete ordered plan fixtures survive task decomposition materialization.
//!
//! Responsibilities:
//! - Assert source section coverage, queue insertion order, provenance, and dependencies.
//! - Keep bulky ordered-plan fixture coverage out of the root decomposition test hub.
//!
//! Scope:
//! - Unit-level materialization/write behavior for deterministic planned nodes.
//!
//! Usage:
//! - Included by `commands::task::decompose::tests`.
//!
//! Invariants/assumptions:
//! - The planner has already emitted a complete ordered tree.
//! - Ralph materialization must preserve node order and sibling dependency mappings.

use anyhow::Result;

use super::{planned_node_with_plan_and_scope, test_resolved};
use crate::commands::task::decompose::types::{
    DecompositionChildPolicy, DecompositionPlan, DecompositionPreview, DecompositionSource,
    DependencyEdgePreview,
};
use crate::contracts::{QueueFile, Task, TaskStatus};
use crate::queue;

const ORDERED_PLAN_PATH: &str = "docs/plans/full-plan-ordering.md";
const ORDERED_PLAN_CONTENT: &str = r#"# Full plan ordering fixture

## Phase 1: Inventory current behavior
Capture current CLI, queue, and prompt behavior.

## Phase 2: Implement prompt and planner guardrails
Tighten plan-file decomposition rules.

## Phase 3: Add regression coverage
Cover normalization, materialization, preview, and write behavior.

## Phase 4: Document acceptance workflow
Update PRD and CLI docs with queue validation/navigation checks.
"#;

const ORDERED_PLAN_SECTIONS: [&str; 4] = [
    "Phase 1: Inventory current behavior",
    "Phase 2: Implement prompt and planner guardrails",
    "Phase 3: Add regression coverage",
    "Phase 4: Document acceptance workflow",
];

#[test]
fn write_task_decomposition_plan_file_preserves_full_ordered_plan_coverage() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    queue::save_queue(&resolved.queue_path, &QueueFile::default())?;

    let phase_nodes = vec![
        planned_node_with_plan_and_scope(
            "phase-1",
            "Phase 1: Inventory current behavior",
            vec![ORDERED_PLAN_PATH, "Phase 1: Inventory current behavior"],
            vec!["Cover source section: Phase 1: Inventory current behavior"],
            vec![],
            vec![],
        ),
        planned_node_with_plan_and_scope(
            "phase-2",
            "Phase 2: Implement prompt and planner guardrails",
            vec![
                ORDERED_PLAN_PATH,
                "Phase 2: Implement prompt and planner guardrails",
            ],
            vec!["Cover source section: Phase 2: Implement prompt and planner guardrails"],
            vec!["phase-1".to_string()],
            vec![],
        ),
        planned_node_with_plan_and_scope(
            "phase-3",
            "Phase 3: Add regression coverage",
            vec![ORDERED_PLAN_PATH, "Phase 3: Add regression coverage"],
            vec!["Cover source section: Phase 3: Add regression coverage"],
            vec!["phase-2".to_string()],
            vec![],
        ),
        planned_node_with_plan_and_scope(
            "phase-4",
            "Phase 4: Document acceptance workflow",
            vec![ORDERED_PLAN_PATH, "Phase 4: Document acceptance workflow"],
            vec!["Cover source section: Phase 4: Document acceptance workflow"],
            vec!["phase-3".to_string()],
            vec![],
        ),
    ];
    let preview = DecompositionPreview {
        source: DecompositionSource::PlanFile {
            path: ORDERED_PLAN_PATH.to_string(),
            content: ORDERED_PLAN_CONTENT.to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node_with_plan_and_scope(
                "root",
                "Full plan ordering fixture",
                vec![ORDERED_PLAN_PATH],
                vec!["Represent the whole ordered source plan"],
                vec![],
                phase_nodes,
            ),
            warnings: vec![],
            total_nodes: 5,
            leaf_nodes: 4,
            dependency_edges: vec![
                DependencyEdgePreview {
                    task_title: "Phase 2: Implement prompt and planner guardrails".to_string(),
                    depends_on_title: "Phase 1: Inventory current behavior".to_string(),
                },
                DependencyEdgePreview {
                    task_title: "Phase 3: Add regression coverage".to_string(),
                    depends_on_title: "Phase 2: Implement prompt and planner guardrails"
                        .to_string(),
                },
                DependencyEdgePreview {
                    task_title: "Phase 4: Document acceptance workflow".to_string(),
                    depends_on_title: "Phase 3: Add regression coverage".to_string(),
                },
            ],
        },
        write_blockers: vec![],
        child_status: TaskStatus::Draft,
        parent_status: TaskStatus::Draft,
        leaf_status: TaskStatus::Draft,
        child_policy: DecompositionChildPolicy::Fail,
        with_dependencies: true,
    };

    let result = super::write_task_decomposition(&resolved, &preview, false)?;
    assert_eq!(result.created_ids.len(), 5);

    let queue_file = queue::load_queue(&resolved.queue_path)?;
    assert_eq!(queue_file.tasks.len(), 5);
    assert_eq!(
        queue_file
            .tasks
            .iter()
            .map(|task| task.title.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Full plan ordering fixture",
            "Phase 1: Inventory current behavior",
            "Phase 2: Implement prompt and planner guardrails",
            "Phase 3: Add regression coverage",
            "Phase 4: Document acceptance workflow",
        ]
    );

    for task in &queue_file.tasks {
        assert_eq!(
            task.request.as_deref(),
            Some("Plan file docs/plans/full-plan-ordering.md")
        );
        assert_eq!(
            task.scope
                .iter()
                .filter(|item| item.as_str() == ORDERED_PLAN_PATH)
                .count(),
            1,
            "{} should include source plan path exactly once",
            task.title
        );
        assert_eq!(
            task.evidence,
            vec![format!(
                "path: {} :: {} :: source plan for this decomposed task",
                ORDERED_PLAN_PATH, task.title
            )]
        );
    }

    for section in ORDERED_PLAN_SECTIONS {
        assert!(
            queue_file
                .tasks
                .iter()
                .any(|task| task_mentions_section(task, section)),
            "missing source section coverage for {section}"
        );
    }

    let phase_1_id = &queue_file.tasks[1].id;
    let phase_2_id = &queue_file.tasks[2].id;
    let phase_3_id = &queue_file.tasks[3].id;
    assert_eq!(queue_file.tasks[2].depends_on, vec![phase_1_id.clone()]);
    assert_eq!(queue_file.tasks[3].depends_on, vec![phase_2_id.clone()]);
    assert_eq!(queue_file.tasks[4].depends_on, vec![phase_3_id.clone()]);

    queue::validation::validate_queue_set(&queue_file, None, "RQ", 4, 10)?;
    Ok(())
}

fn task_mentions_section(task: &Task, section: &str) -> bool {
    task.title.contains(section)
        || task.scope.iter().any(|item| item.contains(section))
        || task.evidence.iter().any(|item| item.contains(section))
        || task.plan.iter().any(|item| item.contains(section))
}
