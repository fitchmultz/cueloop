//! Runner list command.
//!
//! Purpose:
//! - Runner list command.
//!
//! Responsibilities:
//! - List all available runners with brief descriptions.
//!
//! Scope:
//! - Limited to this file's owning feature boundary.
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/Assumptions:
//! - Keep behavior aligned with Ralph's canonical CLI, machine-contract, and queue semantics.

use anyhow::Result;
use serde::Serialize;

use crate::cli::runner::RunnerFormat;
use crate::commands::runner::capabilities::built_in_runner_catalog;

#[derive(Debug, Clone, Serialize)]
pub struct RunnerInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub default_model: String,
}

fn get_all_runners() -> Vec<RunnerInfo> {
    built_in_runner_catalog()
        .into_iter()
        .map(|entry| RunnerInfo {
            provider: provider_for_runner(&entry.id).to_string(),
            default_model: entry.default_model.unwrap_or_else(|| "<unset>".to_string()),
            id: entry.id,
            name: entry.display_name,
        })
        .collect()
}

fn provider_for_runner(id: &str) -> &'static str {
    match id {
        "claude" => "Anthropic",
        "codex" => "OpenAI",
        "opencode" => "Flexible",
        "gemini" => "Google",
        "cursor" => "Cursor",
        "kimi" => "Moonshot AI",
        "pi" => "Pi",
        _ => "Unknown",
    }
}

pub fn handle_list(format: RunnerFormat) -> Result<()> {
    let runners = get_all_runners();

    match format {
        RunnerFormat::Text => {
            println!("Available runners:\n");
            for r in &runners {
                println!("  {:12} {} (default: {})", r.id, r.name, r.default_model);
            }
            println!("\nUse 'cueloop runner capabilities <id>' for details.");
        }
        RunnerFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&runners)?);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_all_runners_returns_all_built_ins() {
        let runners = get_all_runners();
        assert_eq!(runners.len(), 7);

        let ids: Vec<_> = runners.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"claude"));
        assert!(ids.contains(&"codex"));
        assert!(ids.contains(&"opencode"));
        assert!(ids.contains(&"gemini"));
        assert!(ids.contains(&"cursor"));
        assert!(ids.contains(&"kimi"));
        assert!(ids.contains(&"pi"));
    }

    #[test]
    fn runner_info_has_required_fields() {
        let runners = get_all_runners();
        for r in &runners {
            assert!(!r.id.is_empty(), "runner {} has empty id", r.name);
            assert!(!r.name.is_empty(), "runner {} has empty name", r.id);
            assert!(!r.provider.is_empty(), "runner {} has empty provider", r.id);
            assert!(
                !r.default_model.is_empty(),
                "runner {} has empty default_model",
                r.id
            );
        }
    }
}
