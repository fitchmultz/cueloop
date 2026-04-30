//! Task decomposition planning and queue materialization helpers.
//!
//! Purpose:
//! - Task decomposition planning and queue materialization helpers.
//!
//! Responsibilities:
//! - Re-export the task decomposition preview/write API from focused companion modules.
//! - Keep the root module as a thin facade for planner, normalization, and queue-write workflows.
//! - Provide a stable import surface for CLI handlers and neighboring task-command modules.
//!
//! Not handled here:
//! - Planner prompt construction or runner invocation details.
//! - Source/attach validation or queue mutation internals.
//! - Tree normalization and materialization helper implementations.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Preview stays side-effect free with respect to queue/done files.
//! - Write mode re-checks queue state under lock before mutating.

mod checkpoint;
mod planning;
mod resolve;
mod source_file;
mod support;
#[cfg(test)]
mod tests;
mod tree;
mod types;
mod write;

pub use checkpoint::{
    DecompositionPreviewCheckpointRef, load_decomposition_preview_checkpoint,
    save_decomposition_preview_checkpoint,
};
pub use planning::plan_task_decomposition;
pub use source_file::read_plan_file_source;
pub use types::{
    DecompositionAttachTarget, DecompositionChildPolicy, DecompositionPlan, DecompositionPreview,
    DecompositionSource, DependencyEdgePreview, PlannedNode, TaskDecomposeOptions,
    TaskDecomposeSourceInput, TaskDecomposeWriteResult,
};
pub use write::write_task_decomposition;
