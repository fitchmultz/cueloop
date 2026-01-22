//! `ralph tui` command group: Clap types and handler.

use anyhow::{anyhow, Result};
use clap::Args;

use crate::{agent, config, run_cmd, runner, runutil, tui};

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
        let _ = tui::run_tui(
            &resolved,
            force_lock,
            tui::TuiOptions::default(),
            runner_factory,
        )?;
        return Ok(());
    }

    let overrides = agent::resolve_run_agent_overrides(&args.agent)?;

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

    let _ = tui::run_tui(
        &resolved,
        force_lock,
        tui::TuiOptions::default(),
        runner_factory,
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
}
