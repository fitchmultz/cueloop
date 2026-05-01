//! Purpose: guard macOS contract scripts against teardown regressions.
//!
//! Responsibilities:
//! - Verify noninteractive RalphMac contract scripts clean up launched app processes.
//! - Verify those scripts remove disposable temp roots and fail loudly on lingering apps.
//!
//! Scope:
//! - Static contract assertions over repo shell scripts only.
//!
//! Usage:
//! - Run `cargo test --test macos_contract_script_cleanup_test`.
//!
//! Invariants/Assumptions:
//! - Script text is treated as the public contract for repo-owned cleanup behavior.

use anyhow::{Context, Result};
use std::path::PathBuf;

fn repo_root() -> Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .context("resolve repo root")
}

fn read_script(relative_path: &str) -> Result<String> {
    std::fs::read_to_string(repo_root()?.join(relative_path))
        .with_context(|| format!("read script {relative_path}"))
}

#[test]
fn test_macos_contract_scripts_enforce_app_cleanup() -> Result<()> {
    for relative_path in [
        "scripts/macos-settings-smoke.sh",
        "scripts/macos-workspace-routing-contract.sh",
    ] {
        let script = read_script(relative_path)?;
        assert!(
            script.contains("terminate_contract_app()"),
            "{relative_path} should define explicit app termination cleanup"
        );
        assert!(
            script.contains("trap cleanup EXIT INT TERM"),
            "{relative_path} should clean up on normal exit and interruption"
        );
        assert!(
            script.contains("pkill -TERM -f \"$APP_EXECUTABLE\""),
            "{relative_path} should terminate lingering RalphMac contract processes"
        );
        assert!(
            script.contains("lingering app process"),
            "{relative_path} should fail loudly if the app still runs after the contract completes"
        );
        assert!(
            script.contains("rm -rf \"$TEMP_ROOT\""),
            "{relative_path} should remove disposable temp roots during cleanup"
        );
    }

    Ok(())
}
