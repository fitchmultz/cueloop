//! Tests for GitHub CLI preflight helpers.
//!
//! Purpose:
//! - Tests for GitHub CLI preflight helpers.
//!
//! Responsibilities:
//! - Cover `gh` availability and authentication diagnostics.
//! - Keep PR module regression tests near the implementation split.
//!
//! Not handled here:
//! - End-to-end integration with a live GitHub repository.
//! - Managed subprocess behavior already covered elsewhere.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Tests simulate `gh` responses via injected closures instead of spawning `gh`.

use super::gh::check_gh_available_with;

#[test]
fn check_gh_available_fails_when_gh_not_found() {
    let run_gh = |_args: &[&str]| -> anyhow::Result<std::process::Output> {
        Err(anyhow::anyhow!(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No such file or directory",
        )))
    };

    let result = check_gh_available_with(run_gh);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("GitHub CLI (`gh`) not found on PATH"));
    assert!(msg.contains("https://cli.github.com/"));
}

#[test]
fn check_gh_available_fails_when_version_fails() {
    let fail_status = std::process::Command::new("false")
        .status()
        .expect("'false' command should exist");

    let run_gh = |args: &[&str]| -> anyhow::Result<std::process::Output> {
        if args == ["--version"] {
            Ok(std::process::Output {
                status: fail_status,
                stdout: vec![],
                stderr: b"gh: command not recognized".to_vec(),
            })
        } else {
            Ok(std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: vec![],
                stderr: vec![],
            })
        }
    };

    let result = check_gh_available_with(run_gh);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("`gh --version` failed"));
    assert!(msg.contains("gh is not usable"));
}

#[test]
fn check_gh_available_fails_when_auth_fails() {
    let fail_status = std::process::Command::new("false")
        .status()
        .expect("'false' command should exist");

    let run_gh = |args: &[&str]| -> anyhow::Result<std::process::Output> {
        if args == ["--version"] {
            Ok(std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: b"gh version 2.40.0".to_vec(),
                stderr: vec![],
            })
        } else if args == ["auth", "status"] {
            Ok(std::process::Output {
                status: fail_status,
                stdout: vec![],
                stderr: b"You are not logged into any GitHub hosts".to_vec(),
            })
        } else {
            Ok(std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: vec![],
                stderr: vec![],
            })
        }
    };

    let result = check_gh_available_with(run_gh);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("GitHub CLI (`gh`) is not authenticated"));
    assert!(msg.contains("gh auth login"));
}

#[test]
fn check_gh_available_succeeds_when_both_checks_pass() {
    let run_gh = |args: &[&str]| -> anyhow::Result<std::process::Output> {
        if args == ["--version"] {
            Ok(std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: b"gh version 2.40.0".to_vec(),
                stderr: vec![],
            })
        } else if args == ["auth", "status"] {
            Ok(std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: b"Logged in to github.com as user".to_vec(),
                stderr: vec![],
            })
        } else {
            Ok(std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: vec![],
                stderr: vec![],
            })
        }
    };

    assert!(check_gh_available_with(run_gh).is_ok());
}
