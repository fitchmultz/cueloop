//! System and doctor contract coverage for `cueloop machine`.
//!
//! Purpose:
//! - Verify machine-visible system info and doctor report documents.
//!
//! Responsibilities:
//! - Assert machine system info exposes a versioned CLI metadata document.
//! - Assert machine doctor report exposes the versioned blocking document shape.
//! - Keep system-level machine contract regressions isolated from queue and task tests.
//!
//! Non-scope:
//! - Queue/workspace or task mutation coverage.
//! - Parallel runtime blocking scenarios.
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions callers must respect:
//! - Doctor coverage runs against a disposable initialized Ralph repo.
//! - Assertions preserve the legacy suite’s contract expectations.

use super::machine_contract_test_support::{run_in_dir, setup_git_repo, setup_ralph_repo};
use anyhow::Result;
use serde_json::Value;

#[test]
fn machine_system_info_reports_cli_version() -> Result<()> {
    let dir = setup_git_repo()?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["machine", "system", "info"]);
    assert!(
        status.success(),
        "machine system info failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let document: Value = serde_json::from_str(&stdout)?;
    assert_eq!(document["version"], 1);
    assert!(document["cli_version"].as_str().is_some());
    Ok(())
}

#[test]
fn machine_doctor_report_returns_versioned_blocking_document() -> Result<()> {
    let dir = setup_ralph_repo()?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["machine", "doctor", "report"]);
    assert!(
        status.success(),
        "machine doctor report failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let document: Value = serde_json::from_str(&stdout)?;
    assert_eq!(document["version"], 2);
    assert!(document["blocking"].is_object());
    assert_eq!(document["blocking"], document["report"]["blocking"]);
    assert!(document["report"]["checks"].is_array());
    Ok(())
}
