//! Task decomposition tests.
//!
//! Purpose:
//! - Task decomposition tests.
//!
//! Responsibilities:
//! - Cover planner normalization, attach writes, and replace safety behavior.
//! - Exercise decomposition-specific edge cases without invoking external runners.
//!
//! Not handled here:
//! - End-to-end runner execution.
//! - CLI formatting assertions (covered by CLI tests).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Test queues use local temp directories and repo-scoped `.ralph/*.jsonc` files.
//! - Preview/write flows remain deterministic for the same planned tree.

use super::tree::normalize_response;
use super::types::{
    DecompositionAttachTarget, DecompositionChildPolicy, DecompositionPlan, DecompositionPreview,
    DecompositionSource, DependencyEdgePreview, PlannedNode, RawDecompositionResponse,
    RawPlannedNode, SourceKind, TaskDecomposeOptions, TaskDecomposeSourceInput,
};
use super::write_task_decomposition;
use crate::config;
use crate::contracts::{Config, QueueFile, Task, TaskKind, TaskStatus};
use crate::queue;
use anyhow::Result;
use tempfile::TempDir;

mod actionability;
mod checkpoint;
mod plan_file_ordering;

#[test]
fn normalize_response_resolves_sibling_dependencies() -> Result<()> {
    let raw = RawDecompositionResponse {
        warnings: vec![],
        tree: RawPlannedNode {
            key: Some("root".to_string()),
            title: "Ship OAuth".to_string(),
            description: None,
            plan: vec![],
            tags: vec![],
            scope: vec![],
            depends_on: vec![],
            children: vec![
                RawPlannedNode {
                    key: Some("schema".to_string()),
                    title: "Update schema".to_string(),
                    description: None,
                    plan: vec![],
                    tags: vec![],
                    scope: vec![],
                    depends_on: vec![],
                    children: vec![],
                },
                RawPlannedNode {
                    key: Some("ui".to_string()),
                    title: "Wire the UI".to_string(),
                    description: None,
                    plan: vec![],
                    tags: vec![],
                    scope: vec![],
                    depends_on: vec!["schema".to_string()],
                    children: vec![],
                },
            ],
        },
    };
    let opts = TaskDecomposeOptions {
        source: TaskDecomposeSourceInput::Inline("Ship OAuth".to_string()),
        attach_to_task_id: None,
        max_depth: 3,
        max_children: 5,
        max_nodes: 10,
        status: TaskStatus::Draft,
        parent_status: TaskStatus::Draft,
        leaf_status: TaskStatus::Draft,
        child_policy: DecompositionChildPolicy::Fail,
        with_dependencies: true,
        runner_override: None,
        model_override: None,
        reasoning_effort_override: None,
        runner_cli_overrides: crate::contracts::RunnerCliOptionsPatch::default(),
        repoprompt_tool_injection: false,
    };

    let plan = normalize_response(raw, SourceKind::Freeform, &opts, "Ship OAuth")?;
    assert_eq!(plan.root.children.len(), 2);
    assert_eq!(
        plan.root.children[1].depends_on_keys,
        vec!["schema".to_string()]
    );
    assert_eq!(plan.dependency_edges.len(), 1);
    Ok(())
}

#[test]
fn write_task_decomposition_attaches_freeform_subtree_under_existing_parent() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    let parent = test_task("RQ-0001", "Epic", None);
    queue::save_queue(
        &resolved.queue_path,
        &QueueFile {
            version: 1,
            tasks: vec![parent.clone()],
        },
    )?;

    let preview = DecompositionPreview {
        source: DecompositionSource::Freeform {
            request: "Build auth".to_string(),
        },
        attach_target: Some(DecompositionAttachTarget {
            task: Box::new(parent.clone()),
            has_existing_children: false,
        }),
        plan: DecompositionPlan {
            root: planned_node(
                "auth-root",
                "Build auth",
                vec![],
                vec![planned_node("auth-ui", "Wire auth UI", vec![], vec![])],
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
        child_policy: DecompositionChildPolicy::Append,
        with_dependencies: false,
    };

    let result = write_task_decomposition(&resolved, &preview, false)?;
    assert_eq!(result.parent_task_id.as_deref(), Some("RQ-0001"));
    assert_eq!(result.root_group_task_id.as_deref(), Some("RQ-0002"));
    assert_eq!(
        result.first_actionable_leaf_task_id.as_deref(),
        Some("RQ-0003")
    );
    assert_eq!(result.created_ids.len(), 2);

    let queue_file = queue::load_queue(&resolved.queue_path)?;
    assert_eq!(queue_file.tasks.len(), 3);
    assert_eq!(queue_file.tasks[0].kind, TaskKind::Group);
    assert_eq!(queue_file.tasks[1].kind, TaskKind::Group);
    assert_eq!(queue_file.tasks[2].kind, TaskKind::WorkItem);
    assert_eq!(queue_file.tasks[1].parent_id.as_deref(), Some("RQ-0001"));
    assert_eq!(queue_file.tasks[2].parent_id.as_deref(), Some("RQ-0002"));
    Ok(())
}

#[test]
fn write_task_decomposition_replace_rejects_external_references() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    let parent = test_task("RQ-0001", "Epic", None);
    let child = test_task("RQ-0002", "Old child", Some("RQ-0001"));
    let mut external = test_task("RQ-0003", "External", None);
    external.depends_on = vec!["RQ-0002".to_string()];
    queue::save_queue(
        &resolved.queue_path,
        &QueueFile {
            version: 1,
            tasks: vec![parent.clone(), child, external],
        },
    )?;

    let preview = DecompositionPreview {
        source: DecompositionSource::ExistingTask {
            task: Box::new(parent),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node(
                "epic",
                "Epic",
                vec![],
                vec![planned_node("new-child", "New child", vec![], vec![])],
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
        child_policy: DecompositionChildPolicy::Replace,
        with_dependencies: false,
    };

    let err = write_task_decomposition(&resolved, &preview, false).unwrap_err();
    assert!(err.to_string().contains("still reference"));
    Ok(())
}

#[test]
fn write_task_decomposition_materializes_sibling_dependencies() -> Result<()> {
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
                vec![
                    planned_node("schema", "Schema", vec![], vec![]),
                    planned_node("ui", "UI", vec!["schema".to_string()], vec![]),
                ],
            ),
            warnings: vec![],
            total_nodes: 3,
            leaf_nodes: 2,
            dependency_edges: vec![DependencyEdgePreview {
                task_title: "UI".to_string(),
                depends_on_title: "Schema".to_string(),
            }],
        },
        write_blockers: vec![],
        child_status: TaskStatus::Draft,
        parent_status: TaskStatus::Draft,
        leaf_status: TaskStatus::Draft,
        child_policy: DecompositionChildPolicy::Fail,
        with_dependencies: true,
    };

    let result = write_task_decomposition(&resolved, &preview, false)?;
    assert_eq!(result.created_ids.len(), 3);
    assert_eq!(result.root_group_task_id.as_deref(), Some("RQ-0001"));
    assert_eq!(
        result.first_actionable_leaf_task_id.as_deref(),
        Some("RQ-0002")
    );
    let queue_file = queue::load_queue(&resolved.queue_path)?;
    assert_eq!(queue_file.tasks[0].kind, TaskKind::Group);
    assert_eq!(queue_file.tasks[1].kind, TaskKind::WorkItem);
    assert_eq!(queue_file.tasks[2].kind, TaskKind::WorkItem);
    assert_eq!(queue_file.tasks[2].depends_on, vec!["RQ-0002".to_string()]);
    assert!(
        queue_file
            .tasks
            .iter()
            .all(|task| task.status == TaskStatus::Draft)
    );
    Ok(())
}

#[test]
fn write_task_decomposition_append_inserts_after_existing_subtree_without_reordering_siblings()
-> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    let parent = test_task("RQ-0001", "Epic", None);
    let existing_child = test_task("RQ-0002", "Existing child", Some("RQ-0001"));
    let later_sibling = test_task("RQ-0003", "Later sibling", None);
    queue::save_queue(
        &resolved.queue_path,
        &QueueFile {
            version: 1,
            tasks: vec![parent.clone(), existing_child, later_sibling],
        },
    )?;

    let preview = DecompositionPreview {
        source: DecompositionSource::Freeform {
            request: "Build auth".to_string(),
        },
        attach_target: Some(DecompositionAttachTarget {
            task: Box::new(parent),
            has_existing_children: true,
        }),
        plan: DecompositionPlan {
            root: planned_node(
                "auth-root",
                "Auth root",
                vec![],
                vec![planned_node("auth-ui", "Auth UI", vec![], vec![])],
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
        child_policy: DecompositionChildPolicy::Append,
        with_dependencies: false,
    };

    let result = write_task_decomposition(&resolved, &preview, false)?;
    assert_eq!(result.root_group_task_id.as_deref(), Some("RQ-0004"));
    assert_eq!(
        result.first_actionable_leaf_task_id.as_deref(),
        Some("RQ-0005")
    );
    let queue_file = queue::load_queue(&resolved.queue_path)?;
    assert_eq!(
        queue_file
            .tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        vec!["RQ-0001", "RQ-0002", "RQ-0004", "RQ-0005", "RQ-0003"]
    );
    assert_eq!(queue_file.tasks[0].kind, TaskKind::Group);
    assert_eq!(queue_file.tasks[2].kind, TaskKind::Group);
    assert_eq!(queue_file.tasks[3].kind, TaskKind::WorkItem);
    Ok(())
}

#[test]
fn write_task_decomposition_replace_reinserts_new_children_at_removed_subtree_boundary()
-> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    let parent = test_task("RQ-0001", "Epic", None);
    let existing_child = test_task("RQ-0002", "Existing child", Some("RQ-0001"));
    let later_sibling = test_task("RQ-0003", "Later sibling", None);
    queue::save_queue(
        &resolved.queue_path,
        &QueueFile {
            version: 1,
            tasks: vec![parent.clone(), existing_child, later_sibling],
        },
    )?;

    let preview = DecompositionPreview {
        source: DecompositionSource::ExistingTask {
            task: Box::new(parent),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node(
                "epic",
                "Epic",
                vec![],
                vec![planned_node("new-child", "New child", vec![], vec![])],
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
        child_policy: DecompositionChildPolicy::Replace,
        with_dependencies: false,
    };

    let result = write_task_decomposition(&resolved, &preview, false)?;
    assert_eq!(result.replaced_ids, vec!["RQ-0002".to_string()]);
    assert_eq!(result.root_group_task_id.as_deref(), Some("RQ-0001"));
    assert_eq!(
        result.first_actionable_leaf_task_id.as_deref(),
        Some("RQ-0004")
    );

    let queue_file = queue::load_queue(&resolved.queue_path)?;
    assert_eq!(
        queue_file
            .tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        vec!["RQ-0001", "RQ-0004", "RQ-0003"]
    );
    assert_eq!(queue_file.tasks[0].kind, TaskKind::Group);
    assert_eq!(queue_file.tasks[1].kind, TaskKind::WorkItem);
    Ok(())
}

#[test]
fn write_task_decomposition_created_tasks_inherit_request_and_timestamps_from_shared_materializer()
-> Result<()> {
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
        leaf_status: TaskStatus::Draft,
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
    let task = &queue_file.tasks[0];
    assert_eq!(task.kind, TaskKind::WorkItem);
    assert_eq!(task.request.as_deref(), Some("Ship OAuth"));
    assert!(task.evidence.is_empty());
    assert!(task.created_at.is_some());
    assert_eq!(task.created_at, task.updated_at);
    Ok(())
}

#[test]
fn write_task_decomposition_allows_cross_branch_duplicate_planner_keys() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    queue::save_queue(&resolved.queue_path, &QueueFile::default())?;
    let preview = DecompositionPreview {
        source: DecompositionSource::Freeform {
            request: "Ship auth".to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node(
                "root",
                "Ship auth",
                vec![],
                vec![
                    planned_node(
                        "backend",
                        "Backend",
                        vec![],
                        vec![planned_node("tests", "Backend tests", vec![], vec![])],
                    ),
                    planned_node(
                        "frontend",
                        "Frontend",
                        vec![],
                        vec![planned_node("tests", "Frontend tests", vec![], vec![])],
                    ),
                ],
            ),
            warnings: vec![],
            total_nodes: 5,
            leaf_nodes: 2,
            dependency_edges: vec![],
        },
        write_blockers: vec![],
        child_status: TaskStatus::Draft,
        parent_status: TaskStatus::Draft,
        leaf_status: TaskStatus::Draft,
        child_policy: DecompositionChildPolicy::Fail,
        with_dependencies: false,
    };

    let result = write_task_decomposition(&resolved, &preview, false)?;
    assert_eq!(result.created_ids.len(), 5);

    let queue_file = queue::load_queue(&resolved.queue_path)?;
    let backend = queue_file
        .tasks
        .iter()
        .find(|task| task.title == "Backend")
        .expect("backend task");
    let frontend = queue_file
        .tasks
        .iter()
        .find(|task| task.title == "Frontend")
        .expect("frontend task");
    let backend_tests = queue_file
        .tasks
        .iter()
        .find(|task| task.title == "Backend tests")
        .expect("backend tests task");
    let frontend_tests = queue_file
        .tasks
        .iter()
        .find(|task| task.title == "Frontend tests")
        .expect("frontend tests task");

    assert_eq!(
        backend_tests.parent_id.as_deref(),
        Some(backend.id.as_str())
    );
    assert_eq!(
        frontend_tests.parent_id.as_deref(),
        Some(frontend.id.as_str())
    );
    Ok(())
}

#[test]
fn read_plan_file_source_records_repo_relative_path_and_content() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    let plan_dir = resolved.repo_root.join("docs/plans");
    std::fs::create_dir_all(&plan_dir)?;
    let plan_path = plan_dir.join("auth.md");
    std::fs::write(&plan_path, "# Auth plan\n\n- Build OAuth\n")?;

    let source = super::read_plan_file_source(&resolved, &plan_path)?;
    match source {
        TaskDecomposeSourceInput::PlanFile { path, content } => {
            assert_eq!(path, "docs/plans/auth.md");
            assert_eq!(content, "# Auth plan\n\n- Build OAuth\n");
        }
        TaskDecomposeSourceInput::Inline(_) => panic!("expected plan file source"),
    }
    Ok(())
}

#[test]
fn read_plan_file_source_rejects_empty_missing_directory_and_large_files() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    let missing = resolved.repo_root.join("missing.md");
    let err = super::read_plan_file_source(&resolved, &missing).unwrap_err();
    assert!(err.to_string().contains("Plan file not found"));

    let err = super::read_plan_file_source(&resolved, &resolved.repo_root).unwrap_err();
    assert!(err.to_string().contains("not a file"));

    let empty = resolved.repo_root.join("empty.md");
    std::fs::write(&empty, "  \n\t")?;
    let err = super::read_plan_file_source(&resolved, &empty).unwrap_err();
    assert!(err.to_string().contains("is empty"));

    let huge = resolved.repo_root.join("huge.md");
    std::fs::write(
        &huge,
        vec![b'x'; (super::source_file::MAX_PLAN_FILE_BYTES + 1) as usize],
    )?;
    let err = super::read_plan_file_source(&resolved, &huge).unwrap_err();
    assert!(err.to_string().contains("too large"));
    Ok(())
}

#[test]
fn plan_file_preview_serializes_path_without_content() -> Result<()> {
    let preview = DecompositionPreview {
        source: DecompositionSource::PlanFile {
            path: "docs/plans/auth.md".to_string(),
            content: "# Auth plan\nsecret detail".to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node("root", "Auth", vec![], vec![]),
            warnings: vec![],
            total_nodes: 1,
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

    let value = serde_json::to_value(&preview)?;
    assert_eq!(value["source"]["kind"], "plan_file");
    assert_eq!(value["source"]["path"], "docs/plans/auth.md");
    assert!(value["source"].get("content").is_none());
    Ok(())
}

#[test]
fn plan_file_request_context_mentions_path_not_full_content() {
    let preview = DecompositionPreview {
        source: DecompositionSource::PlanFile {
            path: "docs/plans/auth.md".to_string(),
            content: "# Auth plan\nFull content should not be copied.".to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node("root", "Auth", vec![], vec![]),
            warnings: vec![],
            total_nodes: 1,
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

    let context = super::support::request_context(&preview);
    assert_eq!(context, "Plan file docs/plans/auth.md");
    assert!(!context.contains("Full content"));
}

#[test]
fn write_task_decomposition_plan_file_tasks_include_source_plan_provenance() -> Result<()> {
    let (_temp, resolved) = test_resolved()?;
    queue::save_queue(&resolved.queue_path, &QueueFile::default())?;

    let preview = DecompositionPreview {
        source: DecompositionSource::PlanFile {
            path: "docs/plans/auth.md".to_string(),
            content: "# Auth plan\n\n## Backend\nBuild backend auth.\n\n## UI\nBuild auth UI."
                .to_string(),
        },
        attach_target: None,
        plan: DecompositionPlan {
            root: planned_node_with_scope(
                "root",
                "Ship auth plan",
                vec!["docs/plans/auth.md"],
                vec![],
                vec![
                    planned_node_with_scope(
                        "backend",
                        "Backend auth",
                        vec!["crates/ralph/src/backend.rs"],
                        vec![],
                        vec![],
                    ),
                    planned_node("ui", "Auth UI", vec![], vec![]),
                ],
            ),
            warnings: vec![],
            total_nodes: 3,
            leaf_nodes: 2,
            dependency_edges: vec![],
        },
        write_blockers: vec![],
        child_status: TaskStatus::Draft,
        parent_status: TaskStatus::Draft,
        leaf_status: TaskStatus::Draft,
        child_policy: DecompositionChildPolicy::Fail,
        with_dependencies: false,
    };

    let result = write_task_decomposition(&resolved, &preview, false)?;
    assert_eq!(result.created_ids.len(), 3);

    let queue_file = queue::load_queue(&resolved.queue_path)?;
    assert_eq!(queue_file.tasks.len(), 3);
    for task in &queue_file.tasks {
        assert_eq!(
            task.request.as_deref(),
            Some("Plan file docs/plans/auth.md")
        );
        assert_eq!(
            task.evidence,
            vec![format!(
                "path: docs/plans/auth.md :: {} :: source plan for this decomposed task",
                task.title
            )]
        );
        assert_eq!(
            task.scope
                .iter()
                .filter(|item| item.as_str() == "docs/plans/auth.md")
                .count(),
            1
        );
    }

    let backend = queue_file
        .tasks
        .iter()
        .find(|task| task.title == "Backend auth")
        .expect("backend task");
    assert!(
        backend
            .scope
            .iter()
            .any(|item| item == "crates/ralph/src/backend.rs")
    );
    Ok(())
}

pub(super) fn planned_node(
    key: &str,
    title: &str,
    depends_on_keys: Vec<String>,
    children: Vec<PlannedNode>,
) -> PlannedNode {
    PlannedNode {
        planner_key: key.to_string(),
        title: title.to_string(),
        description: None,
        plan: vec![],
        tags: vec![],
        scope: vec![],
        depends_on_keys,
        children,
        dependency_refs: vec![],
    }
}

fn planned_node_with_scope(
    key: &str,
    title: &str,
    scope: Vec<&str>,
    depends_on_keys: Vec<String>,
    children: Vec<PlannedNode>,
) -> PlannedNode {
    planned_node_with_plan_and_scope(key, title, scope, vec![], depends_on_keys, children)
}

pub(super) fn planned_node_with_plan_and_scope(
    key: &str,
    title: &str,
    scope: Vec<&str>,
    plan: Vec<&str>,
    depends_on_keys: Vec<String>,
    children: Vec<PlannedNode>,
) -> PlannedNode {
    let mut node = planned_node(key, title, depends_on_keys, children);
    node.scope = scope.into_iter().map(str::to_string).collect();
    node.plan = plan.into_iter().map(str::to_string).collect();
    node
}

pub(super) fn test_resolved() -> Result<(TempDir, config::Resolved)> {
    let temp = TempDir::new()?;
    let repo_root = temp.path().to_path_buf();
    let ralph_dir = repo_root.join(".ralph");
    std::fs::create_dir_all(&ralph_dir)?;
    let config = Config::default();
    let resolved = config::Resolved {
        config,
        repo_root: repo_root.clone(),
        queue_path: ralph_dir.join("queue.jsonc"),
        done_path: ralph_dir.join("done.jsonc"),
        id_prefix: "RQ".to_string(),
        id_width: 4,
        global_config_path: None,
        project_config_path: Some(ralph_dir.join("config.jsonc")),
    };
    Ok((temp, resolved))
}

fn test_task(id: &str, title: &str, parent_id: Option<&str>) -> Task {
    Task {
        id: id.to_string(),
        title: title.to_string(),
        status: TaskStatus::Todo,
        kind: Default::default(),
        parent_id: parent_id.map(|value| value.to_string()),
        created_at: Some("2026-03-06T00:00:00Z".to_string()),
        updated_at: Some("2026-03-06T00:00:00Z".to_string()),
        ..Task::default()
    }
}
