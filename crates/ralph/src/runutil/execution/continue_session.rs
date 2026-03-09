//! Continue-session policy helpers for runner execution.
//!
//! Responsibilities:
//! - Select a resume session ID.
//! - Decide when to fall back from resume to a fresh invocation.
//! - Execute continue-session or rerun flows through the backend.
//!
//! Not handled here:
//! - Retry policy.
//! - Error shaping after runner failures.
//!
//! Invariants/assumptions:
//! - Continue-session fallbacks stay conservative and runner-specific.

use std::path::Path;
use std::time::Duration;

use crate::commands::run::PhaseType;
use crate::contracts::{ClaudePermissionMode, Model, ReasoningEffort, Runner};
use crate::runner;

use super::backend::RunnerBackend;

fn should_fallback_to_fresh_continue(runner_kind: &Runner, err: &runner::RunnerError) -> bool {
    if runner_kind != &Runner::Pi {
        return false;
    }

    let text = format!("{:#}", err).to_lowercase();
    text.contains("pi session file not found")
        || text.contains("no session found matching")
        || text.contains("read pi session dir")
}

fn choose_continue_session_id<'a>(
    error_session_id: Option<&'a str>,
    invocation_session_id: Option<&'a str>,
) -> Option<&'a str> {
    error_session_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .or_else(|| {
            invocation_session_id
                .map(str::trim)
                .filter(|id| !id.is_empty())
        })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn continue_or_rerun(
    backend: &mut impl RunnerBackend,
    runner_kind: &Runner,
    repo_root: &Path,
    bins: runner::RunnerBinaries<'_>,
    model: &Model,
    reasoning_effort: Option<ReasoningEffort>,
    runner_cli: runner::ResolvedRunnerCliOptions,
    continue_message: &str,
    fresh_prompt: &str,
    timeout: Option<Duration>,
    permission_mode: Option<ClaudePermissionMode>,
    output_handler: Option<runner::OutputHandler>,
    output_stream: runner::OutputStream,
    phase_type: PhaseType,
    invocation_session_id: Option<&str>,
    error_session_id: Option<&str>,
) -> Result<runner::RunnerOutput, runner::RunnerError> {
    let continue_session_id = choose_continue_session_id(error_session_id, invocation_session_id);
    if let Some(session_id) = continue_session_id {
        match backend.resume_session(
            runner_kind.clone(),
            repo_root,
            bins,
            model.clone(),
            reasoning_effort,
            runner_cli,
            session_id,
            continue_message,
            permission_mode,
            timeout,
            output_handler.clone(),
            output_stream,
            phase_type,
            None,
        ) {
            Ok(output) => return Ok(output),
            Err(err) if should_fallback_to_fresh_continue(runner_kind, &err) => {
                log::warn!(
                    "Continue session unavailable for runner {}; rerunning as fresh invocation: {:#}",
                    runner_kind,
                    err
                );
            }
            Err(err) => return Err(err),
        }
    } else {
        log::warn!(
            "Continue requested without session id for runner {}; rerunning as fresh invocation.",
            runner_kind
        );
    }

    backend.run_prompt(
        runner_kind.clone(),
        repo_root,
        bins,
        model.clone(),
        reasoning_effort,
        runner_cli,
        fresh_prompt,
        timeout,
        permission_mode,
        output_handler,
        output_stream,
        phase_type,
        invocation_session_id.map(str::to_string),
        None,
    )
}
