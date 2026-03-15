//! App-launch runtime helpers.
//!
//! Purpose:
//! - Execute the planned launcher commands for `ralph app open`.
//!
//! Responsibilities:
//! - Spawn the initial app-launch command with managed subprocess handling.
//! - Retry workspace URL handoff until the app is ready to accept it.
//! - Keep runtime behavior separate from pure planning helpers.
//!
//! Scope:
//! - Command execution for the app-open workflow only.
//!
//! Usage:
//! - Re-exported as `crate::commands::app::open`.
//!
//! Invariants/assumptions:
//! - The app launch happens before URL handoff so the bootstrap window exists.
//! - URL handoff retries preserve the last launch error for diagnostics.

use anyhow::{Context, Result};
use std::thread;
use std::time::Duration;

use crate::cli::app::AppOpenArgs;
use crate::runutil::{ManagedCommand, TimeoutClass, execute_checked_command};

use super::launch_plan::{current_executable_for_gui, plan_open_command};
use super::model::OpenCommandSpec;
use super::url_plan::{plan_url_command, resolve_workspace_path};

pub(super) fn execute_launch_command(spec: &OpenCommandSpec) -> Result<()> {
    execute_checked_command(ManagedCommand::new(
        spec.to_command(),
        "launch macOS app",
        TimeoutClass::AppLaunch,
    ))
    .context("spawn macOS app launch command")?;
    Ok(())
}

/// Open the Ralph macOS app.
///
/// On macOS, this always launches the app bundle first so the primary workspace
/// window is guaranteed to exist on cold start. If workspace context is available,
/// a follow-up URL handoff repurposes that bootstrap window for the requested
/// workspace.
pub fn open(args: AppOpenArgs) -> Result<()> {
    let cli_executable = current_executable_for_gui();
    let open_spec = plan_open_command(cfg!(target_os = "macos"), &args, cli_executable.as_deref())?;
    execute_launch_command(&open_spec)?;

    let Some(workspace_path) = resolve_workspace_path(&args)? else {
        return Ok(());
    };

    let url_spec = plan_url_command(&workspace_path, &args)?;
    let mut last_error = None;
    for attempt in 0..10 {
        match execute_launch_command(&url_spec) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error);
                if attempt < 9 {
                    thread::sleep(Duration::from_millis(250));
                }
            }
        }
    }

    Err(last_error.expect("url launch attempts should record an error"))
}
