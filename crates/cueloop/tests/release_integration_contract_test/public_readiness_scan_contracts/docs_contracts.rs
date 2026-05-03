//! Public-readiness scan stale-doc contract coverage.
//!
//! Purpose:
//! - Keep targeted stale documentation snippet coverage grouped with the public-readiness scan seam.
//!
//! Responsibilities:
//! - Verify docs mode rejects stale CueLoopMac decomposition guidance and stale session-management contract examples.
//!
//! Scope:
//! - Limited to path-specific stale-doc snippets that should never return once corrected.
//!
//! Usage:
//! - Loaded by `public_readiness_scan_contracts.rs`.
//!
//! Invariants/Assumptions:
//! - The scan must stay narrow and path-specific rather than banning version numbers repo-wide.

use std::process::Command;

use super::super::support::{copy_public_readiness_scan_fixture, write_file};

#[test]
fn public_readiness_scan_docs_mode_rejects_stale_app_decompose_command() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_public_readiness_scan_fixture(repo_root);
    write_file(
        &repo_root.join("docs/features/app.md"),
        "The app calls `cueloop task decompose --format json`.\n",
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.sh"))
        .arg("docs")
        .current_dir(repo_root)
        .output()
        .expect("run public-readiness docs scan");

    assert_eq!(
        output.status.code(),
        Some(1),
        "docs scan should reject the stale app decomposition command snippet"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "docs/features/app.md: use `cueloop machine task decompose` for CueLoopMac decomposition docs"
        ),
        "docs scan should explain the stale app command failure\nstdout:\n{}",
        stdout
    );
}

#[test]
fn public_readiness_scan_docs_mode_rejects_stale_session_resume_event_version() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_public_readiness_scan_fixture(repo_root);
    write_file(
        &repo_root.join("docs/features/session-management.md"),
        r#"```json
{
  "version": 2,
  "kind": "resume_decision",
  "task_id": "RQ-0001"
}
```
"#,
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.sh"))
        .arg("docs")
        .current_dir(repo_root)
        .output()
        .expect("run public-readiness docs scan");

    assert_eq!(
        output.status.code(),
        Some(1),
        "docs scan should reject stale machine run resume event versions"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "docs/features/session-management.md: machine run resume_decision examples must use version 3"
        ),
        "docs scan should explain the stale resume event version failure\nstdout:\n{}",
        stdout
    );
}

#[test]
fn public_readiness_scan_docs_mode_rejects_stale_session_config_version() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_public_readiness_scan_fixture(repo_root);
    write_file(
        &repo_root.join("docs/features/session-management.md"),
        r#"```json
{
  "version": 4,
  "resume_preview": {
    "status": "refusing_to_resume"
  }
}
```
"#,
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.sh"))
        .arg("docs")
        .current_dir(repo_root)
        .output()
        .expect("run public-readiness docs scan");

    assert_eq!(
        output.status.code(),
        Some(1),
        "docs scan should reject stale machine config resolve versions"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "docs/features/session-management.md: machine config resolve examples must use version 5"
        ),
        "docs scan should explain the stale config resolve version failure\nstdout:\n{}",
        stdout
    );
}

#[test]
fn public_readiness_scan_docs_mode_allows_unrelated_version_three_json_blocks() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_public_readiness_scan_fixture(repo_root);
    write_file(
        &repo_root.join("docs/features/session-management.md"),
        r#"
Unrelated machine event example:

```json
{
  "version": 3,
  "kind": "resume_decision",
  "timestamp": "2026-04-26T06:00:00Z"
}
```

Valid config preview example:

```json
{
  "version": 5,
  "resume_preview": {
    "status": "refusing_to_resume"
  }
}
```
"#,
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.sh"))
        .arg("docs")
        .current_dir(repo_root)
        .output()
        .expect("run public-readiness docs scan");

    assert_eq!(
        output.status.code(),
        Some(0),
        "docs scan should ignore unrelated version 3 JSON blocks outside the config preview example"
    );
}

#[test]
fn public_readiness_scan_docs_mode_rejects_quick_start_bare_run_loop() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_public_readiness_scan_fixture(repo_root);
    write_file(
        &repo_root.join("docs/quick-start.md"),
        r#"```bash
cueloop run one
cueloop run loop
```
"#,
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.sh"))
        .arg("docs")
        .current_dir(repo_root)
        .output()
        .expect("run public-readiness docs scan");

    assert_eq!(
        output.status.code(),
        Some(1),
        "docs scan should reject bare run-loop starter examples in quick start"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "docs/quick-start.md: quick start must use `cueloop run one` or a capped `run loop --max-tasks <N>` example"
        ),
        "docs scan should explain the quick-start run-loop failure\nstdout:\n{}",
        stdout
    );
}

#[test]
fn public_readiness_scan_docs_mode_rejects_cli_primary_unlimited_loop_example() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_public_readiness_scan_fixture(repo_root);
    write_file(
        &repo_root.join("docs/cli.md"),
        r#"### Create and Run

```bash
cueloop run one
cueloop run loop --max-tasks 0
```
"#,
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.sh"))
        .arg("docs")
        .current_dir(repo_root)
        .output()
        .expect("run public-readiness docs scan");

    assert_eq!(
        output.status.code(),
        Some(1),
        "docs scan should reject unlimited loop examples in the primary CLI starter block"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "docs/cli.md: CLI Create and Run examples must keep unlimited mode out of the primary starter block"
        ),
        "docs scan should explain the CLI starter-block failure\nstdout:\n{}",
        stdout
    );
}

#[test]
fn public_readiness_scan_docs_mode_allows_cli_advanced_unlimited_loop_section() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_public_readiness_scan_fixture(repo_root);
    write_file(
        &repo_root.join("docs/cli.md"),
        r#"### Create and Run

```bash
cueloop run one
cueloop run loop --max-tasks 1
```

Safe default: use a positive cap.

Advanced unlimited mode:

```bash
cueloop run loop --max-tasks 0
```
"#,
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.sh"))
        .arg("docs")
        .current_dir(repo_root)
        .output()
        .expect("run public-readiness docs scan");

    assert_eq!(
        output.status.code(),
        Some(0),
        "docs scan should allow unlimited loop examples outside the primary starter block\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn public_readiness_scan_docs_mode_rejects_generated_readme_unlimited_default() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_public_readiness_scan_fixture(repo_root);
    write_file(
        &repo_root.join(".cueloop/README.md"),
        r#"- Run multiple tasks:
  - `cueloop run loop --max-tasks 0`
"#,
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.sh"))
        .arg("docs")
        .current_dir(repo_root)
        .output()
        .expect("run public-readiness docs scan");

    assert_eq!(
        output.status.code(),
        Some(1),
        "docs scan should reject generated runtime README unlimited defaults"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            ".cueloop/README.md: generated runtime README must use capped loop examples by default"
        ),
        "docs scan should explain the runtime README failure\nstdout:\n{}",
        stdout
    );
}
