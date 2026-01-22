//! `ralph tui` command group: Clap types and handler.

use anyhow::{anyhow, Result};
use clap::Args;

use crate::{config, runner, tui};

#[derive(Args)]
#[command(
    about = "Launch the TUI for browsing and managing the queue",
    after_long_help = "Notes:\n  - `ralph tui` is browse-only; it does not execute tasks.\n  - To run tasks from the TUI, use `ralph run one -i` or `ralph run loop -i`.\n\nExamples:\n  ralph tui\n  ralph run one -i\n  ralph run loop -i"
)]
pub struct TuiArgs {}

pub fn handle_tui(_args: TuiArgs) -> Result<()> {
    let resolved = config::resolve_from_cwd()?;
    let runner_factory = browse_only_runner;

    let _ = tui::run_tui(&resolved.queue_path, runner_factory)?;
    Ok(())
}

fn browse_only_runner(
    _task_id: String,
    _handler: runner::OutputHandler,
) -> impl FnOnce() -> Result<()> + Send {
    move || {
        Err(anyhow!(
            "Task execution is disabled in browse-only mode. Use `ralph run one -i` or `ralph run loop -i`."
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
        let runner = browse_only_runner("RQ-0001".to_string(), handler);
        let err = runner().expect_err("expected browse-only error");
        assert!(err
            .to_string()
            .contains("Task execution is disabled in browse-only mode"));
    }
}
