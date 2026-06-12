//! Agent ledger document models and builders.
//!
//! Purpose:
//! - Own the compact JSON-friendly documents returned by `cueloop agent` commands.
//!
//! Responsibilities:
//! - Define agent overview, task, mutation, handoff, and validation document shapes.
//! - Build task summaries and handoff documents from queue tasks.
//! - Keep serialization details separate from task mutation and terminal rendering.
//!
//! Non-scope:
//! - Queue file mutation, locking, or persistence.
//! - Writing documents to stdout.
//! - Runner or machine-contract document envelopes.
//!
//! Invariants/assumptions:
//! - These documents are convenience output for agent-ledger commands, not the hidden machine contract.
//! - Location serialization must remain `active` / `done`.
//! - Handoff `next_step` is derived from the latest note prefixed with `Next: `.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::contracts::{Task, TaskStatus};
use crate::queue::operations::QueueTaskLocation;

pub(super) const DOCUMENT_VERSION: u32 = 1;
pub(super) const CLAIM_OWNER_KEY: &str = "agent_claim_owner";
pub(super) const CLAIMED_AT_KEY: &str = "agent_claimed_at";
pub(super) const CLAIM_EXPIRES_AT_KEY: &str = "agent_claim_expires_at";
pub(super) const HANDOFF_NEXT_PREFIX: &str = "Next: ";

#[derive(Debug, Clone, Serialize)]
pub(super) struct AgentOverviewDocument {
    pub(super) version: u32,
    pub(super) active_count: usize,
    pub(super) done_count: usize,
    pub(super) next_runnable_task_id: Option<String>,
    pub(super) doing: Vec<AgentTaskSummary>,
    pub(super) blocked: Vec<AgentBlockedTask>,
    pub(super) active: Vec<AgentTaskSummary>,
    pub(super) recent_done: Vec<AgentTaskSummary>,
    pub(super) validation_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct AgentTaskDocument {
    pub(super) version: u32,
    pub(super) task_id: String,
    pub(super) location: AgentTaskLocation,
    pub(super) task: Task,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct AgentMutationDocument {
    pub(super) version: u32,
    pub(super) task_id: String,
    pub(super) action: String,
    pub(super) task: Option<Task>,
    pub(super) archived: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct AgentHandoffDocument {
    pub(super) version: u32,
    pub(super) task_id: String,
    pub(super) location: AgentTaskLocation,
    pub(super) task: Task,
    pub(super) claim: BTreeMap<String, String>,
    pub(super) recent_notes: Vec<String>,
    pub(super) evidence: Vec<String>,
    pub(super) remaining_plan: Vec<String>,
    pub(super) next_step: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AgentTaskLocation {
    Active,
    Done,
}

impl From<QueueTaskLocation> for AgentTaskLocation {
    fn from(value: QueueTaskLocation) -> Self {
        match value {
            QueueTaskLocation::Active => Self::Active,
            QueueTaskLocation::Done => Self::Done,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct AgentValidateDocument {
    pub(super) version: u32,
    pub(super) valid: bool,
    pub(super) warnings: Vec<String>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct AgentTaskSummary {
    pub(super) id: String,
    pub(super) status: TaskStatus,
    pub(super) title: String,
    pub(super) priority: String,
    pub(super) tags: Vec<String>,
    pub(super) scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct AgentBlockedTask {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) reasons: serde_json::Value,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum AgentAppendField {
    Notes,
    Evidence,
    Plan,
}

pub(super) fn build_task_document(
    located: crate::queue::operations::LocatedTask,
) -> AgentTaskDocument {
    AgentTaskDocument {
        version: DOCUMENT_VERSION,
        task_id: located.task.id.clone(),
        location: located.location.into(),
        task: located.task,
    }
}

pub(super) fn build_mutation_document(
    action: &str,
    task: &Task,
    archived: bool,
) -> AgentMutationDocument {
    AgentMutationDocument {
        version: DOCUMENT_VERSION,
        task_id: task.id.clone(),
        action: action.to_string(),
        task: Some(task.clone()),
        archived,
    }
}

pub(super) fn build_handoff_document(
    location: QueueTaskLocation,
    task: Task,
) -> AgentHandoffDocument {
    let claim = [CLAIM_OWNER_KEY, CLAIMED_AT_KEY, CLAIM_EXPIRES_AT_KEY]
        .into_iter()
        .filter_map(|key| {
            task.custom_fields
                .get(key)
                .map(|value| (key.to_string(), value.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let next_step = task.notes.iter().rev().find_map(|note| {
        note.strip_prefix(HANDOFF_NEXT_PREFIX)
            .map(ToOwned::to_owned)
    });
    AgentHandoffDocument {
        version: DOCUMENT_VERSION,
        task_id: task.id.clone(),
        location: location.into(),
        recent_notes: task.notes.iter().rev().take(5).cloned().collect(),
        evidence: task.evidence.clone(),
        remaining_plan: task.plan.clone(),
        next_step,
        claim,
        task,
    }
}

pub(super) fn task_summary(task: &Task) -> AgentTaskSummary {
    AgentTaskSummary {
        id: task.id.clone(),
        status: task.status,
        title: task.title.clone(),
        priority: task.priority.as_str().to_string(),
        tags: task.tags.clone(),
        scope: task.scope.clone(),
    }
}
