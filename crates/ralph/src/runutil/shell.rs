//! Shell command helpers.
//!
//! Responsibilities:
//! - Build a platform-appropriate shell command wrapper.
//! - Sanitize Ralph run-scoped environment variables from child processes.
//!
//! Not handled here:
//! - Process output handling, streaming, or error classification.
//!
//! Invariants/assumptions:
//! - Unix uses `sh -c`, Windows uses `cmd /C`.
//! - Child processes should not inherit RALPH_*_OVERRIDE variables unless
//!   explicitly needed (e.g., parallel workers set REPO_ROOT_OVERRIDE intentionally).

use std::process::Command;

/// Build a shell command for the current platform (sh -c on Unix, cmd /C on Windows).
pub fn shell_command(command: &str) -> Command {
    if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(command);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd
    }
}

/// Remove all Ralph run-scoped environment override variables from a child process.
///
/// This helper ensures child processes (CI gate, merge-agent, etc.) cannot
/// accidentally access or modify parent queue/done files through inherited
/// environment variables. Call this on any Command before spawning a child
/// that should operate in isolation from the parent's Ralph configuration.
///
/// For parallel workers that need REPO_ROOT_OVERRIDE set to their workspace,
/// call this first, then set the override explicitly.
pub(crate) fn sanitize_run_scoped_overrides(cmd: &mut Command) -> &mut Command {
    cmd.env_remove(crate::config::QUEUE_PATH_OVERRIDE_ENV)
        .env_remove(crate::config::DONE_PATH_OVERRIDE_ENV)
        .env_remove(crate::config::REPO_ROOT_OVERRIDE_ENV)
}
