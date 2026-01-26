//! Runner-specific command assembly for execution.

use anyhow::anyhow;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::fsutil;

use super::super::{
    ClaudePermissionMode, Model, OutputHandler, OutputStream, ReasoningEffort, RunnerError,
    RunnerOutput, OPENCODE_PROMPT_FILE_MESSAGE, TEMP_RETENTION,
};
use super::process::{ensure_self_on_path, run_with_streaming_json};

pub(super) fn permission_mode_to_arg(mode: ClaudePermissionMode) -> &'static str {
    match mode {
        ClaudePermissionMode::AcceptEdits => "acceptEdits",
        ClaudePermissionMode::BypassPermissions => "bypassPermissions",
    }
}

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
    let mut cmd = Command::new(bin);
    cmd.current_dir(work_dir);
    ensure_self_on_path(&mut cmd);
    cmd.arg("exec")
        .arg("--json")
        .arg("--model")
        .arg(model.as_str());

    if let Some(effort) = reasoning_effort {
        cmd.arg("-c").arg(format!(
            "model_reasoning_effort=\"{}\"",
            effort_as_str(effort)
        ));
    }

    cmd.arg("-");
    run_with_streaming_json(
        cmd,
        Some(prompt.as_bytes()),
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
    let mut cmd = Command::new(bin);
    cmd.current_dir(work_dir);
    ensure_self_on_path(&mut cmd);
    cmd.arg("exec")
        .arg("resume")
        .arg(thread_id)
        .arg("--json")
        .arg("--model")
        .arg(model.as_str());

    if let Some(effort) = reasoning_effort {
        cmd.arg("-c").arg(format!(
            "model_reasoning_effort=\"{}\"",
            effort_as_str(effort)
        ));
    }

    cmd.arg(message);
    run_with_streaming_json(cmd, None, bin, timeout, output_handler, output_stream)
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
    if let Err(err) = fsutil::cleanup_default_temp_dirs(TEMP_RETENTION) {
        log::warn!("temp cleanup failed: {:#}", err);
    }

    let temp_dir = fsutil::create_ralph_temp_dir("prompt")
        .map_err(|e| RunnerError::Other(anyhow!("create temp dir: {}", e)))?;
    let mut tmp = tempfile::Builder::new()
        .prefix("prompt_")
        .suffix(".md")
        .tempfile_in(temp_dir.path())
        .map_err(|e| RunnerError::Other(anyhow!("create temp prompt file: {}", e)))?;

    tmp.write_all(prompt.as_bytes())
        .map_err(|e| RunnerError::Other(anyhow!("write prompt file: {}", e)))?;
    tmp.flush()
        .map_err(|e| RunnerError::Other(anyhow!("flush prompt file: {}", e)))?;

    let mut cmd = Command::new(bin);
    cmd.current_dir(work_dir);
    ensure_self_on_path(&mut cmd);
    cmd.arg("run")
        .arg("--model")
        .arg(model.as_str())
        .arg("--format")
        .arg("json")
        .arg("--file")
        .arg(tmp.path())
        .arg("--")
        .arg(OPENCODE_PROMPT_FILE_MESSAGE)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    run_with_streaming_json(cmd, None, bin, timeout, output_handler, output_stream)
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
    let mut cmd = Command::new(bin);
    cmd.current_dir(work_dir);
    ensure_self_on_path(&mut cmd);
    cmd.arg("run")
        .arg("-s")
        .arg(session_id)
        .arg("--model")
        .arg(model.as_str())
        .arg("--format")
        .arg("json")
        .arg("--")
        .arg(message);
    run_with_streaming_json(cmd, None, bin, timeout, output_handler, output_stream)
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
    let mut cmd = Command::new(bin);
    cmd.current_dir(work_dir);
    ensure_self_on_path(&mut cmd);
    cmd.arg("--model")
        .arg(model.as_str())
        .arg("--output-format")
        .arg("stream-json")
        .arg("--approval-mode")
        .arg("yolo");
    run_with_streaming_json(
        cmd,
        Some(prompt.as_bytes()),
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
    let mut cmd = Command::new(bin);
    cmd.current_dir(work_dir);
    ensure_self_on_path(&mut cmd);
    cmd.arg("--resume")
        .arg(session_id)
        .arg("--model")
        .arg(model.as_str())
        .arg("--output-format")
        .arg("stream-json")
        .arg("--approval-mode")
        .arg("yolo")
        .arg(message);
    run_with_streaming_json(cmd, None, bin, timeout, output_handler, output_stream)
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
    let mode = permission_mode.unwrap_or(ClaudePermissionMode::BypassPermissions);
    let mut cmd = Command::new(bin);
    cmd.current_dir(work_dir);
    ensure_self_on_path(&mut cmd);
    cmd.arg("-p")
        .arg("--model")
        .arg(model.as_str())
        .arg("--permission-mode")
        .arg(permission_mode_to_arg(mode))
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose");
    run_with_streaming_json(
        cmd,
        Some(prompt.as_bytes()),
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
    let mode = permission_mode.unwrap_or(ClaudePermissionMode::BypassPermissions);
    let mut cmd = Command::new(bin);
    cmd.current_dir(work_dir);
    ensure_self_on_path(&mut cmd);
    cmd.arg("--resume")
        .arg(session_id)
        .arg("--model")
        .arg(model.as_str())
        .arg("--permission-mode")
        .arg(permission_mode_to_arg(mode))
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("-p")
        .arg(message);
    run_with_streaming_json(cmd, None, bin, timeout, output_handler, output_stream)
}

pub(super) fn effort_as_str(effort: ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::High => "high",
        ReasoningEffort::XHigh => "xhigh",
    }
}
