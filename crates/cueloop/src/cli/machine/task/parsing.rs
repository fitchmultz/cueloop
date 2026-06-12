//! Machine task parsing helpers.
//!
//! Purpose:
//! - Parse string arguments that are intentionally kept as machine CLI text values.
//!
//! Responsibilities:
//! - Parse task statuses accepted by machine task lifecycle/decompose commands.
//! - Parse decomposition child-policy values.
//! - Keep unsupported values as fast errors with stable messages.
//!
//! Not handled here:
//! - Clap argument definitions.
//! - JSON request parsing.
//! - Queue mutation or document construction.
//!
//! Usage:
//! - Used by the machine task router and decomposition handler.
//!
//! Invariants/assumptions:
//! - Parsing is case-insensitive for supported values.
//! - Unsupported values keep the existing error text.

use anyhow::{Result, bail};

use crate::commands::task as task_cmd;
use crate::contracts::TaskStatus;

pub(super) fn parse_optional_task_status(value: Option<&str>) -> Result<Option<TaskStatus>> {
    value.map(parse_task_status).transpose()
}

pub(super) fn parse_task_status(value: &str) -> Result<TaskStatus> {
    match value.trim().to_ascii_lowercase().as_str() {
        "draft" => Ok(TaskStatus::Draft),
        "todo" => Ok(TaskStatus::Todo),
        "doing" => Ok(TaskStatus::Doing),
        "done" => Ok(TaskStatus::Done),
        "rejected" => Ok(TaskStatus::Rejected),
        other => bail!("Unsupported task status '{}'", other),
    }
}

pub(super) fn parse_child_policy(value: &str) -> Result<task_cmd::DecompositionChildPolicy> {
    match value.trim().to_ascii_lowercase().as_str() {
        "fail" => Ok(task_cmd::DecompositionChildPolicy::Fail),
        "append" => Ok(task_cmd::DecompositionChildPolicy::Append),
        "replace" => Ok(task_cmd::DecompositionChildPolicy::Replace),
        other => bail!("Unsupported decomposition child policy '{}'", other),
    }
}
