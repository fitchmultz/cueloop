//! macOS app command facade.
//!
//! Purpose:
//! - Expose the `cueloop app open` implementation from a thin command module.
//!
//! Responsibilities:
//! - Re-export the public app-open entrypoint for the CLI layer.
//! - Keep launch planning, URL handoff construction, and runtime execution in focused helpers.
//!
//! Scope:
//! - This module coordinates the `cueloop app` command implementation only.
//! - It does not define clap parsing or macOS app internals.
//!
//! Usage:
//! - Called by `crate::cli::app` after argument parsing succeeds.
//!
//! Invariants/assumptions:
//! - `open` remains the only public entrypoint exposed here.
//! - Helper modules keep planning logic deterministic and testable.

mod launch_plan;
mod model;
mod runtime;
#[cfg(test)]
mod tests;
mod url_plan;

pub use runtime::open;
