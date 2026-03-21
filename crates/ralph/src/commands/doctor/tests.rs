//! Tests for doctor types and blocking-state aggregation.
//!
//! Responsibilities:
//! - Unit tests for CheckResult factory methods.
//! - Tests for DoctorReport aggregation logic.
//! - Tests for canonical doctor blocking-state derivation.
//!
//! Not handled here:
//! - Integration tests for individual checks (see module tests).
//! - External system validation.
//!
//! Invariants/assumptions:
//! - Blocking-state derivation prefers hard stalls over softer waiting states.

use crate::commands::doctor::{
    derive_check_blocking_state,
    types::{CheckResult, CheckSeverity, DoctorReport},
};
use crate::contracts::{BlockingReason, BlockingState};

#[test]
fn check_result_success_factory() {
    let r = CheckResult::success("git", "binary", "git found");
    assert_eq!(r.category, "git");
    assert_eq!(r.check, "binary");
    assert_eq!(r.severity, CheckSeverity::Success);
    assert_eq!(r.message, "git found");
    assert!(!r.fix_available);
    assert!(r.fix_applied.is_none());
    assert!(r.blocking.is_none());
}

#[test]
fn check_result_warning_factory() {
    let r = CheckResult::warning(
        "queue",
        "orphaned",
        "found orphaned locks",
        true,
        Some("run repair"),
    );
    assert_eq!(r.severity, CheckSeverity::Warning);
    assert!(r.fix_available);
    assert_eq!(r.suggested_fix, Some("run repair".to_string()));
    assert!(r.blocking.is_none());
}

#[test]
fn check_result_error_factory() {
    let r = CheckResult::error("git", "repo", "not a git repo", false, Some("run git init"));
    assert_eq!(r.severity, CheckSeverity::Error);
    assert!(!r.fix_available);
    assert!(r.blocking.is_none());
}

#[test]
fn check_result_with_fix_applied() {
    let r = CheckResult::warning(
        "queue",
        "orphaned",
        "found orphaned locks",
        true,
        Some("run repair"),
    )
    .with_fix_applied(true);
    assert_eq!(r.fix_applied, Some(true));
}

#[test]
fn check_result_with_blocking() {
    let blocking = BlockingState::dependency_blocked(2);
    let result = CheckResult::error("queue", "queue_valid", "queue invalid", false, None)
        .with_blocking(blocking.clone());

    assert_eq!(result.blocking, Some(blocking));
}

#[test]
fn doctor_report_adds_checks() {
    let mut report = DoctorReport::new();
    assert!(report.success);
    assert!(report.blocking.is_none());

    report.add(CheckResult::success("git", "binary", "git found"));
    assert_eq!(report.summary.total, 1);
    assert_eq!(report.summary.passed, 1);
    assert!(report.success);

    report.add(CheckResult::warning(
        "queue",
        "orphaned",
        "found orphaned",
        true,
        None,
    ));
    assert_eq!(report.summary.warnings, 1);
    assert!(report.success);

    report.add(CheckResult::error(
        "git",
        "repo",
        "not a git repo",
        false,
        None,
    ));
    assert_eq!(report.summary.errors, 1);
    assert!(!report.success);
}

#[test]
fn doctor_report_tracks_fixes() {
    let mut report = DoctorReport::new();

    report
        .add(CheckResult::warning("queue", "orphaned", "found", true, None).with_fix_applied(true));
    assert_eq!(report.summary.fixes_applied, 1);

    report
        .add(CheckResult::warning("queue", "another", "found", true, None).with_fix_applied(false));
    assert_eq!(report.summary.fixes_failed, 1);
}

#[test]
fn doctor_report_serializes_blocking() {
    let mut report = DoctorReport::new();
    report.blocking = Some(BlockingState::idle(false));

    let json = serde_json::to_value(&report).expect("report serializes");
    assert_eq!(json["blocking"]["status"], "waiting");
    assert_eq!(json["blocking"]["reason"]["kind"], "idle");
}

#[test]
fn derive_check_blocking_state_prefers_lock_over_runner_recovery() {
    let runner = BlockingState::runner_recovery(
        "runner",
        "runner_binary_missing",
        None,
        "runner missing",
        "codex not found",
    );
    let lock = BlockingState::lock_blocked(
        Some("/tmp/.ralph/lock".to_string()),
        Some("ralph run loop".to_string()),
        Some(1234),
    );

    let checks = vec![
        CheckResult::error("runner", "runner_binary", &runner.message, false, None)
            .with_blocking(runner),
        CheckResult::error("lock", "queue_lock_held", &lock.message, false, None)
            .with_blocking(lock.clone()),
    ];

    assert_eq!(derive_check_blocking_state(&checks), Some(lock));
}

#[test]
fn derive_check_blocking_state_returns_none_when_no_blockers_exist() {
    let checks = vec![CheckResult::success("git", "binary", "git found")];
    assert!(derive_check_blocking_state(&checks).is_none());
}

#[test]
fn blocking_state_reason_round_trips_through_report_json() {
    let mut report = DoctorReport::new();
    report.blocking = Some(BlockingState::dependency_blocked(3));

    let json = serde_json::to_value(report).expect("report serializes");
    assert_eq!(json["blocking"]["reason"]["kind"], "dependency_blocked");
    assert_eq!(json["blocking"]["reason"]["blocked_tasks"], 3);

    let check_blocking =
        CheckResult::error("queue", "queue_valid", "broken", false, None).with_blocking(
            BlockingState::schedule_blocked(1, Some("2026-12-31T00:00:00Z".to_string()), Some(60)),
        );
    let check_json = serde_json::to_value(check_blocking).expect("check serializes");
    assert_eq!(check_json["blocking"]["reason"]["kind"], "schedule_blocked");
}

#[test]
fn dependency_blocked_reason_shape_matches_contract() {
    let blocking = BlockingState::dependency_blocked(4);
    assert!(matches!(
        blocking.reason,
        BlockingReason::DependencyBlocked { blocked_tasks: 4 }
    ));
}
