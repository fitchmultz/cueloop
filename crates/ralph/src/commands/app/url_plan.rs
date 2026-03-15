//! Workspace URL handoff planning helpers.
//!
//! Purpose:
//! - Build deterministic URL-based handoff commands for routing the app to a workspace.
//!
//! Responsibilities:
//! - Construct `ralph://open?...` URLs with safe percent encoding.
//! - Select the correct launcher for app-path vs bundle-id targets.
//! - Resolve an explicit or implicit workspace path for `ralph app open`.
//!
//! Scope:
//! - URL planning only; process execution and initial app launch live elsewhere.
//!
//! Usage:
//! - Called by `runtime.rs` after the app has been launched.
//!
//! Invariants/assumptions:
//! - URL query values must be percent-encoded.
//! - App-path launches use AppleScript so the installed bundle opens the URL directly.

use anyhow::{Result, bail};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

use crate::cli::app::AppOpenArgs;

use super::launch_plan::{default_installed_app_path, resolve_launch_target};
use super::model::{LaunchTarget, OpenCommandSpec};

pub(super) fn plan_url_command(workspace: &Path, args: &AppOpenArgs) -> Result<OpenCommandSpec> {
    plan_url_command_with_installed_path(workspace, args, default_installed_app_path())
}

pub(super) fn plan_url_command_with_installed_path(
    workspace: &Path,
    args: &AppOpenArgs,
    installed_app_path: Option<PathBuf>,
) -> Result<OpenCommandSpec> {
    let encoded_path = percent_encode_path(workspace);
    let url = format!("ralph://open?workspace={}", encoded_path);
    let launch_target = resolve_launch_target(args, installed_app_path)?;

    Ok(match launch_target {
        LaunchTarget::AppPath(path) => plan_applescript_url_command(&path, &url),
        LaunchTarget::BundleId(bundle_id) => OpenCommandSpec {
            program: OsString::from("open"),
            args: vec![
                OsString::from("-b"),
                OsString::from(bundle_id),
                OsString::from(url),
            ],
        },
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

fn plan_applescript_url_command(app_path: &Path, url: &str) -> OpenCommandSpec {
    OpenCommandSpec {
        program: OsString::from("osascript"),
        args: vec![
            OsString::from("-e"),
            OsString::from("on run argv"),
            OsString::from("-e"),
            OsString::from("tell application (item 1 of argv) to open location (item 2 of argv)"),
            OsString::from("-e"),
            OsString::from("end run"),
            app_path.as_os_str().to_os_string(),
            OsString::from(url),
        ],
    }
}
