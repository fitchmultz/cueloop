use crate::config::ConfigLayer;
use crate::contracts::{ClaudePermissionMode, GitRevertMode, Model, ReasoningEffort, Runner};
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFieldKind {
    Cycle,
    Toggle,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigKey {
    ProjectType,
    QueueFile,
    QueueDoneFile,
    QueueIdPrefix,
    QueueIdWidth,
    AgentRunner,
    AgentModel,
    AgentReasoningEffort,
    AgentIterations,
    AgentFollowupReasoningEffort,
    AgentCodexBin,
    AgentOpencodeBin,
    AgentGeminiBin,
    AgentClaudeBin,
    AgentClaudePermissionMode,
    AgentRepopromptPlanRequired,
    AgentRepopromptToolInjection,
    AgentGitRevertMode,
    AgentGitCommitPushEnabled,
    AgentPhases,
}

#[derive(Debug, Clone)]
pub struct ConfigEntry {
    pub key: ConfigKey,
    pub label: &'static str,
    pub value: String,
    pub kind: ConfigFieldKind,
}

fn default_config_value() -> String {
    "(global default)".to_string()
}

fn display_project_type(value: Option<crate::contracts::ProjectType>) -> String {
    match value {
        Some(crate::contracts::ProjectType::Code) => "code".to_string(),
        Some(crate::contracts::ProjectType::Docs) => "docs".to_string(),
        None => default_config_value(),
    }
}

fn display_runner(value: Option<Runner>) -> String {
    match value {
        Some(Runner::Codex) => "codex".to_string(),
        Some(Runner::Opencode) => "opencode".to_string(),
        Some(Runner::Gemini) => "gemini".to_string(),
        Some(Runner::Claude) => "claude".to_string(),
        None => default_config_value(),
    }
}

fn display_reasoning_effort(value: Option<ReasoningEffort>) -> String {
    match value {
        Some(ReasoningEffort::Low) => "low".to_string(),
        Some(ReasoningEffort::Medium) => "medium".to_string(),
        Some(ReasoningEffort::High) => "high".to_string(),
        Some(ReasoningEffort::XHigh) => "xhigh".to_string(),
        None => default_config_value(),
    }
}

fn display_claude_permission_mode(value: Option<ClaudePermissionMode>) -> String {
    match value {
        Some(ClaudePermissionMode::AcceptEdits) => "accept_edits".to_string(),
        Some(ClaudePermissionMode::BypassPermissions) => "bypass_permissions".to_string(),
        None => default_config_value(),
    }
}

fn display_git_revert_mode(value: Option<GitRevertMode>) -> String {
    match value {
        Some(GitRevertMode::Ask) => "ask".to_string(),
        Some(GitRevertMode::Enabled) => "enabled".to_string(),
        Some(GitRevertMode::Disabled) => "disabled".to_string(),
        None => default_config_value(),
    }
}

fn display_model(value: Option<&Model>) -> String {
    match value {
        Some(model) => model.as_str().to_string(),
        None => default_config_value(),
    }
}

fn display_string(value: Option<&String>) -> String {
    match value {
        Some(s) if !s.trim().is_empty() => s.clone(),
        _ => default_config_value(),
    }
}

fn display_path(value: Option<&PathBuf>) -> String {
    match value {
        Some(p) => p.to_string_lossy().to_string(),
        None => default_config_value(),
    }
}

fn display_u8(value: Option<u8>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => default_config_value(),
    }
}

fn display_bool(value: Option<bool>) -> String {
    match value {
        Some(true) => "true".to_string(),
        Some(false) => "false".to_string(),
        None => default_config_value(),
    }
}

pub fn config_entries(project_config: &ConfigLayer) -> Vec<ConfigEntry> {
    vec![
        ConfigEntry {
            key: ConfigKey::ProjectType,
            label: "project_type",
            value: display_project_type(project_config.project_type),
            kind: ConfigFieldKind::Cycle,
        },
        ConfigEntry {
            key: ConfigKey::QueueFile,
            label: "queue.file",
            value: display_path(project_config.queue.file.as_ref()),
            kind: ConfigFieldKind::Text,
        },
        ConfigEntry {
            key: ConfigKey::QueueDoneFile,
            label: "queue.done_file",
            value: display_path(project_config.queue.done_file.as_ref()),
            kind: ConfigFieldKind::Text,
        },
        ConfigEntry {
            key: ConfigKey::QueueIdPrefix,
            label: "queue.id_prefix",
            value: display_string(project_config.queue.id_prefix.as_ref()),
            kind: ConfigFieldKind::Text,
        },
        ConfigEntry {
            key: ConfigKey::QueueIdWidth,
            label: "queue.id_width",
            value: display_u8(project_config.queue.id_width),
            kind: ConfigFieldKind::Text,
        },
        ConfigEntry {
            key: ConfigKey::AgentRunner,
            label: "agent.runner",
            value: display_runner(project_config.agent.runner),
            kind: ConfigFieldKind::Cycle,
        },
        ConfigEntry {
            key: ConfigKey::AgentModel,
            label: "agent.model",
            value: display_model(project_config.agent.model.as_ref()),
            kind: ConfigFieldKind::Text,
        },
        ConfigEntry {
            key: ConfigKey::AgentReasoningEffort,
            label: "agent.reasoning_effort",
            value: display_reasoning_effort(project_config.agent.reasoning_effort),
            kind: ConfigFieldKind::Cycle,
        },
        ConfigEntry {
            key: ConfigKey::AgentIterations,
            label: "agent.iterations",
            value: display_u8(project_config.agent.iterations),
            kind: ConfigFieldKind::Text,
        },
        ConfigEntry {
            key: ConfigKey::AgentFollowupReasoningEffort,
            label: "agent.followup_reasoning_effort",
            value: display_reasoning_effort(project_config.agent.followup_reasoning_effort),
            kind: ConfigFieldKind::Cycle,
        },
        ConfigEntry {
            key: ConfigKey::AgentCodexBin,
            label: "agent.codex_bin",
            value: display_string(project_config.agent.codex_bin.as_ref()),
            kind: ConfigFieldKind::Text,
        },
        ConfigEntry {
            key: ConfigKey::AgentOpencodeBin,
            label: "agent.opencode_bin",
            value: display_string(project_config.agent.opencode_bin.as_ref()),
            kind: ConfigFieldKind::Text,
        },
        ConfigEntry {
            key: ConfigKey::AgentGeminiBin,
            label: "agent.gemini_bin",
            value: display_string(project_config.agent.gemini_bin.as_ref()),
            kind: ConfigFieldKind::Text,
        },
        ConfigEntry {
            key: ConfigKey::AgentClaudeBin,
            label: "agent.claude_bin",
            value: display_string(project_config.agent.claude_bin.as_ref()),
            kind: ConfigFieldKind::Text,
        },
        ConfigEntry {
            key: ConfigKey::AgentClaudePermissionMode,
            label: "agent.claude_permission_mode",
            value: display_claude_permission_mode(project_config.agent.claude_permission_mode),
            kind: ConfigFieldKind::Cycle,
        },
        ConfigEntry {
            key: ConfigKey::AgentRepopromptPlanRequired,
            label: "agent.repoprompt_plan_required",
            value: display_bool(project_config.agent.repoprompt_plan_required),
            kind: ConfigFieldKind::Toggle,
        },
        ConfigEntry {
            key: ConfigKey::AgentRepopromptToolInjection,
            label: "agent.repoprompt_tool_injection",
            value: display_bool(project_config.agent.repoprompt_tool_injection),
            kind: ConfigFieldKind::Toggle,
        },
        ConfigEntry {
            key: ConfigKey::AgentGitRevertMode,
            label: "agent.git_revert_mode",
            value: display_git_revert_mode(project_config.agent.git_revert_mode),
            kind: ConfigFieldKind::Cycle,
        },
        ConfigEntry {
            key: ConfigKey::AgentGitCommitPushEnabled,
            label: "agent.git_commit_push_enabled",
            value: display_bool(project_config.agent.git_commit_push_enabled),
            kind: ConfigFieldKind::Toggle,
        },
        ConfigEntry {
            key: ConfigKey::AgentPhases,
            label: "agent.phases",
            value: display_u8(project_config.agent.phases),
            kind: ConfigFieldKind::Cycle,
        },
    ]
}

pub fn config_value_for_edit(project_config: &ConfigLayer, key: ConfigKey) -> String {
    match key {
        ConfigKey::QueueFile => project_config
            .queue
            .file
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        ConfigKey::QueueDoneFile => project_config
            .queue
            .done_file
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        ConfigKey::QueueIdPrefix => project_config.queue.id_prefix.clone().unwrap_or_default(),
        ConfigKey::QueueIdWidth => project_config
            .queue
            .id_width
            .map(|v| v.to_string())
            .unwrap_or_default(),
        ConfigKey::AgentModel => project_config
            .agent
            .model
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default(),
        ConfigKey::AgentIterations => project_config
            .agent
            .iterations
            .map(|value| value.to_string())
            .unwrap_or_default(),
        ConfigKey::AgentCodexBin => project_config
            .agent
            .codex_bin
            .as_ref()
            .cloned()
            .unwrap_or_default(),
        ConfigKey::AgentOpencodeBin => project_config
            .agent
            .opencode_bin
            .as_ref()
            .cloned()
            .unwrap_or_default(),
        ConfigKey::AgentGeminiBin => project_config
            .agent
            .gemini_bin
            .as_ref()
            .cloned()
            .unwrap_or_default(),
        ConfigKey::AgentClaudeBin => project_config
            .agent
            .claude_bin
            .as_ref()
            .cloned()
            .unwrap_or_default(),
        _ => String::new(),
    }
}

pub fn apply_config_text_value(
    project_config: &mut ConfigLayer,
    dirty_config: &mut bool,
    key: ConfigKey,
    input: &str,
) -> Result<()> {
    let trimmed = input.trim();
    match key {
        ConfigKey::ProjectType => {
            bail!("Config key {:?} does not support cycling", key)
        }
        ConfigKey::AgentRunner => {
            bail!("Config key {:?} does not support cycling", key)
        }
        ConfigKey::QueueFile => {
            let path = if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            };
            project_config.queue.file = path;
            *dirty_config = true;
        }
        ConfigKey::QueueDoneFile => {
            let path = if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            };
            project_config.queue.done_file = path;
            *dirty_config = true;
        }
        ConfigKey::QueueIdPrefix => {
            let prefix = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
            project_config.queue.id_prefix = prefix;
            *dirty_config = true;
        }
        ConfigKey::QueueIdWidth => {
            let width = trimmed
                .parse::<u8>()
                .with_context(|| format!("invalid queue.id_width: {}", trimmed))?;
            project_config.queue.id_width = Some(width);
            *dirty_config = true;
        }
        ConfigKey::AgentModel => {
            bail!("Config key {:?} does not support text editing", key)
        }
        ConfigKey::AgentIterations => {
            let iterations = trimmed
                .parse::<u8>()
                .with_context(|| format!("invalid agent.iterations: {}", trimmed))?;
            project_config.agent.iterations = Some(iterations);
            *dirty_config = true;
        }
        ConfigKey::AgentCodexBin => {
            let bin = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
            project_config.agent.codex_bin = bin;
            *dirty_config = true;
        }
        ConfigKey::AgentOpencodeBin => {
            let bin = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
            project_config.agent.opencode_bin = bin;
            *dirty_config = true;
        }
        ConfigKey::AgentGeminiBin => {
            let bin = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
            project_config.agent.gemini_bin = bin;
            *dirty_config = true;
        }
        ConfigKey::AgentClaudeBin => {
            let bin = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
            project_config.agent.claude_bin = bin;
            *dirty_config = true;
        }
        _ => {
            bail!("Config key {:?} does not support text editing", key)
        }
    }
    Ok(())
}

pub fn cycle_config_value(
    project_config: &mut ConfigLayer,
    dirty_config: &mut bool,
    key: ConfigKey,
) {
    match key {
        ConfigKey::ProjectType => {
            project_config.project_type = cycle_project_type(project_config.project_type);
            *dirty_config = true;
        }
        ConfigKey::AgentRunner => {
            project_config.agent.runner = cycle_runner(project_config.agent.runner);
            *dirty_config = true;
        }
        ConfigKey::AgentReasoningEffort => {
            project_config.agent.reasoning_effort =
                cycle_reasoning_effort(project_config.agent.reasoning_effort);
            *dirty_config = true;
        }
        ConfigKey::AgentFollowupReasoningEffort => {
            project_config.agent.followup_reasoning_effort =
                cycle_reasoning_effort(project_config.agent.followup_reasoning_effort);
            *dirty_config = true;
        }
        ConfigKey::AgentClaudePermissionMode => {
            project_config.agent.claude_permission_mode =
                cycle_claude_permission_mode(project_config.agent.claude_permission_mode);
            *dirty_config = true;
        }
        ConfigKey::AgentRepopromptPlanRequired => {
            project_config.agent.repoprompt_plan_required =
                cycle_bool(project_config.agent.repoprompt_plan_required);
            *dirty_config = true;
        }
        ConfigKey::AgentRepopromptToolInjection => {
            project_config.agent.repoprompt_tool_injection =
                cycle_bool(project_config.agent.repoprompt_tool_injection);
            *dirty_config = true;
        }
        ConfigKey::AgentGitRevertMode => {
            project_config.agent.git_revert_mode =
                cycle_git_revert_mode(project_config.agent.git_revert_mode);
            *dirty_config = true;
        }
        ConfigKey::AgentGitCommitPushEnabled => {
            project_config.agent.git_commit_push_enabled =
                cycle_bool(project_config.agent.git_commit_push_enabled);
            *dirty_config = true;
        }
        ConfigKey::AgentPhases => {
            project_config.agent.phases = cycle_phases(project_config.agent.phases);
            *dirty_config = true;
        }
        _ => {}
    }
}

pub fn clear_config_value(
    project_config: &mut ConfigLayer,
    dirty_config: &mut bool,
    key: ConfigKey,
) {
    match key {
        ConfigKey::ProjectType => project_config.project_type = None,
        ConfigKey::AgentRunner => project_config.agent.runner = None,
        ConfigKey::QueueFile => project_config.queue.file = None,
        ConfigKey::QueueDoneFile => project_config.queue.done_file = None,
        ConfigKey::QueueIdPrefix => project_config.queue.id_prefix = None,
        ConfigKey::QueueIdWidth => project_config.queue.id_width = None,
        ConfigKey::AgentModel => project_config.agent.model = None,
        ConfigKey::AgentIterations => project_config.agent.iterations = None,
        ConfigKey::AgentCodexBin => project_config.agent.codex_bin = None,
        ConfigKey::AgentOpencodeBin => project_config.agent.opencode_bin = None,
        ConfigKey::AgentGeminiBin => project_config.agent.gemini_bin = None,
        ConfigKey::AgentClaudeBin => project_config.agent.claude_bin = None,
        ConfigKey::AgentReasoningEffort => project_config.agent.reasoning_effort = None,
        ConfigKey::AgentFollowupReasoningEffort => {
            project_config.agent.followup_reasoning_effort = None
        }
        ConfigKey::AgentClaudePermissionMode => project_config.agent.claude_permission_mode = None,
        ConfigKey::AgentRepopromptPlanRequired => {
            project_config.agent.repoprompt_plan_required = None
        }
        ConfigKey::AgentRepopromptToolInjection => {
            project_config.agent.repoprompt_tool_injection = None
        }
        ConfigKey::AgentGitRevertMode => project_config.agent.git_revert_mode = None,
        ConfigKey::AgentGitCommitPushEnabled => project_config.agent.git_commit_push_enabled = None,
        ConfigKey::AgentPhases => project_config.agent.phases = None,
    }
    *dirty_config = true;
}

fn cycle_project_type(
    value: Option<crate::contracts::ProjectType>,
) -> Option<crate::contracts::ProjectType> {
    match value {
        None => Some(crate::contracts::ProjectType::Code),
        Some(crate::contracts::ProjectType::Code) => Some(crate::contracts::ProjectType::Docs),
        Some(crate::contracts::ProjectType::Docs) => None,
    }
}

fn cycle_runner(value: Option<Runner>) -> Option<Runner> {
    match value {
        None => Some(Runner::Codex),
        Some(Runner::Codex) => Some(Runner::Opencode),
        Some(Runner::Opencode) => Some(Runner::Gemini),
        Some(Runner::Gemini) => Some(Runner::Claude),
        Some(Runner::Claude) => None,
    }
}

fn cycle_reasoning_effort(value: Option<ReasoningEffort>) -> Option<ReasoningEffort> {
    match value {
        None => Some(ReasoningEffort::Low),
        Some(ReasoningEffort::Low) => Some(ReasoningEffort::Medium),
        Some(ReasoningEffort::Medium) => Some(ReasoningEffort::High),
        Some(ReasoningEffort::High) => Some(ReasoningEffort::XHigh),
        Some(ReasoningEffort::XHigh) => None,
    }
}

fn cycle_claude_permission_mode(
    value: Option<ClaudePermissionMode>,
) -> Option<ClaudePermissionMode> {
    match value {
        None => Some(ClaudePermissionMode::AcceptEdits),
        Some(ClaudePermissionMode::AcceptEdits) => Some(ClaudePermissionMode::BypassPermissions),
        Some(ClaudePermissionMode::BypassPermissions) => None,
    }
}

fn cycle_git_revert_mode(value: Option<GitRevertMode>) -> Option<GitRevertMode> {
    match value {
        None => Some(GitRevertMode::Ask),
        Some(GitRevertMode::Ask) => Some(GitRevertMode::Enabled),
        Some(GitRevertMode::Enabled) => Some(GitRevertMode::Disabled),
        Some(GitRevertMode::Disabled) => None,
    }
}

fn cycle_bool(value: Option<bool>) -> Option<bool> {
    match value {
        None => Some(true),
        Some(true) => Some(false),
        Some(false) => None,
    }
}

fn cycle_phases(value: Option<u8>) -> Option<u8> {
    match value {
        None => Some(1),
        Some(1) => Some(2),
        Some(2) => Some(3),
        Some(3) => None,
        Some(_) => None,
    }
}
