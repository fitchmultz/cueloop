//! Task-oriented machine command handlers.
//!
//! Purpose:
//! - Task-oriented machine command handlers.
//!
//! Responsibilities:
//! - Implement `cueloop machine task ...` operations.
//! - Parse machine task-create/mutate/decompose inputs and emit versioned JSON documents.
//! - Keep machine task writes aligned with queue locking, undo semantics, and continuation guidance.
//!
//! Not handled here:
//! - Queue read/graph/dashboard commands.
//! - Machine run event streaming.
//! - Clap argument definitions or top-level routing.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Machine task requests stay versioned and JSON-only.
//! - Task writes preserve queue locking, undo snapshots, and validation behavior.
//! - Status and child-policy parsing remain strict.

mod continuation;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::agent;
use crate::cli::machine::args::{MachineTaskArgs, MachineTaskCommand};
use crate::cli::machine::common::{done_queue_ref, queue_max_dependency_depth};
use crate::cli::machine::io::{print_json, read_json_input};
use crate::commands::task as task_cmd;
use crate::config;
use crate::contracts::{
    MACHINE_DECOMPOSE_VERSION, MACHINE_TASK_BUILD_VERSION, MACHINE_TASK_CREATE_VERSION,
    MACHINE_TASK_MUTATION_VERSION, MachineDecomposeDocument, MachineTaskBuildDocument,
    MachineTaskBuildRequest, MachineTaskBuildResult, MachineTaskCreateDocument,
    MachineTaskCreateRequest, MachineTaskMutationDocument, RunnerCliOptionsPatch, Task,
    TaskInsertDocument, TaskInsertRequest, TaskStatus,
};
use crate::queue;
use crate::timeutil;

use continuation::{build_continuation, decompose_continuation, mutation_continuation};

pub(super) fn handle_task(args: MachineTaskArgs, force: bool) -> Result<()> {
    let resolved = config::resolve_from_cwd()?;
    match args.command {
        MachineTaskCommand::Build(args) => {
            let raw = read_json_input(args.input.as_deref())?;
            let request: MachineTaskBuildRequest =
                serde_json::from_str(&raw).context("parse machine task build request")?;
            let repoprompt_tool_injection =
                agent::resolve_rp_required(args.agent.repo_prompt, &resolved);
            let overrides = agent::resolve_agent_overrides(&args.agent)?;
            let document = build_task_from_request(
                &resolved,
                &request,
                overrides,
                repoprompt_tool_injection,
                force,
            )?;
            print_json(&document)
        }
        MachineTaskCommand::Create(args) => {
            let raw = read_json_input(args.input.as_deref())?;
            let request: MachineTaskCreateRequest =
                serde_json::from_str(&raw).context("parse machine task create request")?;
            let task = create_task(&resolved, &request, force)?;
            print_json(&MachineTaskCreateDocument {
                version: MACHINE_TASK_CREATE_VERSION,
                task,
            })
        }
        MachineTaskCommand::Insert(args) => {
            let raw = read_json_input(args.input.as_deref())?;
            let request: TaskInsertRequest =
                serde_json::from_str(&raw).context("parse machine task insert request")?;
            let document = insert_tasks(&resolved, &request, args.dry_run, force)?;
            print_json(&document)
        }
        MachineTaskCommand::Mutate(args) => {
            let raw = read_json_input(args.input.as_deref())?;
            let request = serde_json::from_str::<queue::operations::TaskMutationRequest>(&raw)
                .context("parse machine task mutation request")?;

            let _queue_lock =
                queue::acquire_queue_lock(&resolved.repo_root, "machine task mutate", force)?;
            let queue_file = queue::load_queue(&resolved.queue_path)?;
            let done_file = queue::load_queue_or_default(&resolved.done_path)?;
            let done_ref = done_queue_ref(&done_file, &resolved.done_path);
            let now = timeutil::now_utc_rfc3339()?;
            let mut working = queue_file.clone();
            let report = queue::operations::apply_task_mutation_request(
                &mut working,
                done_ref,
                &request,
                &now,
                &resolved.id_prefix,
                resolved.id_width,
                queue_max_dependency_depth(&resolved),
            )?;
            if !args.dry_run {
                crate::undo::create_undo_snapshot(
                    &resolved,
                    &format!("machine task mutate [{} task(s)]", report.tasks.len()),
                )?;
                queue::save_queue(&resolved.queue_path, &working)?;
            }
            print_json(&build_task_mutation_document(&report, args.dry_run)?)
        }
        MachineTaskCommand::Decompose(args) => {
            if let Some(checkpoint_id) = args.from_preview.as_deref() {
                validate_machine_from_preview_args(&args)?;
                let (preview, checkpoint) =
                    task_cmd::load_decomposition_preview_checkpoint(&resolved, checkpoint_id)?;
                let write = Some(task_cmd::write_task_decomposition(
                    &resolved, &preview, force,
                )?);
                return print_json(&build_decompose_document(
                    &preview,
                    write.as_ref(),
                    Some(&checkpoint),
                ));
            }

            let source = machine_decompose_source_from_args(
                &resolved,
                &args.source,
                args.from_file.as_deref(),
            )?;
            let overrides = agent::resolve_agent_overrides(&args.agent)?;
            let status = parse_task_status(&args.status)?;
            let parent_status =
                parse_optional_task_status(args.parent_status.as_deref())?.unwrap_or(status);
            let leaf_status =
                parse_optional_task_status(args.leaf_status.as_deref())?.unwrap_or(status);
            let preview = task_cmd::plan_task_decomposition(
                &resolved,
                &task_cmd::TaskDecomposeOptions {
                    source,
                    attach_to_task_id: args.attach_to,
                    max_depth: args.max_depth,
                    max_children: usize::from(args.max_children),
                    max_nodes: usize::from(args.max_nodes),
                    status,
                    parent_status,
                    leaf_status,
                    child_policy: parse_child_policy(&args.child_policy)?,
                    with_dependencies: args.with_dependencies,
                    runner_override: overrides.runner,
                    model_override: overrides.model,
                    reasoning_effort_override: overrides.reasoning_effort,
                    runner_cli_overrides: overrides.runner_cli,
                    repoprompt_tool_injection: agent::resolve_rp_required(
                        args.agent.repo_prompt,
                        &resolved,
                    ),
                    stream_planner_output: false,
                    force,
                },
            )?;
            let write = if args.write {
                Some(task_cmd::write_task_decomposition(
                    &resolved, &preview, force,
                )?)
            } else {
                None
            };
            let checkpoint = if args.write {
                None
            } else {
                Some(task_cmd::save_decomposition_preview_checkpoint(
                    &resolved, &preview,
                )?)
            };
            print_json(&build_decompose_document(
                &preview,
                write.as_ref(),
                checkpoint.as_ref(),
            ))
        }
    }
}

fn validate_machine_from_preview_args(
    args: &crate::cli::machine::args::MachineTaskDecomposeArgs,
) -> Result<()> {
    if !args.write {
        bail!(
            "`cueloop machine task decompose --from-preview` requires --write for queue mutation."
        );
    }
    if !args.source.is_empty() || args.from_file.is_some() {
        bail!(
            "`cueloop machine task decompose --from-preview` cannot be combined with SOURCE text or --from-file."
        );
    }
    if args.attach_to.is_some()
        || args.with_dependencies
        || args.max_depth != 3
        || args.max_children != 5
        || args.max_nodes != 50
        || args.status != "draft"
        || args.parent_status.is_some()
        || args.leaf_status.is_some()
        || args.child_policy != "fail"
        || args.agent.runner.is_some()
        || args.agent.model.is_some()
        || args.agent.effort.is_some()
        || args.agent.repo_prompt.is_some()
    {
        bail!(
            "`cueloop machine task decompose --from-preview` replays saved preview options and cannot be combined with planner/status flags. Do not add --leaf-status, --parent-status, --with-dependencies, or other planner options; the preview already captured them."
        );
    }
    Ok(())
}

fn machine_decompose_source_from_args(
    resolved: &config::Resolved,
    source_args: &[String],
    from_file: Option<&std::path::Path>,
) -> Result<task_cmd::TaskDecomposeSourceInput> {
    if let Some(path) = from_file {
        if !source_args.is_empty() {
            bail!(
                "`cueloop machine task decompose --from-file` cannot be combined with positional SOURCE text."
            );
        }
        return task_cmd::read_plan_file_source(resolved, path);
    }
    Ok(task_cmd::TaskDecomposeSourceInput::Inline(
        task_cmd::read_request_from_args_or_stdin(source_args)?,
    ))
}

fn build_task_from_request(
    resolved: &config::Resolved,
    request: &MachineTaskBuildRequest,
    overrides: agent::AgentOverrides,
    repoprompt_tool_injection: bool,
    force: bool,
) -> Result<MachineTaskBuildDocument> {
    if request.version != MACHINE_TASK_BUILD_VERSION {
        bail!(
            "Unsupported machine task build request version {}",
            request.version
        );
    }
    if request.request.trim().is_empty() {
        bail!("Task build request cannot be empty");
    }

    let before = queue::load_queue(&resolved.queue_path)?;
    let before_ids = queue::task_id_set(&before);

    task_cmd::build_task(
        resolved,
        task_cmd::TaskBuildOptions {
            request: request.request.trim().to_string(),
            hint_tags: request.tags.join(","),
            hint_scope: request.scope.join(","),
            runner_override: overrides.runner,
            model_override: overrides.model,
            reasoning_effort_override: overrides.reasoning_effort,
            runner_cli_overrides: overrides.runner_cli,
            force,
            repoprompt_tool_injection,
            output: task_cmd::TaskBuildOutputTarget::Quiet,
            template_hint: request.template.clone(),
            template_target: request.target.clone(),
            strict_templates: request.strict_templates,
            estimated_minutes: request.estimated_minutes,
        },
    )?;

    let after = queue::load_queue(&resolved.queue_path)?;
    let added_ids = queue::added_tasks(&before_ids, &after)
        .into_iter()
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    let tasks = after
        .tasks
        .into_iter()
        .filter(|task| added_ids.contains(&task.id))
        .collect::<Vec<_>>();
    let continuation = build_continuation(tasks.len());
    let blocking = continuation.blocking.clone();

    Ok(MachineTaskBuildDocument {
        version: MACHINE_TASK_BUILD_VERSION,
        mode: "write".to_string(),
        blocking,
        result: MachineTaskBuildResult {
            created_count: tasks.len(),
            task_ids: added_ids,
            tasks,
        },
        warnings: Vec::new(),
        continuation,
    })
}

pub(crate) fn build_task_mutation_document(
    report: &queue::operations::TaskMutationReport,
    dry_run: bool,
) -> Result<MachineTaskMutationDocument> {
    let continuation = mutation_continuation(report.tasks.len(), dry_run);
    let blocking = continuation.blocking.clone();

    Ok(MachineTaskMutationDocument {
        version: MACHINE_TASK_MUTATION_VERSION,
        blocking,
        report: serde_json::to_value(report)?,
        continuation,
    })
}

pub(crate) fn build_decompose_document(
    preview: &task_cmd::DecompositionPreview,
    write: Option<&task_cmd::TaskDecomposeWriteResult>,
    checkpoint: Option<&task_cmd::DecompositionPreviewCheckpointRef>,
) -> MachineDecomposeDocument {
    let continuation = decompose_continuation(preview, write, checkpoint);
    let blocking = continuation.blocking.clone();
    let tasks = flatten_decompose_tasks(preview);

    MachineDecomposeDocument {
        version: MACHINE_DECOMPOSE_VERSION,
        blocking,
        result: serde_json::json!({
            "version": 2,
            "mode": if write.is_some() { "write" } else { "preview" },
            "preview": preview,
            "tasks": tasks,
            "write": write,
            "checkpoint": checkpoint,
            "replay_exact": checkpoint.is_some(),
        }),
        continuation,
    }
}

#[derive(Serialize)]
struct FlattenedDecomposeTask<'a> {
    id: Option<&'a str>,
    key: &'a str,
    title: &'a str,
    status: TaskStatus,
    depends_on_keys: &'a [String],
}

fn flatten_decompose_tasks(
    preview: &task_cmd::DecompositionPreview,
) -> Vec<FlattenedDecomposeTask<'_>> {
    let mut tasks = Vec::new();
    flatten_decompose_node(&preview.plan.root, preview, &mut tasks);
    tasks
}

fn flatten_decompose_node<'a>(
    node: &'a task_cmd::PlannedNode,
    preview: &task_cmd::DecompositionPreview,
    tasks: &mut Vec<FlattenedDecomposeTask<'a>>,
) {
    let status = if node.children.is_empty() {
        preview.leaf_status
    } else {
        preview.parent_status
    };
    tasks.push(FlattenedDecomposeTask {
        id: None,
        key: &node.planner_key,
        title: &node.title,
        status,
        depends_on_keys: &node.depends_on_keys,
    });
    for child in &node.children {
        flatten_decompose_node(child, preview, tasks);
    }
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

fn parse_optional_task_status(value: Option<&str>) -> Result<Option<TaskStatus>> {
    value.map(parse_task_status).transpose()
}

fn parse_task_status(value: &str) -> Result<TaskStatus> {
    match value.trim().to_ascii_lowercase().as_str() {
        "draft" => Ok(TaskStatus::Draft),
        "todo" => Ok(TaskStatus::Todo),
        "doing" => Ok(TaskStatus::Doing),
        "done" => Ok(TaskStatus::Done),
        "rejected" => Ok(TaskStatus::Rejected),
        other => bail!("Unsupported task status '{}'", other),
    }
}

fn parse_child_policy(value: &str) -> Result<task_cmd::DecompositionChildPolicy> {
    match value.trim().to_ascii_lowercase().as_str() {
        "fail" => Ok(task_cmd::DecompositionChildPolicy::Fail),
        "append" => Ok(task_cmd::DecompositionChildPolicy::Append),
        "replace" => Ok(task_cmd::DecompositionChildPolicy::Replace),
        other => bail!("Unsupported decomposition child policy '{}'", other),
    }
}
