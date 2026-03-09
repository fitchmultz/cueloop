//! Session persistence and recovery facade.
//!
//! Responsibilities:
//! - Re-export session persistence, validation, recovery UI, and progress helpers.
//! - Keep the public `crate::session::*` surface stable while implementation stays split.
//!
//! Not handled here:
//! - Queue/run-loop orchestration.
//! - Session state schema definitions.
//!
//! Invariants/assumptions:
//! - Persistence, validation, progress mutation, and interactive recovery remain separate.
//! - Re-exports preserve existing caller paths.

mod persistence;
mod progress;
mod recovery;
#[cfg(test)]
mod tests;
mod validation;

pub use persistence::{
    clear_session, get_git_head_commit, load_session, save_session, session_exists, session_path,
};
pub use progress::increment_session_progress;
pub use recovery::{prompt_session_recovery, prompt_session_recovery_timeout};
pub use validation::{
    SessionValidationResult, check_session, validate_session, validate_session_with_now,
};
