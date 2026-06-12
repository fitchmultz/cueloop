//! Agent ledger task mutations.
//!
//! Purpose:
//! - Own state-changing operations behind `cueloop agent` commands.
//!
//! Responsibilities:
//! - Apply claims, releases, non-terminal lifecycle updates, notes, evidence, plans, handoffs, and terminal archive moves.
//! - Reuse queue operation helpers for active-task lookup, validation, lifecycle fields, and archive transitions.
//! - Return updated tasks to command routing without rendering them.
//!
//! Non-scope:
//! - Building overview/show/handoff documents for read-only commands.
//! - Printing human or JSON output.
//! - Runner dispatch or machine contract parsing.
//!
//! Invariants/assumptions:
//! - Mutating commands only target active queue tasks and run inside queue mutation locks.
//! - Completion requires caller-provided evidence before the done archive is changed.
//! - Claim metadata lives in task custom fields under stable `agent_*` keys.

use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::cli::agent_ledger::{
    AgentClaimArgs, AgentCompleteArgs, AgentHandoffArgs, AgentProgressArgs, AgentRejectArgs,
    AgentReleaseArgs, AgentTextArgs,
};
use crate::config;
use crate::contracts::{Task, TaskStatus};
use crate::queue;
use crate::queue::operations::{LifecycleNoteMode, append_redacted_items, apply_lifecycle_update};
use crate::timeutil;

use super::documents::{
    AgentAppendField, CLAIM_EXPIRES_AT_KEY, CLAIM_OWNER_KEY, CLAIMED_AT_KEY, HANDOFF_NEXT_PREFIX,
};

pub(super) fn claim(
    resolved: &config::Resolved,
    args: &AgentClaimArgs,
    force: bool,
) -> Result<Task> {
    if args.owner.trim().is_empty() {
        bail!("claim owner cannot be empty");
    }
    let mut patch = BTreeMap::new();
    let now = timeutil::now_utc_rfc3339()?;
    patch.insert(CLAIM_OWNER_KEY.to_string(), args.owner.trim().to_string());
    patch.insert(CLAIMED_AT_KEY.to_string(), now.clone());
    let ttl_minutes = args.ttl_minutes;
    if let Some(minutes) = ttl_minutes {
        let now_dt = timeutil::parse_rfc3339(&now)?;
        let expires = now_dt + time::Duration::minutes(i64::from(minutes));
        patch.insert(
            CLAIM_EXPIRES_AT_KEY.to_string(),
            timeutil::format_rfc3339(expires)?,
        );
    }
    let owner = args.owner.trim().to_string();
    mutate_active_task(
        resolved,
        &args.task_id,
        "agent claim",
        force,
        |task, updated_at| {
            if !force {
                ensure_claim_available(task, &owner, &now)?;
            }
            for (key, value) in patch {
                task.custom_fields.insert(key, value);
            }
            if ttl_minutes.is_none() {
                task.custom_fields.remove(CLAIM_EXPIRES_AT_KEY);
            }
            task.updated_at = Some(updated_at.to_string());
            Ok(())
        },
    )
}

fn ensure_claim_available(task: &Task, owner: &str, now: &str) -> Result<()> {
    let Some(current_owner) = task.custom_fields.get(CLAIM_OWNER_KEY) else {
        return Ok(());
    };
    if current_owner == owner {
        return Ok(());
    }
    if let Some(expires_at) = task.custom_fields.get(CLAIM_EXPIRES_AT_KEY)
        && let (Ok(expires), Ok(now)) = (
            timeutil::parse_rfc3339(expires_at),
            timeutil::parse_rfc3339(now),
        )
        && expires <= now
    {
        return Ok(());
    }
    bail!(
        "task {} is already claimed by '{}'; use --force to replace the claim",
        task.id,
        current_owner
    )
}

pub(super) fn release(
    resolved: &config::Resolved,
    args: &AgentReleaseArgs,
    force: bool,
) -> Result<Task> {
    mutate_active_task(
        resolved,
        &args.task_id,
        "agent release",
        force,
        |task, now| {
            if let Some(owner) = args.owner.as_ref() {
                let current = task
                    .custom_fields
                    .get(CLAIM_OWNER_KEY)
                    .map(String::as_str)
                    .unwrap_or_default();
                if current != owner.trim() {
                    bail!(
                        "task {} is claimed by '{}', not '{}'",
                        task.id,
                        current,
                        owner.trim()
                    );
                }
            }
            task.custom_fields.remove(CLAIM_OWNER_KEY);
            task.custom_fields.remove(CLAIMED_AT_KEY);
            task.custom_fields.remove(CLAIM_EXPIRES_AT_KEY);
            task.updated_at = Some(now.to_string());
            Ok(())
        },
    )
}

pub(super) fn start(
    resolved: &config::Resolved,
    args: &AgentProgressArgs,
    force: bool,
) -> Result<Task> {
    mutate_active_task(
        resolved,
        &args.task_id,
        "agent start",
        force,
        |task, now| {
            apply_lifecycle_update(
                task,
                TaskStatus::Doing,
                now,
                &args.notes,
                &args.evidence,
                LifecycleNoteMode::Items,
            )
        },
    )
}

pub(super) fn append_text(
    resolved: &config::Resolved,
    args: &AgentTextArgs,
    field: AgentAppendField,
    force: bool,
) -> Result<Task> {
    let action = match field {
        AgentAppendField::Notes => "agent note",
        AgentAppendField::Evidence => "agent evidence",
        AgentAppendField::Plan => "agent plan append",
    };
    mutate_active_task(resolved, &args.task_id, action, force, |task, now| {
        let value = crate::redaction::redact_text(args.text.trim());
        if value.trim().is_empty() {
            bail!("{} text cannot be empty", action);
        }
        match field {
            AgentAppendField::Notes => task.notes.push(value),
            AgentAppendField::Evidence => task.evidence.push(value),
            AgentAppendField::Plan => task.plan.push(value),
        }
        task.updated_at = Some(now.to_string());
        Ok(())
    })
}

pub(super) fn handoff(
    resolved: &config::Resolved,
    args: &AgentHandoffArgs,
    force: bool,
) -> Result<()> {
    if args.notes.is_empty() && args.next.is_none() {
        return Ok(());
    }
    mutate_active_task(
        resolved,
        &args.task_id,
        "agent handoff",
        force,
        |task, now| {
            append_redacted_items(&mut task.notes, &args.notes);
            if let Some(next) = args.next.as_ref() {
                let next = crate::redaction::redact_text(next.trim());
                if !next.trim().is_empty() {
                    task.notes.push(format!("{HANDOFF_NEXT_PREFIX}{next}"));
                }
            }
            task.updated_at = Some(now.to_string());
            Ok(())
        },
    )?;
    Ok(())
}

pub(super) fn complete(
    resolved: &config::Resolved,
    args: &AgentCompleteArgs,
    force: bool,
) -> Result<Task> {
    if args.evidence.iter().all(|item| item.trim().is_empty()) {
        bail!("completion requires at least one non-empty --evidence value");
    }
    complete_to_archive(
        resolved,
        &args.task_id,
        TaskStatus::Done,
        "agent complete",
        &args.notes,
        &args.evidence,
        force,
    )
}

pub(super) fn reject(
    resolved: &config::Resolved,
    args: &AgentRejectArgs,
    force: bool,
) -> Result<Task> {
    let reason = args.reason.trim();
    if reason.is_empty() {
        bail!("rejection reason cannot be empty");
    }
    let notes = vec![format!("Rejected: {reason}")];
    complete_to_archive(
        resolved,
        &args.task_id,
        TaskStatus::Rejected,
        "agent reject",
        &notes,
        &[],
        force,
    )
}

fn complete_to_archive(
    resolved: &config::Resolved,
    task_id: &str,
    status: TaskStatus,
    operation: &str,
    notes: &[String],
    evidence: &[String],
    force: bool,
) -> Result<Task> {
    queue::with_locked_queue_mutation(
        resolved,
        operation,
        format!("{operation} {task_id}"),
        force,
        || {
            queue::operations::complete_active_task_to_archive(
                &resolved.queue_path,
                &resolved.done_path,
                task_id,
                status,
                notes,
                evidence,
                &resolved.id_prefix,
                resolved.id_width,
                resolved.queue_max_dependency_depth(),
            )
        },
    )
}

fn mutate_active_task(
    resolved: &config::Resolved,
    task_id: &str,
    operation: &str,
    force: bool,
    mutate: impl FnOnce(&mut Task, &str) -> Result<()>,
) -> Result<Task> {
    queue::with_locked_queue_mutation(
        resolved,
        operation,
        format!("{operation} {task_id}"),
        force,
        || {
            queue::operations::mutate_active_task_on_disk(
                &resolved.queue_path,
                &resolved.done_path,
                task_id,
                &resolved.id_prefix,
                resolved.id_width,
                resolved.queue_max_dependency_depth(),
                mutate,
            )
        },
    )
}
