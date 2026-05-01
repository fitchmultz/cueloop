//! Session test support and grouped test modules.
//!
//! Purpose:
//! - Session test support and grouped test modules.
//!
//! Responsibilities:
//! - Share focused fixtures across session persistence, validation, decision, and recovery tests.
//! - Route behavior-grouped session tests into companion modules.
//!
//! Not handled here:
//! - Individual behavior assertions; those live in adjacent test modules.
//!
//! Usage:
//! - Compiled through `crate::session` unit tests.
//!
//! Invariants/assumptions:
//! - Time-sensitive validation uses fixed timestamps where practical.

use super::*;
use crate::contracts::{QueueFile, SessionState, Task, TaskPriority, TaskStatus};
use crate::timeutil;

mod decision;
mod persistence;
mod recovery_progress;
mod validation;

fn test_task(id: &str, status: TaskStatus) -> Task {
    Task {
        id: id.to_string(),
        status,
        kind: Default::default(),
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

fn empty_queue() -> QueueFile {
    QueueFile {
        version: 1,
        tasks: vec![],
    }
}
