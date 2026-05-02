//! Mid-suite `pre-public-check.sh` runtime-artifact contracts.
//!
//! Purpose:
//! - Keep target and runtime-state artifact regressions grouped together.
//!
//! Responsibilities:
//! - Verify source-snapshot and tracked runtime-artifact failures, including control-character paths.
//!
//! Scope:
//! - Limited to mid-suite `pre-public-check.sh` runtime-artifact coverage.
//!
//! Usage:
//! - Loaded by `pre_public_check_contracts_mid.rs`.
//!
//! Invariants/Assumptions:
//! - These tests must preserve the existing release-contract assertions verbatim.

use std::process::Command;

use super::super::support::{
    commit_fixture, copy_pre_public_check_fixture, disable_global_excludes, force_stage_all,
    init_git_repo, write_file,
};

#[test]
fn pre_public_check_allow_no_git_rejects_target_directory() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(
        &repo_root.join("target/leakdir/out.txt"),
        "local build output\n",
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args([
            "--skip-ci",
            "--skip-links",
            "--skip-clean",
            "--allow-no-git",
        ])
        .current_dir(repo_root)
        .output()
        .expect("run source-snapshot safety mode with target directory");

    assert!(
        !output.status.success(),
        "source-snapshot safety mode should reject target directory contents\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("local/runtime artifacts") && stderr.contains("target"),
        "target rejection should explain the offending path\nstderr:\n{}",
        stderr
    );
}

#[test]
fn pre_public_check_rejects_tracked_target_artifacts() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(
        &repo_root.join("target/debug/cueloop"),
        "built binary placeholder\n",
    );

    init_git_repo(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci", "--skip-links", "--skip-clean"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with tracked target artifact");

    assert!(
        !output.status.success(),
        "pre-public-check should reject tracked target artifacts\nstdout:\n{}\nstderr:\n{}",
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
            && combined.contains("target/debug/cueloop"),
        "tracked target artifact rejection should explain the offending path\noutput:\n{}",
        combined
    );
}

#[cfg(unix)]
#[test]
fn pre_public_check_rejects_tracked_runtime_artifact_control_character_paths() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(
        &repo_root.join("target/evil\nREADME.md"),
        "tracked runtime artifact newline path\n",
    );

    init_git_repo(repo_root);
    disable_global_excludes(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args([
            "--skip-ci",
            "--skip-links",
            "--skip-secrets",
            "--skip-clean",
        ])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with tracked runtime control-character path");

    assert!(
        !output.status.success(),
        "pre-public-check should reject tracked runtime control-character paths\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("unsupported control characters")
            && combined.contains("target/evil")
            && combined.contains("README.md"),
        "tracked runtime control-character rejection should explain the offending path\noutput:\n{}",
        combined
    );
}

#[cfg(unix)]
#[test]
fn pre_public_check_rejects_tracked_cueloop_control_character_paths() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(
        &repo_root.join(".cueloop/bad\nqueue.jsonc"),
        "tracked .cueloop newline path\n",
    );

    init_git_repo(repo_root);
    disable_global_excludes(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args([
            "--skip-ci",
            "--skip-links",
            "--skip-secrets",
            "--skip-clean",
        ])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with tracked .cueloop control-character path");

    assert!(
        !output.status.success(),
        "pre-public-check should reject tracked .cueloop control-character paths\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("unsupported control characters")
            && combined.contains(".cueloop/bad")
            && combined.contains("queue.jsonc"),
        "tracked .cueloop control-character rejection should explain the offending path\noutput:\n{}",
        combined
    );
}
