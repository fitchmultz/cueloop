//! Continue session support for supervision.
//!
//! Responsibilities:
//! - Define ContinueSession struct for resuming runner sessions.
//! - Define CiContinueContext for CI gate resume callbacks.
//! - Implement resume_continue_session to resume a runner with a message.
//!
//! Not handled here:
//! - CI gate logic (see ci.rs).
//! - Queue operations (see queue_ops.rs).
//!
//! Invariants/assumptions:
//! - Continue prefers same-session resume when a session id is available.
//! - If resume is unavailable (or no session id exists), continue falls back to a fresh invocation.
//! - Runner CLI options and phase type are preserved from the original session.

use crate::commands::run::PhaseType;
use crate::commands::run::phases::generate_phase_session_id;
use crate::contracts::Runner;
use crate::runutil::should_fallback_to_fresh_continue;
use anyhow::{Context, Result};

/// Session state for continuing an interrupted task.
#[derive(Clone)]
pub(crate) struct ContinueSession {
    pub runner: crate::contracts::Runner,
    pub model: crate::contracts::Model,
    pub reasoning_effort: Option<crate::contracts::ReasoningEffort>,
    /// The runner CLI settings resolved for the run that created this continue session.
    /// These must be preserved to avoid losing CLI overrides / task-specific settings.
    pub runner_cli: crate::runner::ResolvedRunnerCliOptions,
    /// The phase that created this continue session. Must be preserved so phase-aware
    /// runners (e.g., Cursor) behave correctly on Continue.
    pub phase_type: PhaseType,
    pub session_id: Option<String>,
    pub output_handler: Option<crate::runner::OutputHandler>,
    pub output_stream: crate::runner::OutputStream,
    /// Number of automatic "fix CI and rerun" retries already sent for the current CI gate loop.
    /// Used to auto-enforce CI compliance without prompting for the first N failures.
    pub ci_failure_retry_count: u8,
    /// The task ID for this continue session (needed for processor hooks).
    pub task_id: String,
    /// The pattern type of the last CI error (e.g., "TOML parse error").
    /// Used to detect repeated failures with the same root cause.
    pub last_ci_error_pattern: Option<String>,
    /// Count of consecutive CI failures with the same error pattern.
    /// Reset when pattern changes or CI passes.
    pub consecutive_same_error_count: u8,
}

/// Context for resuming a runner session during a post-run CI gate failure.
pub(crate) struct CiContinueContext<'a> {
    pub continue_session: &'a mut ContinueSession,
    /// Callback invoked after each resume, receiving both the output and the elapsed duration.
    /// The duration represents the wall-clock time spent in that specific resume session.
    pub on_resume:
        &'a mut dyn FnMut(&crate::runner::RunnerOutput, std::time::Duration) -> Result<()>,
}

fn phase_number(phase_type: PhaseType) -> u8 {
    match phase_type {
        PhaseType::Planning => 1,
        PhaseType::Implementation => 2,
        PhaseType::Review => 3,
        PhaseType::SinglePhase => 0,
    }
}

fn run_fresh_continue(
    resolved: &crate::config::Resolved,
    session: &ContinueSession,
    message: &str,
    plugins: Option<&crate::plugins::registry::PluginRegistry>,
) -> std::result::Result<(crate::runner::RunnerOutput, Option<String>), crate::runner::RunnerError>
{
    let bins = crate::runner::resolve_binaries(&resolved.config.agent);
    let fallback_session_id = match session.runner {
        Runner::Kimi => Some(generate_phase_session_id(
            &session.task_id,
            phase_number(session.phase_type),
        )),
        _ => None,
    };

    let output = crate::runner::run_prompt(
        session.runner.clone(),
        &resolved.repo_root,
        bins,
        session.model.clone(),
        session.reasoning_effort,
        session.runner_cli,
        message,
        None,
        resolved.config.agent.claude_permission_mode,
        session.output_handler.clone(),
        session.output_stream,
        session.phase_type,
        fallback_session_id.clone(),
        plugins,
    )?;

    Ok((output, fallback_session_id))
}

/// Resume a continue session with a message.
///
/// Returns the runner output along with the wall-clock duration of the session.
/// The duration is measured from the start of the function to when the runner
/// output is received.
///
/// Invokes post_run processor hooks after successful resume if plugins are provided.
pub(crate) fn resume_continue_session(
    resolved: &crate::config::Resolved,
    session: &mut ContinueSession,
    message: &str,
    plugins: Option<&crate::plugins::registry::PluginRegistry>,
) -> Result<(crate::runner::RunnerOutput, std::time::Duration)> {
    let start = std::time::Instant::now();
    let bins = crate::runner::resolve_binaries(&resolved.config.agent);
    let mut fallback_session_id: Option<String> = None;
    let mut used_fresh_fallback = false;

    // Prefer same-session continuation. If session resumption is unavailable, fall back
    // to a fresh invocation with the same continue message.
    let output = match session
        .session_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
    {
        Some(session_id) => {
            // Use the stored runner_cli and phase_type from the session to preserve
            // CLI overrides and ensure phase-correct behavior for phase-aware runners.
            match crate::runner::resume_session(
                session.runner.clone(),
                &resolved.repo_root,
                bins,
                session.model.clone(),
                session.reasoning_effort,
                session.runner_cli,
                session_id,
                message,
                resolved.config.agent.claude_permission_mode,
                None,
                session.output_handler.clone(),
                session.output_stream,
                session.phase_type,
                plugins,
            ) {
                Ok(output) => output,
                Err(err) if should_fallback_to_fresh_continue(&session.runner, &err) => {
                    log::warn!(
                        "Continue session unavailable for runner {}; retrying as fresh invocation: {:#}",
                        session.runner,
                        err
                    );
                    let (output, generated_id) =
                        run_fresh_continue(resolved, session, message, plugins)?;
                    used_fresh_fallback = true;
                    fallback_session_id = generated_id;
                    output
                }
                Err(err) => return Err(err.into()),
            }
        }
        None => {
            log::warn!(
                "Continue requested without session id for runner {}; starting a fresh invocation.",
                session.runner
            );
            let (output, generated_id) = run_fresh_continue(resolved, session, message, plugins)?;
            used_fresh_fallback = true;
            fallback_session_id = generated_id;
            output
        }
    };

    let elapsed = start.elapsed();
    if let Some(new_id) = output.session_id.as_ref() {
        session.session_id = Some(new_id.clone());
    } else if let Some(generated_id) = fallback_session_id {
        // Kimi does not emit session IDs in JSON output. Preserve managed ID generated
        // for fresh continue invocations so future continue attempts can reuse it.
        session.session_id = Some(generated_id);
    } else if used_fresh_fallback {
        // Fresh fallback succeeded but did not provide a resumable session identifier.
        // Clear stale resume state so future continues do not keep retrying invalid IDs.
        session.session_id = None;
    }

    // Invoke post_run hooks after successful resume
    if let Some(registry) = plugins {
        let exec = crate::plugins::processor_executor::ProcessorExecutor::new(
            &resolved.repo_root,
            registry,
        );
        exec.post_run(&session.task_id, &output.stdout)
            .with_context(|| "processor post_run hook failed after resume")?;
    }

    Ok((output, elapsed))
}
