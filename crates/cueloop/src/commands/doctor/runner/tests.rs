//! Unit tests for runner doctor helpers and Cursor SDK probing.
//!
//! Purpose:
//! - Keep runner doctor unit tests outside the runtime modules.
//!
//! Responsibilities:
//! - Verify Cursor SDK package discovery, API-key value handling, and Node-version parsing.
//! - Isolate process-global Cursor SDK environment mutations with a mutex.
//!
//! Not handled here:
//! - End-to-end doctor CLI output contracts; those live in `tests/doctor_contract_test`.
//!
//! Invariants/assumptions:
//! - Tests skip Cursor SDK package probes when Node is unavailable on the local machine.

use super::binary::{BinarySelectionCheck, check_runner_binary_selection, select_runner_binary};
use super::cursor::cursor_api_key_value_configured;
use crate::commands::doctor::cursor_sdk_probe::{
    check_cursor_sdk_package, cursor_sdk_blocking_reason, ensure_cursor_sdk_node_version_supported,
};
use crate::commands::doctor::types::DoctorReport;
use crate::config;
use crate::constants::versions::CURSOR_SDK_VERSION;
use crate::contracts::{BlockingReason, Config, Runner};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

static CURSOR_SDK_ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }

    fn unset(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.previous.as_ref() {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn node_bin() -> Option<PathBuf> {
    let output = Command::new("node")
        .args(["-p", "process.execPath"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?;
    Some(PathBuf::from(path.trim()))
}

fn write_workspace_sdk(temp: &tempfile::TempDir, version: &str) -> anyhow::Result<PathBuf> {
    std::fs::write(temp.path().join("package.json"), r#"{"type":"module"}"#)?;
    let sdk_dir = temp.path().join("node_modules/@cursor/sdk");
    std::fs::create_dir_all(&sdk_dir)?;
    std::fs::write(
        sdk_dir.join("package.json"),
        format!(r#"{{"name":"@cursor/sdk","version":"{version}","main":"index.js"}}"#),
    )?;
    std::fs::write(
        sdk_dir.join("index.js"),
        "import fs from 'node:fs'; fs.writeFileSync('sdk-imported', 'yes'); export class Agent {}",
    )?;
    Ok(sdk_dir.join("index.js"))
}

fn resolved_with_project_config(
    root: &Path,
    project_config: &Path,
    bin_path: &Path,
) -> config::Resolved {
    let mut config = Config::default();
    config.agent.runner = Some(Runner::Codex);
    config.agent.codex_bin = Some(bin_path.to_string_lossy().to_string());
    config::Resolved {
        config,
        repo_root: root.to_path_buf(),
        queue_path: root.join(".cueloop/queue.jsonc"),
        done_path: root.join(".cueloop/done.jsonc"),
        id_prefix: "CL".to_string(),
        id_width: 4,
        global_config_path: None,
        project_config_path: Some(project_config.to_path_buf()),
    }
}

fn executable_script(path: &Path, body: &str) -> anyhow::Result<()> {
    let mut file = std::fs::File::create(path)?;
    write!(file, "{body}")?;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

fn write_importable_sdk(root: &std::path::Path, version: &str) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(root)?;
    std::fs::write(root.join("package.json"), r#"{}"#)?;
    let sdk_dir = root.join("@cursor/sdk");
    std::fs::create_dir_all(&sdk_dir)?;
    std::fs::write(
        sdk_dir.join("package.json"),
        format!(
            r#"{{"name":"@cursor/sdk","version":"{version}","type":"module","main":"index.js"}}"#
        ),
    )?;
    std::fs::write(sdk_dir.join("index.js"), "export class Agent {}")?;
    Ok(sdk_dir.join("index.js"))
}

#[test]
fn runner_binary_check_blocks_untrusted_project_override_before_probe() -> anyhow::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let project_config = temp.path().join(".cueloop/config.jsonc");
    std::fs::create_dir_all(project_config.parent().expect("config parent"))?;
    let bin = temp.path().join("codex-runner");
    executable_script(&bin, "#!/bin/sh\necho should-not-run > probe-ran\nexit 0\n")?;
    std::fs::write(
        &project_config,
        format!(
            r#"{{"version":2,"agent":{{"runner":"codex","codex_bin":"{}"}}}}"#,
            bin.display()
        ),
    )?;
    let resolved = resolved_with_project_config(temp.path(), &project_config, &bin);
    let selection = select_runner_binary(&resolved).expect("runner selection");
    let mut report = DoctorReport::new();

    let check = check_runner_binary_selection(&mut report, &resolved, &selection);

    assert!(matches!(
        check,
        BinarySelectionCheck::BlockedProjectOverride
    ));
    assert!(!temp.path().join("probe-ran").exists());
    assert_eq!(report.summary.errors, 1);
    assert!(matches!(
        report.checks[0].blocking.as_ref().map(|blocking| &blocking.reason),
        Some(BlockingReason::RunnerRecovery { reason, .. }) if reason == "project_runner_override_untrusted"
    ));
    Ok(())
}

#[test]
fn runner_binary_check_fail_closes_when_trust_file_is_invalid() -> anyhow::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let project_config = temp.path().join(".cueloop/config.jsonc");
    std::fs::create_dir_all(project_config.parent().expect("config parent"))?;
    let bin = temp.path().join("codex-runner");
    executable_script(&bin, "#!/bin/sh\necho should-not-run > probe-ran\nexit 0\n")?;
    std::fs::write(
        &project_config,
        format!(
            r#"{{"version":2,"agent":{{"runner":"codex","codex_bin":"{}"}}}}"#,
            bin.display()
        ),
    )?;
    std::fs::write(temp.path().join(".cueloop/trust.jsonc"), "{ invalid json")?;
    let resolved = resolved_with_project_config(temp.path(), &project_config, &bin);
    let selection = select_runner_binary(&resolved).expect("runner selection");
    let mut report = DoctorReport::new();

    let check = check_runner_binary_selection(&mut report, &resolved, &selection);

    assert!(matches!(
        check,
        BinarySelectionCheck::BlockedProjectOverride
    ));
    assert!(!temp.path().join("probe-ran").exists());
    assert!(
        report.checks[0]
            .suggested_fix
            .as_deref()
            .unwrap_or_default()
            .contains("Unable to prove repo trust/config safety")
    );
    Ok(())
}

#[test]
fn runner_binary_check_fail_closes_when_project_config_cannot_load() -> anyhow::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let project_config = temp.path().join(".cueloop/config.jsonc");
    std::fs::create_dir_all(project_config.parent().expect("config parent"))?;
    let bin = temp.path().join("codex-runner");
    executable_script(&bin, "#!/bin/sh\necho should-not-run > probe-ran\nexit 0\n")?;
    std::fs::write(&project_config, "{ invalid json")?;
    let resolved = resolved_with_project_config(temp.path(), &project_config, &bin);
    let selection = select_runner_binary(&resolved).expect("runner selection");
    let mut report = DoctorReport::new();

    let check = check_runner_binary_selection(&mut report, &resolved, &selection);

    assert!(matches!(
        check,
        BinarySelectionCheck::BlockedProjectOverride
    ));
    assert!(!temp.path().join("probe-ran").exists());
    Ok(())
}

#[test]
fn runner_binary_check_allows_trusted_project_override_success_path() -> anyhow::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let project_config = temp.path().join(".cueloop/config.jsonc");
    std::fs::create_dir_all(project_config.parent().expect("config parent"))?;
    let bin = temp.path().join("codex-runner");
    executable_script(&bin, "#!/bin/sh\necho codex version 1.0.0\nexit 0\n")?;
    std::fs::write(
        &project_config,
        format!(
            r#"{{"version":2,"agent":{{"runner":"codex","codex_bin":"{}"}}}}"#,
            bin.display()
        ),
    )?;
    std::fs::write(
        temp.path().join(".cueloop/trust.jsonc"),
        r#"{"allow_project_commands":true,"trusted_at":"2026-06-12T00:00:00Z"}"#,
    )?;
    let resolved = resolved_with_project_config(temp.path(), &project_config, &bin);
    let selection = select_runner_binary(&resolved).expect("runner selection");
    let mut report = DoctorReport::new();

    let check = check_runner_binary_selection(&mut report, &resolved, &selection);

    assert!(matches!(check, BinarySelectionCheck::Available));
    assert!(report.checks.is_empty());
    Ok(())
}

#[test]
fn cursor_sdk_workspace_probe_resolves_without_importing_package() -> anyhow::Result<()> {
    let Some(node) = node_bin() else {
        return Ok(());
    };
    let _lock = CURSOR_SDK_ENV_LOCK
        .lock()
        .expect("cursor SDK env lock poisoned");
    let _module_path = EnvGuard::unset("CUELOOP_CURSOR_SDK_MODULE_PATH");
    let _global_root = EnvGuard::unset("CUELOOP_CURSOR_SDK_GLOBAL_ROOT");

    let temp = tempfile::TempDir::new()?;
    write_workspace_sdk(&temp, CURSOR_SDK_VERSION)?;

    let check = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())?;
    let selected = check.selected.as_ref().expect("selected SDK");
    assert_eq!(selected.source, "workspace");
    assert_eq!(selected.sdk_version.as_deref(), Some(CURSOR_SDK_VERSION));
    assert!(!check.version_mismatch());

    assert!(
        !temp.path().join("sdk-imported").exists(),
        "doctor workspace SDK probe must not import repo-local package code"
    );
    Ok(())
}

#[test]
fn cursor_sdk_workspace_probe_warns_wrong_version_without_importing() -> anyhow::Result<()> {
    let Some(node) = node_bin() else {
        return Ok(());
    };
    let _lock = CURSOR_SDK_ENV_LOCK
        .lock()
        .expect("cursor SDK env lock poisoned");
    let _module_path = EnvGuard::unset("CUELOOP_CURSOR_SDK_MODULE_PATH");
    let _global_root = EnvGuard::unset("CUELOOP_CURSOR_SDK_GLOBAL_ROOT");

    let temp = tempfile::TempDir::new()?;
    write_workspace_sdk(&temp, "1.0.10")?;

    let check = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())?;

    let selected = check.selected.as_ref().expect("selected SDK");
    assert_eq!(selected.source, "workspace");
    assert_eq!(selected.sdk_version.as_deref(), Some("1.0.10"));
    assert!(check.version_mismatch());
    assert!(check.proceeded_best_effort);
    assert!(
        check
            .warnings
            .iter()
            .any(|warning| warning.contains("preferred/tested"))
    );
    assert!(check.attempted_sources_summary().contains("workspace"));
    assert!(
        !temp.path().join("sdk-imported").exists(),
        "doctor workspace SDK version probe must not import repo-local package code"
    );
    Ok(())
}

#[test]
fn cursor_sdk_probe_honors_env_override_before_workspace() -> anyhow::Result<()> {
    let Some(node) = node_bin() else {
        return Ok(());
    };
    let _lock = CURSOR_SDK_ENV_LOCK
        .lock()
        .expect("cursor SDK env lock poisoned");
    let _global_root = EnvGuard::unset("CUELOOP_CURSOR_SDK_GLOBAL_ROOT");

    let temp = tempfile::TempDir::new()?;
    write_workspace_sdk(&temp, CURSOR_SDK_VERSION)?;
    let env_root = temp.path().join("env_node_modules");
    let env_entrypoint = write_importable_sdk(&env_root, "1.0.14")?;
    let _module_path = EnvGuard::set("CUELOOP_CURSOR_SDK_MODULE_PATH", &env_entrypoint);

    let check = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())?;

    let selected = check.selected.as_ref().expect("selected SDK");
    assert_eq!(selected.source, "env");
    assert_eq!(
        selected.entrypoint,
        env_entrypoint.to_string_lossy().as_ref()
    );
    assert_eq!(selected.sdk_version.as_deref(), Some("1.0.14"));
    assert!(check.version_mismatch());
    assert!(check.proceeded_best_effort);
    assert!(check.attempted_sources_summary().contains("env"));
    Ok(())
}

#[test]
fn cursor_sdk_probe_falls_back_to_global_root() -> anyhow::Result<()> {
    let Some(node) = node_bin() else {
        return Ok(());
    };
    let _lock = CURSOR_SDK_ENV_LOCK
        .lock()
        .expect("cursor SDK env lock poisoned");
    let _module_path = EnvGuard::unset("CUELOOP_CURSOR_SDK_MODULE_PATH");

    let temp = tempfile::TempDir::new()?;
    std::fs::write(temp.path().join("package.json"), r#"{"type":"module"}"#)?;
    let global_root = temp.path().join("global_node_modules");
    let global_entrypoint = write_importable_sdk(&global_root, "1.0.14")?;
    let _global_root = EnvGuard::set("CUELOOP_CURSOR_SDK_GLOBAL_ROOT", &global_root);

    let check = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())?;

    let selected = check.selected.as_ref().expect("selected SDK");
    assert_eq!(selected.source, "global");
    assert_eq!(
        selected.entrypoint,
        global_entrypoint.canonicalize()?.to_string_lossy().as_ref()
    );
    assert_eq!(selected.sdk_version.as_deref(), Some("1.0.14"));
    assert_eq!(
        selected.global_root.as_deref(),
        Some(global_root.to_string_lossy().as_ref())
    );
    assert!(check.version_mismatch());
    assert!(check.proceeded_best_effort);
    assert!(check.attempted_sources_summary().contains("global"));
    Ok(())
}

#[test]
fn cursor_sdk_probe_errors_when_sdk_missing() -> anyhow::Result<()> {
    let Some(node) = node_bin() else {
        return Ok(());
    };
    let _lock = CURSOR_SDK_ENV_LOCK
        .lock()
        .expect("cursor SDK env lock poisoned");
    let _module_path = EnvGuard::unset("CUELOOP_CURSOR_SDK_MODULE_PATH");
    let empty_global_root = tempfile::TempDir::new()?;
    let _global_root = EnvGuard::set("CUELOOP_CURSOR_SDK_GLOBAL_ROOT", empty_global_root.path());

    let temp = tempfile::TempDir::new()?;
    std::fs::write(temp.path().join("package.json"), r#"{"type":"module"}"#)?;

    let err = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())
        .expect_err("missing Cursor SDK should fail doctor");
    assert!(
        err.to_string().contains("missing_sdk") && err.to_string().contains("attempted_sources"),
        "unexpected error: {err}"
    );
    Ok(())
}

#[test]
fn cursor_sdk_probe_errors_for_structurally_invalid_env_override() -> anyhow::Result<()> {
    let Some(node) = node_bin() else {
        return Ok(());
    };
    let _lock = CURSOR_SDK_ENV_LOCK
        .lock()
        .expect("cursor SDK env lock poisoned");
    let _global_root = EnvGuard::unset("CUELOOP_CURSOR_SDK_GLOBAL_ROOT");

    let temp = tempfile::TempDir::new()?;
    std::fs::write(temp.path().join("package.json"), r#"{"type":"module"}"#)?;
    let sdk_dir = temp.path().join("env_node_modules/@cursor/sdk");
    std::fs::create_dir_all(&sdk_dir)?;
    std::fs::write(
        sdk_dir.join("package.json"),
        format!(
            r#"{{"name":"@cursor/sdk","version":"{CURSOR_SDK_VERSION}","type":"module","main":"index.js"}}"#
        ),
    )?;
    std::fs::write(sdk_dir.join("index.js"), "export const NotAgent = true;")?;
    let _module_path = EnvGuard::set("CUELOOP_CURSOR_SDK_MODULE_PATH", sdk_dir.join("index.js"));

    let err = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())
        .expect_err("invalid Cursor SDK override should fail doctor");
    assert!(
        err.to_string()
            .contains("does not expose required export Agent")
            && err.to_string().contains("invalid_module_path")
            && err.to_string().contains("attempted_sources"),
        "unexpected error: {err}"
    );
    Ok(())
}

#[test]
fn cursor_api_key_check_rejects_missing_or_empty_values() {
    assert!(!cursor_api_key_value_configured(None));
    assert!(!cursor_api_key_value_configured(Some(
        std::ffi::OsString::new()
    )));
    assert!(cursor_api_key_value_configured(Some(
        std::ffi::OsString::from("cursor-key")
    )));
}

#[test]
fn cursor_sdk_blocking_reason_uses_fatal_cause_before_tried_location_text() {
    for (message, reason) in [
        (
            r#"{"fatal_cause":"missing_sdk","message":"tried CUELOOP_CURSOR_SDK_MODULE_PATH, workspace, and global npm roots"}"#,
            "cursor_sdk_missing",
        ),
        (
            r#"{"fatal_cause":"invalid_module_path"}"#,
            "cursor_sdk_invalid_module_path",
        ),
        (
            r#"{"fatal_cause":"incompatible_api"}"#,
            "cursor_sdk_incompatible_api",
        ),
        (
            r#"{"fatal_cause":"import_failed"}"#,
            "cursor_sdk_import_failed",
        ),
    ] {
        assert_eq!(cursor_sdk_blocking_reason(message), reason);
    }
}

#[test]
fn cursor_sdk_node_version_requires_node_18_or_newer() {
    ensure_cursor_sdk_node_version_supported("18.0.0").expect("node 18 should pass");
    ensure_cursor_sdk_node_version_supported("v20.11.1").expect("node 20 should pass");

    let err = ensure_cursor_sdk_node_version_supported("17.9.1").expect_err("node 17 should fail");
    assert!(err.to_string().contains("requires Node 18 or newer"));

    let err = ensure_cursor_sdk_node_version_supported("not-a-version")
        .expect_err("invalid versions should fail");
    assert!(err.to_string().contains("could not parse Node version"));
}
