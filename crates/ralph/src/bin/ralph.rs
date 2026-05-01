//! Legacy Ralph compatibility CLI binary.
//!
//! Purpose:
//! - Provide the legacy `ralph` executable alias during the CueLoop transition window.
//!
//! Responsibilities:
//! - Delegate startup, parsing, logging, sanity checks, and command dispatch to the shared CLI entrypoint.
//!
//! Scope:
//! - Binary bootstrap only; implementation lives in `ralph::cli_entrypoint`.
//!
//! Usage:
//! - Built by Cargo as the backwards-compatible CLI binary.
//!
//! Invariants/assumptions:
//! - The shared entrypoint derives display/help naming from argv[0].

fn main() {
    ralph::cli_entrypoint::main();
}
