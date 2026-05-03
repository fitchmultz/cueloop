//! Purpose: Define atomic task-insert request and response contracts.
//!
//! Responsibilities:
//! - Define the versioned JSON request consumed by `cueloop task insert` and
//!   `cueloop machine task insert`.
//! - Define the stable document returned after lock-aware task insertion.
//!
//! Scope:
//! - Wire contracts only; queue locking and persistence live elsewhere.
//!
//! Usage:
//! - Used by both human CLI and machine-facing task insertion paths.
//!
//! Invariants/Assumptions:
//! - Requests omit durable task IDs; CueLoop allocates them while holding the
//!   queue lock.
//! - Created tasks are returned in request order.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Task, TaskAgent, TaskKind, TaskPriority, TaskStatus};

pub const TASK_INSERT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskInsertRequest {
    pub version: u32,
    #[serde(default)]
    pub tasks: Vec<TaskInsertSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskInsertSpec {
    pub key: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub priority: TaskPriority,
    #[serde(default)]
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "TaskKind::is_work_item")]
    pub kind: TaskKind,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,
    #[serde(default)]
    pub depends_on_keys: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub blocks: Vec<String>,
    #[serde(default)]
    pub relates_to: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicates: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub custom_fields: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<TaskAgent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskInsertDocument {
    pub version: u32,
    pub dry_run: bool,
    pub created_count: usize,
    #[serde(default)]
    pub tasks: Vec<TaskInsertCreatedTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskInsertCreatedTask {
    pub key: String,
    pub task: Task,
}
