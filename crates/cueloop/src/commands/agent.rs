//! Agent-ledger command implementation.
//!
//! Purpose:
//! - Implement `cueloop agent ...`, a no-runner task ledger surface for already-running agents.
//!
//! Responsibilities:
//! - Provide compact queue context, task handoff, claim, note, evidence, and lifecycle helpers.
//! - Reuse the canonical queue persistence, locking, validation, archive, and undo mechanisms.
//! - Keep agent-oriented ledger operations additive to existing human task/run capabilities.
//!
//! Non-scope:
//! - Dispatching runner CLIs, supervising phases, or replacing the human `task` / `queue` commands.
//!
//! Invariants/assumptions:
//! - Mutating commands only target active queue tasks and create undo snapshots.
//! - Completion requires explicit evidence from the caller.
//! - Serialized JSON documents are compact convenience output, not the hidden machine contract.

use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;

use crate::cli::agent_ledger::{
    AgentArgs, AgentClaimArgs, AgentCommand, AgentCompleteArgs, AgentFormatArgs, AgentHandoffArgs,
    AgentNextArgs, AgentOutputFormat, AgentOverviewArgs, AgentProgressArgs, AgentRejectArgs,
    AgentReleaseArgs, AgentTaskReadArgs, AgentTextArgs,
};
use crate::config;
use crate::contracts::{Task, TaskStatus};
use crate::queue;
use crate::queue::operations::{RunnableSelectionOptions, queue_runnability_report};
use crate::timeutil;

const DOCUMENT_VERSION: u32 = 1;
const CLAIM_OWNER_KEY: &str = "agent_claim_owner";
const CLAIMED_AT_KEY: &str = "agent_claimed_at";
const CLAIM_EXPIRES_AT_KEY: &str = "agent_claim_expires_at";
const HANDOFF_NEXT_PREFIX: &str = "Next: ";

pub fn handle(args: AgentArgs, force: bool) -> Result<()> {
    let resolved = config::resolve_from_cwd()?;
    match args.command {
        AgentCommand::Overview(args) => overview(&resolved, &args),
        AgentCommand::Next(args) => next(&resolved, &args),
        AgentCommand::Show(args) => show(&resolved, &args),
        AgentCommand::Claim(args) => claim(&resolved, &args, force),
        AgentCommand::Release(args) => release(&resolved, &args, force),
        AgentCommand::Start(args) => start(&resolved, &args, force),
        AgentCommand::Note(args) => append_text(&resolved, &args, AgentAppendField::Notes, force),
        AgentCommand::Evidence(args) => {
            append_text(&resolved, &args, AgentAppendField::Evidence, force)
        }
        AgentCommand::PlanAppend(args) => {
            append_text(&resolved, &args, AgentAppendField::Plan, force)
        }
        AgentCommand::Handoff(args) => handoff(&resolved, &args, force),
        AgentCommand::Complete(args) => complete(&resolved, &args, force),
        AgentCommand::Reject(args) => reject(&resolved, &args, force),
        AgentCommand::Validate(args) => validate(&resolved, &args),
    }
}

#[derive(Debug, Clone, Serialize)]
struct AgentOverviewDocument {
    version: u32,
    active_count: usize,
    done_count: usize,
    next_runnable_task_id: Option<String>,
    doing: Vec<AgentTaskSummary>,
    blocked: Vec<AgentBlockedTask>,
    active: Vec<AgentTaskSummary>,
    recent_done: Vec<AgentTaskSummary>,
    validation_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AgentTaskDocument {
    version: u32,
    task_id: String,
    location: AgentTaskLocation,
    task: Task,
}

#[derive(Debug, Clone, Serialize)]
struct AgentMutationDocument {
    version: u32,
    task_id: String,
    action: String,
    task: Option<Task>,
    archived: bool,
}

#[derive(Debug, Clone, Serialize)]
struct AgentHandoffDocument {
    version: u32,
    task_id: String,
    location: AgentTaskLocation,
    task: Task,
    claim: BTreeMap<String, String>,
    recent_notes: Vec<String>,
    evidence: Vec<String>,
    remaining_plan: Vec<String>,
    next_step: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AgentValidateDocument {
    version: u32,
    valid: bool,
    warnings: Vec<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AgentTaskSummary {
    id: String,
    status: TaskStatus,
    title: String,
    priority: String,
    tags: Vec<String>,
    scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AgentBlockedTask {
    id: String,
    title: String,
    reasons: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum AgentTaskLocation {
    Active,
    Done,
}

#[derive(Debug, Clone, Copy)]
enum AgentAppendField {
    Notes,
    Evidence,
    Plan,
}

fn overview(resolved: &config::Resolved, args: &AgentOverviewArgs) -> Result<()> {
    let active = queue::load_queue(&resolved.queue_path)?;
    let done = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = queue::optional_done_queue(&done, &resolved.done_path);
    let validation = queue::validate_queue_set(
        &active,
        done_ref,
        &resolved.id_prefix,
        resolved.id_width,
        resolved.queue_max_dependency_depth(),
    );

    let (next_runnable_task_id, blocked, validation_error) = match validation {
        Ok(_) => {
            let report = queue_runnability_report(
                &active,
                done_ref,
                RunnableSelectionOptions::new(false, true),
            )?;
            let next = queue::operations::next_runnable_task(&active, done_ref)
                .map(|task| task.id.clone());
            let blocked = report
                .tasks
                .into_iter()
                .filter(|row| !row.runnable && !row.reasons.is_empty())
                .filter_map(|row| {
                    let task = active.tasks.iter().find(|task| task.id == row.id)?;
                    let reasons = serde_json::to_value(row.reasons).ok()?;
                    Some(AgentBlockedTask {
                        id: task.id.clone(),
                        title: task.title.clone(),
                        reasons,
                    })
                })
                .collect();
            (next, blocked, None)
        }
        Err(err) => (None, Vec::new(), Some(err.to_string())),
    };

    let recent_done = if args.include_done {
        done.tasks
            .iter()
            .rev()
            .take(args.done_limit)
            .map(task_summary)
            .collect()
    } else {
        Vec::new()
    };

    let document = AgentOverviewDocument {
        version: DOCUMENT_VERSION,
        active_count: active.tasks.len(),
        done_count: done.tasks.len(),
        next_runnable_task_id,
        doing: active
            .tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Doing)
            .map(task_summary)
            .collect(),
        blocked,
        active: active.tasks.iter().map(task_summary).collect(),
        recent_done,
        validation_error,
    };

    print_overview(&document, args.format)
}

fn next(resolved: &config::Resolved, args: &AgentNextArgs) -> Result<()> {
    let active = queue::load_queue(&resolved.queue_path)?;
    let done = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = queue::optional_done_queue(&done, &resolved.done_path);
    queue::validate_queue_set(
        &active,
        done_ref,
        &resolved.id_prefix,
        resolved.id_width,
        resolved.queue_max_dependency_depth(),
    )?;
    let task = queue::operations::next_runnable_task(&active, done_ref).cloned();
    if args.format == AgentOutputFormat::Json {
        return print_json(&serde_json::json!({
            "version": DOCUMENT_VERSION,
            "task": task,
        }));
    }
    match task {
        Some(task) if args.with_title => {
            println!("{}\t{}", task.id, task.title);
        }
        Some(task) => println!("{}", task.id),
        None => println!("No runnable task."),
    }
    Ok(())
}

fn show(resolved: &config::Resolved, args: &AgentTaskReadArgs) -> Result<()> {
    let (location, task) = find_task(resolved, &args.task_id)?;
    let document = AgentTaskDocument {
        version: DOCUMENT_VERSION,
        task_id: task.id.clone(),
        location,
        task,
    };
    print_task_document(&document, args.format)
}

fn claim(resolved: &config::Resolved, args: &AgentClaimArgs, force: bool) -> Result<()> {
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
    let task = mutate_active_task(
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
    )?;
    print_mutation("claimed", &task, false, args.format)
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

fn release(resolved: &config::Resolved, args: &AgentReleaseArgs, force: bool) -> Result<()> {
    let task = mutate_active_task(
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
    )?;
    print_mutation("released", &task, false, args.format)
}

fn start(resolved: &config::Resolved, args: &AgentProgressArgs, force: bool) -> Result<()> {
    let task = mutate_active_task(
        resolved,
        &args.task_id,
        "agent start",
        force,
        |task, now| {
            crate::queue::apply_status_policy(task, TaskStatus::Doing, now, None)?;
            append_items(&mut task.notes, &args.notes);
            append_items(&mut task.evidence, &args.evidence);
            Ok(())
        },
    )?;
    print_mutation("started", &task, false, args.format)
}

fn append_text(
    resolved: &config::Resolved,
    args: &AgentTextArgs,
    field: AgentAppendField,
    force: bool,
) -> Result<()> {
    let action = match field {
        AgentAppendField::Notes => "agent note",
        AgentAppendField::Evidence => "agent evidence",
        AgentAppendField::Plan => "agent plan append",
    };
    let task = mutate_active_task(resolved, &args.task_id, action, force, |task, now| {
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
    })?;
    let label = match field {
        AgentAppendField::Notes => "noted",
        AgentAppendField::Evidence => "evidence_added",
        AgentAppendField::Plan => "plan_appended",
    };
    print_mutation(label, &task, false, args.format)
}

fn handoff(resolved: &config::Resolved, args: &AgentHandoffArgs, force: bool) -> Result<()> {
    if !args.notes.is_empty() || args.next.is_some() {
        mutate_active_task(
            resolved,
            &args.task_id,
            "agent handoff",
            force,
            |task, now| {
                append_items(&mut task.notes, &args.notes);
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
    }
    let (location, task) = find_task(resolved, &args.task_id)?;
    let document = build_handoff_document(location, task);
    print_handoff(&document, args.format)
}

fn complete(resolved: &config::Resolved, args: &AgentCompleteArgs, force: bool) -> Result<()> {
    if args.evidence.iter().all(|item| item.trim().is_empty()) {
        bail!("completion requires at least one non-empty --evidence value");
    }
    queue::with_locked_queue_mutation(
        resolved,
        "agent complete",
        format!("agent complete {}", args.task_id),
        force,
        || {
            let now = timeutil::now_utc_rfc3339()?;
            queue::complete_task(
                &resolved.queue_path,
                &resolved.done_path,
                &args.task_id,
                TaskStatus::Done,
                &now,
                &args.notes,
                &args.evidence,
                &resolved.id_prefix,
                resolved.id_width,
                resolved.queue_max_dependency_depth(),
                None,
            )
        },
    )?;
    let (_, task) = find_task(resolved, &args.task_id)?;
    print_mutation("completed", &task, true, args.format)
}

fn reject(resolved: &config::Resolved, args: &AgentRejectArgs, force: bool) -> Result<()> {
    let reason = args.reason.trim();
    if reason.is_empty() {
        bail!("rejection reason cannot be empty");
    }
    let notes = vec![format!("Rejected: {reason}")];
    queue::with_locked_queue_mutation(
        resolved,
        "agent reject",
        format!("agent reject {}", args.task_id),
        force,
        || {
            let now = timeutil::now_utc_rfc3339()?;
            queue::complete_task(
                &resolved.queue_path,
                &resolved.done_path,
                &args.task_id,
                TaskStatus::Rejected,
                &now,
                &notes,
                &[],
                &resolved.id_prefix,
                resolved.id_width,
                resolved.queue_max_dependency_depth(),
                None,
            )
        },
    )?;
    let (_, task) = find_task(resolved, &args.task_id)?;
    print_mutation("rejected", &task, true, args.format)
}

fn validate(resolved: &config::Resolved, args: &AgentFormatArgs) -> Result<()> {
    let active = queue::load_queue(&resolved.queue_path)?;
    let done = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = queue::optional_done_queue(&done, &resolved.done_path);
    let document = match queue::validate_queue_set(
        &active,
        done_ref,
        &resolved.id_prefix,
        resolved.id_width,
        resolved.queue_max_dependency_depth(),
    ) {
        Ok(warnings) => AgentValidateDocument {
            version: DOCUMENT_VERSION,
            valid: true,
            warnings: warnings
                .into_iter()
                .map(|warning| format!("[{}] {}", warning.task_id, warning.message))
                .collect(),
            error: None,
        },
        Err(err) => AgentValidateDocument {
            version: DOCUMENT_VERSION,
            valid: false,
            warnings: Vec::new(),
            error: Some(err.to_string()),
        },
    };

    if args.format == AgentOutputFormat::Json {
        print_json(&document)
    } else if document.valid {
        println!("Queue is valid.");
        for warning in &document.warnings {
            println!("warning: {warning}");
        }
        Ok(())
    } else {
        bail!(
            document
                .error
                .unwrap_or_else(|| "queue is invalid".to_string())
        )
    }
}

fn mutate_active_task(
    resolved: &config::Resolved,
    task_id: &str,
    operation: &str,
    force: bool,
    mutate: impl FnOnce(&mut Task, &str) -> Result<()>,
) -> Result<Task> {
    let mut updated_task = None;
    queue::with_locked_queue_mutation(
        resolved,
        operation,
        format!("{operation} {task_id}"),
        force,
        || {
            let mut active = queue::load_queue(&resolved.queue_path)?;
            let done = queue::load_queue_or_default(&resolved.done_path)?;
            let done_ref = queue::optional_done_queue(&done, &resolved.done_path);
            let now = timeutil::now_utc_rfc3339()?;
            let task = active
                .tasks
                .iter_mut()
                .find(|task| task.id == task_id)
                .ok_or_else(|| anyhow!("Task {task_id} was not found in the active queue."))?;
            mutate(task, &now)?;
            updated_task = Some(task.clone());
            let warnings = queue::validate_queue_set(
                &active,
                done_ref,
                &resolved.id_prefix,
                resolved.id_width,
                resolved.queue_max_dependency_depth(),
            )?;
            queue::log_warnings(&warnings);
            queue::save_queue(&resolved.queue_path, &active)
        },
    )?;
    updated_task.ok_or_else(|| anyhow!("Task {task_id} was not updated."))
}

fn find_task(resolved: &config::Resolved, task_id: &str) -> Result<(AgentTaskLocation, Task)> {
    let active = queue::load_queue(&resolved.queue_path)?;
    if let Some(task) = active.tasks.iter().find(|task| task.id == task_id) {
        return Ok((AgentTaskLocation::Active, task.clone()));
    }
    let done = queue::load_queue_or_default(&resolved.done_path)?;
    if let Some(task) = done.tasks.iter().find(|task| task.id == task_id) {
        return Ok((AgentTaskLocation::Done, task.clone()));
    }
    bail!("Task {task_id} was not found in the active queue or done archive.")
}

fn append_items(target: &mut Vec<String>, items: &[String]) {
    for item in items {
        let redacted = crate::redaction::redact_text(item.trim());
        if !redacted.trim().is_empty() {
            target.push(redacted);
        }
    }
}

fn build_handoff_document(location: AgentTaskLocation, task: Task) -> AgentHandoffDocument {
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
        location,
        recent_notes: task.notes.iter().rev().take(5).cloned().collect(),
        evidence: task.evidence.clone(),
        remaining_plan: task.plan.clone(),
        next_step,
        claim,
        task,
    }
}

fn task_summary(task: &Task) -> AgentTaskSummary {
    AgentTaskSummary {
        id: task.id.clone(),
        status: task.status,
        title: task.title.clone(),
        priority: task.priority.as_str().to_string(),
        tags: task.tags.clone(),
        scope: task.scope.clone(),
    }
}

fn print_overview(document: &AgentOverviewDocument, format: AgentOutputFormat) -> Result<()> {
    if format == AgentOutputFormat::Json {
        return print_json(document);
    }
    println!("Active tasks: {}", document.active_count);
    println!("Done archive tasks: {}", document.done_count);
    match &document.next_runnable_task_id {
        Some(id) => println!("Next runnable: {id}"),
        None => println!("Next runnable: none"),
    }
    if let Some(error) = &document.validation_error {
        println!("Validation: invalid - {error}");
    }
    if !document.doing.is_empty() {
        println!("Doing:");
        for task in &document.doing {
            println!("  {}\t{}", task.id, task.title);
        }
    }
    if !document.active.is_empty() {
        println!("Active queue:");
        for task in &document.active {
            println!("  {}\t{}\t{}", task.id, task.status.as_str(), task.title);
        }
    }
    if !document.recent_done.is_empty() {
        println!("Recent done:");
        for task in &document.recent_done {
            println!("  {}\t{}\t{}", task.id, task.status.as_str(), task.title);
        }
    }
    Ok(())
}

fn print_task_document(document: &AgentTaskDocument, format: AgentOutputFormat) -> Result<()> {
    if format == AgentOutputFormat::Json {
        return print_json(document);
    }
    println!(
        "{}\t{}\t{}",
        document.task.id, document.task.status, document.task.title
    );
    if let Some(description) = document.task.description.as_deref()
        && !description.trim().is_empty()
    {
        println!("\n{description}");
    }
    print_list("Scope", &document.task.scope);
    print_list("Plan", &document.task.plan);
    print_list("Evidence", &document.task.evidence);
    print_list("Notes", &document.task.notes);
    Ok(())
}

fn print_handoff(document: &AgentHandoffDocument, format: AgentOutputFormat) -> Result<()> {
    if format == AgentOutputFormat::Json {
        return print_json(document);
    }
    println!("Handoff for {}: {}", document.task.id, document.task.title);
    println!("Status: {}", document.task.status);
    if !document.claim.is_empty() {
        println!("Claim:");
        for (key, value) in &document.claim {
            println!("  {key}: {value}");
        }
    }
    if let Some(next) = &document.next_step {
        println!("Next: {next}");
    }
    print_list("Remaining plan", &document.remaining_plan);
    print_list("Evidence", &document.evidence);
    print_list("Recent notes", &document.recent_notes);
    Ok(())
}

fn print_mutation(
    action: &str,
    task: &Task,
    archived: bool,
    format: AgentOutputFormat,
) -> Result<()> {
    let document = AgentMutationDocument {
        version: DOCUMENT_VERSION,
        task_id: task.id.clone(),
        action: action.to_string(),
        task: Some(task.clone()),
        archived,
    };
    if format == AgentOutputFormat::Json {
        return print_json(&document);
    }
    println!("{}\t{}\t{}", task.id, action, task.title);
    Ok(())
}

fn print_list(label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    println!("\n{label}:");
    for value in values {
        println!("  - {value}");
    }
}

fn print_json(value: &impl Serialize) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).context("serialize agent ledger JSON")?
    );
    Ok(())
}
