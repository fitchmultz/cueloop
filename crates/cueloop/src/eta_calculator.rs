//! Purpose: Provide the public ETA-calculation API used by queue and report
//! surfaces.
//!
//! Responsibilities:
//! - Declare the `eta_calculator` child modules.
//! - Re-export the stable ETA data-model, calculator, and formatting helpers.
//!
//! Scope:
//! - Thin facade only; implementation lives in sibling files under
//!   `eta_calculator/`.
//!
//! Usage:
//! - Import `EtaCalculator`, `EtaEstimate`, `EtaConfidence`, and `format_eta`
//!   through `crate::eta_calculator`.
//!
//! Invariants/Assumptions:
//! - The public API remains stable across this split.
//! - ETA calculation behavior and confidence semantics remain unchanged.

mod calculator;
mod format;
mod types;

#[cfg(test)]
mod tests;

pub use calculator::EtaCalculator;
pub use format::format_eta;
pub use types::{EtaConfidence, EtaEstimate};
