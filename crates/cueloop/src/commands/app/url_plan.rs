//! Workspace URL handoff planning helpers.
//!
//! Purpose:
//! - Build deterministic URL-based handoff commands for routing the app to a workspace.
//!
//! Responsibilities:
//! - Construct `cueloop://open?...` URLs with safe percent encoding.
//! - Select the correct launcher for app-path vs bundle-id targets.
//! - Resolve an explicit or implicit workspace path for `cueloop app open`.
//!
//! Scope:
//! - URL planning only; process execution and initial app launch live elsewhere.
//!
//! Usage:
//! - Called by `runtime.rs` after the app has been launched.
//!
//! Invariants/assumptions:
//! - URL query values must be percent-encoded.
//! - URL launches should be able to start the app and deliver the workspace handoff in one command.

use anyhow::{Result, bail};
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use crate::cli::app::AppOpenArgs;

use super::launch_plan::{
    append_cli_env_args, append_open_launch_target_args, default_installed_app_path,
    resolve_launch_target,
};
use super::model::OpenCommandSpec;

pub(super) fn plan_url_command(
    workspace: &Path,
    args: &AppOpenArgs,
    cli_executable: Option<&Path>,
) -> Result<OpenCommandSpec> {
    plan_url_command_with_installed_path(
        workspace,
        args,
        cli_executable,
        default_installed_app_path(),
    )
}

pub(super) fn plan_url_command_with_installed_path(
    workspace: &Path,
    args: &AppOpenArgs,
    cli_executable: Option<&Path>,
    installed_app_path: Option<PathBuf>,
) -> Result<OpenCommandSpec> {
    let encoded_path = percent_encode_path(workspace);
    let url = format!("cueloop://open?workspace={}", encoded_path);
    let launch_target = resolve_launch_target(args, installed_app_path)?;

    let mut args_out: Vec<OsString> = Vec::new();
    append_cli_env_args(&mut args_out, cli_executable);
    append_open_launch_target_args(&mut args_out, &launch_target);
    args_out.push(OsString::from(url));

    Ok(OpenCommandSpec {
        program: OsString::from("open"),
        args: args_out,
    })
}

pub(super) fn resolve_workspace_path(args: &AppOpenArgs) -> Result<Option<PathBuf>> {
    if let Some(ref workspace) = args.workspace {
        if !workspace.exists() {
            bail!("Workspace path does not exist: {}", workspace.display());
        }
        return Ok(Some(workspace.clone()));
    }

    Ok(std::env::current_dir().ok().filter(|path| path.exists()))
}

#[cfg(unix)]
pub(super) fn percent_encode_path(path: &Path) -> String {
    percent_encode(path.as_os_str().as_bytes())
}

#[cfg(not(unix))]
pub(super) fn percent_encode_path(path: &Path) -> String {
    percent_encode(path.to_string_lossy().as_bytes())
}

pub(super) fn percent_encode(input: &[u8]) -> String {
    let mut result = String::with_capacity(input.len() * 3);
    for &byte in input {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/') {
            result.push(byte as char);
        } else {
            result.push('%');
            result.push_str(&format!("{:02X}", byte));
        }
    }
    result
}
