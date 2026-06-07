//! Parsing functions for agent-related CLI inputs.
//!
//! Purpose:
//! - Parsing functions for agent-related CLI inputs.
//!
//! Responsibilities:
//! - Parse runner strings into Runner enum variants.
//! - Parse git revert mode strings into GitRevertMode enum.
//! - Parse runner CLI arguments into RunnerCliOptionsPatch structs.
//!
//! Not handled here:
//! - Model parsing (see `crate::runner`).
//! - Reasoning effort parsing (see `crate::runner`).
//! - Override resolution (see `super::resolve`).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Parsing is case-insensitive for runner strings.
//! - Invalid inputs return descriptive errors via anyhow.

use crate::contracts::{GitPublishMode, GitRevertMode, Runner, RunnerCliOptionsPatch};
use anyhow::{Result, anyhow};

use super::args::RunnerCliArgs;

/// Parse a runner string into a Runner enum.
///
/// Rejects plugin runner IDs — CLI `--runner` only accepts built-in names.
pub fn parse_runner(value: &str) -> Result<Runner> {
    let runner: Runner = value.parse().map_err(|err: &str| anyhow!(err))?;
    if runner.is_plugin() {
        anyhow::bail!(
            "Invalid runner: --runner must be 'codex', 'opencode', 'gemini', 'claude', 'cursor', 'kimi', or 'pi' (got: {}). Set a supported runner in .cueloop/config.jsonc or via the --runner flag.",
            value.trim()
        );
    }
    Ok(runner)
}

/// Parse git revert mode from a CLI string.
pub fn parse_git_revert_mode(value: &str) -> Result<GitRevertMode> {
    value.parse().map_err(|err: &str| anyhow!(err))
}

/// Parse git publish mode from a CLI string.
pub fn parse_git_publish_mode(value: &str) -> Result<GitPublishMode> {
    value.parse().map_err(|err: &str| anyhow!(err))
}

/// Parse runner CLI arguments into a patch struct.
pub(crate) fn parse_runner_cli_patch(args: &RunnerCliArgs) -> Result<RunnerCliOptionsPatch> {
    fn parse_opt<T: std::str::FromStr<Err = &'static str>>(
        value: Option<&str>,
    ) -> Result<Option<T>> {
        match value {
            Some(v) => Ok(Some(v.parse::<T>().map_err(|err| anyhow!(err))?)),
            None => Ok(None),
        }
    }

    Ok(RunnerCliOptionsPatch {
        output_format: parse_opt(args.output_format.as_deref())?,
        verbosity: parse_opt(args.verbosity.as_deref())?,
        approval_mode: parse_opt(args.approval_mode.as_deref())?,
        sandbox: parse_opt(args.sandbox.as_deref())?,
        plan_mode: parse_opt(args.plan_mode.as_deref())?,
        unsupported_option_policy: parse_opt(args.unsupported_option_policy.as_deref())?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_runner_accepts_valid_runners() {
        assert!(matches!(parse_runner("codex"), Ok(Runner::Codex)));
        assert!(matches!(parse_runner("opencode"), Ok(Runner::Opencode)));
        assert!(matches!(parse_runner("gemini"), Ok(Runner::Gemini)));
        assert!(matches!(parse_runner("claude"), Ok(Runner::Claude)));
        assert!(matches!(parse_runner("cursor"), Ok(Runner::Cursor)));
        assert!(matches!(parse_runner("kimi"), Ok(Runner::Kimi)));
        assert!(matches!(parse_runner("pi"), Ok(Runner::Pi)));
        assert!(matches!(parse_runner("CODEX"), Ok(Runner::Codex)));
    }

    #[test]
    fn parse_runner_rejects_invalid_runners() {
        assert!(parse_runner("invalid").is_err());
        assert!(parse_runner("").is_err());
    }
}
