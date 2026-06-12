//! Machine task create and insert operations.
//!
//! Purpose:
//! - Own machine task-create and task-insert write paths.
//!
//! Responsibilities:
//! - Parse create/insert JSON requests.
//! - Materialize single manual or template-backed machine task creation.
//! - Apply versioned bulk insert requests with queue locks and undo snapshots.
//!
//! Not handled here:
//! - AI task-build prompt orchestration.
//! - Lifecycle status/archive transitions.
//! - Decomposition planning.
//!
//! Usage:
//! - Called by the machine task router for `create` and `insert` subcommands.
//!
//! Invariants/assumptions:
//! - Create requests are versioned and require non-empty titles.
//! - Non-dry-run inserts and creates create undo snapshots before saving queue changes.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use crate::cli::machine::args::{MachineTaskCreateArgs, MachineTaskInsertArgs};
use crate::cli::machine::common::{done_queue_ref, queue_max_dependency_depth};
use crate::cli::machine::io::read_json_input;
use crate::commands::task as task_cmd;
use crate::config;
use crate::contracts::{
    MACHINE_TASK_CREATE_VERSION, MachineTaskCreateDocument, MachineTaskCreateRequest,
    RunnerCliOptionsPatch, Task, TaskInsertDocument, TaskInsertRequest, TaskStatus,
};
use crate::queue;
use crate::timeutil;

pub(super) fn handle_create(
    resolved: &config::Resolved,
    args: MachineTaskCreateArgs,
    force: bool,
) -> Result<MachineTaskCreateDocument> {
    let raw = read_json_input(args.input.as_deref())?;
    let request: MachineTaskCreateRequest =
        serde_json::from_str(&raw).context("parse machine task create request")?;
    let task = create_task(resolved, &request, force)?;
    Ok(MachineTaskCreateDocument {
        version: MACHINE_TASK_CREATE_VERSION,
        task,
    })
}

pub(super) fn handle_insert(
    resolved: &config::Resolved,
    args: MachineTaskInsertArgs,
    force: bool,
) -> Result<TaskInsertDocument> {
    let raw = read_json_input(args.input.as_deref())?;
    let request: TaskInsertRequest =
        serde_json::from_str(&raw).context("parse machine task insert request")?;
    insert_tasks(resolved, &request, args.dry_run, force)
}

fn insert_tasks(
    resolved: &config::Resolved,
    request: &TaskInsertRequest,
    dry_run: bool,
    force: bool,
) -> Result<TaskInsertDocument> {
    let _queue_lock = queue::acquire_queue_lock(&resolved.repo_root, "machine task insert", force)?;
    let mut active = queue::load_queue(&resolved.queue_path)?;
    let done = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = done_queue_ref(&done, &resolved.done_path);
    let now = timeutil::now_utc_rfc3339()?;
    let document = queue::operations::apply_task_insert_request(
        &mut active,
        done_ref,
        request,
        &now,
        &resolved.id_prefix,
        resolved.id_width,
        queue_max_dependency_depth(resolved),
        dry_run,
    )?;
    if !dry_run {
        crate::undo::create_undo_snapshot(
            resolved,
            &format!("machine task insert [{} task(s)]", document.created_count),
        )?;
        queue::save_queue(&resolved.queue_path, &active)?;
    }
    Ok(document)
}

fn create_task(
    resolved: &config::Resolved,
    request: &MachineTaskCreateRequest,
    force: bool,
) -> Result<Task> {
    if request.version != MACHINE_TASK_CREATE_VERSION {
        bail!(
            "Unsupported machine task create request version {}",
            request.version
        );
    }
    if request.title.trim().is_empty() {
        bail!("Task title cannot be empty");
    }

    if let Some(template) = &request.template {
        let options = task_cmd::TaskBuildOptions {
            request: request.title.clone(),
            hint_tags: request.tags.join(","),
            hint_scope: request.scope.join(","),
            runner_override: None,
            model_override: None,
            reasoning_effort_override: None,
            runner_cli_overrides: RunnerCliOptionsPatch::default(),
            force,
            repoprompt_tool_injection: false,
            output: task_cmd::TaskBuildOutputTarget::Quiet,
            template_hint: Some(template.clone()),
            template_target: request.target.clone(),
            strict_templates: true,
            estimated_minutes: None,
        };
        let created_tasks = task_cmd::build_task_created_tasks(resolved, options)?;
        return match created_tasks.as_slice() {
            [task] => Ok(task.clone()),
            [] => bail!("Template task create completed without creating a task"),
            tasks => bail!(
                "Template task create expected one task but created {}",
                tasks.len()
            ),
        };
    }

    let _queue_lock = queue::acquire_queue_lock(&resolved.repo_root, "machine task create", force)?;
    let active = queue::load_queue(&resolved.queue_path)?;
    let done = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = done_queue_ref(&done, &resolved.done_path);
    let predicted_id = queue::next_id_across(
        &active,
        done_ref,
        &resolved.id_prefix,
        resolved.id_width,
        queue_max_dependency_depth(resolved),
    )?;

    let now = timeutil::now_utc_rfc3339()?;
    let priority = request.priority.parse::<crate::contracts::TaskPriority>()?;
    let task = Task {
        id: predicted_id,
        status: TaskStatus::Todo,
        kind: Default::default(),
        title: request.title.trim().to_string(),
        description: request
            .description
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        priority,
        tags: request.tags.clone(),
        scope: request.scope.clone(),
        evidence: Vec::new(),
        plan: Vec::new(),
        notes: Vec::new(),
        request: None,
        agent: None,
        created_at: Some(now.clone()),
        updated_at: Some(now),
        completed_at: None,
        started_at: None,
        scheduled_start: None,
        estimated_minutes: None,
        actual_minutes: None,
        depends_on: Vec::new(),
        blocks: Vec::new(),
        relates_to: Vec::new(),
        duplicates: None,
        custom_fields: HashMap::new(),
        parent_id: None,
    };

    let mut working = active;
    working.tasks.push(task.clone());
    crate::undo::create_undo_snapshot(resolved, &format!("machine task create [{}]", task.id))?;
    queue::save_queue(&resolved.queue_path, &working)?;
    Ok(task)
}
