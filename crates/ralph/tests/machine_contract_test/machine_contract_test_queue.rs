//! Queue and workspace contract coverage for `ralph machine`.
//!
//! Purpose:
//! - Verify machine queue and workspace JSON documents exposed to app clients.
//!
//! Responsibilities:
//! - Assert queue read success and failure document shapes.
//! - Assert workspace overview bundles queue and config payloads together.
//! - Keep queue/workspace contract regressions isolated from task and recovery flows.
//!
//! Non-scope:
//! - Task mutation behavior.
//! - Parallel runtime or system contract coverage.
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions callers must respect:
//! - Tests execute against disposable repos initialized through the public CLI.
//! - Contract assertions preserve the historical flat suite behavior exactly.

use super::machine_contract_test_support::{run_in_dir, setup_ralph_repo};
use anyhow::Result;
use serde_json::Value;

#[test]
fn machine_queue_read_returns_versioned_snapshot() -> Result<()> {
    let dir = setup_ralph_repo()?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["machine", "queue", "read"]);
    assert!(
        status.success(),
        "machine queue read failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let document: Value = serde_json::from_str(&stdout)?;
    assert_eq!(document["version"], 1);
    assert!(document["paths"]["queue_path"].is_string());
    assert!(document["active"]["tasks"].is_array());
    assert!(document["done"]["tasks"].is_array());
    Ok(())
}

#[test]
fn machine_queue_read_failure_returns_structured_error_document() -> Result<()> {
    let dir = tempfile::tempdir()?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["machine", "queue", "read"]);
    assert!(
        !status.success(),
        "machine queue read should fail outside a Ralph repo\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.trim().is_empty(),
        "failure stdout should stay empty: {stdout}"
    );

    let document: Value = serde_json::from_str(&stderr)?;
    assert_eq!(document["version"], 1);
    assert_eq!(document["code"], "queue_corrupted");
    assert_eq!(document["message"], "No Ralph queue file found.");
    assert_eq!(document["retryable"], false);
    assert!(
        document["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("queue.jsonc")
    );
    Ok(())
}

#[test]
fn machine_workspace_overview_returns_queue_and_config_in_one_document() -> Result<()> {
    let dir = setup_ralph_repo()?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["machine", "workspace", "overview"]);
    assert!(
        status.success(),
        "machine workspace overview failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let document: Value = serde_json::from_str(&stdout)?;
    assert_eq!(document["version"], 1);
    assert_eq!(document["queue"]["version"], 1);
    assert_eq!(document["config"]["version"], 4);
    assert!(document["queue"]["paths"]["queue_path"].is_string());
    assert!(document["queue"]["active"]["tasks"].is_array());
    assert!(document["config"]["paths"]["project_config_path"].is_string());
    assert!(document["config"]["config"].is_object());
    assert!(document["config"]["execution_controls"]["runners"].is_array());
    assert_eq!(
        document["config"]["execution_controls"]["parallel_workers"]["max"],
        255
    );
    Ok(())
}
