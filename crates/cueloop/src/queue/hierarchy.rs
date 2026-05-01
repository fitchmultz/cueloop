//! Purpose: Expose task-hierarchy indexing, cycle detection, and tree rendering through a thin facade.
//!
//! Responsibilities:
//! - Re-export the crate-visible `queue::hierarchy` API from focused companion modules.
//! - Keep indexing, cycle analysis, and rendering concerns split without changing caller imports.
//! - Preserve deterministic active/done hierarchy behavior across queue consumers.
//!
//! Scope:
//! - Facade-only module wiring for hierarchy helpers used by queue validation and CLI flows.
//!
//! Usage:
//! - Imported via `crate::queue::hierarchy` by CLI, validation, and task-decomposition helpers.
//!
//! Invariants/Assumptions:
//! - Task IDs are unique across active + done files.
//! - Ordering remains deterministic: active tasks first, then done tasks.
//! - Cycle detection and rendering semantics stay behavior-compatible with the pre-split module.

mod cycles;
mod index;
mod render;

pub(crate) use cycles::detect_parent_cycles;
pub(crate) use index::{HierarchyIndex, TaskSource};
pub(crate) use render::render_tree;

#[cfg(test)]
mod tests;
