//! Cursor SDK health checks for doctor runner reporting.
//!
//! Purpose:
//! - Validate Cursor's Node runtime, SDK package availability, and API-key setup.
//! - Preserve Cursor-specific doctor warning and blocking diagnostics.
//!
//! Responsibilities:
//! - Run Cursor checks only after the selected Node binary is probeable.
//! - Keep SDK version drift warning-only when the SDK can proceed best-effort.
//! - Stop later doctor runner checks only when the historical package-fatal path does.
//!
//! Not handled here:
//! - Generic runner binary probing.
//! - Generic model compatibility or instruction-file checks.
//!
//! Invariants/assumptions:
//! - Missing/unsupported Node skips package/API-key checks but does not skip model/instruction checks.
//! - Fatal SDK package errors skip the remaining runner doctor checks.

use super::binary::{RunnerBinarySelection, add_runner_binary_success};
use super::runner_blocking_state;
use crate::commands::doctor::cursor_sdk_probe::{
    check_cursor_sdk_node_version, check_cursor_sdk_package, cursor_sdk_blocking_reason,
};
use crate::commands::doctor::types::{CheckResult, DoctorReport};
use crate::config;
use crate::constants::versions::CURSOR_SDK_VERSION;

pub(super) enum CursorSdkHealth {
    Checked,
    FatalPackageError,
}

pub(super) fn check_cursor_sdk_health(
    report: &mut DoctorReport,
    resolved: &config::Resolved,
    selection: &RunnerBinarySelection,
) -> CursorSdkHealth {
    if let Err(e) = check_cursor_sdk_node_version(&selection.bin_name) {
        let message = format!(
            "Cursor SDK Node runtime check failed for '{}': {}",
            selection.bin_name, e
        );
        let guidance =
            "Configure agent.cursor_sdk_node_bin to Node 18 or newer before running Cursor.";
        let blocking = runner_blocking_state(
            "runner",
            "cursor_sdk_node_unsupported",
            "CueLoop is stalled because the Cursor SDK requires Node 18 or newer.",
            guidance,
        );
        report.add(
            CheckResult::error(
                "runner",
                "cursor_sdk_node_version",
                &message,
                false,
                Some(guidance),
            )
            .with_blocking(blocking),
        );
        log::error!("{message}");
        log::error!("{guidance}");
        return CursorSdkHealth::Checked;
    }

    match check_cursor_sdk_package(&selection.bin_name, &resolved.repo_root) {
        Ok(check) if check.version_mismatch() => {
            let selected = check
                .selected
                .as_ref()
                .expect("version mismatch requires selected Cursor SDK");
            let detected = selected.sdk_version.as_deref().unwrap_or("unknown");
            let best_effort = if check.proceeded_best_effort {
                "Cursor runner will try it best-effort"
            } else {
                "Cursor runner can proceed best-effort when the SDK API shape is compatible"
            };
            let mut details = format!(
                "SDK entrypoint: {}; package: {}; global root: {}; fatal cause: {}; tried: {}",
                selected.entrypoint,
                selected.package_json.as_deref().unwrap_or("unknown"),
                selected.global_root.as_deref().unwrap_or("n/a"),
                check.fatal_cause.as_deref().unwrap_or("none"),
                check.attempted_sources_summary()
            );
            if !check.warnings.is_empty() {
                details.push_str(&format!("; warnings: {}", check.warnings.join("; ")));
            }
            report.add(CheckResult::warning(
                "runner",
                "cursor_sdk_package",
                &format!(
                    "@cursor/sdk {detected} from {} differs from CueLoop's preferred/tested {}; {best_effort}",
                    selected.source, check.preferred_sdk_version
                ),
                false,
                Some(&details),
            ));
        }
        Ok(_) => {}
        Err(e) => {
            let message = format!(
                "Cursor SDK package check failed for '{}': {}",
                selection.bin_name, e
            );
            let reason = cursor_sdk_blocking_reason(&e.to_string());
            let guidance = format!(
                "Install @cursor/sdk in this workspace (preferred/tested: `npm install --save-exact @cursor/sdk@{CURSOR_SDK_VERSION}`), \
                        install it globally, or set CUELOOP_CURSOR_SDK_MODULE_PATH to a trusted SDK entrypoint. Version drift is warning-only when the SDK exposes Agent."
            );
            let blocking = runner_blocking_state(
                "runner",
                reason,
                "CueLoop is stalled because the Cursor SDK package is unavailable or unusable.",
                guidance.clone(),
            );
            report.add(
                CheckResult::error(
                    "runner",
                    "cursor_sdk_package",
                    &message,
                    false,
                    Some(&guidance),
                )
                .with_blocking(blocking),
            );
            log::error!("{message}");
            log::error!("{guidance}");
            return CursorSdkHealth::FatalPackageError;
        }
    }

    if !cursor_api_key_configured() {
        let message = "Cursor SDK API key is not configured";
        let guidance = "Export CURSOR_API_KEY before running CueLoop with the Cursor runner.";
        let blocking = runner_blocking_state(
            "runner",
            "cursor_api_key_missing",
            "CueLoop is stalled because CURSOR_API_KEY is required for Cursor SDK runs.",
            guidance,
        );
        report.add(
            CheckResult::error("runner", "cursor_api_key", message, false, Some(guidance))
                .with_blocking(blocking),
        );
        log::error!("{message}");
        log::error!("{guidance}");
    } else {
        add_runner_binary_success(report, selection);
    }

    CursorSdkHealth::Checked
}

fn cursor_api_key_configured() -> bool {
    cursor_api_key_value_configured(std::env::var_os("CURSOR_API_KEY"))
}

pub(super) fn cursor_api_key_value_configured(value: Option<std::ffi::OsString>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}
