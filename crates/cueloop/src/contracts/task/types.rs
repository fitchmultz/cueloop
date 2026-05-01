//! Purpose: Define serialized task and task-agent data models.
//!
//! Responsibilities:
//! - Define `Task`, `TaskStatus`, and `TaskAgent`.
//! - Attach serde and schemars annotations that define the task wire contract.
//!
//! Scope:
//! - Data models only; task priority behavior and serde/schema helper hooks
//!   live in sibling modules.
//!
//! Usage:
//! - Used across queue, CLI, app, and machine surfaces via `crate::contracts`.
//!
//! Invariants/Assumptions:
//! - Serde/schemars attributes are the source of truth for on-disk and
//!   machine-facing task contracts.
//! - Optional timestamps remain RFC3339 UTC strings when present.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::contracts::{
    Model, ModelEffort, PhaseOverrides, ReasoningEffort, Runner, RunnerCliOptionsPatch,
};

use super::priority::TaskPriority;
use super::serde_helpers::{
    custom_fields_schema, deserialize_custom_fields, model_effort_is_default, model_effort_schema,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,

    #[serde(default)]
    pub status: TaskStatus,

    /// Whether this task is executable work or a non-runnable grouping node.
    #[serde(default, skip_serializing_if = "TaskKind::is_work_item")]
    pub kind: TaskKind,

    pub title: String,

    /// Detailed description of the task's context, goal, purpose, and desired outcome.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default)]
    pub priority: TaskPriority,

    #[serde(default)]
    pub tags: Vec<String>,

    #[serde(default)]
    pub scope: Vec<String>,

    #[serde(default)]
    pub evidence: Vec<String>,

    #[serde(default)]
    pub plan: Vec<String>,

    #[serde(default)]
    pub notes: Vec<String>,

    /// Original human request that created the task (Task Builder / Scan).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,

    /// Optional per-task agent override (runner/model/model_effort/phases/iterations/phase_overrides).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<TaskAgent>,

    /// RFC3339 UTC timestamps as strings to keep the contract tool-agnostic.
    #[schemars(required)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[schemars(required)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,

    /// RFC3339 UTC timestamp when work on this task actually started.
    ///
    /// Invariants:
    /// - Must be RFC3339 UTC (Z) if set.
    /// - Should be set when transitioning into `doing` (see status policy).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,

    /// Estimated time to complete this task in minutes.
    /// Optional; used for planning and estimation accuracy tracking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,

    /// Actual time spent on this task in minutes.
    /// Optional; set manually or computed from started_at to completed_at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_minutes: Option<u32>,

    /// RFC3339 timestamp when the task should become runnable (optional scheduling).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_start: Option<String>,

    /// Task IDs that this task depends on (must be Done or Rejected before this task can run).
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Task IDs that this task blocks (must be Done/Rejected before blocked tasks can run).
    /// Semantically different from depends_on: blocks is "I prevent X" vs depends_on "I need X".
    #[serde(default)]
    pub blocks: Vec<String>,

    /// Task IDs that this task relates to (loose coupling, no execution constraint).
    /// Bidirectional awareness but no execution constraint.
    #[serde(default)]
    pub relates_to: Vec<String>,

    /// Task ID that this task duplicates (if any).
    /// Singular reference, not a list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicates: Option<String>,

    /// Custom user-defined fields (key-value pairs for extensibility).
    /// Values may be written as string/number/boolean; Ralph coerces them to strings when loading.
    #[serde(default, deserialize_with = "deserialize_custom_fields")]
    #[schemars(schema_with = "custom_fields_schema")]
    pub custom_fields: HashMap<String, String>,

    /// Parent task ID if this is a subtask (child-to-parent reference).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Executable atomic work item. This is the default for existing queues.
    #[default]
    WorkItem,
    /// Non-executable grouping/decomposition node.
    Group,
}

impl TaskKind {
    pub fn is_executable(self) -> bool {
        matches!(self, TaskKind::WorkItem)
    }

    pub fn is_work_item(&self) -> bool {
        matches!(self, TaskKind::WorkItem)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TaskKind::WorkItem => "work_item",
            TaskKind::Group => "group",
        }
    }
}

impl std::fmt::Display for TaskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Task {
    pub fn is_executable_work_item(&self) -> bool {
        self.kind.is_executable()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Draft,
    #[default]
    Todo,
    Doing,
    Done,
    Rejected,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Draft => "draft",
            TaskStatus::Todo => "todo",
            TaskStatus::Doing => "doing",
            TaskStatus::Done => "done",
            TaskStatus::Rejected => "rejected",
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct TaskAgent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner: Option<Runner>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<Model>,

    /// Per-task reasoning effort override for runners with reasoning controls. Default falls back to config.
    #[serde(default, skip_serializing_if = "model_effort_is_default")]
    #[schemars(schema_with = "model_effort_schema")]
    pub model_effort: ModelEffort,

    /// Number of execution phases for this task (1, 2, or 3), overriding config defaults.
    #[schemars(range(min = 1, max = 3))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phases: Option<u8>,

    /// Number of iterations to run for this task (overrides config).
    #[schemars(range(min = 1))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iterations: Option<u8>,

    /// Reasoning effort override for follow-up iterations (iterations > 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub followup_reasoning_effort: Option<ReasoningEffort>,

    /// Optional normalized runner CLI overrides for this task.
    ///
    /// This is intended to express runner behavior intent (output/approval/sandbox/etc)
    /// without embedding runner-specific flag syntax into the queue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_cli: Option<RunnerCliOptionsPatch>,

    /// Optional per-phase runner/model/effort overrides for this task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_overrides: Option<PhaseOverrides>,
}
