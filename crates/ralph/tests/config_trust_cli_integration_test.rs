//! Integration tests for repo execution trust CLI (`ralph init`, `ralph config trust init`).
//!
//! Purpose:
//! - Integration tests for repo execution trust CLI (`ralph init`, `ralph config trust init`).
//!
//! Responsibilities:
//! - Prove `ralph init` is the canonical trust bootstrap.
//! - Keep trust-only repair behavior covered for already-initialized repos.
//!
//! Scope:
//! - Limited to CLI trust/bootstrap behavior and execution-sensitive project config resolution.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/Assumptions:
//! - Keep behavior aligned with Ralph's canonical CLI, machine-contract, and queue semantics.

use anyhow::Result;
mod test_support;

const SENSITIVE_CONFIG: &str = r#"{
  "version": 2,
  "agent": {
    "runner": "codex",
    "model": "gpt-5.3-codex",
    "codex_bin": "codex"
  }
}"#;

const PROJECT_CURSOR_CONFIG: &str = r#"{
  "version": 2,
  "agent": {
    "runner": "cursor",
    "model": "composer-2"
  }
}"#;

#[test]
fn config_show_rejects_untrusted_project_cursor_selection() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    std::fs::create_dir_all(dir.path().join(".ralph"))?;
    std::fs::write(
        dir.path().join(".ralph/config.jsonc"),
        PROJECT_CURSOR_CONFIG,
    )?;

    let (status, _stdout, stderr) = test_support::run_in_dir(dir.path(), &["config", "show"]);
    assert!(
        !status.success(),
        "expected config show to fail without trust\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("not trusted") && stderr.contains("Cursor"),
        "expected Cursor trust error in stderr, got:\n{stderr}"
    );
    Ok(())
}

#[test]
fn config_trust_init_repairs_missing_trust_for_sensitive_project_config() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    std::fs::create_dir_all(dir.path().join(".ralph"))?;
    std::fs::write(
        dir.path().join(".ralph/queue.jsonc"),
        r#"{"version":1,"tasks":[]}"#,
    )?;
    std::fs::write(
        dir.path().join(".ralph/done.jsonc"),
        r#"{"version":1,"tasks":[]}"#,
    )?;
    std::fs::write(dir.path().join(".ralph/config.jsonc"), SENSITIVE_CONFIG)?;

    let (status, _stdout, stderr) = test_support::run_in_dir(dir.path(), &["config", "show"]);
    assert!(
        !status.success(),
        "expected config show to fail without trust\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("not trusted") && stderr.contains("ralph init"),
        "expected modern trust error in stderr, got:\n{stderr}"
    );

    let (status, _stdout, stderr) =
        test_support::run_in_dir(dir.path(), &["config", "trust", "init"]);
    assert!(
        status.success(),
        "ralph config trust init failed\nstderr:\n{stderr}"
    );

    let (status, _stdout, stderr) = test_support::run_in_dir(dir.path(), &["config", "show"]);
    assert!(
        status.success(),
        "config show should succeed after trust init\nstderr:\n{stderr}"
    );

    let trust_path = dir.path().join(".ralph/trust.jsonc");
    let first = std::fs::read_to_string(&trust_path)?;
    let (status, _stdout, stderr) =
        test_support::run_in_dir(dir.path(), &["config", "trust", "init"]);
    assert!(
        status.success(),
        "second trust init failed\nstderr:\n{stderr}"
    );
    let second = std::fs::read_to_string(&trust_path)?;
    assert_eq!(
        first, second,
        "idempotent trust init must not rewrite trust file bytes"
    );

    Ok(())
}

#[test]
fn init_succeeds_with_existing_sensitive_config_before_trust_exists() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    std::fs::create_dir_all(dir.path().join(".ralph"))?;
    std::fs::write(dir.path().join(".ralph/config.jsonc"), SENSITIVE_CONFIG)?;

    let (status, _stdout, stderr) =
        test_support::run_in_dir(dir.path(), &["init", "--non-interactive"]);
    assert!(
        status.success(),
        "ralph init should bootstrap trust before enforcing sensitive config\nstderr:\n{stderr}"
    );
    assert!(dir.path().join(".ralph/trust.jsonc").exists());

    let (status, _stdout, stderr) = test_support::run_in_dir(dir.path(), &["config", "show"]);
    assert!(
        status.success(),
        "config show should succeed after init-created trust\nstderr:\n{stderr}"
    );
    Ok(())
}

#[test]
fn init_ignores_global_queue_file_when_creating_project_runtime() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    let xdg_config = dir.path().join("xdg");
    let global_dir = xdg_config.join("cueloop");
    std::fs::create_dir_all(&global_dir)?;
    std::fs::write(
        global_dir.join("config.jsonc"),
        r#"{"version":2,"queue":{"file":"external/queue.jsonc","done_file":"external/done.jsonc"}}"#,
    )?;

    let output = test_support::ralph_command(dir.path())
        .env("XDG_CONFIG_HOME", &xdg_config)
        .args(["init", "--force", "--non-interactive"])
        .output()
        .expect("run ralph init");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "ralph init failed\nstderr:\n{stderr}"
    );

    assert!(dir.path().join(".cueloop/queue.jsonc").exists());
    assert!(dir.path().join(".cueloop/done.jsonc").exists());
    assert!(!dir.path().join("external/queue.jsonc").exists());
    assert!(!dir.path().join("external/done.jsonc").exists());
    Ok(())
}

#[test]
fn init_creates_trust_and_allows_later_sensitive_config() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    let (status, _stdout, stderr) =
        test_support::run_in_dir(dir.path(), &["init", "--force", "--non-interactive"]);
    assert!(status.success(), "ralph init failed\nstderr:\n{stderr}");

    let trust_path = dir.path().join(".cueloop/trust.jsonc");
    assert!(trust_path.exists(), "ralph init should create repo trust");
    let gitignore = std::fs::read_to_string(dir.path().join(".gitignore"))?;
    assert!(
        gitignore.contains(".cueloop/trust.jsonc"),
        "init should gitignore repo trust"
    );

    std::fs::write(dir.path().join(".cueloop/config.jsonc"), SENSITIVE_CONFIG)?;

    let (status, _stdout, stderr) = test_support::run_in_dir(dir.path(), &["config", "show"]);
    assert!(
        status.success(),
        "config show should succeed when trust was created at init\nstderr:\n{stderr}"
    );

    Ok(())
}
