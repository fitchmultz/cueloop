//! Integration tests for `ralph plugin install` command.
//!
//! Purpose:
//! - Integration tests for `ralph plugin install` command.
//!
//! Responsibilities:
//! - Cover install-path compatibility behavior that crosses current and legacy plugin roots.
//!
//! Scope:
//! - Limited to plugin install command behavior that requires a real CLI process.
//!
//! Usage:
//! - Run through Cargo's integration test harness.
//!
//! Invariants/Assumptions:
//! - New project-scope plugins install under `.cueloop/plugins`.
//! - Legacy `.ralph/plugins` entries remain compatibility fallbacks and must not be shadowed silently.

use anyhow::Result;
use std::path::Path;
use std::process::Command;

mod test_support;

fn git_init(dir: &Path) -> Result<()> {
    let status = Command::new("git")
        .current_dir(dir)
        .args(["init", "--quiet"])
        .status()?;
    anyhow::ensure!(status.success(), "git init failed");
    std::fs::write(
        dir.join(".gitignore"),
        ".cueloop/lock\n.cueloop/cache/\n.cueloop/logs/\n.ralph/lock\n.ralph/cache/\n.ralph/logs/\n",
    )?;
    Ok(())
}

fn cueloop_init(dir: &Path) -> Result<()> {
    let output = Command::new(test_support::cueloop_bin())
        .current_dir(dir)
        .env_remove("RUST_LOG")
        .args(["init", "--force", "--non-interactive"])
        .output()?;

    anyhow::ensure!(
        output.status.success(),
        "cueloop init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn write_plugin_manifest(dir: &Path, id: &str) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        dir.join("plugin.json"),
        format!(
            r#"{{
  "api_version": 1,
  "id": "{}",
  "version": "0.1.0",
  "name": "{}",
  "runner": {{ "bin": "runner.sh", "supports_resume": false }}
}}"#,
            id, id
        ),
    )?;
    std::fs::write(dir.join("runner.sh"), "#!/usr/bin/env sh\nexit 0\n")?;
    Ok(())
}

#[test]
fn plugin_install_legacy_plugin_collision_fails() -> Result<()> {
    let temp_dir = test_support::temp_dir_outside_repo();
    git_init(temp_dir.path())?;
    cueloop_init(temp_dir.path())?;

    let source_dir = temp_dir.path().join("source-plugin");
    let legacy_dir = temp_dir.path().join(".ralph/plugins/legacy.install");
    write_plugin_manifest(&source_dir, "legacy.install")?;
    write_plugin_manifest(&legacy_dir, "legacy.install")?;

    let output = Command::new(test_support::cueloop_bin())
        .current_dir(temp_dir.path())
        .env_remove("RUST_LOG")
        .args([
            "plugin",
            "install",
            source_dir.to_str().unwrap(),
            "--scope",
            "project",
        ])
        .output()?;

    assert!(!output.status.success(), "legacy collision should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("already installed") && stderr.contains(".ralph/plugins/legacy.install"),
        "expected legacy collision error, got: {}",
        stderr
    );
    assert!(
        !temp_dir
            .path()
            .join(".cueloop/plugins/legacy.install")
            .exists(),
        "current-path plugin should not be installed over a legacy fallback"
    );

    Ok(())
}
