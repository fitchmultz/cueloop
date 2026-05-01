//! Mid-suite `pre-public-check.sh` local-only and git enumeration contracts.
//!
//! Purpose:
//! - Keep tracked local-only and release-context regressions grouped together.
//!
//! Responsibilities:
//! - Verify tracked local-only failures, control-character handling, and git enumeration failures.
//!
//! Scope:
//! - Limited to mid-suite `pre-public-check.sh` coverage.
//!
//! Usage:
//! - Loaded by `pre_public_check_contracts_mid.rs`.
//!
//! Invariants/Assumptions:
//! - These tests must preserve the existing release-contract assertions verbatim.

use std::process::Command;

use super::super::support::{
    break_git_index, commit_fixture, copy_pre_public_check_fixture, disable_global_excludes,
    force_stage_all, init_git_repo, stage_all, write_file,
};

#[test]
fn pre_public_check_rejects_tracked_local_only_files() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(&repo_root.join(".scratchpad.md"), "local operator notes\n");

    init_git_repo(repo_root);
    stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci", "--skip-links"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with tracked local-only file");

    assert!(
        !output.status.success(),
        "pre-public-check should reject tracked local-only files\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Tracked local-only files detected") && stderr.contains(".scratchpad.md"),
        "tracked local-only file rejection should explain the offending path\nstderr:\n{}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn pre_public_check_release_context_rejects_dirty_paths_with_control_characters() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    init_git_repo(repo_root);
    stage_all(repo_root);
    commit_fixture(repo_root);

    write_file(
        &repo_root.join("CHANGELOG.md\nREADME.md"),
        "dirty filename payload\n",
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args([
            "--skip-ci",
            "--skip-links",
            "--skip-secrets",
            "--release-context",
        ])
        .current_dir(repo_root)
        .output()
        .expect("run release-context pre-public-check with dirty control-character path");

    assert!(
        !output.status.success(),
        "release-context pre-public-check should reject dirty paths with control characters\nstdout:\n{}\nstderr:\n{}",
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
            && combined.contains("CHANGELOG.md")
            && combined.contains("README.md"),
        "dirty control-character path rejection should explain the offending path\noutput:\n{}",
        combined
    );
}

#[test]
fn pre_public_check_rejects_git_ls_files_failures() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(
        &repo_root.join(".scratchpad.md"),
        "tracked local-only payload\n",
    );

    init_git_repo(repo_root);
    disable_global_excludes(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);
    break_git_index(repo_root);

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
        .expect("run pre-public-check with broken git ls-files");

    assert!(
        !output.status.success(),
        "pre-public-check should fail closed when git ls-files fails\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("git ls-files -z failed"),
        "git ls-files failure should be reported\noutput:\n{}",
        combined
    );
}

#[cfg(unix)]
#[test]
fn pre_public_check_rejects_tracked_local_only_control_character_paths() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(
        &repo_root.join(".env\nREADME.md"),
        "tracked local-only newline path\n",
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
        .expect("run pre-public-check with tracked local-only control-character path");

    assert!(
        !output.status.success(),
        "pre-public-check should reject tracked local-only control-character paths\nstdout:\n{}\nstderr:\n{}",
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
            && combined.contains(".env")
            && combined.contains("README.md"),
        "tracked local-only control-character rejection should explain the offending path\noutput:\n{}",
        combined
    );
}

#[test]
fn pre_public_check_rejects_tracked_local_only_directory_contents() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(
        &repo_root.join(".env.local/secret.txt"),
        "tracked local-only directory payload\n",
    );

    init_git_repo(repo_root);
    stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci", "--skip-links", "--skip-clean"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with tracked local-only directory contents");

    assert!(
        !output.status.success(),
        "pre-public-check should reject tracked local-only directory contents\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Tracked local-only files detected")
            && combined.contains(".env.local/secret.txt"),
        "tracked local-only directory rejection should explain the offending path\noutput:\n{}",
        combined
    );
}
