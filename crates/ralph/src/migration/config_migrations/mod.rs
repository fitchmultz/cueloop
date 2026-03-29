//! Purpose: Directory-backed facade for config migration helpers.
//!
//! Responsibilities:
//! - Declare focused config migration companion modules.
//! - Re-export the stable config migration API used by migration dispatch code.
//! - Keep regression coverage colocated with the facade.
//!
//! Scope:
//! - Thin facade only; key detection, key rewrite, CI gate migration, and legacy
//!   contract upgrade behavior live in companion modules.
//!
//! Usage:
//! - Imported through `crate::migration::config_migrations::*` by `migration::mod`.
//!
//! Invariants/Assumptions:
//! - Public signatures remain stable across this split.
//! - JSONC-preserving key rename behavior remains unchanged.
//! - Companion modules stay private to the migration boundary.

mod ci_gate;
mod detect;
mod keys;
mod legacy;

#[cfg(test)]
mod tests;

pub use ci_gate::apply_ci_gate_rewrite;
pub use detect::{config_has_key, get_config_value};
pub use keys::{apply_key_remove, apply_key_rename};
pub(crate) use keys::{remove_key_in_file, rename_key_in_file};
pub use legacy::{apply_legacy_contract_upgrade, config_needs_legacy_contract_upgrade};
