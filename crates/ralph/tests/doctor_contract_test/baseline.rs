//! Baseline doctor contract tests.

use super::*;

#[test]
fn doctor_passes_in_clean_env() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    // Setup valid repo
    Command::new("git")
        .current_dir(dir.path())
        .arg("init")
        .status()?;

    // Setup ralph
    ralph_cmd_in_dir(dir.path())
        .current_dir(dir.path())
        .args(["init", "--force", "--non-interactive"])
        .status()?;
    // Setup Makefile
    std::fs::write(dir.path().join("Makefile"), "ci:\n\tcargo test\n")?;
    trust_repo(dir.path())?;

    let output = ralph_cmd_in_dir(dir.path()).arg("doctor").output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    if !output.status.success() {
        println!("STDOUT:\n{stdout}");
        println!("STDERR:\n{stderr}");
    }

    // Missing upstream is now a warning, not a failure, so doctor should pass
    assert!(output.status.success());
    assert!(combined.contains("OK") && combined.contains("git binary found"));
    assert!(combined.contains("OK") && combined.contains("queue valid"));
    assert!(combined.contains("WARN") && combined.contains("no upstream configured"));
    Ok(())
}

#[test]
fn doctor_fails_when_queue_missing() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    Command::new("git")
        .current_dir(dir.path())
        .arg("init")
        .status()?;

    // No ralph init

    let output = ralph_cmd_in_dir(dir.path()).arg("doctor").output()?;

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);
    assert!(combined.contains("FAIL") && combined.contains("queue file missing"));
    Ok(())
}

#[test]
fn doctor_warns_on_missing_upstream() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    // Setup valid repo without upstream
    Command::new("git")
        .current_dir(dir.path())
        .arg("init")
        .status()?;

    // Setup ralph
    ralph_cmd_in_dir(dir.path())
        .current_dir(dir.path())
        .args(["init", "--force", "--non-interactive"])
        .status()?;
    // Setup Makefile
    std::fs::write(dir.path().join("Makefile"), "ci:\n\tcargo test\n")?;
    trust_repo(dir.path())?;

    let output = ralph_cmd_in_dir(dir.path()).arg("doctor").output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    if !output.status.success() {
        println!("STDOUT:\n{stdout}");
        println!("STDERR:\n{stderr}");
    }

    // Should succeed with a warning about missing upstream
    assert!(output.status.success());
    assert!(combined.contains("WARN") && combined.contains("no upstream configured"));
    Ok(())
}
