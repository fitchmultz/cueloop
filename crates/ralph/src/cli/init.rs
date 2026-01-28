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

    // Handle --check mode: verify README is current and exit
    if args.check {
        let check_result = init_cmd::check_readme_current(&resolved)?;
        match check_result {
            init_cmd::ReadmeCheckResult::Current(version) => {
                log::info!("readme: current (version {})", version);
                return Ok(());
            }
            init_cmd::ReadmeCheckResult::Outdated {
                current_version,
                embedded_version,
            } => {
                log::warn!(
                    "readme: outdated (current version {}, embedded version {})",
                    current_version,
                    embedded_version
                );
                log::info!("Run 'ralph init --update-readme' to update");
                std::process::exit(1);
            }
            init_cmd::ReadmeCheckResult::Missing => {
                log::warn!("readme: missing (would be created on normal init)");
                std::process::exit(1);
            }
            init_cmd::ReadmeCheckResult::NotApplicable => {
                log::info!("readme: not applicable (prompts don't reference README)");
                return Ok(());
            }
        }
    }

    let report = init_cmd::run_init(
        &resolved,
        init_cmd::InitOptions {
            force: args.force,
            force_lock,
            interactive,
            update_readme: args.update_readme,
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
            init_cmd::FileInitStatus::Updated => {
                log::info!("{}: updated ({})", label, path.display())
            }
        }
    }

    report_status("queue", report.queue_status, &resolved.queue_path);
    report_status("done", report.done_status, &resolved.done_path);
    if let Some((status, version_info)) = report.readme_status {
        let readme_path = resolved.repo_root.join(".ralph/README.md");
        match status {
            init_cmd::FileInitStatus::Created => {
                if let Some(version) = version_info {
                    log::info!(
                        "readme: created (version {}) ({})",
                        version,
                        readme_path.display()
                    );
                } else {
                    log::info!("readme: created ({})", readme_path.display());
                }
            }
            init_cmd::FileInitStatus::Valid => {
                if let Some(version) = version_info {
                    log::info!(
                        "readme: exists (version {}) ({})",
                        version,
                        readme_path.display()
                    );
                } else {
                    log::info!("readme: exists (valid) ({})", readme_path.display());
                }
            }
            init_cmd::FileInitStatus::Updated => {
                if let Some(version) = version_info {
                    log::info!(
                        "readme: updated (version {}) ({})",
                        version,
                        readme_path.display()
                    );
                } else {
                    log::info!("readme: updated ({})", readme_path.display());
                }
            }
        }
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
    after_long_help = "Examples:\n  ralph init\n  ralph init --force\n  ralph init --interactive\n  ralph init --non-interactive\n  ralph init --update-readme\n  ralph init --check"
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

    /// Update README if it exists (force overwrite with latest template).
    #[arg(long)]
    pub update_readme: bool,

    /// Check if README is current and exit (exit 0 if current, 1 if outdated/missing).
    #[arg(long)]
    pub check: bool,
}
