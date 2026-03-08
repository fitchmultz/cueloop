//! Doctor repo-hygiene auto-fix tests.

use super::*;

#[test]
fn doctor_detects_missing_ralph_logs_gitignore() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    Command::new("git")
        .current_dir(dir.path())
        .arg("init")
        .status()?;

    // Setup ralph (which adds .ralph/logs/ to .gitignore)
    ralph_cmd_in_dir(dir.path())
        .current_dir(dir.path())
        .args(["init", "--force", "--non-interactive"])
        .status()?;

    // Setup Makefile
    std::fs::write(dir.path().join("Makefile"), "ci:\n\tcargo test\n")?;

    // Overwrite .gitignore to intentionally omit .ralph/logs/
    std::fs::write(
        dir.path().join(".gitignore"),
        ".ralph/lock\n.ralph/cache/\n",
    )?;

    // Run doctor with JSON output
    let output = ralph_cmd_in_dir(dir.path())
        .args(["doctor", "--format", "json"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("JSON should be valid. Got stdout: {}", stdout));

    // Should fail because .ralph/logs/ is not gitignored
    assert_eq!(
        json["success"], false,
        "doctor should fail when .ralph/logs/ is not gitignored"
    );

    // Find the gitignore_ralph_logs check
    let checks = json["checks"].as_array().unwrap();
    let logs_check = checks
        .iter()
        .find(|c| c["category"] == "project" && c["check"] == "gitignore_ralph_logs");

    assert!(
        logs_check.is_some(),
        "should have a gitignore_ralph_logs check. Checks: {:?}",
        checks
    );
    let logs_check = logs_check.unwrap();

    assert_eq!(logs_check["severity"], "Error", "should be Error severity");
    assert_eq!(
        logs_check["fix_available"], true,
        "should have fix_available=true"
    );
    assert!(
        logs_check["suggested_fix"]
            .as_str()
            .unwrap_or("")
            .contains(".ralph/logs/"),
        "suggested_fix should mention .ralph/logs/"
    );

    Ok(())
}

#[test]
fn doctor_auto_fix_adds_ralph_logs_gitignore() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    Command::new("git")
        .current_dir(dir.path())
        .arg("init")
        .status()?;

    // Setup ralph (which adds .ralph/logs/ to .gitignore)
    ralph_cmd_in_dir(dir.path())
        .current_dir(dir.path())
        .args(["init", "--force", "--non-interactive"])
        .status()?;

    // Setup Makefile
    std::fs::write(dir.path().join("Makefile"), "ci:\n\tcargo test\n")?;

    // Overwrite .gitignore to intentionally omit .ralph/logs/
    std::fs::write(
        dir.path().join(".gitignore"),
        ".ralph/lock\n.ralph/cache/\n",
    )?;

    // Run doctor with --auto-fix and JSON output
    let output = ralph_cmd_in_dir(dir.path())
        .args(["doctor", "--format", "json", "--auto-fix"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("JSON should be valid. Got stdout: {}", stdout));

    // Find the gitignore_ralph_logs check
    let checks = json["checks"].as_array().unwrap();
    let logs_check = checks
        .iter()
        .find(|c| c["category"] == "project" && c["check"] == "gitignore_ralph_logs");

    assert!(
        logs_check.is_some(),
        "should have a gitignore_ralph_logs check"
    );
    let logs_check = logs_check.unwrap();

    // Verify fix_applied is true
    assert_eq!(
        logs_check["fix_applied"], true,
        "fix_applied should be true after auto-fix"
    );

    // Verify .gitignore now contains .ralph/logs/
    let gitignore_content = std::fs::read_to_string(dir.path().join(".gitignore"))?;
    assert!(
        gitignore_content.contains(".ralph/logs/"),
        ".gitignore should now contain .ralph/logs/. Content: {}",
        gitignore_content
    );

    Ok(())
}
