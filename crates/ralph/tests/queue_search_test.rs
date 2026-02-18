//! Integration tests for `ralph queue search`.
//!
//! Responsibilities:
//! - Verify substring, regex, and fuzzy search modes work via CLI
//! - Verify filter combinations (status, tag, scope, scheduled)
//! - Verify include-done and only-done archive search
//! - Verify output formats (compact, long, json)
//! - Verify result limiting and --all flag
//! - Verify error handling for conflicting flags
//!
//! Not handled here:
//! - Unit tests for search internals (covered by src/queue/search/*.rs)
//! - Performance benchmarks
//!
//! Invariants/assumptions:
//! - `ralph init --force --non-interactive` creates usable .ralph/ structure
//! - Search operates on .ralph/queue.json and .ralph/done.json

use anyhow::Result;
use ralph::contracts::TaskStatus;

mod test_support;

#[test]
fn queue_search_substring_mode_finds_matches() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let mut t1 =
        test_support::make_test_task("RQ-0001", "Fix authentication bug", TaskStatus::Todo);
    t1.tags = vec!["auth".to_string()];

    let mut t2 = test_support::make_test_task("RQ-0002", "Update documentation", TaskStatus::Todo);
    t2.tags = vec!["docs".to_string()];

    test_support::write_queue(dir.path(), &[t1, t2])?;

    let (status, stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &["queue", "search", "authentication", "--format", "json"],
    );
    anyhow::ensure!(
        status.success(),
        "search failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Parse JSON output and verify only RQ-0001 is returned
    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.is_array(), "expected JSON array");
    let arr = tasks.as_array().unwrap();
    anyhow::ensure!(arr.len() == 1, "expected 1 result, got {}", arr.len());
    anyhow::ensure!(arr[0]["id"] == "RQ-0001", "expected RQ-0001");

    Ok(())
}

#[test]
fn queue_search_regex_mode_patterns_work() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let t1 = test_support::make_test_task("RQ-0001", "Fix RQ-1234 bug", TaskStatus::Todo);
    let t2 = test_support::make_test_task("RQ-0002", "Update docs", TaskStatus::Todo);

    test_support::write_queue(dir.path(), &[t1, t2])?;

    let (status, stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "queue",
            "search",
            "RQ-\\d{4}",
            "--regex",
            "--format",
            "json",
        ],
    );
    anyhow::ensure!(
        status.success(),
        "search failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(
        tasks.as_array().unwrap().len() == 1,
        "expected 1 regex match"
    );
    anyhow::ensure!(tasks[0]["id"] == "RQ-0001");

    Ok(())
}

#[test]
fn queue_search_fuzzy_mode_tolerance_works() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let t1 = test_support::make_test_task("RQ-0001", "Implement authentication", TaskStatus::Todo);
    test_support::write_queue(dir.path(), &[t1])?;

    // Typo: "authntication" should still match "authentication" with fuzzy
    let (status, stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "queue",
            "search",
            "authntication",
            "--fuzzy",
            "--format",
            "json",
        ],
    );
    anyhow::ensure!(
        status.success(),
        "search failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.as_array().unwrap().len() == 1, "expected fuzzy match");

    Ok(())
}

#[test]
fn queue_search_case_sensitive_mode() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let t1 = test_support::make_test_task("RQ-0001", "Fix LOGIN Bug", TaskStatus::Todo);
    test_support::write_queue(dir.path(), &[t1])?;

    // Case-insensitive (default): should match
    let (status, stdout, _) = test_support::run_in_dir(
        dir.path(),
        &["queue", "search", "login", "--format", "json"],
    );
    anyhow::ensure!(status.success());
    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.as_array().unwrap().len() == 1);

    // Case-sensitive: should NOT match
    let (status, stdout, _) = test_support::run_in_dir(
        dir.path(),
        &[
            "queue",
            "search",
            "login",
            "--match-case",
            "--format",
            "json",
        ],
    );
    anyhow::ensure!(status.success());
    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(
        tasks.as_array().unwrap().is_empty(),
        "expected no case-sensitive match"
    );

    // Case-sensitive with correct case: should match
    let (status, stdout, _) = test_support::run_in_dir(
        dir.path(),
        &[
            "queue",
            "search",
            "LOGIN",
            "--match-case",
            "--format",
            "json",
        ],
    );
    anyhow::ensure!(status.success());
    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.as_array().unwrap().len() == 1);

    Ok(())
}

#[test]
fn queue_search_with_tag_filter() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let mut t1 = test_support::make_test_task("RQ-0001", "Fix bug", TaskStatus::Todo);
    t1.tags = vec!["rust".to_string()];

    let mut t2 = test_support::make_test_task("RQ-0002", "Fix bug", TaskStatus::Todo);
    t2.tags = vec!["python".to_string()];

    test_support::write_queue(dir.path(), &[t1, t2])?;

    let (status, stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "queue", "search", "Fix", "--tag", "rust", "--format", "json",
        ],
    );
    anyhow::ensure!(
        status.success(),
        "search failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.as_array().unwrap().len() == 1);
    anyhow::ensure!(tasks[0]["id"] == "RQ-0001");

    Ok(())
}

#[test]
fn queue_search_with_scope_filter() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let mut t1 = test_support::make_test_task("RQ-0001", "Fix bug", TaskStatus::Todo);
    t1.scope = vec!["crates/ralph".to_string()];

    let mut t2 = test_support::make_test_task("RQ-0002", "Fix bug", TaskStatus::Todo);
    t2.scope = vec!["apps/RalphMac".to_string()];

    test_support::write_queue(dir.path(), &[t1, t2])?;

    let (status, stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "queue",
            "search",
            "Fix",
            "--scope",
            "crates/ralph",
            "--format",
            "json",
        ],
    );
    anyhow::ensure!(
        status.success(),
        "search failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.as_array().unwrap().len() == 1);
    anyhow::ensure!(tasks[0]["id"] == "RQ-0001");

    Ok(())
}

#[test]
fn queue_search_with_status_filter() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let t1 = test_support::make_test_task("RQ-0001", "Fix bug", TaskStatus::Todo);
    let t2 = test_support::make_test_task("RQ-0002", "Fix bug", TaskStatus::Doing);

    test_support::write_queue(dir.path(), &[t1, t2])?;

    let (status, stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "queue", "search", "Fix", "--status", "todo", "--format", "json",
        ],
    );
    anyhow::ensure!(
        status.success(),
        "search failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.as_array().unwrap().len() == 1);
    anyhow::ensure!(tasks[0]["id"] == "RQ-0001");

    Ok(())
}

#[test]
fn queue_search_include_done_and_only_done() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let t1 = test_support::make_test_task("RQ-0001", "Fix auth bug", TaskStatus::Todo);
    let mut t2 = test_support::make_test_task("RQ-0002", "Fix auth bug", TaskStatus::Done);
    t2.completed_at = Some("2026-01-20T00:00:00Z".to_string());

    test_support::write_queue(dir.path(), &[t1])?;
    test_support::write_done(dir.path(), &[t2])?;

    // Search only active queue: should find 1
    let (status, stdout, _) =
        test_support::run_in_dir(dir.path(), &["queue", "search", "auth", "--format", "json"]);
    anyhow::ensure!(status.success());
    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.as_array().unwrap().len() == 1);

    // Search with --include-done: should find 2
    let (status, stdout, _) = test_support::run_in_dir(
        dir.path(),
        &[
            "queue",
            "search",
            "auth",
            "--include-done",
            "--format",
            "json",
        ],
    );
    anyhow::ensure!(status.success());
    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(
        tasks.as_array().unwrap().len() == 2,
        "expected 2 with --include-done"
    );

    // Search with --only-done: should find 1
    let (status, stdout, _) = test_support::run_in_dir(
        dir.path(),
        &["queue", "search", "auth", "--only-done", "--format", "json"],
    );
    anyhow::ensure!(status.success());
    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.as_array().unwrap().len() == 1);
    anyhow::ensure!(tasks[0]["id"] == "RQ-0002");

    Ok(())
}

#[test]
fn queue_search_scheduled_filter() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let mut t1 = test_support::make_test_task("RQ-0001", "Fix bug", TaskStatus::Todo);
    t1.scheduled_start = Some("2026-02-20T00:00:00Z".to_string());

    let t2 = test_support::make_test_task("RQ-0002", "Fix bug", TaskStatus::Todo);

    test_support::write_queue(dir.path(), &[t1, t2])?;

    let (status, stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &["queue", "search", "Fix", "--scheduled", "--format", "json"],
    );
    anyhow::ensure!(
        status.success(),
        "search failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.as_array().unwrap().len() == 1);
    anyhow::ensure!(tasks[0]["id"] == "RQ-0001");

    Ok(())
}

#[test]
fn queue_search_output_formats() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let t1 = test_support::make_test_task("RQ-0001", "Test task", TaskStatus::Todo);
    test_support::write_queue(dir.path(), &[t1])?;

    // Compact format (default): single line with ID and title
    let (status, stdout, _) = test_support::run_in_dir(dir.path(), &["queue", "search", "Test"]);
    anyhow::ensure!(status.success());
    anyhow::ensure!(stdout.contains("RQ-0001"));
    let line_count = stdout.lines().count();
    anyhow::ensure!(
        line_count <= 2,
        "compact should be brief, got {} lines",
        line_count
    );

    // Long format: more details
    let (status, stdout, _) =
        test_support::run_in_dir(dir.path(), &["queue", "search", "Test", "--format", "long"]);
    anyhow::ensure!(status.success());
    anyhow::ensure!(stdout.contains("RQ-0001"));

    // JSON format: valid JSON array
    let (status, stdout, _) =
        test_support::run_in_dir(dir.path(), &["queue", "search", "Test", "--format", "json"]);
    anyhow::ensure!(status.success());
    let _: serde_json::Value = serde_json::from_str(&stdout)?;

    Ok(())
}

#[test]
fn queue_search_limit_and_all() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let t1 = test_support::make_test_task("RQ-0001", "Fix bug A", TaskStatus::Todo);
    let t2 = test_support::make_test_task("RQ-0002", "Fix bug B", TaskStatus::Todo);
    let t3 = test_support::make_test_task("RQ-0003", "Fix bug C", TaskStatus::Todo);

    test_support::write_queue(dir.path(), &[t1, t2, t3])?;

    // Default limit: should return up to 50
    let (status, stdout, _) =
        test_support::run_in_dir(dir.path(), &["queue", "search", "Fix", "--format", "json"]);
    anyhow::ensure!(status.success());
    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.as_array().unwrap().len() == 3);

    // Custom limit: should return only 2
    let (status, stdout, _) = test_support::run_in_dir(
        dir.path(),
        &["queue", "search", "Fix", "--limit", "2", "--format", "json"],
    );
    anyhow::ensure!(status.success());
    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(
        tasks.as_array().unwrap().len() == 2,
        "expected 2 with --limit 2"
    );

    // --all: should return all (same as default here, but verifies flag works)
    let (status, stdout, _) = test_support::run_in_dir(
        dir.path(),
        &["queue", "search", "Fix", "--all", "--format", "json"],
    );
    anyhow::ensure!(status.success());
    let tasks: serde_json::Value = serde_json::from_str(&stdout)?;
    anyhow::ensure!(tasks.as_array().unwrap().len() == 3);

    Ok(())
}

#[test]
fn queue_search_conflicting_flags_errors() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::ralph_init(dir.path())?;

    let t1 = test_support::make_test_task("RQ-0001", "Test", TaskStatus::Todo);
    test_support::write_queue(dir.path(), &[t1])?;

    // --fuzzy and --regex are mutually exclusive
    let (status, _stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &["queue", "search", "Test", "--fuzzy", "--regex"],
    );
    anyhow::ensure!(!status.success(), "expected failure for conflicting flags");
    anyhow::ensure!(
        stderr.contains("Conflicting flags") || stderr.contains("mutually exclusive"),
        "expected conflict error message, got: {stderr}"
    );

    // --include-done and --only-done are mutually exclusive
    let (status, _stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &["queue", "search", "Test", "--include-done", "--only-done"],
    );
    anyhow::ensure!(!status.success(), "expected failure for conflicting flags");
    anyhow::ensure!(
        stderr.contains("Conflicting flags") || stderr.contains("mutually exclusive"),
        "expected conflict error message, got: {stderr}"
    );

    Ok(())
}
