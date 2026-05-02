//! App-launch planning helpers.
//!
//! Purpose:
//! - Build deterministic `open ...` command specs for launching the Ralph macOS app.
//!
//! Responsibilities:
//! - Validate launch-target arguments.
//! - Resolve installed-app fallbacks and CLI environment propagation.
//! - Convert launch targets into `open -a` / `open -b` argv.
//!
//! Scope:
//! - Launch planning only; execution and URL handoff live in sibling modules.
//!
//! Usage:
//! - Called by `runtime.rs` and unit tests.
//!
//! Invariants/assumptions:
//! - `--path` and `--bundle-id` remain mutually exclusive.
//! - Default launch prefers installed app paths before bundle lookup.

use anyhow::{Result, bail};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::cli::app::AppOpenArgs;

use super::model::{
    DEFAULT_APP_NAME, DEFAULT_BUNDLE_ID, GUI_CLI_BIN_ENV, LaunchTarget, OpenCommandSpec,
};

pub(super) fn plan_open_command(
    is_macos: bool,
    args: &AppOpenArgs,
    cli_executable: Option<&Path>,
) -> Result<OpenCommandSpec> {
    plan_open_command_with_installed_path(
        is_macos,
        args,
        cli_executable,
        default_installed_app_path(),
    )
}

pub(super) fn plan_open_command_with_installed_path(
    is_macos: bool,
    args: &AppOpenArgs,
    cli_executable: Option<&Path>,
    installed_app_path: Option<PathBuf>,
) -> Result<OpenCommandSpec> {
    if !is_macos {
        bail!("`cueloop app open` is macOS-only.");
    }

    if args.path.is_some() && args.bundle_id.is_some() {
        bail!("--path and --bundle-id cannot be used together.");
    }

    let mut args_out: Vec<OsString> = Vec::new();
    if let Some(cli_executable) = cli_executable {
        args_out.push(OsString::from("--env"));
        args_out.push(env_assignment_for_path(cli_executable));
    }

    let launch_target = resolve_launch_target(args, installed_app_path)?;
    append_open_launch_target_args(&mut args_out, &launch_target);

    Ok(OpenCommandSpec {
        program: OsString::from("open"),
        args: args_out,
    })
}

pub(super) fn resolve_launch_target(
    args: &AppOpenArgs,
    installed_app_path: Option<PathBuf>,
) -> Result<LaunchTarget> {
    if let Some(path) = args.path.as_deref() {
        ensure_exists(path)?;
        return Ok(LaunchTarget::AppPath(path.to_path_buf()));
    }

    if let Some(bundle_id) = args.bundle_id.as_deref() {
        let bundle_id = bundle_id.trim();
        if bundle_id.is_empty() {
            bail!("Bundle id is empty.");
        }

        return Ok(LaunchTarget::BundleId(bundle_id.to_string()));
    }

    if let Some(path) = installed_app_path {
        return Ok(LaunchTarget::AppPath(path));
    }

    let bundle_id = DEFAULT_BUNDLE_ID.trim();
    if bundle_id.is_empty() {
        bail!("Bundle id is empty.");
    }

    Ok(LaunchTarget::BundleId(bundle_id.to_string()))
}

pub(super) fn append_open_launch_target_args(
    args_out: &mut Vec<OsString>,
    launch_target: &LaunchTarget,
) {
    match launch_target {
        LaunchTarget::AppPath(path) => {
            args_out.push(OsString::from("-a"));
            args_out.push(path.as_os_str().to_os_string());
        }
        LaunchTarget::BundleId(bundle_id) => {
            args_out.push(OsString::from("-b"));
            args_out.push(OsString::from(bundle_id));
        }
    }
}

pub(super) fn default_installed_app_path() -> Option<PathBuf> {
    installed_app_candidates()
        .into_iter()
        .find(|candidate| candidate.exists())
}

pub(super) fn installed_app_candidates_for_home(home: Option<PathBuf>) -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from("/Applications").join(DEFAULT_APP_NAME)];
    if let Some(home) = home {
        candidates.push(home.join("Applications").join(DEFAULT_APP_NAME));
    }
    candidates
}

pub(super) fn current_executable_for_gui() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    if exe.exists() { Some(exe) } else { None }
}

#[cfg(unix)]
pub(super) fn env_assignment_for_path(path: &Path) -> OsString {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let mut bytes = Vec::from(format!("{GUI_CLI_BIN_ENV}=").as_bytes());
    bytes.extend_from_slice(path.as_os_str().as_bytes());
    OsString::from_vec(bytes)
}

#[cfg(not(unix))]
pub(super) fn env_assignment_for_path(path: &Path) -> OsString {
    OsString::from(format!("{GUI_CLI_BIN_ENV}={}", path.to_string_lossy()))
}

fn ensure_exists(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    bail!("Path does not exist: {}", path.display());
}

fn installed_app_candidates() -> Vec<PathBuf> {
    installed_app_candidates_for_home(std::env::var_os("HOME").map(PathBuf::from))
}
