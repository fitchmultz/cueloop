//! Test support helpers.
//!
//! Responsibilities:
//! - Centralize reusable helpers for unit tests under `crates/ralph/src`.
//! - Provide shared utilities without expanding the public API.
//!
//! Not handled:
//! - Integration test helpers (see `crates/ralph/tests/test_support.rs`).
//! - Production runtime behavior.
//!
//! Invariants/assumptions:
//! - Compiled only for tests via `#[cfg(test)]` in `lib.rs`.
//! - Helpers are used only in unit test contexts.

pub(crate) mod git;
pub(crate) mod path;
pub(crate) mod runner;

// Re-export test sync utilities from the crate root to avoid circular imports
pub use crate::test_sync::INTERRUPT_TEST_MUTEX;
pub use crate::test_sync::reset_ctrlc_interrupt_flag;
