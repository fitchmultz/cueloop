//! Runner binary selection and doctor probe reporting.
//!
//! Purpose:
//! - Resolve the configured/default runner binary for doctor checks.
//! - Report untrusted project overrides and missing runner binaries.
//! - Reuse the canonical runner binary probe from `commands::runner::detection`.
//!
//! Responsibilities:
//! - Preserve doctor guidance and blocking diagnostics for runner binary failures.
//! - Keep config-key mapping centralized for runner doctor modules.
//!
//! Not handled here:
//! - Cursor SDK package/API-key checks.
//! - Model compatibility and instruction-file reporting.
//!
//! Invariants/assumptions:
//! - Plugin runners are skipped because doctor cannot infer their executable from agent config.
//! - Project runner overrides are execution-sensitive unless the repo trust file allows them.

use std::path::PathBuf;

use anyhow::{Context, Result};

use super::runner_blocking_state;
use crate::commands::doctor::types::{CheckResult, DoctorReport};
use crate::commands::runner::detection::probe_runner_binary;
use crate::config;
use crate::contracts::Runner;

pub(super) struct RunnerBinarySelection {
    pub(super) runner: Runner,
    pub(super) bin_name: String,
    runner_configured: bool,
}

pub(super) enum BinarySelectionCheck {
    Available,
    Missing,
    BlockedProjectOverride,
}

pub(super) fn select_runner_binary(resolved: &config::Resolved) -> Option<RunnerBinarySelection> {
    let runner = resolved.config.agent.runner.clone().unwrap_or_default();
    let bin_name = match &runner {
        Runner::Codex => resolved
            .config
            .agent
            .codex_bin
            .as_deref()
            .unwrap_or("codex"),
        Runner::Opencode => resolved
            .config
            .agent
            .opencode_bin
            .as_deref()
            .unwrap_or("opencode"),
        Runner::Gemini => resolved
            .config
            .agent
            .gemini_bin
            .as_deref()
            .unwrap_or("gemini"),
        Runner::Claude => resolved
            .config
            .agent
            .claude_bin
            .as_deref()
            .unwrap_or("claude"),
        Runner::Cursor => resolved
            .config
            .agent
            .cursor_sdk_node_bin
            .as_deref()
            .unwrap_or("node"),
        Runner::Kimi => resolved.config.agent.kimi_bin.as_deref().unwrap_or("kimi"),
        Runner::Pi => resolved.config.agent.pi_bin.as_deref().unwrap_or("pi"),
        Runner::Plugin(_plugin_id) => {
            // For plugin runners, we can't determine the binary name from config.
            // The plugin registry would need to be consulted.
            return None;
        }
    };

    Some(RunnerBinarySelection {
        runner,
        bin_name: bin_name.to_string(),
        runner_configured: runner_configured(resolved),
    })
}

pub(super) fn check_runner_binary_selection(
    report: &mut DoctorReport,
    resolved: &config::Resolved,
    selection: &RunnerBinarySelection,
) -> BinarySelectionCheck {
    if let Some(blocked) = blocked_project_runner_override(resolved, &selection.runner) {
        let message = format!(
            "project config defines execution-sensitive runner override '{}', but this repo is not trusted",
            blocked.config_key
        );
        let guidance = format!(
            "Move agent.{} to trusted global config or create .cueloop/trust.jsonc before running doctor checks that execute runner binaries. Config file: {}. {}",
            blocked.config_key,
            blocked.config_path.display(),
            blocked.reason
        );
        report.add(
            CheckResult::error(
                "runner",
                "runner_binary",
                &message,
                false,
                Some(&guidance),
            )
            .with_blocking(runner_blocking_state(
                "runner",
                "project_runner_override_untrusted",
                "CueLoop is stalled because project runner overrides are blocked until the repo is trusted.",
                guidance.clone(),
            )),
        );
        log::error!("{message}");
        log::error!("{guidance}");
        return BinarySelectionCheck::BlockedProjectOverride;
    }

    if let Err(e) = probe_runner_binary(&selection.bin_name) {
        let config_key = get_runner_config_key(&selection.runner);
        let message = format!(
            "runner binary '{}' ({:?}) check failed: {}",
            selection.bin_name, selection.runner, e
        );

        let guidance = if selection.runner_configured {
            format!(
                "Install the runner binary, or configure a custom path in .cueloop/config.jsonc: {{ \"agent\": {{ \"{}\": \"/path/to/{}\" }} }}",
                config_key, selection.bin_name
            )
        } else {
            format!(
                "Install the default runner binary, or configure agent.runner plus agent.{config_key} in .cueloop/config.jsonc before running CueLoop."
            )
        };
        let blocking = runner_blocking_state(
            "runner",
            "runner_binary_missing",
            format!(
                "CueLoop is stalled because runner binary '{}' is unavailable.",
                selection.bin_name
            ),
            format!(
                "Configured/default runner {:?} cannot execute because '{}' is not on PATH or not executable.",
                selection.runner, selection.bin_name
            ),
        );
        let result =
            CheckResult::error("runner", "runner_binary", &message, false, Some(&guidance))
                .with_blocking(blocking);
        report.add(result);
        log::error!("");
        log::error!("To fix this issue:");
        log::error!("  1. Install the runner binary, or");
        log::error!("  2. Configure a custom path in .cueloop/config.jsonc:");
        log::error!("     {{");
        log::error!("       \"agent\": {{");
        log::error!(
            "         \"{}\": \"/path/to/{}\"",
            config_key,
            selection.bin_name
        );
        log::error!("       }}");
        log::error!("     }}");
        log::error!("  3. Run 'cueloop doctor' to verify the fix");
        BinarySelectionCheck::Missing
    } else {
        BinarySelectionCheck::Available
    }
}

pub(super) fn add_runner_binary_success(
    report: &mut DoctorReport,
    selection: &RunnerBinarySelection,
) {
    report.add(CheckResult::success(
        "runner",
        "runner_binary",
        &format!(
            "runner binary '{}' ({:?}) found",
            selection.bin_name, selection.runner
        ),
    ));
}

#[derive(Debug, Clone)]
struct BlockedProjectRunnerOverride {
    config_key: &'static str,
    config_path: PathBuf,
    reason: String,
}

fn blocked_project_runner_override(
    resolved: &config::Resolved,
    runner: &Runner,
) -> Option<BlockedProjectRunnerOverride> {
    match check_project_runner_override_trust(resolved, runner) {
        Ok(blocked) => blocked,
        Err(err) => {
            let config_key = get_runner_config_key(runner);
            resolved
                .project_config_path
                .as_ref()
                .map(|config_path| BlockedProjectRunnerOverride {
                    config_key,
                    config_path: config_path.clone(),
                    reason: format!("Unable to prove repo trust/config safety: {err}"),
                })
        }
    }
}

fn check_project_runner_override_trust(
    resolved: &config::Resolved,
    runner: &Runner,
) -> Result<Option<BlockedProjectRunnerOverride>> {
    let config_key = get_runner_config_key(runner);
    if config_key == "plugin_bin" {
        return Ok(None);
    }

    let Some(project_path) = resolved.project_config_path.as_ref() else {
        return Ok(None);
    };
    if !project_path.exists() {
        return Ok(None);
    }

    let layer = config::load_layer(project_path)
        .with_context(|| format!("load project config layer {}", project_path.display()))?;
    if !runner_override_is_configured(&layer.agent, runner) {
        return Ok(None);
    }

    let repo_trust = config::load_repo_trust(&resolved.repo_root)
        .with_context(|| "load repo trust for project runner override check")?;
    if repo_trust.is_trusted() {
        return Ok(None);
    }

    Ok(Some(BlockedProjectRunnerOverride {
        config_key,
        config_path: project_path.clone(),
        reason: "Repo trust is not enabled for project-local runner overrides.".to_string(),
    }))
}

fn runner_override_is_configured(agent: &crate::contracts::AgentConfig, runner: &Runner) -> bool {
    match runner {
        Runner::Codex => agent.codex_bin.is_some(),
        Runner::Opencode => agent.opencode_bin.is_some(),
        Runner::Gemini => agent.gemini_bin.is_some(),
        Runner::Claude => agent.claude_bin.is_some(),
        Runner::Cursor => agent.cursor_sdk_node_bin.is_some(),
        Runner::Kimi => agent.kimi_bin.is_some(),
        Runner::Pi => agent.pi_bin.is_some(),
        Runner::Plugin(_) => false,
    }
}

fn runner_configured(resolved: &config::Resolved) -> bool {
    let mut configured = false;
    let mut consider_layer = |path: &std::path::Path| {
        if configured {
            return;
        }
        let layer = match config::load_layer(path) {
            Ok(layer) => layer,
            Err(err) => {
                log::warn!("Unable to load config layer at {}: {}", path.display(), err);
                return;
            }
        };
        configured = layer.agent.runner.is_some()
            || layer.agent.codex_bin.is_some()
            || layer.agent.opencode_bin.is_some()
            || layer.agent.gemini_bin.is_some()
            || layer.agent.claude_bin.is_some()
            || layer.agent.cursor_sdk_node_bin.is_some()
            || layer.agent.kimi_bin.is_some()
            || layer.agent.pi_bin.is_some();
    };

    if let Some(path) = resolved.global_config_path.as_ref()
        && path.exists()
    {
        consider_layer(path);
    }
    if let Some(path) = resolved.project_config_path.as_ref()
        && path.exists()
    {
        consider_layer(path);
    }

    configured
}

/// Get the config key for a runner's binary path override.
pub(super) fn get_runner_config_key(runner: &Runner) -> &'static str {
    match runner {
        Runner::Codex => "codex_bin",
        Runner::Opencode => "opencode_bin",
        Runner::Gemini => "gemini_bin",
        Runner::Claude => "claude_bin",
        Runner::Cursor => "cursor_sdk_node_bin",
        Runner::Kimi => "kimi_bin",
        Runner::Pi => "pi_bin",
        Runner::Plugin(_) => "plugin_bin",
    }
}
