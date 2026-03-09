//! Lock unit tests.
//!
//! Responsibilities:
//! - Cover split lock helpers that are easiest to exercise without integration harnesses.
//!
//! Not handled here:
//! - Multi-process integration coverage in `crates/ralph/tests/`.
//!
//! Invariants/assumptions:
//! - Current-process PID should be observable on supported platforms.

use super::*;

#[test]
fn pid_is_running_current_process() {
    let current_pid = std::process::id();
    assert_eq!(pid_is_running(current_pid), Some(true));
}

#[test]
fn pid_is_running_nonexistent_pid_never_reports_running() {
    assert_ne!(pid_is_running(0xFFFF_FFFE), Some(true));
}

#[test]
fn pid_is_running_system_idle_is_not_definitively_dead() {
    assert_ne!(pid_is_running(0), Some(false));
}

#[test]
fn is_task_owner_file_matches_expected_patterns() {
    assert!(is_task_owner_file("owner_task_1234"));
    assert!(is_task_owner_file("owner_task_1234_0"));
    assert!(is_task_owner_file("owner_task_1234_42"));
    assert!(!is_task_owner_file("owner"));
    assert!(!is_task_owner_file("owner_other"));
    assert!(!is_task_owner_file("owner_task"));
    assert!(!is_task_owner_file(""));
    assert!(!is_task_owner_file("task_owner_1234"));
}

#[test]
fn pid_liveness_helpers_are_consistent() {
    assert!(PidLiveness::NotRunning.is_definitely_not_running());
    assert!(!PidLiveness::Running.is_definitely_not_running());
    assert!(!PidLiveness::Indeterminate.is_definitely_not_running());

    assert!(PidLiveness::Running.is_running_or_indeterminate());
    assert!(PidLiveness::Indeterminate.is_running_or_indeterminate());
    assert!(!PidLiveness::NotRunning.is_running_or_indeterminate());
}

#[test]
fn pid_liveness_wraps_pid_is_running() {
    assert_eq!(pid_liveness(std::process::id()), PidLiveness::Running);
    assert_ne!(pid_liveness(0xFFFF_FFFE), PidLiveness::Running);
}
