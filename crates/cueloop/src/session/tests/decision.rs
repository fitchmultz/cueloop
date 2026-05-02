//! Session resume decision tests.
//!
//! Purpose:
//! - Session resume decision tests.
//!
//! Responsibilities:
//! - Verify resume/fallback/refusal decisions and operator blocking-state projection.
//!
//! Scope:
//! - Decision modeling only; persistence and validation basics live in sibling modules.
//!
//! Usage:
//! - Compiled through `crate::session::tests`.
//!
//! Invariants/assumptions:
//! - Execute mode may quarantine or clear corrupt caches; preview mode must preserve them.

use super::*;
use tempfile::TempDir;
use time::Duration;

#[test]
fn resolve_run_session_decision_corrupt_json_falls_back_fresh_and_quarantines() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    std::fs::write(session_path(&cache_dir), "{ definitely not valid json").unwrap();
    let queue = empty_queue();

    let resolution = resolve_run_session_decision(
        &cache_dir,
        &queue,
        RunSessionDecisionOptions {
            timeout_hours: Some(24),
            behavior: ResumeBehavior::AutoResume,
            non_interactive: true,
            explicit_task_id: None,
            announce_missing_session: false,
            mode: ResumeDecisionMode::Execute,
        },
    )
    .unwrap();

    assert_eq!(resolution.resume_task_id, None);
    let decision = resolution.decision.expect("decision");
    assert_eq!(decision.status, ResumeStatus::FallingBackToFreshInvocation);
    assert_eq!(decision.reason, ResumeReason::SessionCacheCorrupt);
    assert!(decision.blocking_state().is_none());
    assert!(!session_exists(&cache_dir));
    let quarantine_dir = cache_dir.join("session-quarantine");
    assert!(quarantine_dir.exists());
    assert_eq!(std::fs::read_dir(quarantine_dir).unwrap().count(), 1);
}

#[test]
fn resolve_run_session_decision_corrupt_json_preview_refuses_and_preserves_cache() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let original_path = session_path(&cache_dir);
    std::fs::write(&original_path, "not-json").unwrap();
    let queue = empty_queue();

    let resolution = resolve_run_session_decision(
        &cache_dir,
        &queue,
        RunSessionDecisionOptions {
            timeout_hours: Some(24),
            behavior: ResumeBehavior::Prompt,
            non_interactive: true,
            explicit_task_id: None,
            announce_missing_session: false,
            mode: ResumeDecisionMode::Preview,
        },
    )
    .unwrap();

    let decision = resolution.decision.expect("decision");
    assert_eq!(decision.status, ResumeStatus::RefusingToResume);
    assert_eq!(decision.reason, ResumeReason::SessionCacheCorrupt);
    assert!(original_path.exists());
    assert!(!cache_dir.join("session-quarantine").exists());
}

#[test]
fn resolve_run_session_decision_corrupt_json_prompt_execute_refuses_and_quarantines() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    std::fs::write(session_path(&cache_dir), "not-json").unwrap();
    let queue = empty_queue();

    let resolution = resolve_run_session_decision(
        &cache_dir,
        &queue,
        RunSessionDecisionOptions {
            timeout_hours: Some(24),
            behavior: ResumeBehavior::Prompt,
            non_interactive: true,
            explicit_task_id: None,
            announce_missing_session: false,
            mode: ResumeDecisionMode::Execute,
        },
    )
    .unwrap();

    let decision = resolution.decision.expect("decision");
    assert_eq!(decision.status, ResumeStatus::RefusingToResume);
    assert_eq!(decision.reason, ResumeReason::SessionCacheCorrupt);
    assert!(!session_exists(&cache_dir));
    assert!(cache_dir.join("session-quarantine").exists());
}

#[test]
fn resume_decision_blocking_state_for_corrupt_cache() {
    let decision = ResumeDecision {
        status: ResumeStatus::RefusingToResume,
        scope: ResumeScope::RunSession,
        reason: ResumeReason::SessionCacheCorrupt,
        task_id: None,
        message:
            "Resume: refusing to guess because the saved session cache is corrupt or unreadable."
                .to_string(),
        detail: "Inspect .cueloop/cache/session.jsonc.".to_string(),
    };

    let blocking = decision.blocking_state().expect("blocking state");
    assert!(matches!(
        blocking.reason,
        crate::contracts::BlockingReason::RunnerRecovery { ref reason, .. }
            if reason == "session_cache_corrupt"
    ));
}

#[test]
fn resolve_run_session_decision_auto_resume_resumes_valid_session() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let mut session = test_session_with_time("RQ-0001", &timeutil::now_utc_rfc3339_or_fallback());
    session.current_phase = 2;
    session.tasks_completed_in_loop = 3;
    save_session(&cache_dir, &session).unwrap();

    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    let resolution = resolve_run_session_decision(
        &cache_dir,
        &queue,
        RunSessionDecisionOptions {
            timeout_hours: Some(24),
            behavior: ResumeBehavior::AutoResume,
            non_interactive: true,
            explicit_task_id: None,
            announce_missing_session: false,
            mode: ResumeDecisionMode::Execute,
        },
    )
    .unwrap();

    assert_eq!(resolution.resume_task_id.as_deref(), Some("RQ-0001"));
    assert_eq!(resolution.completed_count, 3);
    let decision = resolution.decision.expect("decision present");
    assert_eq!(decision.status, ResumeStatus::ResumingSameSession);
    assert_eq!(decision.reason, ResumeReason::SessionValid);
}

#[test]
fn resolve_run_session_decision_marks_stale_session_as_fresh_start() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let session = test_session("RQ-0001");
    save_session(&cache_dir, &session).unwrap();

    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Done)],
    };

    let resolution = resolve_run_session_decision(
        &cache_dir,
        &queue,
        RunSessionDecisionOptions {
            timeout_hours: Some(24),
            behavior: ResumeBehavior::AutoResume,
            non_interactive: true,
            explicit_task_id: None,
            announce_missing_session: false,
            mode: ResumeDecisionMode::Execute,
        },
    )
    .unwrap();

    assert_eq!(resolution.resume_task_id, None);
    let decision = resolution.decision.expect("decision present");
    assert_eq!(decision.status, ResumeStatus::FallingBackToFreshInvocation);
    assert_eq!(decision.reason, ResumeReason::SessionStale);
    assert!(!session_exists(&cache_dir));
}

#[test]
fn resolve_run_session_decision_hides_missing_session_when_not_requested() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let queue = empty_queue();

    let resolution = resolve_run_session_decision(
        &cache_dir,
        &queue,
        RunSessionDecisionOptions {
            timeout_hours: Some(24),
            behavior: ResumeBehavior::AutoResume,
            non_interactive: true,
            explicit_task_id: None,
            announce_missing_session: false,
            mode: ResumeDecisionMode::Execute,
        },
    )
    .unwrap();

    assert_eq!(resolution.resume_task_id, None);
    assert!(resolution.decision.is_none());
}

#[test]
fn resolve_run_session_decision_preview_stale_session_preserves_cache() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let session = test_session("RQ-0001");
    save_session(&cache_dir, &session).unwrap();

    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Done)],
    };

    let resolution = resolve_run_session_decision(
        &cache_dir,
        &queue,
        RunSessionDecisionOptions {
            timeout_hours: Some(24),
            behavior: ResumeBehavior::AutoResume,
            non_interactive: true,
            explicit_task_id: None,
            announce_missing_session: false,
            mode: ResumeDecisionMode::Preview,
        },
    )
    .unwrap();

    assert_eq!(resolution.resume_task_id, None);
    assert_eq!(
        resolution.decision.expect("decision").reason,
        ResumeReason::SessionStale
    );
    assert!(session_exists(&cache_dir));
}

#[test]
fn resolve_run_session_decision_timed_out_noninteractive_refusal_keeps_cache() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let stale_time = time::OffsetDateTime::now_utc() - Duration::hours(72);
    let session = test_session_with_time("RQ-0001", &timeutil::format_rfc3339(stale_time).unwrap());
    save_session(&cache_dir, &session).unwrap();

    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    let resolution = resolve_run_session_decision(
        &cache_dir,
        &queue,
        RunSessionDecisionOptions {
            timeout_hours: Some(24),
            behavior: ResumeBehavior::Prompt,
            non_interactive: true,
            explicit_task_id: None,
            announce_missing_session: false,
            mode: ResumeDecisionMode::Execute,
        },
    )
    .unwrap();

    assert_eq!(resolution.resume_task_id, None);
    let decision = resolution.decision.expect("decision");
    assert_eq!(decision.status, ResumeStatus::RefusingToResume);
    assert_eq!(
        decision.reason,
        ResumeReason::SessionTimedOutRequiresConfirmation
    );
    assert!(session_exists(&cache_dir));
}

#[test]
fn resolve_run_session_decision_refuses_prompt_required_noninteractive_resume() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let session = test_session_with_time("RQ-0001", &timeutil::now_utc_rfc3339_or_fallback());
    save_session(&cache_dir, &session).unwrap();

    let queue = QueueFile {
        version: 1,
        tasks: vec![test_task("RQ-0001", TaskStatus::Doing)],
    };

    let resolution = resolve_run_session_decision(
        &cache_dir,
        &queue,
        RunSessionDecisionOptions {
            timeout_hours: Some(24),
            behavior: ResumeBehavior::Prompt,
            non_interactive: true,
            explicit_task_id: None,
            announce_missing_session: false,
            mode: ResumeDecisionMode::Execute,
        },
    )
    .unwrap();

    assert_eq!(resolution.resume_task_id, None);
    let decision = resolution.decision.expect("decision present");
    assert_eq!(decision.status, ResumeStatus::RefusingToResume);
    assert_eq!(decision.reason, ResumeReason::ResumeConfirmationRequired);
    assert!(session_exists(&cache_dir));
}

#[test]
fn resolve_run_session_decision_explicit_task_overrides_unrelated_session() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let session = test_session_with_time("RQ-0001", &timeutil::now_utc_rfc3339_or_fallback());
    save_session(&cache_dir, &session).unwrap();

    let queue = QueueFile {
        version: 1,
        tasks: vec![
            test_task("RQ-0001", TaskStatus::Doing),
            test_task("RQ-0002", TaskStatus::Todo),
        ],
    };

    let resolution = resolve_run_session_decision(
        &cache_dir,
        &queue,
        RunSessionDecisionOptions {
            timeout_hours: Some(24),
            behavior: ResumeBehavior::AutoResume,
            non_interactive: true,
            explicit_task_id: Some("RQ-0002"),
            announce_missing_session: false,
            mode: ResumeDecisionMode::Execute,
        },
    )
    .unwrap();

    assert_eq!(resolution.resume_task_id, None);
    let decision = resolution.decision.expect("decision present");
    assert_eq!(
        decision.reason,
        ResumeReason::ExplicitTaskSelectionOverridesSession
    );
}

#[test]
fn resume_decision_blocking_state_for_confirmation_required() {
    let decision = ResumeDecision {
        status: ResumeStatus::RefusingToResume,
        scope: ResumeScope::RunSession,
        reason: ResumeReason::ResumeConfirmationRequired,
        task_id: Some("RQ-0007".to_string()),
        message: "Resume: refusing to guess.".to_string(),
        detail: "Confirmation is unavailable.".to_string(),
    };

    let blocking = decision.blocking_state().expect("blocking state");
    assert_eq!(blocking.task_id.as_deref(), Some("RQ-0007"));
    assert!(matches!(
        blocking.reason,
        crate::contracts::BlockingReason::RunnerRecovery { .. }
    ));
}

#[test]
fn resume_decision_without_recovery_blocker_has_no_blocking_state() {
    let decision = ResumeDecision {
        status: ResumeStatus::FallingBackToFreshInvocation,
        scope: ResumeScope::RunSession,
        reason: ResumeReason::SessionStale,
        task_id: None,
        message: "Resume: starting fresh because the saved session is stale.".to_string(),
        detail: "The session no longer matches the queue.".to_string(),
    };

    assert!(decision.blocking_state().is_none());
}
