//! Exact tracked runtime-artifact path contracts for `pre-public-check.sh`.
//!
//! Purpose:
//! - Keep the exact tracked runtime-artifact rejection coverage grouped together.
//!
//! Responsibilities:
//! - Verify exact `target`, `.venv`, and `.pytest_cache` tracked paths remain rejected.
//!
//! Scope:
//! - Limited to exact tracked runtime-artifact coverage.
//!
//! Usage:
//! - Loaded by `pre_public_check_contracts_tracked_paths.rs`.
//!
//! Invariants/Assumptions:
//! - These tests must preserve the existing release-contract assertions verbatim.

use std::process::Command;

use super::super::support::{
    commit_fixture, copy_pre_public_check_fixture, disable_global_excludes, force_stage_all,
    init_git_repo, write_file,
};

#[test]
fn pre_public_check_rejects_tracked_exact_target_path() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(&repo_root.join("target"), "tracked exact target path\n");

    init_git_repo(repo_root);
    disable_global_excludes(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci", "--skip-links", "--skip-clean"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with exact tracked target path");

    assert!(
        !output.status.success(),
        "pre-public-check should reject exact tracked target paths\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Tracked runtime/build artifacts detected")
            && combined.contains("target"),
        "exact tracked target path rejection should explain the offending path\noutput:\n{}",
        combined
    );
}

#[test]
fn pre_public_check_rejects_tracked_virtualenv_artifacts() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(
        &repo_root.join(".venv/bin/python"),
        "#!/usr/bin/env python3\n",
    );

    init_git_repo(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci", "--skip-links", "--skip-clean"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with tracked virtualenv artifact");

    assert!(
        !output.status.success(),
        "pre-public-check should reject tracked virtualenv artifacts\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Tracked runtime/build artifacts detected")
            && combined.contains(".venv/bin/python"),
        "tracked virtualenv artifact rejection should explain the offending path\noutput:\n{}",
        combined
    );
}

#[test]
fn pre_public_check_rejects_tracked_exact_virtualenv_path() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(&repo_root.join(".venv"), "tracked exact .venv path\n");

    init_git_repo(repo_root);
    disable_global_excludes(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci", "--skip-links", "--skip-clean"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with exact tracked virtualenv path");

    assert!(
        !output.status.success(),
        "pre-public-check should reject exact tracked virtualenv paths\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Tracked runtime/build artifacts detected") && combined.contains(".venv"),
        "exact tracked virtualenv path rejection should explain the offending path\noutput:\n{}",
        combined
    );
}

#[test]
fn pre_public_check_rejects_tracked_pytest_cache_artifacts() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(&repo_root.join(".pytest_cache/v/cache/nodeids"), "[]\n");

    init_git_repo(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci", "--skip-links", "--skip-clean"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with tracked pytest cache artifact");

    assert!(
        !output.status.success(),
        "pre-public-check should reject tracked pytest cache artifacts\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Tracked runtime/build artifacts detected")
            && combined.contains(".pytest_cache/v/cache/nodeids"),
        "tracked pytest cache artifact rejection should explain the offending path\noutput:\n{}",
        combined
    );
}

#[test]
fn pre_public_check_rejects_tracked_exact_pytest_cache_path() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(
        &repo_root.join(".pytest_cache"),
        "tracked exact .pytest_cache path\n",
    );

    init_git_repo(repo_root);
    disable_global_excludes(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci", "--skip-links", "--skip-clean"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with exact tracked pytest cache path");

    assert!(
        !output.status.success(),
        "pre-public-check should reject exact tracked pytest cache paths\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Tracked runtime/build artifacts detected")
            && combined.contains(".pytest_cache"),
        "exact tracked pytest cache path rejection should explain the offending path\noutput:\n{}",
        combined
    );
}
