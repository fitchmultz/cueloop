//! Source-snapshot runtime-state path contracts for `pre-public-check.sh`.
//!
//! Purpose:
//! - Keep `--allow-no-git` `.cueloop`/`.ralph` path rejection coverage grouped together.
//!
//! Responsibilities:
//! - Verify non-directory runtime roots, symlinked allowlisted files, and unallowlisted runtime paths remain rejected.
//!
//! Scope:
//! - Limited to source-snapshot runtime-state path coverage.
//!
//! Usage:
//! - Loaded by `source_snapshot_safety.rs`.
//!
//! Invariants/Assumptions:
//! - These tests must preserve the existing release-contract assertions verbatim.

use std::process::Command;

use super::super::super::support::{copy_pre_public_check_fixture, write_file};

#[cfg(unix)]
#[test]
fn pre_public_check_allow_no_git_rejects_non_directory_ralph_roots() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let outside_dir = tempfile::tempdir().expect("create outside dir");

    let cases = [
        "broken-symlink",
        "internal-symlink",
        "external-symlink",
        "regular-file",
    ];
    for case_name in cases {
        let repo_root = temp_dir.path().join(case_name);
        std::fs::create_dir_all(&repo_root).expect("create case repo root");
        copy_pre_public_check_fixture(&repo_root);

        match case_name {
            "broken-symlink" => symlink("DOES_NOT_EXIST", repo_root.join(".ralph"))
                .expect("create broken .ralph symlink"),
            "internal-symlink" => {
                std::fs::create_dir_all(repo_root.join("internal-ralph"))
                    .expect("create internal .ralph target");
                symlink("internal-ralph", repo_root.join(".ralph"))
                    .expect("create internal .ralph symlink");
            }
            "external-symlink" => symlink(outside_dir.path(), repo_root.join(".ralph"))
                .expect("create external .ralph symlink"),
            "regular-file" => {
                write_file(&repo_root.join(".ralph"), "not a directory\n");
            }
            _ => unreachable!("unexpected case"),
        }

        let output = Command::new("bash")
            .arg(repo_root.join("scripts/pre-public-check.sh"))
            .args([
                "--skip-ci",
                "--skip-links",
                "--skip-clean",
                "--allow-no-git",
            ])
            .current_dir(&repo_root)
            .output()
            .unwrap_or_else(|err| panic!("run source-snapshot safety mode for {case_name}: {err}"));

        assert!(
            !output.status.success(),
            "source-snapshot safety mode should reject {case_name} .ralph roots\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("local/runtime artifacts") && stderr.contains(".ralph"),
            "{case_name} .ralph root rejection should explain the offending path\nstderr:\n{}",
            stderr
        );
    }
}

#[cfg(unix)]
#[test]
fn pre_public_check_allow_no_git_rejects_symlinked_allowlisted_ralph_files() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let outside_dir = tempfile::tempdir().expect("create outside dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    std::fs::create_dir_all(repo_root.join(".ralph")).expect("create .ralph dir");
    std::fs::write(outside_dir.path().join("outside.md"), "outside\n")
        .expect("write outside markdown");
    symlink(
        outside_dir.path().join("outside.md"),
        repo_root.join(".ralph/README.md"),
    )
    .expect("create symlinked allowlisted .ralph readme");

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
        .expect("run source-snapshot safety mode with symlinked allowlisted .ralph file");

    assert!(
        !output.status.success(),
        "source-snapshot safety mode should reject symlinked allowlisted .ralph files\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("local/runtime artifacts") && stderr.contains(".ralph/README.md"),
        "symlinked allowlisted .ralph file rejection should explain the offending path\nstderr:\n{}",
        stderr
    );
}

#[test]
fn pre_public_check_allow_no_git_rejects_unallowlisted_ralph_paths() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(
        &repo_root.join(".ralph/plugins/test.plugin/plugin.json"),
        "{\"name\":\"test.plugin\"}\n",
    );
    write_file(
        &repo_root.join(".ralph/trust.json"),
        "{\"allow_project_commands\":true}\n",
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
        .expect("run source-snapshot safety mode with unallowlisted .ralph paths");

    assert!(
        !output.status.success(),
        "source-snapshot safety mode should reject unallowlisted .ralph paths\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(".ralph/plugins/test.plugin/plugin.json")
            && stderr.contains(".ralph/trust.json"),
        "unallowlisted .ralph rejection should enumerate the offending paths\nstderr:\n{}",
        stderr
    );
}

#[test]
fn pre_public_check_allow_no_git_rejects_unallowlisted_cueloop_paths() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(
        &repo_root.join(".cueloop/plugins/test.plugin/plugin.json"),
        "{\"name\":\"test.plugin\"}\n",
    );
    write_file(
        &repo_root.join(".cueloop/trust.json"),
        "{\"allow_project_commands\":true}\n",
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
        .expect("run source-snapshot safety mode with unallowlisted .cueloop paths");

    assert!(
        !output.status.success(),
        "source-snapshot safety mode should reject unallowlisted .cueloop paths\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(".cueloop/plugins/test.plugin/plugin.json")
            && stderr.contains(".cueloop/trust.json"),
        "unallowlisted .cueloop rejection should enumerate the offending paths\nstderr:\n{}",
        stderr
    );
}
