//! Cursor runner plugin implementation.
//!
//! Purpose:
//! - Cursor runner plugin implementation.
//!
//! Responsibilities:
//! - Build Cursor CLI commands for run and resume operations.
//! - Parse Cursor JSON response format.
//!
//! Not handled here:
//! - Process execution (handled by parent module).
//! - CLI option resolution (handled by cli_spec module).
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/Assumptions:
//! - Keep behavior aligned with CueLoop's canonical CLI, machine-contract, and queue semantics.

use std::any::Any;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{Value as JsonValue, json};

use crate::commands::run::PhaseType;
use crate::constants::defaults::DEFAULT_CURSOR_MODEL;
use crate::contracts::{Runner, RunnerSandboxMode};
use crate::fsutil;
use crate::runner::RunnerError;

use super::super::command::RunnerCommandBuilder;
use super::super::plugin_trait::{
    PluginCommandParts, ResponseParser, ResumeContext, RunContext, RunnerMetadata, RunnerPlugin,
};
use super::apply_analytics_env;

const CURSOR_SDK_RUNNER: &str = include_str!("../../../../assets/cursor_sdk_runner.mjs");

fn assistant_stream_chunk(content: &JsonValue) -> Option<String> {
    match content {
        JsonValue::String(text) => {
            if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            }
        }
        JsonValue::Array(items) => {
            let mut out = String::new();
            for item in items {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    out.push_str(text);
                }
            }
            if out.is_empty() { None } else { Some(out) }
        }
        _ => None,
    }
}

fn assistant_message(json: &JsonValue) -> Option<&JsonValue> {
    json.get("message")
        .filter(|message| message.get("role").and_then(|r| r.as_str()) == Some("assistant"))
}

/// Cursor plugin implementation.
pub struct CursorPlugin;

impl RunnerPlugin for CursorPlugin {
    fn metadata(&self) -> RunnerMetadata {
        super::BuiltInRunnerPlugin::Cursor.metadata()
    }

    fn build_run_command(&self, ctx: RunContext<'_>) -> Result<PluginCommandParts, RunnerError> {
        let (helper_path, helper_guards) = write_cursor_sdk_helper(ctx.bin)?;
        let request = cursor_sdk_request(CursorSdkRequest {
            operation: "run",
            work_dir: ctx.work_dir,
            model: &ctx.model,
            message: ctx.prompt,
            agent_id: None,
            opts: ctx.runner_cli,
            phase_type: ctx.phase_type.unwrap_or(PhaseType::Implementation),
            force: false,
        })?;
        let builder = RunnerCommandBuilder::new(ctx.bin, ctx.work_dir)
            .args([helper_path.as_os_str()])
            .stdin_payload(Some(request));
        let builder = apply_analytics_env(builder, &Runner::Cursor, &ctx.model);
        Ok(with_additional_guards(builder.build(), helper_guards))
    }

    fn build_resume_command(
        &self,
        ctx: ResumeContext<'_>,
    ) -> Result<PluginCommandParts, RunnerError> {
        let (helper_path, helper_guards) = write_cursor_sdk_helper(ctx.bin)?;
        let request = cursor_sdk_request(CursorSdkRequest {
            operation: "resume",
            work_dir: ctx.work_dir,
            model: &ctx.model,
            message: ctx.message,
            agent_id: Some(ctx.session_id),
            opts: ctx.runner_cli,
            phase_type: ctx.phase_type.unwrap_or(PhaseType::Implementation),
            force: ctx.force,
        })?;
        let builder = RunnerCommandBuilder::new(ctx.bin, ctx.work_dir)
            .args([helper_path.as_os_str()])
            .stdin_payload(Some(request));
        let builder = apply_analytics_env(builder, &Runner::Cursor, &ctx.model);
        Ok(with_additional_guards(builder.build(), helper_guards))
    }

    fn parse_response_line(&self, line: &str, buffer: &mut String) -> Option<String> {
        let json = serde_json::from_str(line)
            .inspect_err(|e| log::trace!("Cursor response not valid JSON: {}", e))
            .ok()?;
        CursorResponseParser.parse_json(&json, buffer)
    }
}

fn write_cursor_sdk_helper(
    bin: &str,
) -> Result<(PathBuf, Vec<Box<dyn Any + Send + Sync>>), RunnerError> {
    let temp_dir = fsutil::create_cueloop_temp_dir("cursor-sdk-runner").map_err(|err| {
        RunnerError::Other(anyhow::anyhow!(
            "Cursor SDK runner setup failed (bin={bin}, step=create_temp_dir): {err}"
        ))
    })?;
    let mut helper = tempfile::Builder::new()
        .prefix("cursor_sdk_runner_")
        .suffix(".mjs")
        .tempfile_in(temp_dir.path())
        .map_err(|err| {
            RunnerError::Other(anyhow::anyhow!(
                "Cursor SDK runner setup failed (bin={bin}, step=create_helper_file): {err}"
            ))
        })?;
    helper
        .write_all(CURSOR_SDK_RUNNER.as_bytes())
        .map_err(|err| {
            RunnerError::Other(anyhow::anyhow!(
                "Cursor SDK runner setup failed (bin={bin}, step=write_helper_file): {err}"
            ))
        })?;
    helper.flush().map_err(|err| {
        RunnerError::Other(anyhow::anyhow!(
            "Cursor SDK runner setup failed (bin={bin}, step=flush_helper_file): {err}"
        ))
    })?;

    let helper_path = helper.path().to_path_buf();
    Ok((helper_path, vec![Box::new(helper), Box::new(temp_dir)]))
}

struct CursorSdkRequest<'a> {
    operation: &'a str,
    work_dir: &'a Path,
    model: &'a crate::contracts::Model,
    message: &'a str,
    agent_id: Option<&'a str>,
    opts: super::super::cli_options::ResolvedRunnerCliOptions,
    phase_type: PhaseType,
    force: bool,
}

fn cursor_sdk_request(args: CursorSdkRequest<'_>) -> Result<Vec<u8>, RunnerError> {
    let mut request = json!({
        "operation": args.operation,
        "cwd": args.work_dir.to_string_lossy(),
        "model": cursor_sdk_model_id(args.model),
        "message": args.message,
        "agent_id": args.agent_id,
        "sandbox_enabled": cursor_sandbox_enabled(args.opts, args.phase_type),
    });
    if args.force {
        request["force"] = json!(true);
    }
    serde_json::to_vec(&request).map_err(|err| {
        RunnerError::Other(anyhow::anyhow!(
            "Cursor SDK runner setup failed (step=serialize_request): {err}"
        ))
    })
}

fn cursor_sdk_model_id(model: &crate::contracts::Model) -> &str {
    match model.as_str() {
        "auto" | "gpt-5.4" | "gpt-5.3" | "gpt-5.3-codex" | "gpt-5.3-codex-spark" => {
            DEFAULT_CURSOR_MODEL
        }
        other => other,
    }
}

fn cursor_sandbox_enabled(
    opts: super::super::cli_options::ResolvedRunnerCliOptions,
    phase_type: PhaseType,
) -> bool {
    match opts.sandbox {
        RunnerSandboxMode::Enabled => true,
        RunnerSandboxMode::Disabled => false,
        RunnerSandboxMode::Default => phase_type == PhaseType::Planning,
    }
}

fn with_additional_guards(
    mut parts: PluginCommandParts,
    mut guards: Vec<Box<dyn Any + Send + Sync>>,
) -> PluginCommandParts {
    parts.2.append(&mut guards);
    parts
}

/// Response parser for Cursor's JSON format.
pub struct CursorResponseParser;

impl CursorResponseParser {
    /// Parse Cursor JSON response format.
    ///
    /// CueLoop's Cursor SDK helper emits normalized SDK stream events plus a terminal
    /// `result` event with `run.wait().result`. Legacy Cursor Agent CLI envelopes are
    /// still accepted by tests and offline transcript parsing.
    ///
    /// `assistant` events follow a Gemini-style `delta` flag when present: `delta: true`
    /// appends to the streaming buffer; explicit `delta: false` replaces the buffer with a
    /// full snapshot (replay-safe when the same full snapshot is seen twice). When `delta` is
    /// omitted, chunks still append so legacy Cursor streams without the flag keep working.
    pub(crate) fn parse_json(&self, json: &JsonValue, buffer: &mut String) -> Option<String> {
        match json.get("type").and_then(|t| t.as_str()) {
            Some("assistant") if let Some(message) = assistant_message(json) => {
                let content = message.get("content")?;
                let delta_flag = json
                    .get("delta")
                    .or_else(|| message.get("delta"))
                    .and_then(|d| d.as_bool());

                match delta_flag {
                    Some(false) => {
                        let text = super::extract_text_content(content)?;
                        buffer.clear();
                        buffer.push_str(&text);
                        Some(buffer.clone())
                    }
                    Some(true) | None => {
                        let chunk = assistant_stream_chunk(content)?;
                        buffer.push_str(&chunk);
                        Some(buffer.clone())
                    }
                }
            }
            Some("assistant") => None,
            // Legacy/alternate envelope used by some Cursor Agent builds.
            Some("message_end") if let Some(message) = assistant_message(json) => {
                let content = message.get("content")?;
                let text = super::extract_text_content(content)?;
                buffer.clear();
                buffer.push_str(&text);
                Some(buffer.clone())
            }
            Some("message_end") => None,
            Some("result") => {
                let result = json.get("result")?;
                let text = super::extract_text_content(result)?;
                buffer.clear();
                buffer.push_str(&text);
                Some(buffer.clone())
            }
            _ => None,
        }
    }
}

impl ResponseParser for CursorResponseParser {
    fn parse(&self, json: &JsonValue, buffer: &mut String) -> Option<String> {
        self.parse_json(json, buffer)
    }

    fn runner_id(&self) -> &str {
        "cursor"
    }
}

#[cfg(test)]
mod tests {
    use super::CURSOR_SDK_RUNNER;
    use serde_json::json;
    use std::process::Command;

    fn node_available() -> bool {
        Command::new("node")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn write_fake_sdk(root: &std::path::Path, version: &str) -> anyhow::Result<std::path::PathBuf> {
        let sdk_dir = root.join("@cursor/sdk");
        std::fs::create_dir_all(&sdk_dir)?;
        std::fs::write(
            sdk_dir.join("package.json"),
            format!(
                r#"{{"name":"@cursor/sdk","version":"{version}","type":"module","main":"index.js"}}"#
            ),
        )?;
        std::fs::write(
            sdk_dir.join("index.js"),
            r#"
export class Agent {
  constructor(id = 'agent-1') { this.agentId = id; this.closed = false; }
  static async create() { return new Agent(); }
  static async resume(id) { return new Agent(id); }
  async send() {
    return {
      id: 'run-1',
      async *stream() {
        yield { type: 'assistant', message: { content: [{ type: 'text', text: 'hello' }] } };
      },
      async wait() { return { status: 'finished', result: 'hello', id: 'run-1' }; }
    };
  }
  async close() { this.closed = true; }
}
"#,
        )?;
        Ok(sdk_dir.join("index.js"))
    }

    fn run_helper(work_dir: &std::path::Path) -> anyhow::Result<std::process::Output> {
        let helper = tempfile::NamedTempFile::with_suffix(".mjs")?;
        std::fs::write(helper.path(), CURSOR_SDK_RUNNER)?;
        let request = json!({
            "operation": "run",
            "cwd": work_dir,
            "model": "composer-2",
            "message": "hi"
        });
        let mut child = Command::new("node")
            .arg(helper.path())
            .current_dir(work_dir)
            .env("CURSOR_API_KEY", "fake-key")
            .env_remove("CUELOOP_CURSOR_SDK_MODULE_PATH")
            .env_remove("CUELOOP_CURSOR_SDK_GLOBAL_ROOT")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        use std::io::Write as _;
        child
            .stdin
            .take()
            .expect("helper stdin should be piped")
            .write_all(serde_json::to_string(&request)?.as_bytes())?;
        Ok(child.wait_with_output()?)
    }

    #[test]
    fn cursor_sdk_runner_warns_but_runs_with_workspace_version_drift() -> anyhow::Result<()> {
        if !node_available() {
            return Ok(());
        }
        let temp = tempfile::TempDir::new()?;
        std::fs::write(temp.path().join("package.json"), r#"{"type":"module"}"#)?;
        write_fake_sdk(&temp.path().join("node_modules"), "1.0.13")?;

        let output = run_helper(temp.path())?;

        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout)?;
        assert!(
            stdout.contains(r#""subtype":"cursor_sdk_warning""#),
            "stdout: {stdout}"
        );
        assert!(
            stdout.contains(r#""sdk_version":"1.0.13""#),
            "stdout: {stdout}"
        );
        assert!(stdout.contains(r#""type":"result""#), "stdout: {stdout}");
        Ok(())
    }

    #[test]
    fn cursor_sdk_runner_fails_for_structurally_invalid_workspace_sdk() -> anyhow::Result<()> {
        if !node_available() {
            return Ok(());
        }
        let temp = tempfile::TempDir::new()?;
        std::fs::write(temp.path().join("package.json"), r#"{"type":"module"}"#)?;
        let sdk_dir = temp.path().join("node_modules/@cursor/sdk");
        std::fs::create_dir_all(&sdk_dir)?;
        std::fs::write(
            sdk_dir.join("package.json"),
            r#"{"name":"@cursor/sdk","version":"1.0.12","type":"module","main":"index.js"}"#,
        )?;
        std::fs::write(sdk_dir.join("index.js"), "export const NotAgent = true;")?;

        let output = run_helper(temp.path())?;

        assert!(!output.status.success());
        let stdout = String::from_utf8(output.stdout)?;
        assert!(stdout.contains("does not expose Agent"), "stdout: {stdout}");
        Ok(())
    }
}
