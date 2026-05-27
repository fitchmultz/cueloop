//! Pi runner plugin implementation.
//!
//! Purpose:
//! - Pi runner plugin implementation.
//!
//! Responsibilities:
//! - Build Pi CLI commands for run and resume operations.
//! - Wrap Pi's Node entrypoint so its process-title mutation cannot expose inherited secrets.
//! - Parse Pi JSON response format.
//! - Pass Pi 0.76+ `--session-id` for deterministic project-local sessions.
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

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;

use crate::contracts::Runner;
use crate::fsutil;
use crate::runner::{ResolvedRunnerCliOptions, RunnerError, runner_execution_error_with_source};

use super::super::cli_spec::apply_pi_options;
use super::super::command::RunnerCommandBuilder;
use super::super::plugin_trait::{
    PluginCommandParts, ResponseParser, ResumeContext, RunContext, RunnerMetadata, RunnerPlugin,
};
use super::{apply_analytics_env, extract_text_content};

/// Pi plugin implementation.
pub struct PiPlugin;

struct PiCommandRequest<'a> {
    work_dir: &'a Path,
    bin: &'a str,
    runner_cli: ResolvedRunnerCliOptions,
    model: &'a crate::contracts::Model,
    prompt: &'a str,
    session_arg: Option<PiSessionArg<'a>>,
    reasoning_effort: Option<crate::contracts::ReasoningEffort>,
}

enum PiSessionArg<'a> {
    Id(&'a str),
    FilePath(&'a str),
}

impl<'a> PiSessionArg<'a> {
    fn from_stored(value: &'a str) -> Self {
        if Path::new(value).is_file() {
            Self::FilePath(value)
        } else {
            Self::Id(value)
        }
    }

    fn apply_to(self, builder: RunnerCommandBuilder) -> RunnerCommandBuilder {
        match self {
            Self::Id(id) => builder.arg("--session-id").arg(id),
            Self::FilePath(path) => builder.arg("--session").arg(path),
        }
    }
}

impl RunnerPlugin for PiPlugin {
    fn metadata(&self) -> RunnerMetadata {
        super::BuiltInRunnerPlugin::Pi.metadata()
    }

    fn build_run_command(&self, ctx: RunContext<'_>) -> Result<PluginCommandParts, RunnerError> {
        self.build_pi_command(PiCommandRequest {
            work_dir: ctx.work_dir,
            bin: ctx.bin,
            runner_cli: ctx.runner_cli,
            model: &ctx.model,
            prompt: ctx.prompt,
            session_arg: ctx.session_id.as_deref().map(PiSessionArg::from_stored),
            reasoning_effort: ctx.reasoning_effort,
        })
    }

    fn build_resume_command(
        &self,
        ctx: ResumeContext<'_>,
    ) -> Result<PluginCommandParts, RunnerError> {
        self.build_pi_command(PiCommandRequest {
            work_dir: ctx.work_dir,
            bin: ctx.bin,
            runner_cli: ctx.runner_cli,
            model: &ctx.model,
            prompt: ctx.message,
            session_arg: Some(PiSessionArg::from_stored(ctx.session_id)),
            reasoning_effort: ctx.reasoning_effort,
        })
    }

    fn parse_response_line(&self, line: &str, _buffer: &mut String) -> Option<String> {
        let json = serde_json::from_str(line)
            .inspect_err(|e| log::trace!("Pi response not valid JSON: {}", e))
            .ok()?;
        PiResponseParser.parse_json(&json)
    }
}

impl PiPlugin {
    fn build_pi_command(
        &self,
        request: PiCommandRequest<'_>,
    ) -> Result<PluginCommandParts, RunnerError> {
        let (builder, mut temp_resources) =
            if let Some(entrypoint) = pi_node_entrypoint(request.bin) {
                pi_wrapper_builder(request.work_dir, request.bin, &entrypoint)?
            } else {
                (
                    RunnerCommandBuilder::new(request.bin, request.work_dir),
                    Vec::new(),
                )
            };
        let builder = apply_analytics_env(builder, &Runner::Pi, request.model);
        let builder = apply_pi_options(builder, request.runner_cli);

        let builder = match request.session_arg {
            Some(session_arg) => session_arg.apply_to(builder),
            None => builder,
        };

        let (cmd, payload, mut builder_resources) = builder
            .model(request.model)
            .thinking_level(request.reasoning_effort)
            .arg("--mode")
            .arg("json")
            .arg(request.prompt)
            .build();
        temp_resources.append(&mut builder_resources);
        Ok((cmd, payload, temp_resources))
    }
}

fn pi_wrapper_builder(
    work_dir: &Path,
    bin: &str,
    entrypoint: &Path,
) -> Result<
    (
        RunnerCommandBuilder,
        Vec<Box<dyn std::any::Any + Send + Sync>>,
    ),
    RunnerError,
> {
    let temp_dir = fsutil::create_cueloop_temp_dir("pi-wrapper").map_err(|err| {
        runner_execution_error_with_source(&Runner::Pi, bin, "create pi wrapper temp dir", err)
    })?;
    let mut wrapper = tempfile::Builder::new()
        .prefix("cueloop_pi_wrapper_")
        .suffix(".mjs")
        .tempfile_in(temp_dir.path())
        .map_err(|err| {
            runner_execution_error_with_source(&Runner::Pi, bin, "create pi wrapper", err)
        })?;

    wrapper
        .write_all(PI_WRAPPER_SOURCE.as_bytes())
        .map_err(|err| {
            runner_execution_error_with_source(&Runner::Pi, bin, "write pi wrapper", err)
        })?;
    wrapper.flush().map_err(|err| {
        runner_execution_error_with_source(&Runner::Pi, bin, "flush pi wrapper", err)
    })?;

    let wrapper_path = wrapper.path().to_string_lossy().to_string();
    let entrypoint_path = entrypoint.to_string_lossy().to_string();
    let builder = RunnerCommandBuilder::new("node", work_dir)
        .arg(&wrapper_path)
        .env("CUELOOP_PI_BIN", bin)
        .env("CUELOOP_PI_ENTRYPOINT", &entrypoint_path);
    Ok((builder, vec![Box::new(wrapper), Box::new(temp_dir)]))
}

fn pi_node_entrypoint(bin: &str) -> Option<PathBuf> {
    resolve_executable_path(bin).filter(|path| is_node_script(path))
}

fn resolve_executable_path(bin: &str) -> Option<PathBuf> {
    let direct = Path::new(bin);
    if direct.is_absolute() || direct.components().count() > 1 {
        return direct.canonicalize().ok();
    }

    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(bin))
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| candidate.canonicalize().ok())
}

fn is_node_script(path: &Path) -> bool {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension, "js" | "mjs" | "cjs"))
        .unwrap_or(false)
    {
        return true;
    }

    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };
    let mut buffer = [0_u8; 128];
    let bytes = match file.read(&mut buffer) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let prefix = String::from_utf8_lossy(&buffer[..bytes]);
    prefix.starts_with("#!") && prefix.contains("node")
}

const PI_WRAPPER_SOURCE: &str = r#"
import { existsSync, realpathSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

const piBin = process.env.CUELOOP_PI_BIN;
if (!piBin) {
  throw new Error("CUELOOP_PI_BIN is required");
}
const piEntrypoint = process.env.CUELOOP_PI_ENTRYPOINT;
if (!piEntrypoint) {
  throw new Error("CUELOOP_PI_ENTRYPOINT is required");
}

Object.defineProperty(process, "title", {
  configurable: false,
  enumerable: true,
  get() {
    return "pi";
  },
  set(_) {}
});

process.argv = [process.argv[0], piBin, ...process.argv.slice(2)];
process.env.PI_CODING_AGENT = "true";
process.emitWarning = (() => {});

const entrypoint = realpathSync(piEntrypoint);
const piMain = join(dirname(entrypoint), "main.js");
if (existsSync(piMain)) {
  const { main } = await import(pathToFileURL(piMain).href);
  await main(process.argv.slice(2));
  process.exit(process.exitCode ?? 0);
} else {
  await import(pathToFileURL(entrypoint).href);
}
"#;

/// Response parser for Pi's JSON format.
pub struct PiResponseParser;

impl PiResponseParser {
    /// Parse Pi JSON response format.
    pub(crate) fn parse_json(&self, json: &JsonValue) -> Option<String> {
        match json.get("type").and_then(|t| t.as_str()) {
            // Pi emits result objects in --mode json output.
            Some("result") => {
                let result = json.get("result")?;
                extract_text_content(result)
            }
            // Some Pi builds emit assistant output in message_end envelopes.
            Some("message_end") => {
                let message = json.get("message")?;
                if message.get("role").and_then(|r| r.as_str()) != Some("assistant") {
                    return None;
                }
                let content = message.get("content")?;
                extract_text_content(content)
            }
            _ => None,
        }
    }
}

impl ResponseParser for PiResponseParser {
    fn parse(&self, json: &JsonValue, _buffer: &mut String) -> Option<String> {
        self.parse_json(json)
    }

    fn runner_id(&self) -> &str {
        "pi"
    }
}
