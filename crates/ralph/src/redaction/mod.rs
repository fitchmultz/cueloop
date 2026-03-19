//! Purpose: Provide the public redaction API used to scrub secrets from
//! strings, logs, and runner-facing diagnostics.
//!
//! Responsibilities:
//! - Declare the `redaction` child modules.
//! - Re-export the stable public redaction surface used across the crate.
//!
//! Scope:
//! - Thin facade only; implementation lives in sibling files under
//!   `redaction/`.
//!
//! Usage:
//! - Import `redact_text`, `RedactedString`, and `RedactedLogger` through
//!   `crate::redaction`.
//!
//! Invariants/Assumptions:
//! - The public API remains stable across the split.
//! - Redaction coverage and logger behavior remain unchanged.

mod env;
mod logging;
mod patterns;

#[cfg(test)]
mod tests;

pub use env::{is_path_like_env_key, looks_sensitive_env_key};
pub use logging::{RedactedLogger, RedactedString};
pub use patterns::redact_text;
