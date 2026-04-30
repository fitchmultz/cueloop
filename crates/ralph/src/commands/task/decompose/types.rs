//! Shared task decomposition data models.
//!
//! Purpose:
//! - Shared task decomposition data models.
//!
//! Responsibilities:
//! - Define the public preview/write types exposed to CLI and machine consumers.
//! - Hold planner-response parsing structs shared by normalization helpers.
//! - Keep decomposition-only internal state localized away from the facade module.
//!
//! Not handled here:
//! - Runner invocation, prompt rendering, or queue mutation logic.
//! - Tree normalization algorithms or task materialization helpers.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Serialized public types remain stable for current CLI and machine output contracts.
//! - Internal planner structs mirror the planner JSON schema with unknown fields rejected.

use crate::contracts::{Model, ReasoningEffort, Runner, Task, TaskKind, TaskStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecompositionChildPolicy {
    Fail,
    Append,
    Replace,
}

#[derive(Debug, Clone)]
pub struct TaskDecomposeOptions {
    pub source: TaskDecomposeSourceInput,
    pub attach_to_task_id: Option<String>,
    pub max_depth: u8,
    pub max_children: usize,
    pub max_nodes: usize,
    pub status: TaskStatus,
    pub parent_status: TaskStatus,
    pub leaf_status: TaskStatus,
    pub child_policy: DecompositionChildPolicy,
    pub with_dependencies: bool,
    pub runner_override: Option<Runner>,
    pub model_override: Option<Model>,
    pub reasoning_effort_override: Option<ReasoningEffort>,
    pub runner_cli_overrides: crate::contracts::RunnerCliOptionsPatch,
    pub repoprompt_tool_injection: bool,
}

#[derive(Debug, Clone)]
pub enum TaskDecomposeSourceInput {
    Inline(String),
    PlanFile { path: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DecompositionSource {
    Freeform {
        request: String,
    },
    ExistingTask {
        task: Box<Task>,
    },
    PlanFile {
        path: String,
        #[serde(skip_serializing, default)]
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionAttachTarget {
    pub task: Box<Task>,
    pub has_existing_children: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionPreview {
    pub source: DecompositionSource,
    pub attach_target: Option<DecompositionAttachTarget>,
    pub plan: DecompositionPlan,
    pub write_blockers: Vec<String>,
    pub child_status: TaskStatus,
    pub parent_status: TaskStatus,
    pub leaf_status: TaskStatus,
    pub child_policy: DecompositionChildPolicy,
    pub with_dependencies: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DecompositionPlan {
    pub root: PlannedNode,
    pub warnings: Vec<String>,
    pub total_nodes: usize,
    pub leaf_nodes: usize,
    pub dependency_edges: Vec<DependencyEdgePreview>,
}

impl DecompositionPreview {
    pub fn all_generated_tasks_draft(&self) -> bool {
        self.leaf_status == TaskStatus::Draft
            && (!self.plan.has_generated_group_nodes() || self.parent_status == TaskStatus::Draft)
    }

    pub fn has_runnable_generated_leaf(&self) -> bool {
        self.leaf_status == TaskStatus::Todo
    }
}

impl DecompositionPlan {
    fn has_generated_group_nodes(&self) -> bool {
        self.total_nodes > self.leaf_nodes
    }
}

impl DecompositionPlan {
    pub fn actionability(&self) -> DecompositionActionabilitySummary {
        let root_kind = kind_for_planned_node(&self.root);
        let first_leaf = first_leaf_node(&self.root);
        DecompositionActionabilitySummary {
            root_group: DecompositionTaskLocator {
                planner_key: Some(self.root.planner_key.clone()),
                task_id: None,
                title: self.root.title.clone(),
                kind: root_kind,
            },
            first_actionable_leaf: Some(DecompositionTaskLocator {
                planner_key: Some(first_leaf.planner_key.clone()),
                task_id: None,
                title: first_leaf.title.clone(),
                kind: TaskKind::WorkItem,
            }),
        }
    }
}

impl Serialize for DecompositionPlan {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("DecompositionPlan", 6)?;
        state.serialize_field("root", &self.root)?;
        state.serialize_field("warnings", &self.warnings)?;
        state.serialize_field("total_nodes", &self.total_nodes)?;
        state.serialize_field("leaf_nodes", &self.leaf_nodes)?;
        state.serialize_field("dependency_edges", &self.dependency_edges)?;
        state.serialize_field("actionability", &self.actionability())?;
        state.end()
    }
}

fn kind_for_planned_node(node: &PlannedNode) -> TaskKind {
    if node.children.is_empty() {
        TaskKind::WorkItem
    } else {
        TaskKind::Group
    }
}

fn first_leaf_node(node: &PlannedNode) -> &PlannedNode {
    node.children.first().map(first_leaf_node).unwrap_or(node)
}

#[derive(Debug, Clone, Serialize)]
pub struct DecompositionActionabilitySummary {
    pub root_group: DecompositionTaskLocator,
    pub first_actionable_leaf: Option<DecompositionTaskLocator>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecompositionTaskLocator {
    pub planner_key: Option<String>,
    pub task_id: Option<String>,
    pub title: String,
    pub kind: TaskKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdgePreview {
    pub task_title: String,
    pub depends_on_title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskDecomposeWriteResult {
    pub root_task_id: Option<String>,
    pub root_group_task_id: Option<String>,
    pub first_actionable_leaf_task_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub created_ids: Vec<String>,
    pub replaced_ids: Vec<String>,
    pub parent_annotated: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawDecompositionResponse {
    #[serde(default)]
    pub(super) warnings: Vec<String>,
    pub(super) tree: RawPlannedNode,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawPlannedNode {
    #[serde(default)]
    pub(super) key: Option<String>,
    pub(super) title: String,
    #[serde(default)]
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) plan: Vec<String>,
    #[serde(default)]
    pub(super) tags: Vec<String>,
    #[serde(default)]
    pub(super) scope: Vec<String>,
    #[serde(default)]
    pub(super) depends_on: Vec<String>,
    #[serde(default)]
    pub(super) children: Vec<RawPlannedNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedNode {
    pub planner_key: String,
    pub title: String,
    pub description: Option<String>,
    pub plan: Vec<String>,
    pub tags: Vec<String>,
    pub scope: Vec<String>,
    pub depends_on_keys: Vec<String>,
    pub children: Vec<PlannedNode>,
    #[serde(skip_serializing, default)]
    pub(crate) dependency_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceKind {
    Freeform,
    ExistingTask,
    PlanFile,
}

pub(super) struct PlannerState {
    pub(super) remaining_nodes: usize,
    pub(super) warnings: Vec<String>,
    pub(super) with_dependencies: bool,
}
