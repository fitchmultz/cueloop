//! Plugin discovery-backed registry and executable resolution.
//!
//! Purpose:
//! - Combine discovered plugin manifests with config enablement and executable lookup.
//!
//! Responsibilities:
//! - Expose discovered plugins after trust-gating project scope.
//! - Answer enablement and config-blob queries for plugin IDs.
//! - Resolve runner and processor executables without allowing path escapes.
//!
//! Scope:
//! - Plugin discovery results, enablement state, and manifest-relative executable resolution.
//!
//! Usage:
//! - Consumed by plugin commands, runner dispatch, and processor execution paths.
//!
//! Invariants/assumptions:
//! - Disabled plugins MUST NOT be executed.
//! - Plugin executables MUST remain plugin-dir-relative after canonical path resolution.
//! - Existing symlinked files or ancestors MUST NOT escape the plugin directory.

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use anyhow::Context;

use crate::config::load_repo_trust;
use crate::contracts::Config;
use crate::plugins::discovery::{DiscoveredPlugin, PluginScope, discover_plugins};

#[derive(Debug, Clone)]
pub(crate) struct PluginRegistry {
    discovered: BTreeMap<String, DiscoveredPlugin>,
    config: crate::contracts::PluginsConfig,
}

impl PluginRegistry {
    pub(crate) fn load(repo_root: &Path, cfg: &Config) -> anyhow::Result<Self> {
        let repo_trust = load_repo_trust(repo_root)?;
        let mut discovered = discover_plugins(repo_root)?;
        if !repo_trust.is_trusted() {
            discovered.retain(|_, plugin| plugin.scope != PluginScope::Project);
        }

        Ok(Self {
            discovered,
            config: cfg.plugins.clone(),
        })
    }

    pub(crate) fn discovered(&self) -> &BTreeMap<String, DiscoveredPlugin> {
        &self.discovered
    }

    pub(crate) fn is_enabled(&self, plugin_id: &str) -> bool {
        self.config
            .plugins
            .get(plugin_id)
            .and_then(|p| p.enabled)
            .unwrap_or(false)
    }

    pub(crate) fn plugin_config_blob(&self, plugin_id: &str) -> Option<serde_json::Value> {
        self.config
            .plugins
            .get(plugin_id)
            .and_then(|p| p.config.clone())
    }

    pub(crate) fn resolve_runner_bin(&self, plugin_id: &str) -> anyhow::Result<PathBuf> {
        let discovered = self
            .discovered
            .get(plugin_id)
            .ok_or_else(|| anyhow::anyhow!("plugin not found: {plugin_id}"))?;

        let runner = discovered
            .manifest
            .runner
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("plugin {plugin_id} does not provide a runner"))?;

        resolve_plugin_relative_exe(&discovered.plugin_dir, &runner.bin)
    }

    pub(crate) fn resolve_processor_bin(&self, plugin_id: &str) -> anyhow::Result<PathBuf> {
        let discovered = self
            .discovered
            .get(plugin_id)
            .ok_or_else(|| anyhow::anyhow!("plugin not found: {plugin_id}"))?;

        let proc = discovered
            .manifest
            .processors
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("plugin {plugin_id} does not provide processors"))?;

        resolve_plugin_relative_exe(&discovered.plugin_dir, &proc.bin)
    }
}

pub(crate) fn resolve_plugin_relative_exe(plugin_dir: &Path, bin: &str) -> anyhow::Result<PathBuf> {
    let relative_path = Path::new(bin);
    if relative_path.is_absolute() {
        anyhow::bail!("plugin executable path must be relative to the plugin directory: {bin}");
    }

    if relative_path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        anyhow::bail!("plugin executable path must stay within the plugin directory: {bin}");
    }

    let canonical_plugin_dir = std::fs::canonicalize(plugin_dir)
        .with_context(|| format!("canonicalize plugin directory {}", plugin_dir.display()))?;
    let candidate = plugin_dir.join(relative_path);

    if let Some(canonical_candidate) = canonicalize_if_exists(&candidate)
        .with_context(|| format!("canonicalize plugin executable {}", candidate.display()))?
    {
        ensure_within_plugin_dir(&canonical_plugin_dir, &canonical_candidate, bin)?;
        return Ok(canonical_candidate);
    }

    let canonical_ancestor = canonicalize_existing_ancestor(
        candidate.parent().unwrap_or(plugin_dir),
    )
    .with_context(|| {
        format!(
            "canonicalize plugin executable ancestor {}",
            candidate.display()
        )
    })?;
    ensure_within_plugin_dir(&canonical_plugin_dir, &canonical_ancestor, bin)?;
    Ok(candidate)
}

fn canonicalize_if_exists(path: &Path) -> anyhow::Result<Option<PathBuf>> {
    match std::fs::canonicalize(path) {
        Ok(canonical) => Ok(Some(canonical)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn canonicalize_existing_ancestor(path: &Path) -> anyhow::Result<PathBuf> {
    let mut current = path;
    loop {
        match std::fs::canonicalize(current) {
            Ok(canonical) => return Ok(canonical),
            Err(err) if err.kind() == ErrorKind::NotFound => {
                current = current.parent().ok_or_else(|| {
                    anyhow::anyhow!(
                        "plugin executable path does not have an existing ancestor: {}",
                        path.display()
                    )
                })?;
            }
            Err(err) => return Err(err.into()),
        }
    }
}

fn ensure_within_plugin_dir(plugin_dir: &Path, candidate: &Path, bin: &str) -> anyhow::Result<()> {
    if candidate.starts_with(plugin_dir) {
        return Ok(());
    }

    anyhow::bail!(
        "plugin executable path must stay within the plugin directory after canonical resolution: {bin}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::{PluginManifest, RunnerPlugin};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_plugin(dir: &Path, id: &str) -> anyhow::Result<()> {
        let manifest = PluginManifest {
            api_version: crate::plugins::PLUGIN_API_VERSION,
            id: id.to_string(),
            version: "1.0.0".to_string(),
            name: format!("Plugin {}", id),
            description: None,
            runner: Some(RunnerPlugin {
                bin: "runner.sh".to_string(),
                supports_resume: None,
                default_model: None,
            }),
            processors: None,
        };
        let path = dir.join("plugin.json");
        let mut file = std::fs::File::create(&path)?;
        file.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
        Ok(())
    }

    fn trust_repo(repo_root: &Path) {
        let cueloop_dir = repo_root.join(".cueloop");
        std::fs::create_dir_all(&cueloop_dir).unwrap();
        std::fs::write(
            cueloop_dir.join("trust.jsonc"),
            r#"{"allow_project_commands": true}"#,
        )
        .unwrap();
    }

    #[test]
    fn is_enabled_defaults_to_false() {
        let tmp = TempDir::new().unwrap();
        trust_repo(tmp.path());
        let plugin_dir = tmp.path().join(".cueloop/plugins/test.plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        create_test_plugin(&plugin_dir, "test.plugin").unwrap();

        let cfg = Config::default();
        let registry = PluginRegistry::load(tmp.path(), &cfg).unwrap();

        assert!(!registry.is_enabled("test.plugin"));
    }

    #[test]
    fn is_enabled_respects_config() {
        let tmp = TempDir::new().unwrap();
        trust_repo(tmp.path());
        let plugin_dir = tmp.path().join(".cueloop/plugins/test.plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        create_test_plugin(&plugin_dir, "test.plugin").unwrap();

        let mut cfg = Config::default();
        cfg.plugins.plugins.insert(
            "test.plugin".to_string(),
            crate::contracts::PluginConfig {
                enabled: Some(true),
                ..Default::default()
            },
        );

        let registry = PluginRegistry::load(tmp.path(), &cfg).unwrap();
        assert!(registry.is_enabled("test.plugin"));
    }

    #[test]
    fn resolve_runner_bin_rejects_disabled_plugin() {
        let tmp = TempDir::new().unwrap();
        trust_repo(tmp.path());
        let plugin_dir = tmp.path().join(".cueloop/plugins/test.plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        create_test_plugin(&plugin_dir, "test.plugin").unwrap();

        let cfg = Config::default();
        let registry = PluginRegistry::load(tmp.path(), &cfg).unwrap();

        // Plugin exists but is not enabled - bin resolution still works
        // (enable check is done at higher level)
        let bin = registry.resolve_runner_bin("test.plugin");
        assert!(bin.is_ok());
    }

    #[test]
    fn resolve_runner_bin_fails_for_missing_plugin() {
        let tmp = TempDir::new().unwrap();
        let cfg = Config::default();
        let registry = PluginRegistry::load(tmp.path(), &cfg).unwrap();

        let err = registry.resolve_runner_bin("nonexistent");
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn resolve_plugin_relative_exe_rejects_parent_dir() {
        let tmp = TempDir::new().unwrap();
        let result = resolve_plugin_relative_exe(tmp.path(), "../escape.sh");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must stay within the plugin directory")
        );
    }

    #[test]
    fn resolve_plugin_relative_exe_accepts_relative_path() {
        let tmp = TempDir::new().unwrap();
        let result = resolve_plugin_relative_exe(tmp.path(), "runner.sh").unwrap();
        assert_eq!(result, tmp.path().join("runner.sh"));
    }

    #[test]
    fn resolve_plugin_relative_exe_rejects_absolute_path() {
        let tmp = TempDir::new().unwrap();
        let abs = tmp.path().join("absolute_runner.sh");
        let err = resolve_plugin_relative_exe(tmp.path(), abs.to_str().unwrap()).unwrap_err();
        assert!(err.to_string().contains("relative to the plugin directory"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_plugin_relative_exe_rejects_symlinked_file_escape() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("plugin");
        let outside_dir = tmp.path().join("outside");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();
        let outside_runner = outside_dir.join("runner.sh");
        std::fs::write(&outside_runner, "#!/bin/sh\nexit 0\n").unwrap();
        symlink(&outside_runner, plugin_dir.join("runner.sh")).unwrap();

        let err = resolve_plugin_relative_exe(&plugin_dir, "runner.sh").unwrap_err();
        assert!(
            err.to_string()
                .contains("must stay within the plugin directory after canonical resolution")
        );
    }

    #[cfg(unix)]
    #[test]
    fn resolve_plugin_relative_exe_rejects_symlinked_directory_escape() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("plugin");
        let outside_dir = tmp.path().join("outside/bin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();
        std::fs::write(outside_dir.join("runner.sh"), "#!/bin/sh\nexit 0\n").unwrap();
        symlink(tmp.path().join("outside/bin"), plugin_dir.join("bin")).unwrap();

        let err = resolve_plugin_relative_exe(&plugin_dir, "bin/runner.sh").unwrap_err();
        assert!(
            err.to_string()
                .contains("must stay within the plugin directory after canonical resolution")
        );
    }

    #[cfg(unix)]
    #[test]
    fn resolve_plugin_relative_exe_accepts_symlinked_file_within_plugin_dir() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("plugin");
        let internal_dir = plugin_dir.join("bin");
        std::fs::create_dir_all(&internal_dir).unwrap();
        let internal_runner = internal_dir.join("runner.sh");
        std::fs::write(&internal_runner, "#!/bin/sh\nexit 0\n").unwrap();
        symlink(&internal_runner, plugin_dir.join("runner.sh")).unwrap();

        let resolved = resolve_plugin_relative_exe(&plugin_dir, "runner.sh").unwrap();
        assert_eq!(resolved, std::fs::canonicalize(&internal_runner).unwrap());
    }

    #[test]
    fn load_ignores_project_plugins_in_untrusted_repo() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join(".cueloop/plugins/test.plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        create_test_plugin(&plugin_dir, "test.plugin").unwrap();

        let cfg = Config::default();
        let registry = PluginRegistry::load(tmp.path(), &cfg).unwrap();

        assert!(registry.discovered().is_empty());
    }

    #[test]
    fn load_keeps_project_plugins_in_trusted_repo() {
        let tmp = TempDir::new().unwrap();
        let cueloop_dir = tmp.path().join(".cueloop");
        let plugin_dir = cueloop_dir.join("plugins/test.plugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(
            cueloop_dir.join("trust.jsonc"),
            r#"{"allow_project_commands": true}"#,
        )
        .unwrap();
        create_test_plugin(&plugin_dir, "test.plugin").unwrap();

        let cfg = Config::default();
        let registry = PluginRegistry::load(tmp.path(), &cfg).unwrap();

        assert!(registry.discovered().contains_key("test.plugin"));
    }
}
