//! Runner-specific command assembly for execution.

use std::path::Path;
use std::time::Duration;

use super::super::{
    ClaudePermissionMode, Model, OutputHandler, OutputStream, ReasoningEffort, RunnerError,
    RunnerOutput,
};
use super::command::RunnerCommandBuilder;
use super::process::run_with_streaming_json;

#[allow(clippy::too_many_arguments)]
pub fn run_codex(
    work_dir: &Path,
    bin: &str,
    model: Model,
    reasoning_effort: Option<ReasoningEffort>,
    prompt: &str,
    timeout: Option<Duration>,
    output_handler: Option<OutputHandler>,
    output_stream: OutputStream,
) -> Result<RunnerOutput, RunnerError> {
    let (cmd, payload, _guards) = RunnerCommandBuilder::new(bin, work_dir)
        .arg("exec")
        .legacy_json_format()
        .model(&model)
        .reasoning_effort(reasoning_effort)
        .arg("-")
        .stdin_payload(Some(prompt.as_bytes().to_vec()))
        .build();

    run_with_streaming_json(
        cmd,
        payload.as_deref(),
        bin,
        timeout,
        output_handler,
        output_stream,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_codex_resume(
    work_dir: &Path,
    bin: &str,
    model: Model,
    reasoning_effort: Option<ReasoningEffort>,
    thread_id: &str,
    message: &str,
    timeout: Option<Duration>,
    output_handler: Option<OutputHandler>,
    output_stream: OutputStream,
) -> Result<RunnerOutput, RunnerError> {
    let (cmd, payload, _guards) = RunnerCommandBuilder::new(bin, work_dir)
        .arg("exec")
        .arg("resume")
        .arg(thread_id)
        .legacy_json_format()
        .model(&model)
        .reasoning_effort(reasoning_effort)
        .arg(message)
        .build();

    run_with_streaming_json(
        cmd,
        payload.as_deref(),
        bin,
        timeout,
        output_handler,
        output_stream,
    )
}

pub fn run_opencode(
    work_dir: &Path,
    bin: &str,
    model: &Model,
    prompt: &str,
    timeout: Option<Duration>,
    output_handler: Option<OutputHandler>,
    output_stream: OutputStream,
) -> Result<RunnerOutput, RunnerError> {
    let (cmd, payload, _guards) = RunnerCommandBuilder::new(bin, work_dir)
        .arg("run")
        .model(model)
        .opencode_format()
        .with_temp_prompt_file(prompt)
        .map_err(RunnerError::Other)?
        .build();

    run_with_streaming_json(
        cmd,
        payload.as_deref(),
        bin,
        timeout,
        output_handler,
        output_stream,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_opencode_resume(
    work_dir: &Path,
    bin: &str,
    model: &Model,
    session_id: &str,
    message: &str,
    timeout: Option<Duration>,
    output_handler: Option<OutputHandler>,
    output_stream: OutputStream,
) -> Result<RunnerOutput, RunnerError> {
    let (cmd, payload, _guards) = RunnerCommandBuilder::new(bin, work_dir)
        .arg("run")
        .arg("-s")
        .arg(session_id)
        .model(model)
        .opencode_format()
        .arg("--")
        .arg(message)
        .build();

    run_with_streaming_json(
        cmd,
        payload.as_deref(),
        bin,
        timeout,
        output_handler,
        output_stream,
    )
}

pub fn run_gemini(
    work_dir: &Path,
    bin: &str,
    model: Model,
    prompt: &str,
    timeout: Option<Duration>,
    output_handler: Option<OutputHandler>,
    output_stream: OutputStream,
) -> Result<RunnerOutput, RunnerError> {
    let (cmd, payload, _guards) = RunnerCommandBuilder::new(bin, work_dir)
        .model(&model)
        .output_format("stream-json")
        .arg("--approval-mode")
        .arg("yolo")
        .stdin_payload(Some(prompt.as_bytes().to_vec()))
        .build();

    run_with_streaming_json(
        cmd,
        payload.as_deref(),
        bin,
        timeout,
        output_handler,
        output_stream,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_gemini_resume(
    work_dir: &Path,
    bin: &str,
    model: Model,
    session_id: &str,
    message: &str,
    timeout: Option<Duration>,
    output_handler: Option<OutputHandler>,
    output_stream: OutputStream,
) -> Result<RunnerOutput, RunnerError> {
    let (cmd, payload, _guards) = RunnerCommandBuilder::new(bin, work_dir)
        .arg("--resume")
        .arg(session_id)
        .model(&model)
        .output_format("stream-json")
        .arg("--approval-mode")
        .arg("yolo")
        .arg(message)
        .build();

    run_with_streaming_json(
        cmd,
        payload.as_deref(),
        bin,
        timeout,
        output_handler,
        output_stream,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_claude(
    work_dir: &Path,
    bin: &str,
    model: Model,
    prompt: &str,
    timeout: Option<Duration>,
    permission_mode: Option<ClaudePermissionMode>,
    output_handler: Option<OutputHandler>,
    output_stream: OutputStream,
) -> Result<RunnerOutput, RunnerError> {
    let (cmd, payload, _guards) = RunnerCommandBuilder::new(bin, work_dir)
        .arg("-p")
        .model(&model)
        .permission_mode(permission_mode)
        .output_format("stream-json")
        .arg("--verbose")
        .stdin_payload(Some(prompt.as_bytes().to_vec()))
        .build();

    run_with_streaming_json(
        cmd,
        payload.as_deref(),
        bin,
        timeout,
        output_handler,
        output_stream,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_claude_resume(
    work_dir: &Path,
    bin: &str,
    model: Model,
    session_id: &str,
    message: &str,
    timeout: Option<Duration>,
    permission_mode: Option<ClaudePermissionMode>,
    output_handler: Option<OutputHandler>,
    output_stream: OutputStream,
) -> Result<RunnerOutput, RunnerError> {
    let (cmd, payload, _guards) = RunnerCommandBuilder::new(bin, work_dir)
        .arg("--resume")
        .arg(session_id)
        .model(&model)
        .permission_mode(permission_mode)
        .output_format("stream-json")
        .arg("--verbose")
        .arg("-p")
        .arg(message)
        .build();

    run_with_streaming_json(
        cmd,
        payload.as_deref(),
        bin,
        timeout,
        output_handler,
        output_stream,
    )
}
