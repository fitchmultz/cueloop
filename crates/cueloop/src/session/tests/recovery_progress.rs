//! Session recovery prompt and progress tests.
//!
//! Purpose:
//! - Session recovery prompt and progress tests.
//!
//! Responsibilities:
//! - Verify non-interactive recovery prompt behavior and progress persistence helpers.
//!
//! Scope:
//! - Prompt/progress helpers only; resume decision behavior lives in `decision.rs`.
//!
//! Usage:
//! - Compiled through `crate::session::tests`.
//!
//! Invariants/assumptions:
//! - Non-interactive prompts must never block waiting for user input.

use super::*;
use tempfile::TempDir;

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
