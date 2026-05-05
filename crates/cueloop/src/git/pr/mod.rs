//! GitHub PR helpers using the `gh` CLI.
//!
//! Purpose:
//! - GitHub PR helpers using the `gh` CLI.
//!
//! Responsibilities:
//! - Expose the crate-facing PR operations and status types.
//! - Keep the public surface stable while delegating execution/parsing to focused submodules.
//! - Colocate PR-specific tests near the implementation.
//!
//! Not handled here:
//! - Task selection or worker execution (see `commands::run::parallel`).
//! - Direct-push parallel integration logic (see `commands::run::parallel::integration`).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - `gh` is installed and authenticated for command execution paths.
//! - Repo root points to a GitHub-backed repository.

mod gh;

pub(crate) use gh::check_gh_available;

#[cfg(test)]
mod tests;
