//! Runner/model compatibility doctor check.
//!
//! Purpose:
//! - Validate the configured model against the selected runner during doctor checks.
//!
//! Responsibilities:
//! - Resolve the effective model with the same defaults used by runner execution.
//! - Emit the existing doctor success/error diagnostics and blocking state.
//!
//! Not handled here:
//! - Binary availability, Cursor SDK health, or instruction-file reporting.
//! - Model execution or provider API validation.
//!
//! Invariants/assumptions:
//! - The compatibility check remains non-networked and based on local runner metadata.

use super::runner_blocking_state;
use crate::commands::doctor::types::{CheckResult, DoctorReport};
use crate::config;
use crate::contracts::Runner;
use crate::runner;

pub(super) fn check_model_compatibility(
    report: &mut DoctorReport,
    resolved: &config::Resolved,
    runner: &Runner,
) {
    let model = runner::resolve_model_for_runner(
        runner,
        None,
        None,
        resolved.config.agent.model.clone(),
        false,
    );
    if let Err(e) = runner::validate_model_for_runner(runner, &model) {
        report.add(
            CheckResult::error(
                "runner",
                "model_compatibility",
                &format!("config model/runner mismatch: {}", e),
                false,
                Some("Check the model is compatible with the selected runner in config"),
            )
            .with_blocking(runner_blocking_state(
                "runner",
                "model_incompatible",
                "CueLoop is stalled because the selected runner/model combination is invalid.",
                e.to_string(),
            )),
        );
    } else {
        report.add(CheckResult::success(
            "runner",
            "model_compatibility",
            &format!(
                "model '{}' compatible with runner '{:?}'",
                model.as_str(),
                runner
            ),
        ));
    }
}
