//! Queue file loading facade with explicit read-vs-repair semantics.
//!
//! Responsibilities:
//! - Expose queue-file load helpers for plain reads, parse repair, and validation.
//! - Coordinate queue/done loading for read-only and explicit repair flows.
//! - Keep repair-only timestamp maintenance separate from pure read paths.
//!
//! Not handled here:
//! - Queue file saving (see `queue::save`).
//! - ID generation or backup management.
//!
//! Invariants/assumptions:
//! - Missing queue files return default empty queues.
//! - Pure load/validate APIs never write to disk.
//! - Callers must hold locks before invoking explicit repair-and-save APIs.

mod maintenance;
mod read;
mod validation;

pub use read::{
    load_and_validate_queues, load_queue, load_queue_or_default, load_queue_with_repair,
    load_queue_with_repair_and_validate, repair_and_validate_queues,
};

#[cfg(test)]
mod tests;
