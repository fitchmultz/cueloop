//! Runner-related configuration contracts.
//!
//! Purpose:
//! - Runner-related configuration contracts.
//!
//! Responsibilities:
//! - Define runner identity (`Runner`) as a string-serialized value (built-ins + plugins).
//! - Define runner CLI normalization types (approval/sandbox/plan/etc).
//!
//! Not handled here:
//! - Plugin discovery / registry (see `crate::plugins`).
//! - Runner execution dispatch (see `crate::runner`).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - `Runner` MUST serialize to a single string token for config/CLI stability.
//! - Unknown tokens are treated as plugin runner ids (non-empty, trimmed).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use std::collections::BTreeMap;

use super::enum_parse::snake_case_from_str;

pub(crate) const RUNNER_SCHEMA_DESCRIPTION: &str = concat!(
    "Runner id. Built-in runner IDs: codex, opencode, gemini, claude, cursor, kimi, pi. ",
    "Plugin runner IDs are also supported as non-empty strings."
);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum Runner {
    Codex,
    Opencode,
    Gemini,
    Cursor,
    #[default]
    Claude,
    Kimi,
    Pi,
    Plugin(String),
}

impl Runner {
    /// Returns the string representation of the runner.
    pub fn as_str(&self) -> &str {
        match self {
            Runner::Codex => "codex",
            Runner::Opencode => "opencode",
            Runner::Gemini => "gemini",
            Runner::Cursor => "cursor",
            Runner::Claude => "claude",
            Runner::Kimi => "kimi",
            Runner::Pi => "pi",
            Runner::Plugin(id) => id.as_str(),
        }
    }

    pub fn id(&self) -> &str {
        self.as_str()
    }

    pub fn is_plugin(&self) -> bool {
        matches!(self, Runner::Plugin(_))
    }
}

impl std::fmt::Display for Runner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id())
    }
}

impl std::str::FromStr for Runner {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let token = value.trim();
        if token.is_empty() {
            return Err("runner must be non-empty");
        }
        Ok(
            match super::enum_parse::normalize_enum_token(token).as_str() {
                "codex" => Runner::Codex,
                "opencode" => Runner::Opencode,
                "gemini" => Runner::Gemini,
                "cursor" => Runner::Cursor,
                "claude" => Runner::Claude,
                "kimi" => Runner::Kimi,
                "pi" => Runner::Pi,
                _ => Runner::Plugin(token.to_string()),
            },
        )
    }
}

// Keep config/CLI stable: serialize as a single string token.
impl Serialize for Runner {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for Runner {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        raw.parse::<Runner>().map_err(serde::de::Error::custom)
    }
}

// Schema: treat as string; docs enumerate built-ins, but allow arbitrary plugin ids.
impl JsonSchema for Runner {
    fn schema_name() -> Cow<'static, str> {
        "Runner".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = <String as JsonSchema>::json_schema(generator);
        let obj = schema.ensure_object();
        obj.entry("description".to_string())
            .or_insert_with(|| json!(RUNNER_SCHEMA_DESCRIPTION));
        obj.insert(
            "examples".to_string(),
            json!(["claude", "acme.super_runner"]),
        );
        schema
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaudePermissionMode {
    #[default]
    AcceptEdits,
    BypassPermissions,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunnerOutputFormat {
    /// Newline-delimited JSON objects (required for CueLoop's streaming parser).
    #[default]
    StreamJson,
    /// JSON output (may not be streaming; currently treated as unsupported by CueLoop execution).
    Json,
    /// Plain text output (currently treated as unsupported by CueLoop execution).
    Text,
}

snake_case_from_str! {
    RunnerOutputFormat {
        StreamJson => "stream_json",
        Json => "json",
        Text => "text",
    }
    "output_format must be 'stream_json', 'json', or 'text'"
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunnerVerbosity {
    Quiet,
    #[default]
    Normal,
    Verbose,
}

snake_case_from_str! {
    RunnerVerbosity {
        Quiet => "quiet",
        Normal => "normal",
        Verbose => "verbose",
    }
    "verbosity must be 'quiet', 'normal', or 'verbose'"
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunnerApprovalMode {
    /// Do not apply any approval flags; runner defaults apply.
    Default,
    /// Attempt to auto-approve edits but not all tool actions (runner-specific).
    AutoEdits,
    /// Bypass approvals / run headless (runner-specific).
    #[default]
    Yolo,
    /// Strict safety mode. Warning: some runners may become interactive and hang.
    Safe,
}

snake_case_from_str! {
    RunnerApprovalMode {
        Default => "default",
        AutoEdits => "auto_edits",
        Yolo => "yolo",
        Safe => "safe",
    }
    "approval_mode must be 'default', 'auto_edits', 'yolo', or 'safe'"
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunnerSandboxMode {
    #[default]
    Default,
    Enabled,
    Disabled,
}

snake_case_from_str! {
    RunnerSandboxMode {
        Default => "default",
        Enabled => "enabled",
        Disabled => "disabled",
    }
    "sandbox must be 'default', 'enabled', or 'disabled'"
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunnerPlanMode {
    #[default]
    Default,
    Enabled,
    Disabled,
}

snake_case_from_str! {
    RunnerPlanMode {
        Default => "default",
        Enabled => "enabled",
        Disabled => "disabled",
    }
    "plan_mode must be 'default', 'enabled', or 'disabled'"
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedOptionPolicy {
    Ignore,
    #[default]
    Warn,
    Error,
}

snake_case_from_str! {
    UnsupportedOptionPolicy {
        Ignore => "ignore",
        Warn => "warn",
        Error => "error",
    }
    "unsupported_option_policy must be 'ignore', 'warn', or 'error'"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct RunnerCliConfigRoot {
    /// Default normalized runner CLI options applied to all runners (unless overridden).
    pub defaults: RunnerCliOptionsPatch,

    /// Optional per-runner overrides, merged leaf-wise over `defaults`.
    pub runners: BTreeMap<Runner, RunnerCliOptionsPatch>,
}

impl RunnerCliConfigRoot {
    pub fn merge_from(&mut self, other: Self) {
        self.defaults.merge_from(other.defaults);
        for (runner, patch) in other.runners {
            self.runners
                .entry(runner)
                .and_modify(|existing| existing.merge_from(patch.clone()))
                .or_insert(patch);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct RunnerCliOptionsPatch {
    /// Desired output format for runner execution.
    pub output_format: Option<RunnerOutputFormat>,

    /// Desired verbosity (when supported by the runner).
    pub verbosity: Option<RunnerVerbosity>,

    /// Desired approval/permission behavior.
    pub approval_mode: Option<RunnerApprovalMode>,

    /// Desired sandbox behavior (when supported by the runner).
    pub sandbox: Option<RunnerSandboxMode>,

    /// Desired plan/read-only behavior (when supported by the runner).
    pub plan_mode: Option<RunnerPlanMode>,

    /// Policy for unsupported options (warn/error/ignore).
    pub unsupported_option_policy: Option<UnsupportedOptionPolicy>,
}

impl RunnerCliOptionsPatch {
    pub fn merge_from(&mut self, other: Self) {
        fn merge<T>(target: &mut Option<T>, incoming: Option<T>) {
            if incoming.is_some() {
                *target = incoming;
            }
        }
        merge(&mut self.output_format, other.output_format);
        merge(&mut self.verbosity, other.verbosity);
        merge(&mut self.approval_mode, other.approval_mode);
        merge(&mut self.sandbox, other.sandbox);
        merge(&mut self.plan_mode, other.plan_mode);
        merge(
            &mut self.unsupported_option_policy,
            other.unsupported_option_policy,
        );
    }
}

#[cfg(test)]
mod tests;
