//! Plugin system for CueLoop (runners + task processors).
//!
//! Purpose:
//! - Plugin system for CueLoop (runners + task processors).
//!
//! Responsibilities:
//! - Define plugin manifest contracts and validation.
//! - Discover plugin packages from global + project plugin directories.
//! - Provide a registry for resolving enabled plugins, runner binaries, and processor hooks.
//!
//! Not handled here:
//! - CLI Clap argument definitions (see `crate::cli::plugin`).
//! - Execution-phase orchestration (see `crate::commands::run`).
//! - Streaming JSON parsing (see `crate::runner::execution::process`).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Plugins are discovered from current and legacy roots:
//!   - Current global:  ~/.config/cueloop/plugins/<plugin_id>/plugin.json
//!   - Legacy global:   ~/.config/cueloop/plugins/<plugin_id>/plugin.json
//!   - Current project: .cueloop/plugins/<plugin_id>/plugin.json
//!   - Legacy project:  .cueloop/plugins/<plugin_id>/plugin.json
//! - Current roots override legacy roots; project plugins override global plugins of the same id.
//! - Plugins are disabled unless enabled in config.
//! - Plugin executables are NOT sandboxed by CueLoop; enabling a plugin is equivalent to trusting it.

pub(crate) mod discovery;
pub(crate) mod manifest;
pub(crate) mod processor_executor;
pub(crate) mod registry;

pub(crate) const PLUGIN_API_VERSION: u32 = 1;
