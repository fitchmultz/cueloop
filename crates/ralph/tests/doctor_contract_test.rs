//! Contract tests for `ralph doctor` output and diagnostics.
//!
//! Responsibilities:
//! - Provide shared command helpers for doctor contract suites.
//! - Split baseline, runner-binary, JSON, and auto-fix behavior into focused modules.

use anyhow::Result;
use std::process::Command;

mod test_support;

fn ralph_cmd() -> Command {
    let mut cmd = Command::new(test_support::ralph_bin());
    cmd.env_remove("RUST_LOG");
    cmd
}

/// Create a ralph command scoped to the given directory.
fn ralph_cmd_in_dir(dir: &std::path::Path) -> Command {
    let mut cmd = ralph_cmd();
    cmd.current_dir(dir);
    cmd
}

fn trust_repo(dir: &std::path::Path) -> Result<()> {
    let ralph_dir = dir.join(".ralph");
    std::fs::create_dir_all(&ralph_dir)?;
    std::fs::write(
        ralph_dir.join("trust.jsonc"),
        r#"{"allow_project_commands": true}"#,
    )?;
    Ok(())
}

#[path = "doctor_contract_test/auto_fix.rs"]
mod auto_fix;
#[path = "doctor_contract_test/baseline.rs"]
mod baseline;
#[path = "doctor_contract_test/json_output.rs"]
mod json_output;
#[path = "doctor_contract_test/repo_hygiene.rs"]
mod repo_hygiene;
#[path = "doctor_contract_test/runner_binaries.rs"]
mod runner_binaries;
