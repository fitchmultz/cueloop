//! Runner execution facade with consistent error handling.
//!
//! Responsibilities:
//! - Re-export the runner execution types and orchestration entrypoints.
//! - Keep backend wiring, continue-session policy, and orchestration split by concern.
//!
//! Not handled here:
//! - Prompt template rendering.
//! - Queue/task persistence.
//!
//! Invariants/assumptions:
//! - Re-exports preserve the existing `crate::runutil::execution::*` test surface.

mod backend;
mod continue_session;
mod orchestration;
mod retry_policy;

#[cfg(test)]
pub(crate) use backend::RunnerBackend;
pub(crate) use backend::{RunnerErrorMessages, RunnerInvocation};
pub(crate) use orchestration::run_prompt_with_handling;
#[cfg(test)]
pub(crate) use orchestration::run_prompt_with_handling_backend;
