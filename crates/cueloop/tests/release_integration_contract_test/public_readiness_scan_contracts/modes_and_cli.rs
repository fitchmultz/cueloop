//! Public-readiness scan mode and CLI usage contracts.
//!
//! Purpose:
//! - Keep scanner mode selection and CLI misuse coverage grouped together.
//!
//! Responsibilities:
//! - Verify missing-root handling, combined/docs modes, and shell helper argument validation.
//!
//! Scope:
//! - Limited to public-readiness mode and usage contracts.
//!
//! Usage:
//! - Loaded by `public_readiness_scan_contracts.rs`.
//!
//! Invariants/Assumptions:
//! - These tests must preserve the existing public-readiness helper contract assertions verbatim.

use std::process::Command;

use super::super::support::{
    assert_output_redacts_secret, public_readiness_scan_python_path,
    public_readiness_scan_shell_helper_path,
};

#[test]
fn public_readiness_scan_rejects_missing_repo_root() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let missing_repo_root = temp_dir.path().join("missing-repo-root");

    let output = Command::new("python3")
        .arg(public_readiness_scan_python_path())
        .arg("links")
        .arg(&missing_repo_root)
        .output()
        .expect("run public-readiness scan helper");

    assert_eq!(
        output.status.code(),
        Some(2),
        "public-readiness scan scanner should reject a missing repo root"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("repository root does not exist or is not a directory"),
        "scanner should explain why the provided repo root was rejected"
    );
}

#[test]
fn public_readiness_scan_all_mode_combines_content_checks() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path().join("repo");
    let secret_token = ["gh", "p_12345678901234567890"].concat();
    std::fs::create_dir(&repo_root).expect("create temp repo root");
    std::fs::write(repo_root.join("README.md"), "[broken](missing.md)\n")
        .expect("write markdown fixture");
    std::fs::write(
        repo_root.join("AGENTS.md"),
        "session cache: .ralph/cache/session.json\n",
    )
    .expect("write session-path fixture");
    std::fs::write(repo_root.join("notes.txt"), format!("{secret_token}\n"))
        .expect("write secret fixture");

    let output = Command::new("python3")
        .arg(public_readiness_scan_python_path())
        .arg("all")
        .arg(&repo_root)
        .env("RALPH_PUBLIC_SCAN_EXCLUDES", "")
        .output()
        .expect("run public-readiness combined scan helper");

    assert_eq!(
        output.status.code(),
        Some(1),
        "combined public-readiness scan should report all enabled content checks"
    );
    assert_output_redacts_secret(&output, &secret_token);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("README.md: missing target -> missing.md")
            && stdout.contains("AGENTS.md:1: use .ralph/cache/session.jsonc")
            && stdout.contains("notes.txt:1: github_classic_token: [REDACTED length=24]"),
        "combined scan should report markdown, session-path, and secret findings\nstdout:\n{}",
        stdout
    );
}

#[test]
fn public_readiness_scan_docs_mode_skips_secret_scan() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path().join("repo");
    let secret_token = ["gh", "p_12345678901234567890"].concat();
    std::fs::create_dir(&repo_root).expect("create temp repo root");
    std::fs::write(repo_root.join("README.md"), format!("{secret_token}\n"))
        .expect("write markdown fixture");

    let output = Command::new("python3")
        .arg(public_readiness_scan_python_path())
        .arg("docs")
        .arg(&repo_root)
        .env("RALPH_PUBLIC_SCAN_EXCLUDES", "")
        .output()
        .expect("run public-readiness docs scan helper");

    assert_eq!(
        output.status.code(),
        Some(0),
        "docs mode should combine documentation checks without running the secret scan"
    );
    assert_output_redacts_secret(&output, &secret_token);
}

#[test]
fn public_readiness_scan_rejects_help_with_extra_args() {
    let output = Command::new("bash")
        .arg(public_readiness_scan_shell_helper_path())
        .arg("--help")
        .arg("extra")
        .output()
        .expect("run public-readiness scan helper");

    assert_eq!(
        output.status.code(),
        Some(2),
        "public-readiness scan helper should reject unexpected positional arguments"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Usage:"),
        "helper should print usage for invalid argument combinations"
    );
}

#[test]
fn public_readiness_scan_rejects_links_with_extra_args() {
    let output = Command::new("bash")
        .arg(public_readiness_scan_shell_helper_path())
        .arg("links")
        .arg("extra")
        .output()
        .expect("run public-readiness scan helper");

    assert_eq!(
        output.status.code(),
        Some(2),
        "public-readiness scan helper should reject extra args for normal modes"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Usage:"),
        "helper should print usage for invalid argument combinations"
    );
}
