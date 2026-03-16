//! Doctor JSON output contract tests.
//!
//! Responsibilities:
//! - Verify stable JSON structure for successful and failing doctor runs.
//! - Keep missing-queue coverage on git-only repos while success cases use seeded fixtures.
//!
//! Not handled here:
//! - Human-readable output formatting.
//!
//! Invariants/assumptions:
//! - JSON payloads come from stdout while logs may still go to stderr.
//! - Missing-queue coverage intentionally omits seeded `.ralph/` files.

use super::*;

#[test]
fn doctor_json_output_format() -> Result<()> {
    let dir = setup_doctor_repo()?;

    let output = ralph_cmd_in_dir(dir.path())
        .args(["doctor", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("JSON should be valid. Got stdout: {}", stdout));

    assert!(
        json.get("success").is_some(),
        "JSON should have 'success' field"
    );
    assert!(
        json.get("checks").is_some(),
        "JSON should have 'checks' field"
    );
    assert!(
        json.get("summary").is_some(),
        "JSON should have 'summary' field"
    );

    let summary = json.get("summary").unwrap();
    assert!(
        summary.get("total").is_some(),
        "summary should have 'total' field"
    );
    assert!(
        summary.get("passed").is_some(),
        "summary should have 'passed' field"
    );
    assert!(
        summary.get("warnings").is_some(),
        "summary should have 'warnings' field"
    );
    assert!(
        summary.get("errors").is_some(),
        "summary should have 'errors' field"
    );
    assert!(
        summary.get("fixes_applied").is_some(),
        "summary should have 'fixes_applied' field"
    );
    assert!(
        summary.get("fixes_failed").is_some(),
        "summary should have 'fixes_failed' field"
    );

    let checks = json
        .get("checks")
        .unwrap()
        .as_array()
        .expect("checks should be an array");
    assert!(!checks.is_empty(), "should have at least one check");

    let first_check = &checks[0];
    assert!(
        first_check.get("category").is_some(),
        "check should have 'category' field"
    );
    assert!(
        first_check.get("check").is_some(),
        "check should have 'check' field"
    );
    assert!(
        first_check.get("severity").is_some(),
        "check should have 'severity' field"
    );
    assert!(
        first_check.get("message").is_some(),
        "check should have 'message' field"
    );
    assert!(
        first_check.get("fix_available").is_some(),
        "check should have 'fix_available' field"
    );

    Ok(())
}

#[test]
fn doctor_json_output_with_failed_check() -> Result<()> {
    let dir = setup_git_repo()?;

    let output = ralph_cmd_in_dir(dir.path())
        .args(["doctor", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("JSON should be valid");

    assert_eq!(
        json["success"], false,
        "success should be false when checks fail"
    );
    assert!(
        json["summary"]["errors"].as_u64().unwrap_or(0) > 0,
        "should have errors"
    );

    let checks = json["checks"].as_array().unwrap();
    let queue_error = checks
        .iter()
        .find(|c| c["category"] == "queue" && c["severity"] == "Error");
    assert!(queue_error.is_some(), "should have a queue error check");

    Ok(())
}
