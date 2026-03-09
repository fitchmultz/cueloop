//! Session module tests.
//!
//! Responsibilities:
//! - Verify persistence, validation, progress tracking, and non-interactive recovery behavior.
//!
//! Not handled here:
//! - Full run-loop integration.
//!
//! Invariants/assumptions:
//! - Time-sensitive validation uses fixed timestamps where practical.

use super::*;
use crate::contracts::{QueueFile, SessionState, Task, TaskPriority, TaskStatus};
use crate::testsupport::git as git_test;
use crate::timeutil;
use tempfile::TempDir;
use time::Duration;

fn test_task(id: &str, status: TaskStatus) -> Task {
    Task {
        id: id.to_string(),
        status,
        title: "Test".to_string(),
        description: None,
        priority: TaskPriority::Medium,
        tags: vec![],
        scope: vec![],
        evidence: vec![],
        plan: vec![],
        notes: vec![],
        request: None,
        agent: None,
        created_at: None,
        updated_at: None,
        completed_at: None,
        started_at: None,
        scheduled_start: None,
        depends_on: vec![],
        blocks: vec![],
        relates_to: vec![],
        duplicates: None,
        custom_fields: Default::default(),
        parent_id: None,
        estimated_minutes: None,
        actual_minutes: None,
    }
}

const TEST_NOW: &str = "2026-02-07T12:00:00.000000000Z";

fn test_now() -> time::OffsetDateTime {
    timeutil::parse_rfc3339(TEST_NOW).unwrap()
}

fn test_session_with_time(task_id: &str, last_updated_at: &str) -> SessionState {
    SessionState::new(
        "test-session-id".to_string(),
        task_id.to_string(),
        last_updated_at.to_string(),
        1,
        crate::contracts::Runner::Claude,
        "sonnet".to_string(),
        0,
        None,
        None,
    )
}

fn test_session(task_id: &str) -> SessionState {
    test_session_with_time(task_id, TEST_NOW)
}

#[test]
fn get_git_head_commit_returns_current_head() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    git_test::init_repo(temp_dir.path())?;
    std::fs::write(temp_dir.path().join("README.md"), "session commit")?;
    git_test::commit_all(temp_dir.path(), "init")?;

    let commit = get_git_head_commit(temp_dir.path());
    let expected = git_test::git_output(temp_dir.path(), &["rev-parse", "HEAD"])?;

    assert_eq!(commit.as_deref(), Some(expected.as_str()));
    Ok(())
}

#[test]
fn save_and_load_session_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let session = test_session("RQ-0001");

    save_session(temp_dir.path(), &session).unwrap();
    let loaded = load_session(temp_dir.path()).unwrap().unwrap();

    assert_eq!(loaded.session_id, session.session_id);
    assert_eq!(loaded.task_id, session.task_id);
    assert_eq!(loaded.iterations_planned, session.iterations_planned);
}

#[test]
fn clear_session_removes_file() {
    let temp_dir = TempDir::new().unwrap();
    let session = test_session("RQ-0001");

    save_session(temp_dir.path(), &session).unwrap();
    assert!(session_exists(temp_dir.path()));

    clear_session(temp_dir.path()).unwrap();
    assert!(!session_exists(temp_dir.path()));
}

#[test]
fn validate_session_valid_when_task_doing() {
    let session = test_session("RQ-0001");
    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    assert!(matches!(
        validate_session(&session, &queue, None),
        SessionValidationResult::Valid(_)
    ));
}

#[test]
fn validate_session_stale_when_task_not_doing() {
    let session = test_session("RQ-0001");
    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Todo)],
    };

    assert!(matches!(
        validate_session(&session, &queue, None),
        SessionValidationResult::Stale { .. }
    ));
}

#[test]
fn validate_session_stale_when_task_missing() {
    let session = test_session("RQ-0001");
    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0002", TaskStatus::Doing)],
    };

    assert!(matches!(
        validate_session(&session, &queue, None),
        SessionValidationResult::Stale { .. }
    ));
}

#[test]
fn check_session_returns_no_session_when_file_missing() {
    let temp_dir = TempDir::new().unwrap();
    let queue = QueueFile {
        version: 1,
        tasks: vec![],
    };

    assert_eq!(
        check_session(temp_dir.path(), &queue, None).unwrap(),
        SessionValidationResult::NoSession
    );
}

#[test]
fn session_path_returns_correct_path() {
    let temp_dir = TempDir::new().unwrap();
    assert_eq!(
        session_path(temp_dir.path()),
        temp_dir.path().join("session.jsonc")
    );
}

#[test]
fn prompt_session_recovery_returns_false_when_non_interactive() {
    let session = test_session("RQ-0001");
    assert!(!prompt_session_recovery(&session, true).unwrap());
}

#[test]
fn prompt_session_recovery_timeout_returns_false_when_non_interactive() {
    let session = test_session("RQ-0001");
    assert!(!prompt_session_recovery_timeout(&session, 48, 24, true).unwrap());
}

#[test]
fn validate_session_returns_timeout_when_older_than_threshold() {
    let now = test_now();
    let session_time = now - Duration::hours(48);
    let session =
        test_session_with_time("RQ-0001", &timeutil::format_rfc3339(session_time).unwrap());
    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    match validate_session_with_now(&session, &queue, Some(24), now) {
        SessionValidationResult::Timeout {
            hours,
            session: timed_out,
        } => {
            assert_eq!(hours, 48);
            assert_eq!(timed_out.task_id, session.task_id);
            assert_eq!(timed_out.session_id, session.session_id);
        }
        other => panic!("expected Timeout, got {other:?}"),
    }
}

#[test]
fn check_session_returns_timeout_and_includes_loaded_session() {
    let temp_dir = TempDir::new().unwrap();
    let session_time = time::OffsetDateTime::now_utc() - Duration::days(365);
    let session =
        test_session_with_time("RQ-0001", &timeutil::format_rfc3339(session_time).unwrap());
    save_session(temp_dir.path(), &session).unwrap();

    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    match check_session(temp_dir.path(), &queue, Some(24)).unwrap() {
        SessionValidationResult::Timeout {
            hours,
            session: timed_out,
        } => {
            assert!(hours >= 24);
            assert_eq!(timed_out.task_id, session.task_id);
            assert_eq!(timed_out.session_id, session.session_id);
            assert_eq!(timed_out.last_updated_at, session.last_updated_at);
        }
        other => panic!("expected Timeout, got {other:?}"),
    }
}

#[test]
fn validate_session_returns_valid_when_within_custom_threshold() {
    let now = test_now();
    let session_time = now - Duration::hours(12);
    let session =
        test_session_with_time("RQ-0001", &timeutil::format_rfc3339(session_time).unwrap());
    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    assert!(matches!(
        validate_session_with_now(&session, &queue, Some(48), now),
        SessionValidationResult::Valid(_)
    ));
}

#[test]
fn validate_session_returns_valid_when_within_default_threshold() {
    let now = test_now();
    let session_time = now - Duration::hours(1);
    let session =
        test_session_with_time("RQ-0001", &timeutil::format_rfc3339(session_time).unwrap());
    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    assert!(matches!(
        validate_session_with_now(&session, &queue, Some(24), now),
        SessionValidationResult::Valid(_)
    ));
}

#[test]
fn validate_session_returns_valid_when_no_timeout_configured() {
    let session = test_session("RQ-0001");
    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    assert!(matches!(
        validate_session(&session, &queue, None),
        SessionValidationResult::Valid(_)
    ));
}

#[test]
fn validate_session_invalid_last_updated_does_not_timeout() {
    let session = test_session_with_time("RQ-0001", "not-a-valid-timestamp");
    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    assert!(matches!(
        validate_session_with_now(&session, &queue, Some(1), test_now()),
        SessionValidationResult::Valid(_)
    ));
}

#[test]
fn validate_session_exact_boundary_returns_timeout() {
    let now = test_now();
    let session_time = now - Duration::hours(24);
    let session =
        test_session_with_time("RQ-0001", &timeutil::format_rfc3339(session_time).unwrap());
    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    assert!(matches!(
        validate_session_with_now(&session, &queue, Some(24), now),
        SessionValidationResult::Timeout { .. }
    ));
}

#[test]
fn validate_session_future_timestamp_no_timeout() {
    let now = test_now();
    let session_time = now + Duration::hours(1);
    let session =
        test_session_with_time("RQ-0001", &timeutil::format_rfc3339(session_time).unwrap());
    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    assert!(matches!(
        validate_session_with_now(&session, &queue, Some(1), now),
        SessionValidationResult::Valid(_)
    ));
}

#[test]
fn increment_session_progress_updates_and_persists() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let session = test_session("RQ-0001");
    save_session(&cache_dir, &session).unwrap();
    assert_eq!(session.tasks_completed_in_loop, 0);

    increment_session_progress(&cache_dir).unwrap();
    let loaded = load_session(&cache_dir).unwrap().unwrap();
    assert_eq!(loaded.tasks_completed_in_loop, 1);

    increment_session_progress(&cache_dir).unwrap();
    let loaded = load_session(&cache_dir).unwrap().unwrap();
    assert_eq!(loaded.tasks_completed_in_loop, 2);
}

#[test]
fn increment_session_progress_handles_missing_session() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    increment_session_progress(&cache_dir).unwrap();
}
