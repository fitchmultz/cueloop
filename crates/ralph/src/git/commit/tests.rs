//! Commit-module regression tests.
//!
//! Purpose:
//! - Verify rebase-aware push and upstream comparison helpers keep their contracts.
//!
//! Responsibilities:
//! - Cover non-fast-forward recovery, upstream tracking, and behind detection.
//!
//! Scope:
//! - Unit tests for `crate::git::commit` internals only.
//!
//! Usage:
//! - Compiled with `cargo test` when the commit module is exercised.
//!
//! Invariants/assumptions:
//! - Tests operate on temporary repositories and local bare remotes only.
//! - Upstream comparisons rely on real git command behavior, not mocks.

use tempfile::TempDir;

use super::*;
use crate::testsupport::git as git_test;

#[test]
fn push_upstream_with_rebase_recovers_from_non_fast_forward() -> anyhow::Result<()> {
    let remote = TempDir::new()?;
    git_test::init_bare_repo(remote.path())?;

    let repo_a = TempDir::new()?;
    git_test::init_repo(repo_a.path())?;
    git_test::add_remote(repo_a.path(), "origin", remote.path())?;

    std::fs::write(repo_a.path().join("base.txt"), "init\n")?;
    git_test::commit_all(repo_a.path(), "init")?;
    git_test::git_run(repo_a.path(), &["push", "-u", "origin", "HEAD"])?;

    let repo_b = TempDir::new()?;
    git_test::clone_repo(remote.path(), repo_b.path())?;
    git_test::configure_user(repo_b.path())?;
    std::fs::write(repo_b.path().join("remote.txt"), "remote\n")?;
    git_test::commit_all(repo_b.path(), "remote update")?;
    git_test::git_run(repo_b.path(), &["push"])?;

    std::fs::write(repo_a.path().join("local.txt"), "local\n")?;
    git_test::commit_all(repo_a.path(), "local update")?;

    push_upstream_with_rebase(repo_a.path())?;

    let counts = git_test::git_output(
        repo_a.path(),
        &["rev-list", "--left-right", "--count", "@{u}...HEAD"],
    )?;
    let parts: Vec<&str> = counts.split_whitespace().collect();
    assert_eq!(parts, vec!["0", "0"]);

    Ok(())
}

#[test]
fn push_upstream_with_rebase_recovers_no_upstream_branch_behind_remote() -> anyhow::Result<()> {
    let remote = TempDir::new()?;
    git_test::init_bare_repo(remote.path())?;

    let seed = TempDir::new()?;
    git_test::init_repo(seed.path())?;
    git_test::add_remote(seed.path(), "origin", remote.path())?;
    std::fs::write(seed.path().join("base.txt"), "base\n")?;
    git_test::commit_all(seed.path(), "init")?;
    git_test::git_run(seed.path(), &["push", "-u", "origin", "HEAD"])?;
    git_test::git_run(seed.path(), &["checkout", "-b", "ralph/RQ-0940"])?;
    std::fs::write(seed.path().join("task.txt"), "remote-only\n")?;
    git_test::commit_all(seed.path(), "remote task")?;
    git_test::git_run(seed.path(), &["push", "-u", "origin", "ralph/RQ-0940"])?;

    let local = TempDir::new()?;
    git_test::clone_repo(remote.path(), local.path())?;
    git_test::configure_user(local.path())?;
    git_test::git_run(
        local.path(),
        &[
            "checkout",
            "--no-track",
            "-b",
            "ralph/RQ-0940",
            "origin/main",
        ],
    )?;
    git_test::git_run(local.path(), &["fetch", "origin", "ralph/RQ-0940"])?;

    assert!(git_test::git_run(local.path(), &["rev-parse", "--abbrev-ref", "@{u}"]).is_err());
    let before_counts = git_test::git_output(
        local.path(),
        &[
            "rev-list",
            "--left-right",
            "--count",
            "origin/ralph/RQ-0940...HEAD",
        ],
    )?;
    assert_eq!(
        before_counts.split_whitespace().collect::<Vec<_>>(),
        vec!["1", "0"]
    );

    push_upstream_with_rebase(local.path())?;

    let upstream = git_test::git_output(
        local.path(),
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )?;
    assert_eq!(upstream, "origin/ralph/RQ-0940");

    let after_counts = git_test::git_output(
        local.path(),
        &["rev-list", "--left-right", "--count", "@{u}...HEAD"],
    )?;
    assert_eq!(
        after_counts.split_whitespace().collect::<Vec<_>>(),
        vec!["0", "0"]
    );
    assert_eq!(
        std::fs::read_to_string(local.path().join("task.txt"))?,
        "remote-only\n"
    );

    Ok(())
}

#[test]
fn is_behind_upstream_detects_remote_only_commits() -> anyhow::Result<()> {
    let remote = TempDir::new()?;
    git_test::init_bare_repo(remote.path())?;

    let seed = TempDir::new()?;
    git_test::init_repo(seed.path())?;
    git_test::add_remote(seed.path(), "origin", remote.path())?;
    std::fs::write(seed.path().join("base.txt"), "base\n")?;
    git_test::commit_all(seed.path(), "init")?;
    git_test::git_run(seed.path(), &["push", "-u", "origin", "HEAD"])?;

    let local = TempDir::new()?;
    git_test::clone_repo(remote.path(), local.path())?;
    git_test::configure_user(local.path())?;

    std::fs::write(seed.path().join("remote.txt"), "remote only\n")?;
    git_test::commit_all(seed.path(), "remote ahead")?;
    git_test::git_run(seed.path(), &["push"])?;

    assert!(is_behind_upstream(local.path(), "main")?);
    Ok(())
}
