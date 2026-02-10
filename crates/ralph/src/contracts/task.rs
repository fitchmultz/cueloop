//! Task contracts for Ralph queue entries.
//!
//! Responsibilities:
//! - Define task payloads, enums, and schema helpers.
//! - Provide ordering/cycling helpers for task priority.
//!
//! Not handled here:
//! - Queue ordering or persistence logic (see `crate::queue`).
//! - Config contract definitions (see `super::config`).
//!
//! Invariants/assumptions:
//! - Serde/schemars attributes define the task wire contract.
//! - Task priority ordering is critical > high > medium > low.

use anyhow::{Result, bail};
use schemars::JsonSchema;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;

use super::RunnerCliOptionsPatch;
use super::{Model, ModelEffort, PhaseOverrides, ReasoningEffort, Runner};

/* ------------------------------ Task (JSON) ------------------------------ */

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,

    #[serde(default)]
    pub status: TaskStatus,

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
pub enum TaskStatus {
    Draft,
    #[default]
    Todo,
    Doing,
    Done,
    Rejected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Critical,
    High,
    #[default]
    Medium,
    Low,
}

// Custom PartialOrd implementation: Critical > High > Medium > Low
impl PartialOrd for TaskPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Custom Ord implementation: Critical > High > Medium > Low (semantically)
// Higher priority = Greater in comparison, so Critical > High > Medium > Low
impl Ord for TaskPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare by weight: higher weight = higher priority = Greater
        self.weight().cmp(&other.weight())
    }
}

impl TaskPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskPriority::Critical => "critical",
            TaskPriority::High => "high",
            TaskPriority::Medium => "medium",
            TaskPriority::Low => "low",
        }
    }

    pub fn weight(self) -> u8 {
        match self {
            TaskPriority::Critical => 3,
            TaskPriority::High => 2,
            TaskPriority::Medium => 1,
            TaskPriority::Low => 0,
        }
    }

    /// Cycle to the next priority in ascending order, wrapping after Critical.
    pub fn cycle(self) -> Self {
        match self {
            TaskPriority::Low => TaskPriority::Medium,
            TaskPriority::Medium => TaskPriority::High,
            TaskPriority::High => TaskPriority::Critical,
            TaskPriority::Critical => TaskPriority::Low,
        }
    }
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TaskPriority {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let token = value.trim();

        if token.eq_ignore_ascii_case("critical") {
            return Ok(TaskPriority::Critical);
        }
        if token.eq_ignore_ascii_case("high") {
            return Ok(TaskPriority::High);
        }
        if token.eq_ignore_ascii_case("medium") {
            return Ok(TaskPriority::Medium);
        }
        if token.eq_ignore_ascii_case("low") {
            return Ok(TaskPriority::Low);
        }

        bail!(
            "Invalid priority: '{}'. Expected one of: critical, high, medium, low.",
            token
        )
    }
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

    /// Per-task reasoning effort override for Codex models. Default falls back to config.
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

fn model_effort_is_default(value: &ModelEffort) -> bool {
    matches!(value, ModelEffort::Default)
}

fn model_effort_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    let mut schema = <ModelEffort as JsonSchema>::json_schema(generator);
    schema
        .ensure_object()
        .insert("default".to_string(), json!("default"));
    schema
}

/// Custom deserializer for `custom_fields` that coerces scalar values (string/number/bool)
/// to strings, while rejecting null, arrays, and objects with descriptive errors.
fn deserialize_custom_fields<'de, D>(deserializer: D) -> Result<HashMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: serde_json::Value = serde_json::Value::deserialize(deserializer)?;
    let raw = match value {
        serde_json::Value::Object(map) => map,
        serde_json::Value::Null => {
            return Err(de::Error::custom(
                "custom_fields must be an object (map); null is not allowed",
            ));
        }
        other => {
            return Err(de::Error::custom(format!(
                "custom_fields must be an object (map); got {}",
                other
            )));
        }
    };

    raw.into_iter()
        .map(|(k, v)| {
            let s = match v {
                serde_json::Value::String(s) => s,
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => {
                    return Err(de::Error::custom(format!(
                        "custom_fields['{}'] must be a string/number/boolean (null is not allowed)",
                        k
                    )));
                }
                serde_json::Value::Array(_) => {
                    return Err(de::Error::custom(format!(
                        "custom_fields['{}'] must be a scalar (string/number/boolean); arrays are not allowed",
                        k
                    )));
                }
                serde_json::Value::Object(_) => {
                    return Err(de::Error::custom(format!(
                        "custom_fields['{}'] must be a scalar (string/number/boolean); objects are not allowed",
                        k
                    )));
                }
            };
            Ok((k, s))
        })
        .collect()
}

/// Schema generator for `custom_fields` that accepts string/number/boolean values.
fn custom_fields_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "object",
        "description": "Custom user-defined fields. Values may be written as string/number/boolean; Ralph coerces them to strings when loading the queue.",
        "additionalProperties": {
            "anyOf": [
                {"type": "string"},
                {"type": "number"},
                {"type": "boolean"}
            ]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{Task, TaskPriority};
    use crate::contracts::{Model, PhaseOverrideConfig, PhaseOverrides, ReasoningEffort, Runner};
    use std::collections::HashMap;

    #[test]
    fn task_priority_cycle_wraps_through_all_values() {
        assert_eq!(TaskPriority::Low.cycle(), TaskPriority::Medium);
        assert_eq!(TaskPriority::Medium.cycle(), TaskPriority::High);
        assert_eq!(TaskPriority::High.cycle(), TaskPriority::Critical);
        assert_eq!(TaskPriority::Critical.cycle(), TaskPriority::Low);
    }

    #[test]
    fn task_priority_from_str_is_case_insensitive_and_trims() {
        assert_eq!("HIGH".parse::<TaskPriority>().unwrap(), TaskPriority::High);
        assert_eq!(
            "Medium".parse::<TaskPriority>().unwrap(),
            TaskPriority::Medium
        );
        assert_eq!(" low ".parse::<TaskPriority>().unwrap(), TaskPriority::Low);
        assert_eq!(
            "CRITICAL".parse::<TaskPriority>().unwrap(),
            TaskPriority::Critical
        );
    }

    #[test]
    fn task_priority_from_str_invalid_has_canonical_error_message() {
        let err = "nope".parse::<TaskPriority>().unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid priority: 'nope'. Expected one of: critical, high, medium, low."
        );
    }

    #[test]
    fn task_priority_from_str_empty_string_errors() {
        let err = "".parse::<TaskPriority>().unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid priority: ''. Expected one of: critical, high, medium, low."
        );
    }

    #[test]
    fn task_custom_fields_deserialize_coerces_scalars_to_strings() {
        let raw = r#"{
            "id": "RQ-0001",
            "title": "t",
            "custom_fields": {
                "guide_line_count": 1411,
                "enabled": true,
                "owner": "ralph"
            }
        }"#;

        let task: Task = serde_json::from_str(raw).expect("deserialize");
        assert_eq!(
            task.custom_fields
                .get("guide_line_count")
                .map(String::as_str),
            Some("1411")
        );
        assert_eq!(
            task.custom_fields.get("enabled").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            task.custom_fields.get("owner").map(String::as_str),
            Some("ralph")
        );
    }

    #[test]
    fn task_custom_fields_deserialize_rejects_null() {
        let raw = r#"{"id":"RQ-0001","title":"t","custom_fields":{"x":null}}"#;
        let err = serde_json::from_str::<Task>(raw).unwrap_err();
        let err_msg = err.to_string().to_lowercase();
        assert!(
            err_msg.contains("custom_fields"),
            "error should mention custom_fields: {}",
            err_msg
        );
        assert!(
            err_msg.contains("null"),
            "error should mention null: {}",
            err_msg
        );
    }

    #[test]
    fn task_custom_fields_deserialize_rejects_custom_fields_null() {
        let raw = r#"{"id":"RQ-0001","title":"t","custom_fields":null}"#;
        let err = serde_json::from_str::<Task>(raw).unwrap_err();
        let err_msg = err.to_string().to_lowercase();
        assert!(
            err_msg.contains("custom_fields"),
            "error should mention custom_fields: {}",
            err_msg
        );
        assert!(
            err_msg.contains("null"),
            "error should mention null: {}",
            err_msg
        );
    }

    #[test]
    fn task_custom_fields_deserialize_rejects_custom_fields_non_object() {
        let raw = r#"{"id":"RQ-0001","title":"t","custom_fields":123}"#;
        let err = serde_json::from_str::<Task>(raw).unwrap_err();
        let err_msg = err.to_string().to_lowercase();
        assert!(
            err_msg.contains("custom_fields"),
            "error should mention custom_fields: {}",
            err_msg
        );
        assert!(
            err_msg.contains("object") || err_msg.contains("map"),
            "error should mention object/map: {}",
            err_msg
        );
    }

    #[test]
    fn task_custom_fields_deserialize_rejects_object_and_array_values() {
        let raw_obj = r#"{"id":"RQ-0001","title":"t","custom_fields":{"x":{"a":1}}}"#;
        let raw_arr = r#"{"id":"RQ-0001","title":"t","custom_fields":{"x":[1,2]}}"#;

        let err_obj = serde_json::from_str::<Task>(raw_obj).unwrap_err();
        let err_arr = serde_json::from_str::<Task>(raw_arr).unwrap_err();

        let err_obj_msg = err_obj.to_string().to_lowercase();
        let err_arr_msg = err_arr.to_string().to_lowercase();

        assert!(
            err_obj_msg.contains("custom_fields"),
            "object error should mention custom_fields: {}",
            err_obj_msg
        );
        assert!(
            err_arr_msg.contains("custom_fields"),
            "array error should mention custom_fields: {}",
            err_arr_msg
        );
    }

    #[test]
    fn task_custom_fields_serializes_as_strings() {
        let mut custom_fields = HashMap::new();
        custom_fields.insert("count".to_string(), "42".to_string());
        custom_fields.insert("enabled".to_string(), "true".to_string());

        let task = Task {
            id: "RQ-0001".to_string(),
            title: "Test".to_string(),
            custom_fields,
            ..Default::default()
        };

        let json = serde_json::to_string(&task).expect("serialize");
        assert!(json.contains("\"count\":\"42\""));
        assert!(json.contains("\"enabled\":\"true\""));
    }

    #[test]
    fn task_agent_deserializes_phases_and_phase_overrides() {
        let raw = r#"{
            "id":"RQ-0001",
            "title":"Task with agent overrides",
            "agent":{
                "runner":"codex",
                "model":"gpt-5.3-codex",
                "model_effort":"high",
                "phases":2,
                "iterations":1,
                "phase_overrides":{
                    "phase1":{"runner":"codex","model":"gpt-5.3-codex","reasoning_effort":"high"},
                    "phase2":{"runner":"kimi","model":"kimi-code/kimi-for-coding"}
                }
            }
        }"#;

        let task: Task = serde_json::from_str(raw).expect("deserialize");
        let agent = task.agent.expect("agent should be set");
        assert_eq!(agent.runner, Some(Runner::Codex));
        assert_eq!(agent.model, Some(Model::Gpt53Codex));
        assert_eq!(agent.phases, Some(2));
        assert_eq!(agent.iterations, Some(1));

        let phase_overrides = agent
            .phase_overrides
            .expect("phase overrides should be set");
        let phase1 = phase_overrides.phase1.expect("phase1 should be set");
        assert_eq!(phase1.runner, Some(Runner::Codex));
        assert_eq!(phase1.reasoning_effort, Some(ReasoningEffort::High));
        let phase2 = phase_overrides.phase2.expect("phase2 should be set");
        assert_eq!(phase2.runner, Some(Runner::Kimi));
    }

    #[test]
    fn task_agent_omits_default_phase_and_effort_fields_when_serializing() {
        let task = Task {
            id: "RQ-0001".to_string(),
            title: "Serialize defaults".to_string(),
            agent: Some(crate::contracts::TaskAgent {
                runner: Some(Runner::Codex),
                model: Some(Model::Gpt53Codex),
                model_effort: crate::contracts::ModelEffort::Default,
                phases: None,
                iterations: None,
                followup_reasoning_effort: None,
                runner_cli: None,
                phase_overrides: Some(PhaseOverrides {
                    phase1: Some(PhaseOverrideConfig {
                        runner: Some(Runner::Codex),
                        model: Some(Model::Gpt53Codex),
                        reasoning_effort: Some(ReasoningEffort::Medium),
                    }),
                    ..Default::default()
                }),
            }),
            ..Default::default()
        };

        let value = serde_json::to_value(task).expect("serialize");
        let agent = value
            .get("agent")
            .and_then(|v| v.as_object())
            .expect("agent object should exist");
        assert!(!agent.contains_key("model_effort"));
        assert!(!agent.contains_key("phases"));
        assert!(agent.contains_key("phase_overrides"));
    }
}
