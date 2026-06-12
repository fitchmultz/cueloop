//! Instruction-file doctor reporting for runner configuration.
//!
//! Purpose:
//! - Report configured instruction-file validity and repo-root `AGENTS.md` injection status.
//!
//! Responsibilities:
//! - Surface warnings produced by prompt instruction-file validation.
//! - Preserve doctor status messages for configured and unconfigured `AGENTS.md` files.
//!
//! Not handled here:
//! - Prompt rendering or instruction-file injection during runner execution.
//! - Runner binary, Cursor SDK, or model checks.
//!
//! Invariants/assumptions:
//! - Missing instruction files are warnings, not blocking errors.
//! - Repo-root `AGENTS.md` is only successful when explicitly configured and readable.

use crate::commands::doctor::types::{CheckResult, DoctorReport};
use crate::config;
use crate::prompts;

pub(super) fn check_instruction_files(report: &mut DoctorReport, resolved: &config::Resolved) {
    let instruction_warnings =
        prompts::instruction_file_warnings(&resolved.repo_root, &resolved.config);

    let repo_agents_configured = resolved
        .config
        .agent
        .instruction_files
        .as_ref()
        .map(|files| {
            files.iter().any(|p| {
                let resolved = resolved.repo_root.join(p);
                resolved.ends_with("AGENTS.md")
            })
        })
        .unwrap_or(false);
    let repo_agents_path = resolved.repo_root.join("AGENTS.md");
    let repo_agents_exists = repo_agents_path.exists();

    if instruction_warnings.is_empty() {
        if let Some(files) = resolved.config.agent.instruction_files.as_ref()
            && !files.is_empty()
        {
            report.add(CheckResult::success(
                "runner",
                "instruction_files",
                &format!(
                    "instruction_files valid ({} configured file(s))",
                    files.len()
                ),
            ));
        }
        if repo_agents_configured && repo_agents_exists {
            report.add(CheckResult::success(
                "runner",
                "agents_md",
                "AGENTS.md configured and readable",
            ));
        } else if repo_agents_exists && !repo_agents_configured {
            report.add(CheckResult::warning(
                "runner",
                "agents_md",
                "AGENTS.md exists at repo root but is not configured for injection. \
                 To enable, add 'AGENTS.md' to agent.instruction_files in your config.",
                false,
                Some("Add 'AGENTS.md' to agent.instruction_files in .cueloop/config.jsonc"),
            ));
        }
    } else {
        for warning in instruction_warnings {
            report.add(CheckResult::warning(
                "runner",
                "instruction_files",
                &warning,
                false,
                Some("Check instruction file paths in config"),
            ));
        }
    }
}
