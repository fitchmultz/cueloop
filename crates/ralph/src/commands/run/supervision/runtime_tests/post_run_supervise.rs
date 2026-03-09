//! Post-run supervision coverage for queue, archive, and git finalization behavior.
//!
//! Responsibilities:
//! - Validate task archival, commit/push behavior, and productivity-file handling.
//! - Keep dirty-vs-clean supervision regressions localized to post-run flows.
//!
//! Not handled here:
//! - Continue-session resume fallback logic.
//! - Parallel-worker bookkeeping restore.
//!
//! Invariants/assumptions:
//! - Tests run against disposable git repositories.
//! - Queue fixtures always archive `RQ-0001`.

use super::support::{resolved_for_repo, write_queue};
use crate::commands::run::supervision::{PushPolicy, post_run_supervise};
use crate::contracts::GitRevertMode;
use crate::contracts::TaskStatus;
use crate::queue;
use crate::testsupport::git as git_test;
use tempfile::TempDir;

#[test]
fn post_run_supervise_commits_and_cleans_when_enabled() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    git_test::init_repo(temp.path())?;
    write_queue(temp.path(), TaskStatus::Todo)?;
    git_test::commit_all(temp.path(), "init")?;
    std::fs::write(temp.path().join("work.txt"), "change")?;

    let resolved = resolved_for_repo(temp.path());
    post_run_supervise(
        &resolved,
        "RQ-0001",
        GitRevertMode::Disabled,
        true,
        PushPolicy::RequireUpstream,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
    )?;

    let status = git_test::git_output(temp.path(), &["status", "--porcelain"])?;
    anyhow::ensure!(status.trim().is_empty(), "expected clean repo");

    let done_file = queue::load_queue_or_default(&resolved.done_path)?;
    anyhow::ensure!(
        done_file.tasks.iter().any(|task| task.id == "RQ-0001"),
        "expected task in done archive"
    );
    Ok(())
}

#[test]
fn post_run_supervise_skips_commit_when_disabled() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    git_test::init_repo(temp.path())?;
    write_queue(temp.path(), TaskStatus::Todo)?;
    git_test::commit_all(temp.path(), "init")?;
    std::fs::write(temp.path().join("work.txt"), "change")?;

    let resolved = resolved_for_repo(temp.path());
    post_run_supervise(
        &resolved,
        "RQ-0001",
        GitRevertMode::Disabled,
        false,
        PushPolicy::RequireUpstream,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
    )?;

    let status = git_test::git_output(temp.path(), &["status", "--porcelain"])?;
    anyhow::ensure!(!status.trim().is_empty(), "expected dirty repo");
    Ok(())
}

#[test]
fn post_run_supervise_backfills_missing_completed_at() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    git_test::init_repo(temp.path())?;
    write_queue(temp.path(), TaskStatus::Done)?;
    git_test::commit_all(temp.path(), "init")?;

    let resolved = resolved_for_repo(temp.path());
    post_run_supervise(
        &resolved,
        "RQ-0001",
        GitRevertMode::Disabled,
        false,
        PushPolicy::RequireUpstream,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
    )?;

    let done_file = queue::load_queue_or_default(&resolved.done_path)?;
    let task = done_file
        .tasks
        .iter()
        .find(|task| task.id == "RQ-0001")
        .expect("expected task in done archive");
    let completed_at = task
        .completed_at
        .as_deref()
        .expect("completed_at should be stamped");

    crate::timeutil::parse_rfc3339(completed_at)?;
    Ok(())
}

#[test]
fn post_run_supervise_errors_on_push_failure_when_enabled() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    git_test::init_repo(temp.path())?;
    write_queue(temp.path(), TaskStatus::Todo)?;
    git_test::commit_all(temp.path(), "init")?;

    let remote = TempDir::new()?;
    git_test::git_run(remote.path(), &["init", "--bare"])?;
    let branch = git_test::git_output(temp.path(), &["rev-parse", "--abbrev-ref", "HEAD"])?;
    git_test::git_run(
        temp.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    )?;
    git_test::git_run(temp.path(), &["push", "-u", "origin", &branch])?;
    let missing_remote = temp.path().join("missing-remote");
    git_test::git_run(
        temp.path(),
        &[
            "remote",
            "set-url",
            "origin",
            missing_remote.to_str().unwrap(),
        ],
    )?;

    std::fs::write(temp.path().join("work.txt"), "change")?;

    let resolved = resolved_for_repo(temp.path());
    let err = post_run_supervise(
        &resolved,
        "RQ-0001",
        GitRevertMode::Disabled,
        true,
        PushPolicy::RequireUpstream,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
    )
    .expect_err("expected push failure");
    assert!(format!("{err:#}").contains("Git push failed"));
    Ok(())
}

#[test]
fn post_run_supervise_skips_push_when_disabled() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    git_test::init_repo(temp.path())?;
    write_queue(temp.path(), TaskStatus::Todo)?;
    git_test::commit_all(temp.path(), "init")?;

    let remote = TempDir::new()?;
    git_test::git_run(remote.path(), &["init", "--bare"])?;
    let branch = git_test::git_output(temp.path(), &["rev-parse", "--abbrev-ref", "HEAD"])?;
    git_test::git_run(
        temp.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    )?;
    git_test::git_run(temp.path(), &["push", "-u", "origin", &branch])?;
    let missing_remote = temp.path().join("missing-remote");
    git_test::git_run(
        temp.path(),
        &[
            "remote",
            "set-url",
            "origin",
            missing_remote.to_str().unwrap(),
        ],
    )?;

    std::fs::write(temp.path().join("work.txt"), "change")?;

    let resolved = resolved_for_repo(temp.path());
    post_run_supervise(
        &resolved,
        "RQ-0001",
        GitRevertMode::Disabled,
        false,
        PushPolicy::RequireUpstream,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
    )?;
    Ok(())
}

#[test]
fn post_run_supervise_allows_productivity_json_dirty() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    git_test::init_repo(temp.path())?;
    write_queue(temp.path(), TaskStatus::Done)?;
    git_test::commit_all(temp.path(), "init")?;

    let cache_dir = temp.path().join(".ralph").join("cache");
    std::fs::create_dir_all(&cache_dir)?;
    std::fs::write(
        cache_dir.join("productivity.json"),
        r#"{"version":1,"total_completed":1}"#,
    )?;
    std::fs::write(temp.path().join("work.txt"), "change")?;

    let resolved = resolved_for_repo(temp.path());
    post_run_supervise(
        &resolved,
        "RQ-0001",
        GitRevertMode::Disabled,
        true,
        PushPolicy::RequireUpstream,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
    )?;

    let done_file = queue::load_queue_or_default(&resolved.done_path)?;
    anyhow::ensure!(
        done_file.tasks.iter().any(|task| task.id == "RQ-0001"),
        "expected task in done archive"
    );

    let status = git_test::git_output(temp.path(), &["status", "--porcelain"])?;
    anyhow::ensure!(
        status.trim().is_empty(),
        "expected clean repo after commit, but found: {status}"
    );
    Ok(())
}
