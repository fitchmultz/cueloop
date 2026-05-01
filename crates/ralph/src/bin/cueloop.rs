//! CueLoop primary CLI binary.
//!
//! Purpose:
//! - Provide the primary `cueloop` executable entrypoint.
//!
//! Responsibilities:
//! - Delegate startup, parsing, logging, sanity checks, and command dispatch to the shared CLI entrypoint.
//!
//! Scope:
//! - Binary bootstrap only; implementation lives in `ralph::cli_entrypoint`.
//!
//! Usage:
//! - Built by Cargo as the primary CueLoop CLI binary.
//!
//! Invariants/assumptions:
//! - The shared entrypoint derives display/help naming from argv[0].

fn main() {
    ralph::cli_entrypoint::main();
}
