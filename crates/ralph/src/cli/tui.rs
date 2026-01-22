//! `ralph tui` command group: Clap types and handler.

use anyhow::{anyhow, Result};
use clap::Args;

use crate::{agent, config, run_cmd, runner, runutil, scan_cmd, tui};

#[derive(Args)]
#[command(
    about = "Launch the interactive TUI (queue management + execution + loop)",
    after_long_help = "Notes:\n\
 - `ralph tui` is the primary interactive UI entry point.\n\
 - By default, execution is enabled (press Enter to run the selected task).\n\
 - Use `--read-only` to disable execution.\n\
 - `ralph run one -i` and `ralph run loop -i` launch the same TUI for compatibility.\n\
\n\
Examples:\n\
 ralph tui\n\
 ralph tui --read-only\n\
 ralph tui --runner codex --model gpt-5.2-codex --effort high\n\
 ralph tui --runner claude --model opus\n\
 ralph tui --runner opencode --model gpt-5.2\n"
)]
pub struct TuiArgs {
    /// Disable task execution (browse/edit only).
    #[arg(long)]
    pub read_only: bool,

    #[command(flatten)]
    pub agent: crate::agent::RunAgentArgs,
}

pub fn handle_tui(args: TuiArgs, force_lock: bool) -> Result<()> {
    let resolved = config::resolve_from_cwd()?;

    if args.read_only {
        let runner_factory = browse_only_runner;
        let scan_factory = browse_only_scan_runner;
        let _ = tui::run_tui(
            &resolved,
            force_lock,
            tui::TuiOptions::default(),
            runner_factory,
            scan_factory,
        )?;
        return Ok(());
    }

    let overrides = agent::resolve_run_agent_overrides(&args.agent)?;
    let scan_settings = runner::resolve_agent_settings(
        overrides.runner,
        overrides.model.clone(),
        overrides.reasoning_effort,
        None,
        &resolved.config.agent,
    )?;
    let scan_repoprompt_required =
        agent::resolve_rp_required(args.agent.rp_on, args.agent.rp_off, &resolved);
    let scan_git_revert_mode = overrides
        .git_revert_mode
        .or(resolved.config.agent.git_revert_mode)
        .unwrap_or(crate::contracts::GitRevertMode::Ask);

    // Capture the values we need by moving them into the factory.
    let resolved_clone = resolved.clone();
    let runner_factory =
        move |task_id: String,
              handler: runner::OutputHandler,
              revert_prompt: runutil::RevertPromptHandler| {
            let resolved = resolved_clone.clone();
            let overrides = overrides.clone();
            let force = force_lock;
            move || {
                run_cmd::run_one_with_id_locked(
                    &resolved,
                    &overrides,
                    force,
                    &task_id,
                    Some(handler),
                    Some(revert_prompt),
                )
            }
        };
    let resolved_scan = resolved.clone();
    let scan_factory = move |focus: String,
                             handler: runner::OutputHandler,
                             revert_prompt: runutil::RevertPromptHandler| {
        let resolved = resolved_scan.clone();
        let settings = scan_settings.clone();
        let force = force_lock;
        let repoprompt_required = scan_repoprompt_required;
        let git_revert_mode = scan_git_revert_mode;
        move || {
            scan_cmd::run_scan(
                &resolved,
                scan_cmd::ScanOptions {
                    focus,
                    runner: settings.runner,
                    model: settings.model,
                    reasoning_effort: settings.reasoning_effort,
                    force,
                    repoprompt_required,
                    git_revert_mode,
                    lock_mode: scan_cmd::ScanLockMode::Held,
                    output_handler: Some(handler),
                    revert_prompt: Some(revert_prompt),
                },
            )
        }
    };

    let _ = tui::run_tui(
        &resolved,
        force_lock,
        tui::TuiOptions::default(),
        runner_factory,
        scan_factory,
    )?;
    Ok(())
}

fn browse_only_runner(
    _task_id: String,
    _handler: runner::OutputHandler,
    _revert_prompt: runutil::RevertPromptHandler,
) -> impl FnOnce() -> Result<()> + Send {
    move || {
        Err(anyhow!(
            "Task execution is disabled in read-only mode. Re-run without `--read-only`."
        ))
    }
}

fn browse_only_scan_runner(
    _focus: String,
    _handler: runner::OutputHandler,
    _revert_prompt: runutil::RevertPromptHandler,
) -> impl FnOnce() -> Result<()> + Send {
    move || {
        Err(anyhow!(
            "Scan is disabled in read-only mode. Re-run without `--read-only`."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn browse_only_runner_rejects_execution() {
        let handler: runner::OutputHandler = Arc::new(Box::new(|_text: &str| {}));
        let revert_prompt: runutil::RevertPromptHandler =
            Arc::new(|_label: &str| runutil::RevertDecision::Keep);
        let runner = browse_only_runner("RQ-0001".to_string(), handler, revert_prompt);
        let err = runner().expect_err("expected browse-only error");
        assert!(err
            .to_string()
            .contains("Task execution is disabled in read-only mode"));
    }

    #[test]
    fn browse_only_scan_runner_rejects_scan() {
        let handler: runner::OutputHandler = Arc::new(Box::new(|_text: &str| {}));
        let revert_prompt: runutil::RevertPromptHandler =
            Arc::new(|_label: &str| runutil::RevertDecision::Keep);
        let runner = browse_only_scan_runner("".to_string(), handler, revert_prompt);
        let err = runner().expect_err("expected browse-only scan error");
        assert!(err
            .to_string()
            .contains("Scan is disabled in read-only mode"));
    }
}
