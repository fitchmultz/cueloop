//! Shared app-launch planning models.
//!
//! Purpose:
//! - Centralize command-spec and launch-target data shared across app helper modules.
//!
//! Responsibilities:
//! - Represent a planned launcher command before execution.
//! - Capture whether app launch targets resolve to a bundle path or bundle identifier.
//! - Hold app-launch constants shared by planning helpers.
//!
//! Scope:
//! - Pure data-model support for `crate::commands::app`.
//! - No filesystem probing or process execution lives here.
//!
//! Usage:
//! - Consumed by launch planning, URL handoff planning, runtime execution, and tests.
//!
//! Invariants/assumptions:
//! - `OpenCommandSpec` always stores the exact program and argv to execute.
//! - `LaunchTarget` variants are mutually exclusive.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

pub(super) const DEFAULT_BUNDLE_ID: &str = "com.mitchfultz.cueloop";
pub(super) const DEFAULT_APP_NAME: &str = "CueLoopMac.app";
pub(super) const GUI_CLI_BIN_ENV: &str = "CUELOOP_BIN_PATH";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OpenCommandSpec {
    pub(super) program: OsString,
    pub(super) args: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LaunchTarget {
    AppPath(PathBuf),
    BundleId(String),
}

impl OpenCommandSpec {
    pub(super) fn to_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        cmd
    }
}
