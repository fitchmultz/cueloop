//! Purpose: Facade for webhook diagnostics, failure-history persistence, and replay surfaces.
//!
//! Responsibilities:
//! - Re-export the operator-facing diagnostics snapshot, replay, and failure-record APIs.
//! - Keep runtime metrics, failure-store persistence, replay logic, and test helpers split into focused companions.
//!
//! Scope:
//! - Thin module root only; implementation lives in sibling `diagnostics/*` companions.
//!
//! Usage:
//! - Used by webhook worker/runtime code, CLI commands, and webhook tests through stable facade re-exports.
//!
//! Invariants/Assumptions:
//! - Re-exports preserve the existing diagnostics API surface.
//! - Failure persistence remains bounded and secret-safe.
//! - Replay remains explicit and bounded by caller-provided selectors and limits.

mod failure_store;
mod metrics;
mod replay;

#[cfg(test)]
mod tests;

pub use failure_store::{WebhookFailureRecord, failure_store_path};
pub use metrics::{WebhookDiagnostics, diagnostics_snapshot};
pub use replay::{ReplayCandidate, ReplayReport, ReplaySelector, replay_failed_deliveries};

pub(crate) use metrics::{
    note_delivery_failure, note_delivery_success, note_dropped_message, note_enqueue_success,
    note_queue_dequeue, note_retry_attempt, note_retry_requeue, set_queue_capacity,
};

#[cfg(test)]
pub(crate) use tests::{
    load_failure_records_for_tests, persist_failed_delivery_for_tests,
    reset_webhook_metrics_for_tests, update_replay_counts_for_tests,
    write_failure_records_for_tests,
};
