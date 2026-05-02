//! Public-readiness scan secret-detection contracts.
//!
//! Purpose:
//! - Keep secret-scanning and allowlist regression coverage grouped together.
//!
//! Responsibilities:
//! - Verify excluded build artifacts, allowlisted `.ralph` files, injected helper/docs secrets, and private-key detection.
//!
//! Scope:
//! - Limited to public-readiness secret-scanning contracts.
//!
//! Usage:
//! - Loaded by `public_readiness_scan_contracts.rs`.
//!
//! Invariants/Assumptions:
//! - These tests must preserve the existing public-readiness helper contract assertions verbatim.

use std::process::Command;

use super::super::support::{
    assert_output_redacts_secret, copy_public_readiness_scan_fixture,
    public_readiness_scan_python_path, read_repo_file, write_file,
};

#[test]
fn public_readiness_scan_excludes_macos_target_build_artifacts() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();
    let secret_token = ["gh", "p_12345678901234567890"].concat();

    copy_public_readiness_scan_fixture(repo_root);
    write_file(&repo_root.join("README.md"), "ok\n");
    write_file(
        &repo_root.join("apps/CueLoopMac/target/tmp/generated.txt"),
        &format!("{secret_token}\n"),
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.sh"))
        .arg("secrets")
        .current_dir(repo_root)
        .output()
        .expect("run public-readiness secret scan with macOS target artifact");

    assert_eq!(
        output.status.code(),
        Some(0),
        "secret scan should exclude generated macOS target build artifacts"
    );
    assert_output_redacts_secret(&output, &secret_token);
}

#[test]
fn public_readiness_scan_scans_allowlisted_ralph_files_for_secrets() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();
    let secret_token = ["gh", "p_12345678901234567890"].concat();

    copy_public_readiness_scan_fixture(repo_root);
    write_file(
        &repo_root.join(".ralph/config.jsonc"),
        &format!("token: {secret_token}\n"),
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.sh"))
        .arg("secrets")
        .current_dir(repo_root)
        .output()
        .expect("run public-readiness secret scan over allowlisted .ralph file");

    assert_eq!(
        output.status.code(),
        Some(1),
        "public-readiness scan should inspect allowlisted .ralph files for secrets"
    );
    assert_output_redacts_secret(&output, &secret_token);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(".ralph/config.jsonc:1: github_classic_token: [REDACTED length=24]"),
        "secret scan should report secrets inside allowlisted .ralph files\nstdout:\n{}",
        stdout
    );
}

#[test]
fn public_readiness_scan_rejects_injected_secret_in_scan_helper_source() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path().join("repo");
    let secret_token = ["gh", "p_12345678901234567890"].concat();
    std::fs::create_dir_all(repo_root.join("scripts/lib")).expect("create scripts/lib dir");
    let scan_source = read_repo_file("scripts/lib/public_readiness_scan.py");
    std::fs::write(
        repo_root.join("scripts/lib/public_readiness_scan.py"),
        format!("# {secret_token}\n{scan_source}"),
    )
    .expect("write injected scan helper source");
    std::fs::write(repo_root.join("README.md"), "ok\n").expect("write readme fixture");

    let output = Command::new("python3")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.py"))
        .arg("secrets")
        .arg(&repo_root)
        .env("RALPH_PUBLIC_SCAN_EXCLUDES", "")
        .output()
        .expect("run public-readiness scan helper against injected source");

    assert_eq!(
        output.status.code(),
        Some(1),
        "secret scan should not file-wide allowlist the scan helper source"
    );
    assert_output_redacts_secret(&output, &secret_token);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("scripts/lib/public_readiness_scan.py:")
            && stdout.contains("github_classic_token: [REDACTED length=24]"),
        "injected helper secret should be reported\nstdout:\n{}",
        stdout
    );
}

#[test]
fn public_readiness_scan_rejects_same_line_secret_in_security_docs_allowlist() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path().join("repo");
    let secret_token = ["gh", "p_12345678901234567890"].concat();
    std::fs::create_dir_all(repo_root.join("docs/features")).expect("create docs/features dir");
    let aws_example = ["AKIA", "IOSFODNN7EXAMPLE"].concat();
    let exact_allowlisted_line =
        format!("| **AWS Keys** | AKIA-prefixed access keys | `{aws_example}` → `[REDACTED]` |");
    std::fs::write(
        repo_root.join("docs/features/security.md"),
        format!("{exact_allowlisted_line} {secret_token}\n"),
    )
    .expect("write security.md fixture");
    std::fs::write(repo_root.join("README.md"), "ok\n").expect("write readme fixture");

    let output = Command::new("python3")
        .arg(public_readiness_scan_python_path())
        .arg("secrets")
        .arg(&repo_root)
        .env("RALPH_PUBLIC_SCAN_EXCLUDES", "")
        .output()
        .expect("run public-readiness scan over same-line injected security docs secret");

    assert_eq!(
        output.status.code(),
        Some(1),
        "secret scan should reject same-line injected secrets in allowlisted docs lines"
    );
    assert_output_redacts_secret(&output, &secret_token);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("docs/features/security.md:")
            && stdout.contains("github_classic_token: [REDACTED length=24]"),
        "same-line injected docs secret should be reported\nstdout:\n{}",
        stdout
    );
}

#[test]
fn public_readiness_scan_rejects_same_line_secret_in_scan_helper_source() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path().join("repo");
    let secret_token = ["gh", "p_12345678901234567890"].concat();
    std::fs::create_dir_all(repo_root.join("scripts/lib")).expect("create scripts/lib dir");

    let scan_source = read_repo_file("scripts/lib/public_readiness_scan.py");
    let target_line = "AWS_DOCS_ALLOWLIST_LINE = (";
    let injected_line = format!("{target_line}  # {secret_token}");
    let injected_source = scan_source.replacen(target_line, &injected_line, 1);
    assert_ne!(
        injected_source, scan_source,
        "fixture should replace the targeted scan-helper source line"
    );
    std::fs::write(
        repo_root.join("scripts/lib/public_readiness_scan.py"),
        injected_source,
    )
    .expect("write injected scan helper source");
    std::fs::write(repo_root.join("README.md"), "ok\n").expect("write readme fixture");

    let output = Command::new("python3")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.py"))
        .arg("secrets")
        .arg(&repo_root)
        .env("RALPH_PUBLIC_SCAN_EXCLUDES", "")
        .output()
        .expect("run public-readiness scan helper against same-line injected source");

    assert_eq!(
        output.status.code(),
        Some(1),
        "secret scan should reject same-line injected secrets in scan-helper source"
    );
    assert_output_redacts_secret(&output, &secret_token);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("scripts/lib/public_readiness_scan.py:")
            && stdout.contains("github_classic_token: [REDACTED length=24]"),
        "same-line injected scan-helper secret should be reported\nstdout:\n{}",
        stdout
    );
}

#[test]
fn public_readiness_scan_rejects_private_key_in_pre_public_check_script() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path().join("repo");
    let private_key_body = "MIIEpAIBAAKCAQEA75abcdef1234567890";
    std::fs::create_dir_all(repo_root.join("scripts")).expect("create scripts dir");
    std::fs::write(
        repo_root.join("scripts/pre-public-check.sh"),
        format!(
            "-----BEGIN {} PRIVATE KEY-----\n{}\n-----END {} PRIVATE KEY-----\n",
            "RSA", private_key_body, "RSA"
        ),
    )
    .expect("write pre-public-check fixture");
    std::fs::write(repo_root.join("README.md"), "ok\n").expect("write readme fixture");

    let output = Command::new("python3")
        .arg(public_readiness_scan_python_path())
        .arg("secrets")
        .arg(&repo_root)
        .env("RALPH_PUBLIC_SCAN_EXCLUDES", "")
        .output()
        .expect("run public-readiness scan over pre-public-check fixture");

    assert_eq!(
        output.status.code(),
        Some(1),
        "secret scan should reject private keys in pre-public-check.sh"
    );
    let private_key_header = ["BEGIN", " RSA PRIVATE KEY"].concat();
    assert_output_redacts_secret(&output, &private_key_header);
    assert_output_redacts_secret(&output, private_key_body);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("scripts/pre-public-check.sh:1: private_key: [REDACTED length=21]"),
        "private key in pre-public-check.sh should be reported\nstdout:\n{}",
        stdout
    );
}
