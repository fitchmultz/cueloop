//! Runner doctor orchestration and shared runner-check diagnostics.
//!
//! Purpose:
//! - Coordinate doctor checks for runner configuration, binary probing, Cursor SDK health,
//!   model compatibility, and instruction-file reporting.
//!
//! Responsibilities:
//! - Keep the `doctor` runner check order and output contract stable.
//! - Delegate concern-specific checks to sibling modules.
//! - Provide common runner blocking-state construction for concern modules.
//!
//! Not handled here:
//! - Runner execution and session lifecycle.
//! - Runner capability listing outside doctor checks.
//!
//! Invariants/assumptions:
//! - Check ordering and early-return behavior must match the historical doctor contract.
//! - Binary probing goes through `commands::runner::detection` as the canonical path.

mod binary;
mod cursor;
mod instructions;
mod model;

#[cfg(test)]
mod tests;

use crate::commands::doctor::types::DoctorReport;
use crate::config;
use crate::contracts::BlockingState;

fn runner_blocking_state(
    scope: &str,
    reason: &str,
    message: impl Into<String>,
    detail: impl Into<String>,
) -> BlockingState {
    BlockingState::runner_recovery(scope, reason, None, message, detail)
        .with_observed_at(crate::timeutil::now_utc_rfc3339_or_fallback())
}

pub(crate) fn check_runner(report: &mut DoctorReport, resolved: &config::Resolved) {
    let selection = match binary::select_runner_binary(resolved) {
        Some(selection) => selection,
        None => return,
    };

    match binary::check_runner_binary_selection(report, resolved, &selection) {
        binary::BinarySelectionCheck::BlockedProjectOverride => return,
        binary::BinarySelectionCheck::Missing => {}
        binary::BinarySelectionCheck::Available => {
            if selection.runner == crate::contracts::Runner::Cursor {
                match cursor::check_cursor_sdk_health(report, resolved, &selection) {
                    cursor::CursorSdkHealth::FatalPackageError => return,
                    cursor::CursorSdkHealth::Checked => {}
                }
            } else {
                binary::add_runner_binary_success(report, &selection);
            }
        }
    }

    model::check_model_compatibility(report, resolved, &selection.runner);
    instructions::check_instruction_files(report, resolved);
}
