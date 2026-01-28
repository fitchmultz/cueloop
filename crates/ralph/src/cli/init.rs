//! `ralph init` command: Clap types and handler.
//!
//! Responsibilities:
//! - Parse CLI arguments for the init command.
//! - Determine interactive vs non-interactive mode based on flags and TTY detection.
//! - Delegate to the init command implementation.
//!
//! Not handled here:
//! - Actual file creation logic (see `crate::commands::init`).
//! - Interactive wizard implementation (see `crate::commands::init`).
//!
//! Invariants/assumptions:
//! - `--interactive` and `--non-interactive` are mutually exclusive.
//! - TTY detection is used to auto-select mode when neither flag is provided.

use anyhow::Result;
use clap::Args;

use crate::{commands::init as init_cmd, config};

/// Determine if stdout is a TTY.
fn is_tty() -> bool {
    atty::is(atty::Stream::Stdout)
}

pub fn handle_init(args: InitArgs, force_lock: bool) -> Result<()> {
    let resolved = config::resolve_from_cwd()?;

    // Determine interactive mode: explicit flags override TTY detection
    let interactive = match (args.interactive, args.non_interactive) {
        (true, _) => true,  // --interactive flag wins
        (_, true) => false, // --non-interactive flag wins
        _ => is_tty(),      // auto-detect based on TTY
    };

    let report = init_cmd::run_init(
        &resolved,
        init_cmd::InitOptions {
            force: args.force,
            force_lock,
            interactive,
        },
    )?;

    fn report_status(label: &str, status: init_cmd::FileInitStatus, path: &std::path::Path) {
        match status {
            init_cmd::FileInitStatus::Created => {
                log::info!("{}: created ({})", label, path.display())
            }
            init_cmd::FileInitStatus::Valid => {
                log::info!("{}: exists (valid) ({})", label, path.display())
            }
        }
    }

    report_status("queue", report.queue_status, &resolved.queue_path);
    report_status("done", report.done_status, &resolved.done_path);
    if let Some(status) = report.readme_status {
        let readme_path = resolved.repo_root.join(".ralph/README.md");
        report_status("readme", status, &readme_path);
    }
    if let Some(path) = resolved.project_config_path.as_ref() {
        report_status("config", report.config_status, path);
    } else {
        log::info!("config: unavailable");
    }
    Ok(())
}

#[derive(Args)]
#[command(
    about = "Bootstrap Ralph files in the current repository",
    after_long_help = "Examples:\n  ralph init\n  ralph init --force\n  ralph init --interactive\n  ralph init --non-interactive"
)]
pub struct InitArgs {
    /// Overwrite existing files if they already exist.
    #[arg(long)]
    pub force: bool,

    /// Run interactive onboarding wizard (auto-detected if TTY).
    #[arg(short, long)]
    pub interactive: bool,

    /// Skip interactive prompts even if running in a TTY.
    #[arg(long)]
    pub non_interactive: bool,
}
