//! Machine task decomposition operation.
//!
//! Purpose:
//! - Own machine task decomposition preview, write, and checkpoint replay flows.
//!
//! Responsibilities:
//! - Validate machine-only `--from-preview` replay constraints.
//! - Build decomposition sources from inline text, stdin, or files.
//! - Plan decomposition, optionally write it, and emit the machine decomposition document.
//!
//! Not handled here:
//! - Decomposition document envelope construction.
//! - Continuation text construction.
//! - Human task-decompose CLI routing.
//!
//! Usage:
//! - Called by the machine task router for `machine task decompose`.
//!
//! Invariants/assumptions:
//! - Preview replay must use the exact saved checkpoint options.
//! - Preview mode saves a checkpoint; write mode writes immediately and does not save one.

use anyhow::{Result, bail};

use crate::agent;
use crate::cli::machine::args::MachineTaskDecomposeArgs;
use crate::commands::task as task_cmd;
use crate::config;
use crate::contracts::MachineDecomposeDocument;

use super::documents::build_decompose_document;
use super::parsing::{parse_child_policy, parse_optional_task_status, parse_task_status};

pub(super) fn handle_decompose(
    resolved: &config::Resolved,
    args: MachineTaskDecomposeArgs,
    force: bool,
) -> Result<MachineDecomposeDocument> {
    if let Some(checkpoint_id) = args.from_preview.as_deref() {
        validate_machine_from_preview_args(&args)?;
        let (preview, checkpoint) =
            task_cmd::load_decomposition_preview_checkpoint(resolved, checkpoint_id)?;
        let write = Some(task_cmd::write_task_decomposition(
            resolved, &preview, force,
        )?);
        return Ok(build_decompose_document(
            &preview,
            write.as_ref(),
            Some(&checkpoint),
        ));
    }

    let source =
        machine_decompose_source_from_args(resolved, &args.source, args.from_file.as_deref())?;
    let overrides = agent::resolve_agent_overrides(&args.agent)?;
    let status = parse_task_status(&args.status)?;
    let parent_status =
        parse_optional_task_status(args.parent_status.as_deref())?.unwrap_or(status);
    let leaf_status = parse_optional_task_status(args.leaf_status.as_deref())?.unwrap_or(status);
    let preview = task_cmd::plan_task_decomposition(
        resolved,
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
            repoprompt_tool_injection: agent::resolve_rp_required(args.agent.repo_prompt, resolved),
            stream_planner_output: false,
            force,
        },
    )?;
    let write = if args.write {
        Some(task_cmd::write_task_decomposition(
            resolved, &preview, force,
        )?)
    } else {
        None
    };
    let checkpoint = if args.write {
        None
    } else {
        Some(task_cmd::save_decomposition_preview_checkpoint(
            resolved, &preview,
        )?)
    };
    Ok(build_decompose_document(
        &preview,
        write.as_ref(),
        checkpoint.as_ref(),
    ))
}

fn validate_machine_from_preview_args(args: &MachineTaskDecomposeArgs) -> Result<()> {
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
