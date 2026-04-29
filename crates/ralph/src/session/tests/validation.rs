//! Session validation tests.
//!
//! Purpose:
//! - Session validation tests.
//!
//! Responsibilities:
//! - Verify task/status matching, timeout classification, and corrupt-cache diagnostics.
//!
//! Scope:
//! - Validation and read-only check behavior only; resume decision execution lives separately.
//!
//! Usage:
//! - Compiled through `crate::session::tests`.
//!
//! Invariants/assumptions:
//! - Timeout assertions use deterministic clocks where practical.

use super::*;
use tempfile::TempDir;
use time::Duration;

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
    let queue = empty_queue();

    assert_eq!(
        check_session(temp_dir.path(), &queue, None).unwrap(),
        SessionValidationResult::NoSession
    );
}

#[test]
fn check_session_classifies_malformed_json_as_corrupt_cache() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    std::fs::write(session_path(&cache_dir), "{ definitely not valid json").unwrap();
    let queue = empty_queue();

    match check_session(&cache_dir, &queue, Some(24)).unwrap() {
        SessionValidationResult::CorruptCache(corruption) => {
            assert_eq!(corruption.path, session_path(&cache_dir));
            assert!(corruption.diagnostic.contains("parse session file"));
            assert!(!corruption.diagnostic.contains("definitely not valid json"));
        }
        other => panic!("expected corrupt cache, got {other:?}"),
    }
}

#[test]
fn check_session_classifies_session_path_directory_as_corrupt_cache() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(session_path(&cache_dir)).unwrap();
    let queue = empty_queue();

    match check_session(&cache_dir, &queue, Some(24)).unwrap() {
        SessionValidationResult::CorruptCache(corruption) => {
            assert_eq!(corruption.path, session_path(&cache_dir));
            assert!(corruption.diagnostic.contains("read session file"));
        }
        other => panic!("expected corrupt cache, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn check_session_classifies_uninspectable_session_path_as_corrupt_cache() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    std::fs::write(session_path(&cache_dir), "{}").unwrap();

    let original_mode = std::fs::metadata(&cache_dir).unwrap().permissions().mode();
    let mut locked_permissions = std::fs::metadata(&cache_dir).unwrap().permissions();
    locked_permissions.set_mode(0o000);
    std::fs::set_permissions(&cache_dir, locked_permissions).unwrap();

    let result = check_session(&cache_dir, &empty_queue(), Some(24));

    let mut restored_permissions = std::fs::metadata(temp_dir.path().join("cache"))
        .unwrap()
        .permissions();
    restored_permissions.set_mode(original_mode);
    std::fs::set_permissions(&cache_dir, restored_permissions).unwrap();

    match result.unwrap() {
        SessionValidationResult::CorruptCache(corruption) => {
            assert_eq!(corruption.path, session_path(&cache_dir));
            assert!(corruption.diagnostic.contains("inspect session file"));
        }
        other => panic!("expected corrupt cache, got {other:?}"),
    }
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
