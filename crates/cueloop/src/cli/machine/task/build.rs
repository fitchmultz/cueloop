//! Machine task AI-build execution.
//!
//! Purpose:
//! - Turn a versioned machine task-build request into queued task drafts.
//!
//! Responsibilities:
//! - Parse task-build JSON input.
//! - Resolve agent overrides and repo-prompt injection for build mode.
//! - Call the human task builder quietly and wrap the created tasks in a machine document.
//!
//! Not handled here:
//! - CLI subcommand routing.
//! - Manual task creation or generic task insertion.
//! - Machine continuation copy not specific to task builds.
//!
//! Usage:
//! - Called by `cli::machine::task::handle_task` for `machine task build`.
//!
//! Invariants/assumptions:
//! - Build requests are versioned and reject empty request text.
//! - Created task detection compares queue task IDs before and after the builder runs.

use anyhow::{Context, Result, bail};

use crate::agent;
use crate::cli::machine::args::MachineTaskBuildArgs;
use crate::cli::machine::io::read_json_input;
use crate::commands::task as task_cmd;
use crate::config;
use crate::contracts::{
    MACHINE_TASK_BUILD_VERSION, MachineTaskBuildDocument, MachineTaskBuildRequest,
    MachineTaskBuildResult,
};
use crate::queue;

use super::continuation::build_continuation;

pub(super) fn handle_build(
    resolved: &config::Resolved,
    args: MachineTaskBuildArgs,
    force: bool,
) -> Result<MachineTaskBuildDocument> {
    let raw = read_json_input(args.input.as_deref())?;
    let request: MachineTaskBuildRequest =
        serde_json::from_str(&raw).context("parse machine task build request")?;
    let repoprompt_tool_injection = agent::resolve_rp_required(args.agent.repo_prompt, resolved);
    let overrides = agent::resolve_agent_overrides(&args.agent)?;
    build_task_from_request(
        resolved,
        &request,
        overrides,
        repoprompt_tool_injection,
        force,
    )
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
