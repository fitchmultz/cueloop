//! Shared helpers for plugin commands.
//!
//! Purpose:
//! - Shared helpers for plugin commands.
//!
//! Responsibilities:
//! - Resolve plugin installation roots from repo/config scope choices.
//! - Emit shared operator guidance reused by multiple plugin commands.
//!
//! Not handled here:
//! - Plugin manifest validation.
//! - Plugin file creation, copying, or deletion workflows.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Project-scope plugins are installed under `.cueloop/plugins`.
//! - Global-scope plugins are installed under `~/.config/cueloop/plugins`.

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

use crate::cli::plugin::PluginScopeArg;
use crate::constants::identity::{GLOBAL_CONFIG_DIR, PROJECT_RUNTIME_DIR};

pub(super) fn scope_root(repo_root: &Path, scope: PluginScopeArg) -> Result<PathBuf> {
    plugin_scope_root(repo_root, scope)
}

pub(super) fn existing_plugin_dir(
    repo_root: &Path,
    scope: PluginScopeArg,
    plugin_id: &str,
) -> Result<Option<PathBuf>> {
    let candidate = plugin_scope_root(repo_root, scope)?.join(plugin_id);
    Ok(candidate.exists().then_some(candidate))
}

fn plugin_scope_root(repo_root: &Path, scope: PluginScopeArg) -> Result<PathBuf> {
    Ok(match scope {
        PluginScopeArg::Project => repo_root.join(PROJECT_RUNTIME_DIR).join("plugins"),
        PluginScopeArg::Global => {
            let home = std::env::var_os("HOME")
                .ok_or_else(|| anyhow!("HOME environment variable not set"))?;
            PathBuf::from(home)
                .join(".config")
                .join(GLOBAL_CONFIG_DIR)
                .join("plugins")
        }
    })
}

pub(super) fn print_enable_hint(plugin_id: &str) {
    println!();
    println!("NOTE: The plugin is NOT automatically enabled.");
    println!("To enable it, add to your config:");
    println!(
        r#"  {{ "plugins": {{ "plugins": {{ "{}": {{ "enabled": true }} }} }} }}"#,
        plugin_id
    );
}
