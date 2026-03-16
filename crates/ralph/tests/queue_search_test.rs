//! Purpose: integration coverage for `ralph queue search` CLI behavior.
//!
//! Responsibilities:
//! - Verify substring, regex, and fuzzy search modes through the real CLI.
//! - Verify search filters across queue/done state, tags, scope, status, scheduling, and limits.
//! - Verify output modes and conflicting-flag failures.
//!
//! Scope:
//! - CLI-driven queue-search scenarios over disposable seeded repos.
//!
//! Usage:
//! - Each test creates a fresh `SearchRepo`, writes queue/done fixtures, and invokes `ralph queue search`.
//! - JSON assertions use the suite-local `search_json()` helper.
//!
//! Invariants/Assumptions:
//! - The suite preserves end-to-end CLI coverage and does not bypass the command implementation.
//! - Repo bootstrap uses cached git+`.ralph/` fixtures instead of repeating per-test init work.
//! - Search operates on `.ralph/queue.jsonc` and `.ralph/done.jsonc` within the disposable repo.

#[path = "queue_search_test/support.rs"]
mod support;
mod test_support;

use anyhow::Result;
use ralph::contracts::TaskStatus;
use support::SearchRepo;

#[test]
fn queue_search_substring_mode_finds_matches() -> Result<()> {
    let repo = SearchRepo::new()?;

    let mut t1 =
        test_support::make_test_task("RQ-0001", "Fix authentication bug", TaskStatus::Todo);
    t1.tags = vec!["auth".to_string()];

    let mut t2 = test_support::make_test_task("RQ-0002", "Update documentation", TaskStatus::Todo);
    t2.tags = vec!["docs".to_string()];

    repo.write_queue(&[t1, t2])?;

    let tasks = repo.search_json(&["queue", "search", "authentication", "--format", "json"])?;
    let arr = tasks.as_array().expect("expected JSON array");
    anyhow::ensure!(arr.len() == 1, "expected 1 result, got {}", arr.len());
    anyhow::ensure!(arr[0]["id"] == "RQ-0001", "expected RQ-0001");
    Ok(())
}

#[test]
fn queue_search_regex_mode_patterns_work() -> Result<()> {
    let repo = SearchRepo::new()?;
    let t1 = test_support::make_test_task("RQ-0001", "Fix RQ-1234 bug", TaskStatus::Todo);
    let t2 = test_support::make_test_task("RQ-0002", "Update docs", TaskStatus::Todo);
    repo.write_queue(&[t1, t2])?;

    let tasks = repo.search_json(&[
        "queue",
        "search",
        "RQ-\\d{4}",
        "--regex",
        "--format",
        "json",
    ])?;
    anyhow::ensure!(
        tasks.as_array().expect("array").len() == 1,
        "expected 1 regex match"
    );
    anyhow::ensure!(tasks[0]["id"] == "RQ-0001");
    Ok(())
}

#[test]
fn queue_search_fuzzy_mode_tolerance_works() -> Result<()> {
    let repo = SearchRepo::new()?;
    let t1 = test_support::make_test_task("RQ-0001", "Implement authentication", TaskStatus::Todo);
    repo.write_queue(&[t1])?;

    let tasks = repo.search_json(&[
        "queue",
        "search",
        "authntication",
        "--fuzzy",
        "--format",
        "json",
    ])?;
    anyhow::ensure!(
        tasks.as_array().expect("array").len() == 1,
        "expected fuzzy match"
    );
    Ok(())
}

#[test]
fn queue_search_case_sensitive_mode() -> Result<()> {
    let repo = SearchRepo::new()?;
    let t1 = test_support::make_test_task("RQ-0001", "Fix LOGIN Bug", TaskStatus::Todo);
    repo.write_queue(&[t1])?;

    let tasks = repo.search_json(&["queue", "search", "login", "--format", "json"])?;
    anyhow::ensure!(tasks.as_array().expect("array").len() == 1);

    let tasks = repo.search_json(&[
        "queue",
        "search",
        "login",
        "--match-case",
        "--format",
        "json",
    ])?;
    anyhow::ensure!(
        tasks.as_array().expect("array").is_empty(),
        "expected no case-sensitive match"
    );

    let tasks = repo.search_json(&[
        "queue",
        "search",
        "LOGIN",
        "--match-case",
        "--format",
        "json",
    ])?;
    anyhow::ensure!(tasks.as_array().expect("array").len() == 1);
    Ok(())
}

#[test]
fn queue_search_with_tag_filter() -> Result<()> {
    let repo = SearchRepo::new()?;

    let mut t1 = test_support::make_test_task("RQ-0001", "Fix bug", TaskStatus::Todo);
    t1.tags = vec!["rust".to_string()];

    let mut t2 = test_support::make_test_task("RQ-0002", "Fix bug", TaskStatus::Todo);
    t2.tags = vec!["python".to_string()];

    repo.write_queue(&[t1, t2])?;

    let tasks = repo.search_json(&[
        "queue", "search", "Fix", "--tag", "rust", "--format", "json",
    ])?;
    anyhow::ensure!(tasks.as_array().expect("array").len() == 1);
    anyhow::ensure!(tasks[0]["id"] == "RQ-0001");
    Ok(())
}

#[test]
fn queue_search_with_scope_filter() -> Result<()> {
    let repo = SearchRepo::new()?;

    let mut t1 = test_support::make_test_task("RQ-0001", "Fix bug", TaskStatus::Todo);
    t1.scope = vec!["crates/ralph".to_string()];

    let mut t2 = test_support::make_test_task("RQ-0002", "Fix bug", TaskStatus::Todo);
    t2.scope = vec!["apps/RalphMac".to_string()];

    repo.write_queue(&[t1, t2])?;

    let tasks = repo.search_json(&[
        "queue",
        "search",
        "Fix",
        "--scope",
        "crates/ralph",
        "--format",
        "json",
    ])?;
    anyhow::ensure!(tasks.as_array().expect("array").len() == 1);
    anyhow::ensure!(tasks[0]["id"] == "RQ-0001");
    Ok(())
}

#[test]
fn queue_search_with_status_filter() -> Result<()> {
    let repo = SearchRepo::new()?;
    let t1 = test_support::make_test_task("RQ-0001", "Fix bug", TaskStatus::Todo);
    let t2 = test_support::make_test_task("RQ-0002", "Fix bug", TaskStatus::Doing);
    repo.write_queue(&[t1, t2])?;

    let tasks = repo.search_json(&[
        "queue", "search", "Fix", "--status", "todo", "--format", "json",
    ])?;
    anyhow::ensure!(tasks.as_array().expect("array").len() == 1);
    anyhow::ensure!(tasks[0]["id"] == "RQ-0001");
    Ok(())
}

#[test]
fn queue_search_include_done_and_only_done() -> Result<()> {
    let repo = SearchRepo::new()?;

    let t1 = test_support::make_test_task("RQ-0001", "Fix auth bug", TaskStatus::Todo);
    let mut t2 = test_support::make_test_task("RQ-0002", "Fix auth bug", TaskStatus::Done);
    t2.completed_at = Some("2026-01-20T00:00:00Z".to_string());

    repo.write_queue(&[t1])?;
    repo.write_done(&[t2])?;

    let tasks = repo.search_json(&["queue", "search", "auth", "--format", "json"])?;
    anyhow::ensure!(tasks.as_array().expect("array").len() == 1);

    let tasks = repo.search_json(&[
        "queue",
        "search",
        "auth",
        "--include-done",
        "--format",
        "json",
    ])?;
    anyhow::ensure!(
        tasks.as_array().expect("array").len() == 2,
        "expected 2 with --include-done"
    );

    let tasks =
        repo.search_json(&["queue", "search", "auth", "--only-done", "--format", "json"])?;
    anyhow::ensure!(tasks.as_array().expect("array").len() == 1);
    anyhow::ensure!(tasks[0]["id"] == "RQ-0002");
    Ok(())
}

#[test]
fn queue_search_scheduled_filter() -> Result<()> {
    let repo = SearchRepo::new()?;

    let mut t1 = test_support::make_test_task("RQ-0001", "Fix bug", TaskStatus::Todo);
    t1.scheduled_start = Some("2026-02-20T00:00:00Z".to_string());
    let t2 = test_support::make_test_task("RQ-0002", "Fix bug", TaskStatus::Todo);
    repo.write_queue(&[t1, t2])?;

    let tasks = repo.search_json(&["queue", "search", "Fix", "--scheduled", "--format", "json"])?;
    anyhow::ensure!(tasks.as_array().expect("array").len() == 1);
    anyhow::ensure!(tasks[0]["id"] == "RQ-0001");
    Ok(())
}

#[test]
fn queue_search_output_formats() -> Result<()> {
    let repo = SearchRepo::new()?;
    let t1 = test_support::make_test_task("RQ-0001", "Test task", TaskStatus::Todo);
    repo.write_queue(&[t1])?;

    let (stdout, _stderr) = repo.search_ok(&["queue", "search", "Test"])?;
    anyhow::ensure!(stdout.contains("RQ-0001"));
    anyhow::ensure!(
        stdout.lines().count() <= 2,
        "compact should be brief, got {} lines",
        stdout.lines().count()
    );

    let (stdout, _stderr) = repo.search_ok(&["queue", "search", "Test", "--format", "long"])?;
    anyhow::ensure!(stdout.contains("RQ-0001"));

    let _: serde_json::Value =
        repo.search_json(&["queue", "search", "Test", "--format", "json"])?;
    Ok(())
}

#[test]
fn queue_search_limit_and_all() -> Result<()> {
    let repo = SearchRepo::new()?;
    let t1 = test_support::make_test_task("RQ-0001", "Fix bug A", TaskStatus::Todo);
    let t2 = test_support::make_test_task("RQ-0002", "Fix bug B", TaskStatus::Todo);
    let t3 = test_support::make_test_task("RQ-0003", "Fix bug C", TaskStatus::Todo);
    repo.write_queue(&[t1, t2, t3])?;

    let tasks = repo.search_json(&["queue", "search", "Fix", "--format", "json"])?;
    anyhow::ensure!(tasks.as_array().expect("array").len() == 3);

    let tasks =
        repo.search_json(&["queue", "search", "Fix", "--limit", "2", "--format", "json"])?;
    anyhow::ensure!(
        tasks.as_array().expect("array").len() == 2,
        "expected 2 with --limit 2"
    );

    let tasks = repo.search_json(&["queue", "search", "Fix", "--all", "--format", "json"])?;
    anyhow::ensure!(tasks.as_array().expect("array").len() == 3);
    Ok(())
}

#[test]
fn queue_search_conflicting_flags_errors() -> Result<()> {
    let repo = SearchRepo::new()?;
    let t1 = test_support::make_test_task("RQ-0001", "Test", TaskStatus::Todo);
    repo.write_queue(&[t1])?;

    let (status, _stdout, stderr) = repo.search(&["queue", "search", "Test", "--fuzzy", "--regex"]);
    anyhow::ensure!(!status.success(), "expected failure for conflicting flags");
    anyhow::ensure!(
        stderr.contains("Conflicting flags") || stderr.contains("mutually exclusive"),
        "expected conflict error message, got: {stderr}"
    );

    let (status, _stdout, stderr) =
        repo.search(&["queue", "search", "Test", "--include-done", "--only-done"]);
    anyhow::ensure!(!status.success(), "expected failure for conflicting flags");
    anyhow::ensure!(
        stderr.contains("Conflicting flags") || stderr.contains("mutually exclusive"),
        "expected conflict error message, got: {stderr}"
    );
    Ok(())
}
