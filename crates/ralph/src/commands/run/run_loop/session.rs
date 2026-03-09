//! Session recovery helpers for the sequential run loop.
//!
//! Responsibilities:
//! - Resolve whether the loop should resume a prior task session.
//! - Centralize stale/timeout recovery prompts and progress carry-over.
//!
//! Not handled here:
//! - Queue waiting or task execution.
//! - Session progress persistence after task execution.
//!
//! Invariants/assumptions:
//! - Session timeout uses configured hours (defaulting to the shared constant).
//! - Clearing stale/timed-out sessions is best-effort but failures are surfaced.

use crate::config;
use crate::session::{self, SessionValidationResult};
use anyhow::Result;

use super::RunLoopOptions;

pub(super) struct ResumeState {
    pub(super) resume_task_id: Option<String>,
    pub(super) completed_count: u32,
}

pub(super) fn resolve_resume_state(
    resolved: &config::Resolved,
    opts: &RunLoopOptions,
) -> Result<ResumeState> {
    let cache_dir = resolved.repo_root.join(".ralph/cache");
    let queue_file = crate::queue::load_queue(&resolved.queue_path)?;
    let session_timeout_hours = resolved.config.agent.session_timeout_hours;

    let (resume_task_id, completed_count) =
        match session::check_session(&cache_dir, &queue_file, session_timeout_hours)? {
            SessionValidationResult::NoSession => (None, opts.starting_completed),
            SessionValidationResult::Valid(session) => {
                if opts.auto_resume {
                    log::info!("Auto-resuming session for task {}", session.task_id);
                    (Some(session.task_id), session.tasks_completed_in_loop)
                } else {
                    match session::prompt_session_recovery(&session, opts.non_interactive)? {
                        true => (Some(session.task_id), session.tasks_completed_in_loop),
                        false => {
                            session::clear_session(&cache_dir)?;
                            (None, opts.starting_completed)
                        }
                    }
                }
            }
            SessionValidationResult::Stale { reason } => {
                log::info!("Stale session cleared: {}", reason);
                session::clear_session(&cache_dir)?;
                (None, opts.starting_completed)
            }
            SessionValidationResult::Timeout { hours, session } => {
                let threshold = session_timeout_hours
                    .unwrap_or(crate::constants::timeouts::DEFAULT_SESSION_TIMEOUT_HOURS);
                match session::prompt_session_recovery_timeout(
                    &session,
                    hours,
                    threshold,
                    opts.non_interactive,
                )? {
                    true => (Some(session.task_id), session.tasks_completed_in_loop),
                    false => {
                        session::clear_session(&cache_dir)?;
                        (None, opts.starting_completed)
                    }
                }
            }
        };

    Ok(ResumeState {
        resume_task_id,
        completed_count,
    })
}
