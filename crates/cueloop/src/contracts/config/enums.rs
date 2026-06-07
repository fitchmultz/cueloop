//! Configuration enums for project type, git revert/publish mode, and scan prompt version.
//!
//! Purpose:
//! - Configuration enums for project type, git revert/publish mode, and scan prompt version.
//!
//! Responsibilities:
//! - Define simple enum types used across configuration.
//!
//! Not handled here:
//! - Complex config structs with merge behavior (see other config modules).
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/Assumptions:
//! - Keep behavior aligned with CueLoop's canonical CLI, machine-contract, and queue semantics.

use crate::contracts::enum_parse::snake_case_from_str;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Project type classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    #[default]
    Code,
    Docs,
}

/// Git revert mode for handling runner/supervision errors.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GitRevertMode {
    #[default]
    Ask,
    Enabled,
    Disabled,
}

snake_case_from_str! {
    GitRevertMode {
        Ask => "ask",
        Enabled => "enabled",
        Disabled => "disabled",
    }
    "git_revert_mode must be 'ask', 'enabled', or 'disabled'"
}

/// Git publish mode for post-run repository changes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GitPublishMode {
    /// Leave the repository dirty after queue/done updates.
    #[default]
    Off,
    /// Create a local commit but do not push.
    Commit,
    /// Create a local commit and push it using CueLoop's guarded push flow.
    CommitAndPush,
}

impl GitPublishMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            GitPublishMode::Off => "off",
            GitPublishMode::Commit => "commit",
            GitPublishMode::CommitAndPush => "commit_and_push",
        }
    }
}

snake_case_from_str! {
    GitPublishMode {
        Off => "off",
        Commit => "commit",
        CommitAndPush => "commit_and_push",
    }
    "git_publish_mode must be 'off', 'commit', or 'commit_and_push'"
}

/// Scan prompt version to use for scan operations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScanPromptVersion {
    /// Version 1: Original rule-based scan prompts with fixed minimum task counts.
    V1,
    /// Version 2: Rubric-based scan prompts with quality-focused STOP CONDITION (default).
    #[default]
    V2,
}
