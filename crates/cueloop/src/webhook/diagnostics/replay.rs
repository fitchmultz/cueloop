//! Purpose: Select, report, and replay persisted webhook delivery failures.
//!
//! Responsibilities:
//! - Define replay selector and reporting types.
//! - Filter persisted failure records into bounded replay candidates.
//! - Execute explicit replay requests through the normal enqueue path.
//!
//! Scope:
//! - Replay validation, candidate selection, dry-run reporting, and live replay execution only.
//!
//! Usage:
//! - Called by CLI webhook replay commands through the diagnostics facade.
//!
//! Invariants/Assumptions:
//! - Replay is always explicit and bounded by caller-provided selectors or limits.
//! - Dry-run replay never mutates persisted replay counters.
//! - Live replay increments replay counts only for successfully enqueued records.

use super::super::{WebhookPayload, enqueue_webhook_payload_for_replay, resolve_webhook_config};
use super::failure_store::{failure_store_path, load_failure_records, update_replay_counts};
use crate::contracts::WebhookConfig;
use anyhow::{Result, bail};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ReplaySelector {
    pub ids: Vec<String>,
    pub event: Option<String>,
    pub task_id: Option<String>,
    pub limit: usize,
    pub max_replay_attempts: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayCandidate {
    pub id: String,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub failed_at: String,
    pub attempts: u32,
    pub replay_count: u32,
    pub error: String,
    pub eligible_for_replay: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayReport {
    pub dry_run: bool,
    pub matched_count: usize,
    pub eligible_count: usize,
    pub replayed_count: usize,
    pub skipped_max_replay_attempts: usize,
    pub skipped_enqueue_failures: usize,
    pub candidates: Vec<ReplayCandidate>,
}

#[derive(Debug, Clone)]
struct SelectedReplayRecord {
    id: String,
    replay_count: u32,
    payload: WebhookPayload,
}

pub fn replay_failed_deliveries(
    repo_root: &Path,
    config: &WebhookConfig,
    selector: &ReplaySelector,
    dry_run: bool,
) -> Result<ReplayReport> {
    if selector.ids.is_empty() && selector.event.is_none() && selector.task_id.is_none() {
        bail!(
            "refusing unbounded replay (would redeliver all failures)\n\
             this could overwhelm external systems or re-trigger side effects\n\
             \n\
             examples:\n\
             \n\
             ralph webhook replay --id <failure-id>     # replay specific failure\n\
             ralph webhook replay --event task.done     # replay all task.done failures\n\
             ralph webhook replay --task-id RQ-0001     # replay failures for a task\n\
             ralph webhook replay --id a,b,c --limit 5  # replay up to 5 specific failures"
        );
    }

    if selector.max_replay_attempts == 0 {
        bail!("max_replay_attempts must be greater than 0");
    }

    if !dry_run {
        let resolved = resolve_webhook_config(config);
        if !resolved.enabled {
            bail!("Webhook replay requires agent.webhook.enabled=true");
        }
        if resolved
            .url
            .as_deref()
            .is_none_or(|url| url.trim().is_empty())
        {
            bail!("Webhook replay requires agent.webhook.url to be configured");
        }
    }

    let path = failure_store_path(repo_root);
    let records = load_failure_records(&path)?;
    let limit = if selector.limit == 0 {
        usize::MAX
    } else {
        selector.limit
    };
    let id_filter = selector
        .ids
        .iter()
        .map(std::string::String::as_str)
        .collect::<HashSet<_>>();

    let mut selected_records = Vec::new();
    let mut candidates = Vec::new();

    for record in records.iter().rev() {
        if selected_records.len() >= limit {
            break;
        }
        if !id_filter.is_empty() && !id_filter.contains(record.id.as_str()) {
            continue;
        }
        if let Some(event_filter) = selector.event.as_deref()
            && record.event != event_filter
        {
            continue;
        }
        if let Some(task_filter) = selector.task_id.as_deref()
            && record.task_id.as_deref() != Some(task_filter)
        {
            continue;
        }

        let eligible = record.replay_count < selector.max_replay_attempts;
        candidates.push(ReplayCandidate {
            id: record.id.clone(),
            event: record.event.clone(),
            task_id: record.task_id.clone(),
            failed_at: record.failed_at.clone(),
            attempts: record.attempts,
            replay_count: record.replay_count,
            error: record.error.clone(),
            eligible_for_replay: eligible,
        });
        selected_records.push(SelectedReplayRecord {
            id: record.id.clone(),
            replay_count: record.replay_count,
            payload: record.payload.clone(),
        });
    }

    let matched_count = candidates.len();
    let eligible_count = candidates
        .iter()
        .filter(|candidate| candidate.eligible_for_replay)
        .count();

    if dry_run {
        return Ok(ReplayReport {
            dry_run,
            matched_count,
            eligible_count,
            replayed_count: 0,
            skipped_max_replay_attempts: matched_count.saturating_sub(eligible_count),
            skipped_enqueue_failures: 0,
            candidates,
        });
    }

    let mut replayed_count = 0usize;
    let mut skipped_max_replay_attempts = 0usize;
    let mut skipped_enqueue_failures = 0usize;
    let mut replayed_ids = Vec::new();

    for record in selected_records {
        if record.replay_count >= selector.max_replay_attempts {
            skipped_max_replay_attempts += 1;
            continue;
        }

        if enqueue_webhook_payload_for_replay(record.payload, config) {
            replayed_ids.push(record.id);
            replayed_count += 1;
        } else {
            skipped_enqueue_failures += 1;
        }
    }

    if !replayed_ids.is_empty() {
        update_replay_counts(&path, &replayed_ids)?;
    }

    Ok(ReplayReport {
        dry_run,
        matched_count,
        eligible_count,
        replayed_count,
        skipped_max_replay_attempts,
        skipped_enqueue_failures,
        candidates,
    })
}
