//! Git commit and push facade.
//!
//! Purpose:
//! - Expose commit, restore, upstream, and rebase-aware push operations from a thin git module.
//!
//! Responsibilities:
//! - Re-export working-tree mutation helpers.
//! - Re-export upstream/query helpers and rebase-aware push behavior.
//! - Keep tests and sub-concerns in focused companions.
//!
//! Scope:
//! - Git commit/push workflows only.
//! - Error types, status helpers, and branch lookup remain in sibling git modules.
//!
//! Usage:
//! - Imported through `crate::git` by command workflows and tests.
//!
//! Invariants/assumptions:
//! - Public APIs preserve existing behavior and error types.
//! - Rebase-aware push remains the single entrypoint for retrying non-fast-forward pushes.

mod rebase_push;
#[cfg(test)]
mod tests;
mod upstream;
mod working_tree;

pub use rebase_push::push_upstream_with_rebase;
pub use upstream::{
    abort_rebase, fetch_branch, is_ahead_of_upstream, is_behind_upstream, list_conflict_files,
    push_current_branch, push_head_to_branch, push_upstream, push_upstream_allow_create,
    rebase_onto, upstream_ref,
};
pub use working_tree::{
    add_paths_force, commit_all, restore_tracked_paths_to_head, revert_uncommitted,
};
