//! Doctor auto-fix behavior tests.
//!
//! Responsibilities:
//! - Verify doctor reports and repairs orphaned locks when requested.
//! - Verify queue repair behavior for invalid repo state.
//!
//! Not handled here:
//! - Human-readable baseline diagnostics unrelated to auto-fix.
//!
//! Invariants/assumptions:
//! - Tests start from seeded doctor fixtures, then inject broken state explicitly.
//! - Auto-fix assertions validate both output and resulting filesystem state.

use super::*;

#[test]
fn doctor_auto_fix_removes_orphaned_locks() -> Result<()> {
    let dir = setup_doctor_repo()?;

    let lock_dir = dir.path().join(".ralph/lock/orphaned-test-lock");
    std::fs::create_dir_all(&lock_dir)?;
    let owner_file = lock_dir.join("owner");
    std::fs::write(&owner_file, "pid:999999\nstarted:1234567890\n")?;

    assert!(
        lock_dir.exists(),
        "lock directory should exist before doctor run"
    );

    let output = ralph_cmd_in_dir(dir.path())
        .args(["doctor", "--auto-fix"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    assert!(
        combined.contains("orphaned") || combined.contains("lock"),
        "should mention orphaned locks. Output: {}",
        combined
    );

    assert!(
        !lock_dir.exists(),
        "orphaned lock directory should be removed after auto-fix"
    );

    Ok(())
}

#[test]
fn doctor_auto_fix_without_flag_reports_but_does_not_remove() -> Result<()> {
    let dir = setup_doctor_repo()?;

    let lock_dir = dir.path().join(".ralph/lock/orphaned-test-lock-no-fix");
    std::fs::create_dir_all(&lock_dir)?;
    let owner_file = lock_dir.join("owner");
    std::fs::write(&owner_file, "pid:999998\nstarted:1234567890\n")?;

    let output = ralph_cmd_in_dir(dir.path()).arg("doctor").output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    assert!(
        combined.contains("orphaned") || combined.contains("WARN"),
        "should warn about orphaned locks. Output: {}",
        combined
    );

    assert!(
        lock_dir.exists(),
        "lock directory should still exist without --auto-fix"
    );

    let _ = std::fs::remove_dir_all(&lock_dir);

    Ok(())
}

#[test]
fn doctor_json_output_with_auto_fix() -> Result<()> {
    let dir = setup_doctor_repo()?;

    let lock_dir = dir.path().join(".ralph/lock/orphaned-test-lock-json");
    std::fs::create_dir_all(&lock_dir)?;
    let owner_file = lock_dir.join("owner");
    std::fs::write(&owner_file, "pid:999997\nstarted:1234567890\n")?;

    let output = ralph_cmd_in_dir(dir.path())
        .args(["doctor", "--format", "json", "--auto-fix"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("JSON should be valid");

    let fixes_applied = json["summary"]["fixes_applied"].as_u64().unwrap_or(0);
    assert!(
        fixes_applied > 0,
        "should have fixes_applied > 0 when auto-fix removes locks"
    );

    let checks = json["checks"].as_array().unwrap();
    let lock_check = checks
        .iter()
        .find(|c| c["category"] == "lock" && c["check"] == "orphaned_locks");

    if let Some(check) = lock_check {
        assert_eq!(
            check["fix_applied"], true,
            "fix_applied should be true for orphaned locks"
        );
    }

    Ok(())
}

#[test]
fn doctor_auto_fix_repairs_invalid_queue() -> Result<()> {
    let dir = setup_doctor_repo()?;

    let invalid_queue = r#"{
  "version": 1,
  "tasks": [
    {
      "id": "RQ-0001",
      "title": "",
      "status": "todo",
      "priority": "medium",
      "tags": [],
      "scope": [],
      "depends_on": [],
      "evidence": [],
      "plan": [],
      "notes": [],
      "created_at": "2026-01-01T00:00:00Z",
      "updated_at": "2026-01-01T00:00:00Z"
    }
  ]
}"#;
    std::fs::write(dir.path().join(".ralph/queue.jsonc"), invalid_queue)?;

    let output = ralph_cmd_in_dir(dir.path()).arg("doctor").output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    assert!(
        !output.status.success(),
        "doctor should fail with invalid queue"
    );
    assert!(
        combined.contains("queue validation failed") || combined.contains("FAIL"),
        "should report queue validation failed. Output: {}",
        combined
    );

    let output = ralph_cmd_in_dir(dir.path())
        .args(["doctor", "--auto-fix"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    assert!(
        output.status.success(),
        "doctor should pass after auto-fix. Output: {}",
        combined
    );
    assert!(
        combined.contains("queue valid")
            || combined.contains("repair")
            || combined.contains("FIXED"),
        "should report queue was repaired or is now valid. Output: {}",
        combined
    );

    Ok(())
}
