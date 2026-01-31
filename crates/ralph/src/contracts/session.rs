//! Session state contract for crash recovery.
//!
//! Responsibilities:
//! - Define the session state schema for run loop recovery.
//! - Provide serialization/deserialization for session persistence.
//!
//! Not handled here:
//! - Session persistence operations (see crate::session).
//! - Session validation logic (see crate::session).
//!
//! Invariants/assumptions:
//! - Session state is written atomically to prevent corruption.
//! - Timestamps are RFC3339 UTC format.

use crate::constants::versions::SESSION_STATE_VERSION;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Runner;

/// Session state persisted to enable crash recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionState {
    /// Schema version for forward compatibility.
    pub version: u32,

    /// Unique session ID (UUID v4) for this run session.
    pub session_id: String,

    /// The task currently being executed.
    pub task_id: String,

    /// When the session/run started (RFC3339 UTC).
    pub run_started_at: String,

    /// When the session state was last updated (RFC3339 UTC).
    pub last_updated_at: String,

    /// Total number of iterations planned for the current task.
    pub iterations_planned: u8,

    /// Number of iterations completed so far.
    pub iterations_completed: u8,

    /// Current phase being executed (1, 2, or 3).
    pub current_phase: u8,

    /// Runner being used for this session.
    pub runner: Runner,

    /// Model being used for this session.
    pub model: String,

    /// Number of tasks completed in this loop session (for loop progress tracking).
    pub tasks_completed_in_loop: u32,

    /// Maximum tasks to run in this loop (0 = no limit).
    pub max_tasks: u32,

    /// Git HEAD commit at session start (for advanced recovery validation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_head_commit: Option<String>,
}

impl SessionState {
    /// Create a new session state for the given task.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: String,
        task_id: String,
        run_started_at: String,
        iterations_planned: u8,
        runner: Runner,
        model: String,
        max_tasks: u32,
        git_head_commit: Option<String>,
    ) -> Self {
        Self {
            version: SESSION_STATE_VERSION,
            session_id,
            task_id,
            run_started_at: run_started_at.clone(),
            last_updated_at: run_started_at,
            iterations_planned,
            iterations_completed: 0,
            current_phase: 1,
            runner,
            model,
            tasks_completed_in_loop: 0,
            max_tasks,
            git_head_commit,
        }
    }

    /// Update the session after iteration completion.
    pub fn mark_iteration_complete(&mut self, completed_at: String) {
        self.iterations_completed += 1;
        self.last_updated_at = completed_at;
    }

    /// Update the session after phase completion.
    pub fn set_phase(&mut self, phase: u8, updated_at: String) {
        self.current_phase = phase;
        self.last_updated_at = updated_at;
    }

    /// Update the session after task completion.
    pub fn mark_task_complete(&mut self, updated_at: String) {
        self.tasks_completed_in_loop += 1;
        self.last_updated_at = updated_at;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session() -> SessionState {
        SessionState::new(
            "test-session-id".to_string(),
            "RQ-0001".to_string(),
            "2026-01-30T00:00:00.000000000Z".to_string(),
            2,
            Runner::Claude,
            "sonnet".to_string(),
            10,
            Some("abc123".to_string()),
        )
    }

    #[test]
    fn session_new_sets_defaults() {
        let session = test_session();

        assert_eq!(session.version, SESSION_STATE_VERSION);
        assert_eq!(session.session_id, "test-session-id");
        assert_eq!(session.task_id, "RQ-0001");
        assert_eq!(session.iterations_planned, 2);
        assert_eq!(session.iterations_completed, 0);
        assert_eq!(session.current_phase, 1);
        assert_eq!(session.tasks_completed_in_loop, 0);
        assert_eq!(session.max_tasks, 10);
        assert_eq!(session.git_head_commit, Some("abc123".to_string()));
    }

    #[test]
    fn session_mark_iteration_complete_increments_count() {
        let mut session = test_session();

        session.mark_iteration_complete("2026-01-30T00:01:00.000000000Z".to_string());

        assert_eq!(session.iterations_completed, 1);
        assert_eq!(session.last_updated_at, "2026-01-30T00:01:00.000000000Z");
    }

    #[test]
    fn session_set_phase_updates_phase() {
        let mut session = test_session();

        session.set_phase(2, "2026-01-30T00:02:00.000000000Z".to_string());

        assert_eq!(session.current_phase, 2);
        assert_eq!(session.last_updated_at, "2026-01-30T00:02:00.000000000Z");
    }

    #[test]
    fn session_mark_task_complete_increments_count() {
        let mut session = test_session();

        session.mark_task_complete("2026-01-30T00:03:00.000000000Z".to_string());

        assert_eq!(session.tasks_completed_in_loop, 1);
        assert_eq!(session.last_updated_at, "2026-01-30T00:03:00.000000000Z");
    }

    #[test]
    fn session_serialization_roundtrip() {
        let session = test_session();

        let json = serde_json::to_string(&session).expect("serialize");
        let deserialized: SessionState = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.session_id, session.session_id);
        assert_eq!(deserialized.task_id, session.task_id);
        assert_eq!(deserialized.iterations_planned, session.iterations_planned);
        assert_eq!(deserialized.runner, session.runner);
        assert_eq!(deserialized.model, session.model);
    }

    #[test]
    fn session_deserialization_ignores_optional_git_commit_when_none() {
        let session = SessionState::new(
            "test-id".to_string(),
            "RQ-0001".to_string(),
            "2026-01-30T00:00:00.000000000Z".to_string(),
            1,
            Runner::Claude,
            "sonnet".to_string(),
            0,
            None,
        );

        let json = serde_json::to_string(&session).expect("serialize");
        assert!(!json.contains("git_head_commit"));

        let deserialized: SessionState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.git_head_commit, None);
    }
}
