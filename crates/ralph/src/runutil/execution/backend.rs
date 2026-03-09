//! Runner execution backend types and utility hooks.
//!
//! Responsibilities:
//! - Define the invocation/config types for runner execution orchestration.
//! - Provide the real backend implementation that delegates to `crate::runner`.
//! - Host output-capture/logging helpers reused across orchestration paths.
//!
//! Not handled here:
//! - Retry/continue-session policy.
//! - Main error-handling state machine.
//!
//! Invariants/assumptions:
//! - Output capture remains bounded.
//! - Callers provide validated runner/model settings.

use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::commands::run::PhaseType;
use crate::constants::buffers::{OUTPUT_TAIL_LINE_MAX_CHARS, OUTPUT_TAIL_LINES};
use crate::contracts::{ClaudePermissionMode, GitRevertMode, Model, ReasoningEffort, Runner};
use crate::{outpututil, runner};

pub(crate) struct RunnerInvocation<'a> {
    pub repo_root: &'a Path,
    pub runner_kind: Runner,
    pub bins: runner::RunnerBinaries<'a>,
    pub model: Model,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub runner_cli: runner::ResolvedRunnerCliOptions,
    pub prompt: &'a str,
    pub timeout: Option<Duration>,
    pub permission_mode: Option<ClaudePermissionMode>,
    pub revert_on_error: bool,
    pub git_revert_mode: GitRevertMode,
    pub output_handler: Option<runner::OutputHandler>,
    pub output_stream: runner::OutputStream,
    pub revert_prompt: Option<super::super::revert::RevertPromptHandler>,
    pub phase_type: PhaseType,
    pub session_id: Option<String>,
    pub retry_policy: super::super::RunnerRetryPolicy,
}

pub(crate) struct RunnerErrorMessages<'a, FNonZero, FOther>
where
    FNonZero: FnMut(i32) -> String,
    FOther: FnOnce(runner::RunnerError) -> String,
{
    pub log_label: &'a str,
    pub interrupted_msg: &'a str,
    pub timeout_msg: &'a str,
    pub terminated_msg: &'a str,
    pub non_zero_msg: FNonZero,
    pub other_msg: FOther,
}

pub(crate) trait RunnerBackend {
    #[allow(clippy::too_many_arguments)]
    fn run_prompt<'a>(
        &mut self,
        runner_kind: Runner,
        work_dir: &Path,
        bins: runner::RunnerBinaries<'a>,
        model: Model,
        reasoning_effort: Option<ReasoningEffort>,
        runner_cli: runner::ResolvedRunnerCliOptions,
        prompt: &str,
        timeout: Option<Duration>,
        permission_mode: Option<ClaudePermissionMode>,
        output_handler: Option<runner::OutputHandler>,
        output_stream: runner::OutputStream,
        phase_type: PhaseType,
        session_id: Option<String>,
        plugins: Option<&crate::plugins::registry::PluginRegistry>,
    ) -> Result<runner::RunnerOutput, runner::RunnerError>;

    #[allow(clippy::too_many_arguments)]
    fn resume_session<'a>(
        &mut self,
        runner_kind: Runner,
        work_dir: &Path,
        bins: runner::RunnerBinaries<'a>,
        model: Model,
        reasoning_effort: Option<ReasoningEffort>,
        runner_cli: runner::ResolvedRunnerCliOptions,
        session_id: &str,
        message: &str,
        permission_mode: Option<ClaudePermissionMode>,
        timeout: Option<Duration>,
        output_handler: Option<runner::OutputHandler>,
        output_stream: runner::OutputStream,
        phase_type: PhaseType,
        plugins: Option<&crate::plugins::registry::PluginRegistry>,
    ) -> Result<runner::RunnerOutput, runner::RunnerError>;
}

pub(super) struct RealRunnerBackend;

impl RunnerBackend for RealRunnerBackend {
    fn run_prompt<'a>(
        &mut self,
        runner_kind: Runner,
        work_dir: &Path,
        bins: runner::RunnerBinaries<'a>,
        model: Model,
        reasoning_effort: Option<ReasoningEffort>,
        runner_cli: runner::ResolvedRunnerCliOptions,
        prompt: &str,
        timeout: Option<Duration>,
        permission_mode: Option<ClaudePermissionMode>,
        output_handler: Option<runner::OutputHandler>,
        output_stream: runner::OutputStream,
        phase_type: PhaseType,
        session_id: Option<String>,
        plugins: Option<&crate::plugins::registry::PluginRegistry>,
    ) -> Result<runner::RunnerOutput, runner::RunnerError> {
        runner::run_prompt(
            runner_kind,
            work_dir,
            bins,
            model,
            reasoning_effort,
            runner_cli,
            prompt,
            timeout,
            permission_mode,
            output_handler,
            output_stream,
            phase_type,
            session_id,
            plugins,
        )
    }

    fn resume_session<'a>(
        &mut self,
        runner_kind: Runner,
        work_dir: &Path,
        bins: runner::RunnerBinaries<'a>,
        model: Model,
        reasoning_effort: Option<ReasoningEffort>,
        runner_cli: runner::ResolvedRunnerCliOptions,
        session_id: &str,
        message: &str,
        permission_mode: Option<ClaudePermissionMode>,
        timeout: Option<Duration>,
        output_handler: Option<runner::OutputHandler>,
        output_stream: runner::OutputStream,
        phase_type: PhaseType,
        plugins: Option<&crate::plugins::registry::PluginRegistry>,
    ) -> Result<runner::RunnerOutput, runner::RunnerError> {
        runner::resume_session(
            runner_kind,
            work_dir,
            bins,
            model,
            reasoning_effort,
            runner_cli,
            session_id,
            message,
            permission_mode,
            timeout,
            output_handler,
            output_stream,
            phase_type,
            plugins,
        )
    }
}

pub(super) fn wrap_output_handler_with_capture(
    existing: Option<runner::OutputHandler>,
    max_bytes: usize,
) -> (Arc<Mutex<String>>, Option<runner::OutputHandler>) {
    let capture = Arc::new(Mutex::new(String::new()));
    let capture_for_handler = capture.clone();
    let existing_for_handler = existing.clone();

    let handler: runner::OutputHandler = Arc::new(Box::new(move |chunk: &str| {
        fn append_chunk(buf: &mut String, chunk: &str, max_bytes: usize) {
            buf.push_str(chunk);
            if buf.len() > max_bytes {
                let excess = buf.len() - max_bytes;
                buf.drain(..excess);
            }
        }

        match capture_for_handler.lock() {
            Ok(mut buf) => append_chunk(&mut buf, chunk, max_bytes),
            Err(poisoned) => {
                log::warn!("timeout_stdout_capture mutex poisoned; recovering captured output");
                let mut buf = poisoned.into_inner();
                append_chunk(&mut buf, chunk, max_bytes);
            }
        }
        if let Some(existing) = existing_for_handler.as_ref() {
            (existing)(chunk);
        }
    }));

    (capture, Some(handler))
}

pub(super) fn emit_operation(handler: &Option<runner::OutputHandler>, msg: &str) {
    if let Some(handler) = handler.as_ref() {
        (handler)(&format!("RALPH_OPERATION: {}\n", msg));
    }
}

pub(super) fn log_stderr_tail(label: &str, stderr: &str) {
    let tail = outpututil::tail_lines(stderr, OUTPUT_TAIL_LINES, OUTPUT_TAIL_LINE_MAX_CHARS);
    if tail.is_empty() {
        return;
    }

    crate::rerror!("{label} stderr (tail):");
    for line in tail {
        crate::rinfo!("{label}: {line}");
    }
}
