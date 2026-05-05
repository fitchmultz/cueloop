//! GitHub CLI preflight helpers.
//!
//! Purpose:
//! - GitHub CLI preflight helpers.
//!
//! Responsibilities:
//! - Run `gh` availability and authentication checks.
//! - Keep command execution concerns separate from callers that need GitHub CLI preflight.
//!
//! Not handled here:
//! - Higher-level PR create/merge workflows.
//! - Rendering or logging beyond managed-command diagnostics.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Repo-scoped commands run from the target repository unless explicitly isolated elsewhere.
//! - `gh --version` and `gh auth status` are the preflight contract for availability checks.

use anyhow::{Context, Result, bail};
use std::process::Output;

use crate::git::github_cli::run_gh_command;
use crate::runutil::TimeoutClass;

pub(crate) fn check_gh_available() -> Result<()> {
    check_gh_available_with(run_gh_with_no_update)
}

pub(super) fn check_gh_available_with<F>(run_gh: F) -> Result<()>
where
    F: Fn(&[&str]) -> Result<Output>,
{
    let version_output = run_gh(&["--version"]).with_context(|| {
        "GitHub CLI (`gh`) not found on PATH. Install it from https://cli.github.com/ and re-run."
            .to_string()
    })?;

    if !version_output.status.success() {
        let stderr = String::from_utf8_lossy(&version_output.stderr);
        bail!(
            "`gh --version` failed (gh is not usable). Details: {}. Install/repair `gh` from https://cli.github.com/ and re-run.",
            stderr.trim()
        );
    }

    let auth_output = run_gh(&["auth", "status"]).with_context(|| {
        "Failed to run `gh auth status`. Ensure `gh` is properly installed.".to_string()
    })?;

    if !auth_output.status.success() {
        let stdout = String::from_utf8_lossy(&auth_output.stdout);
        let stderr = String::from_utf8_lossy(&auth_output.stderr);
        let details = if !stderr.is_empty() {
            stderr.trim()
        } else {
            stdout.trim()
        };
        bail!(
            "GitHub CLI (`gh`) is not authenticated. Run `gh auth login` and re-run. Details: {}",
            details
        );
    }

    Ok(())
}

fn run_gh_with_no_update(args: &[&str]) -> Result<Output> {
    let mut command = crate::git::github_cli::gh_command_in(&std::env::temp_dir());
    command.args(args);
    run_gh_command(
        command,
        format!("gh {}", args.join(" ")),
        TimeoutClass::Probe,
        "gh",
    )
    .with_context(|| format!("run gh {}", args.join(" ")))
}
