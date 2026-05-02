//! Configuration resolution for CueLoop.
//!
//! Purpose:
//! - Configuration resolution for CueLoop's transitional `ralph` CLI.
//!
//! Responsibilities:
//! - Resolve configuration from multiple layers: global, project, and defaults.
//! - Discover repository root via current `.cueloop/`, legacy `.ralph/`, or `.git/` markers.
//! - Resolve active runtime layout and queue/done file paths.
//! - Apply profile patches after base config resolution.
//!
//! Not handled here:
//! - Config file loading/parsing (see `super::layer`).
//! - Config validation (see `super::validation`).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Config layers are applied in order: defaults, legacy global, current global, project.
//! - Paths are resolved relative to repo root unless absolute.
//! - Current global config resolves from `~/.config/cueloop/config.jsonc`.
//! - Legacy global config at `~/.config/ralph/config.jsonc` remains a fallback.
//! - Project config resolves from the active runtime dir: `.cueloop/config.jsonc` for
//!   current/uninitialized repos, `.ralph/config.jsonc` for legacy repos.

use crate::constants::defaults::DEFAULT_ID_WIDTH;
use crate::constants::identity::{
    GLOBAL_CONFIG_DIR, LEGACY_GLOBAL_CONFIG_DIR, LEGACY_PROJECT_RUNTIME_DIR, PROJECT_RUNTIME_DIR,
};
use crate::constants::queue::{
    DEFAULT_DONE_FILE, DEFAULT_ID_PREFIX, DEFAULT_QUEUE_FILE, LEGACY_DEFAULT_DONE_FILE,
    LEGACY_DEFAULT_QUEUE_FILE,
};
use crate::contracts::Config;
use crate::fsutil;
use crate::prompts_internal::validate_instruction_file_paths;
use anyhow::{Context, Result, bail};
use std::env;
use std::path::{Path, PathBuf};

use super::Resolved;
use super::layer::{ConfigLayer, apply_layer, load_layer};
use super::trust::load_repo_trust;
use super::validation::{
    validate_config, validate_project_execution_trust, validate_queue_done_file_override,
    validate_queue_file_override, validate_queue_id_prefix_override,
    validate_queue_id_width_override,
};

const CONFIG_FILE_NAME: &str = "config.jsonc";
const QUEUE_FILE_NAME: &str = "queue.jsonc";
const DONE_FILE_NAME: &str = "done.jsonc";
const TRUST_FILE_NAME: &str = "trust.jsonc";
const LEGACY_QUEUE_JSON_FILE_NAME: &str = "queue.json";
const LEGACY_DONE_JSON_FILE_NAME: &str = "done.json";
const LEGACY_CONFIG_JSON_FILE_NAME: &str = "config.json";
const RUNTIME_MARKER_FILES: &[&str] = &[
    QUEUE_FILE_NAME,
    DONE_FILE_NAME,
    CONFIG_FILE_NAME,
    TRUST_FILE_NAME,
    LEGACY_QUEUE_JSON_FILE_NAME,
    LEGACY_DONE_JSON_FILE_NAME,
    LEGACY_CONFIG_JSON_FILE_NAME,
];

/// Active repo-local runtime layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectRuntimeLayout {
    /// Current CueLoop runtime directory (`.cueloop`).
    Current,
    /// Legacy Ralph runtime directory (`.ralph`).
    Legacy,
    /// No runtime markers exist yet; new writes should use `.cueloop`.
    Uninitialized,
}

impl ProjectRuntimeLayout {
    fn directory_name(self) -> &'static str {
        match self {
            Self::Current | Self::Uninitialized => PROJECT_RUNTIME_DIR,
            Self::Legacy => LEGACY_PROJECT_RUNTIME_DIR,
        }
    }
}

/// Resolve configuration from the current working directory.
pub fn resolve_from_cwd() -> Result<Resolved> {
    resolve_from_cwd_internal(true, true, None)
}

/// Resolve like `resolve_from_cwd`, but skip project-layer execution trust validation.
///
/// Used when the operator is explicitly opting into trust (for example `cueloop init`) so
/// initialization can proceed before the repo-local trust file exists, then the trust file is
/// written afterward.
pub fn resolve_from_cwd_skipping_project_execution_trust() -> Result<Resolved> {
    resolve_from_cwd_internal(true, false, None)
}

/// Resolve config with an optional profile selection.
///
/// The profile is applied after base config resolution but before instruction_files validation.
pub fn resolve_from_cwd_with_profile(profile: Option<&str>) -> Result<Resolved> {
    resolve_from_cwd_internal(true, true, profile)
}

/// Resolve config for the doctor command, skipping instruction_files validation.
/// This allows doctor to diagnose and warn about missing files without failing early.
pub fn resolve_from_cwd_for_doctor() -> Result<Resolved> {
    resolve_from_cwd_internal(false, false, None)
}

fn resolve_from_cwd_internal(
    validate_instruction_files: bool,
    validate_execution_trust: bool,
    profile: Option<&str>,
) -> Result<Resolved> {
    let cwd = env::current_dir().context("resolve current working directory")?;
    log::debug!("resolving configuration from cwd: {}", cwd.display());
    let repo_root = find_repo_root(&cwd);

    let global_path = global_config_path();
    let global_layer_paths = global_config_layer_paths();
    let project_path = project_config_path(&repo_root);
    let repo_trust = load_repo_trust(&repo_root)?;

    let mut cfg = Config::default();
    let mut project_layer: Option<ConfigLayer> = None;
    let mut queue_file_explicit = false;
    let mut done_file_explicit = false;

    for path in &global_layer_paths {
        log::debug!("checking global config at: {}", path.display());
        if path.exists() {
            log::debug!("loading global config: {}", path.display());
            let layer = load_layer(path)
                .with_context(|| format!("load global config {}", path.display()))?;
            queue_file_explicit |= layer.queue.file.is_some();
            done_file_explicit |= layer.queue.done_file.is_some();
            cfg = apply_layer(cfg, layer)
                .with_context(|| format!("apply global config {}", path.display()))?;
        }
    }

    log::debug!("checking project config at: {}", project_path.display());
    if project_path.exists() {
        log::debug!("loading project config: {}", project_path.display());
        let layer = load_layer(&project_path)
            .with_context(|| format!("load project config {}", project_path.display()))?;
        queue_file_explicit |= layer.queue.file.is_some();
        done_file_explicit |= layer.queue.done_file.is_some();
        project_layer = Some(layer.clone());
        cfg = apply_layer(cfg, layer)
            .with_context(|| format!("apply project config {}", project_path.display()))?;
    }

    if validate_execution_trust {
        validate_project_execution_trust(project_layer.as_ref(), &repo_trust)?;
    }
    validate_config(&cfg)?;

    // Apply selected profile if specified
    if let Some(name) = profile {
        apply_profile_patch(&mut cfg, name)?;
        validate_config(&cfg)?;
    }

    // Validate instruction_files early for fast feedback (before runtime prompt rendering)
    if validate_instruction_files {
        validate_instruction_file_paths(&repo_root, &cfg)
            .with_context(|| "validate instruction_files from config")?;
    }

    let id_prefix = resolve_id_prefix(&cfg)?;
    let id_width = resolve_id_width(&cfg)?;
    let queue_path = resolve_queue_path_with_source(&repo_root, &cfg, queue_file_explicit)?;
    let done_path = resolve_done_path_with_source(&repo_root, &cfg, done_file_explicit)?;
    let resolved_global_path = global_layer_paths
        .iter()
        .rev()
        .find(|path| path.exists())
        .cloned()
        .or(global_path);

    log::debug!("resolved repo_root: {}", repo_root.display());
    log::debug!("resolved queue_path: {}", queue_path.display());
    log::debug!("resolved done_path: {}", done_path.display());

    Ok(Resolved {
        config: cfg,
        repo_root,
        queue_path,
        done_path,
        id_prefix,
        id_width,
        global_config_path: resolved_global_path,
        project_config_path: Some(project_path),
    })
}

/// Apply a named profile patch to the resolved config.
///
/// Profile values are merged into `cfg.agent` using leaf-wise merge semantics.
fn apply_profile_patch(cfg: &mut Config, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        bail!("Invalid --profile: name cannot be empty");
    }

    let patch =
        crate::agent::resolve_profile_patch(name, cfg.profiles.as_ref()).ok_or_else(|| {
            let names = crate::agent::all_profile_names(cfg.profiles.as_ref());
            if names.is_empty() {
                anyhow::anyhow!(
                    "Unknown profile: {name:?}. No profiles are configured. Define profiles under the `profiles` key in .cueloop/config.jsonc or legacy .ralph/config.jsonc."
                )
            } else {
                anyhow::anyhow!(
                    "Unknown profile: {name:?}. Available configured profiles: {}",
                    names.into_iter().collect::<Vec<_>>().join(", ")
                )
            }
        })?;

    cfg.agent.merge_from(patch);
    Ok(())
}

/// Resolve the queue ID prefix from config.
pub fn resolve_id_prefix(cfg: &Config) -> Result<String> {
    validate_queue_id_prefix_override(cfg.queue.id_prefix.as_deref())?;
    let raw = cfg.queue.id_prefix.as_deref().unwrap_or(DEFAULT_ID_PREFIX);
    Ok(raw.trim().to_uppercase())
}

/// Resolve the queue ID width from config.
pub fn resolve_id_width(cfg: &Config) -> Result<usize> {
    validate_queue_id_width_override(cfg.queue.id_width)?;
    Ok(cfg.queue.id_width.unwrap_or(DEFAULT_ID_WIDTH as u8) as usize)
}

/// Resolve the queue file path from config.
pub fn resolve_queue_path(repo_root: &Path, cfg: &Config) -> Result<PathBuf> {
    resolve_queue_path_with_source(repo_root, cfg, false)
}

fn resolve_queue_path_with_source(
    repo_root: &Path,
    cfg: &Config,
    queue_file_explicit: bool,
) -> Result<PathBuf> {
    validate_queue_file_override(cfg.queue.file.as_deref())?;

    let raw = default_aware_runtime_path(
        repo_root,
        cfg.queue.file.as_deref(),
        queue_file_explicit,
        QUEUE_FILE_NAME,
        DEFAULT_QUEUE_FILE,
        LEGACY_DEFAULT_QUEUE_FILE,
    );

    resolve_repo_path(repo_root, &raw)
}

/// Resolve the done file path from config.
pub fn resolve_done_path(repo_root: &Path, cfg: &Config) -> Result<PathBuf> {
    resolve_done_path_with_source(repo_root, cfg, false)
}

fn resolve_done_path_with_source(
    repo_root: &Path,
    cfg: &Config,
    done_file_explicit: bool,
) -> Result<PathBuf> {
    validate_queue_done_file_override(cfg.queue.done_file.as_deref())?;

    let raw = default_aware_runtime_path(
        repo_root,
        cfg.queue.done_file.as_deref(),
        done_file_explicit,
        DONE_FILE_NAME,
        DEFAULT_DONE_FILE,
        LEGACY_DEFAULT_DONE_FILE,
    );

    resolve_repo_path(repo_root, &raw)
}

fn default_aware_runtime_path(
    repo_root: &Path,
    configured: Option<&Path>,
    configured_explicitly: bool,
    file_name: &str,
    current_default: &str,
    legacy_default: &str,
) -> PathBuf {
    match configured {
        Some(path)
            if !configured_explicitly
                && (path == Path::new(current_default) || path == Path::new(legacy_default)) =>
        {
            default_runtime_relative_path(repo_root, file_name)
        }
        Some(path) => path.to_path_buf(),
        None => default_runtime_relative_path(repo_root, file_name),
    }
}

fn resolve_repo_path(repo_root: &Path, raw: &Path) -> Result<PathBuf> {
    let value = fsutil::expand_tilde(raw);
    Ok(if value.is_absolute() {
        value
    } else {
        repo_root.join(value)
    })
}

fn default_runtime_relative_path(repo_root: &Path, file_name: &str) -> PathBuf {
    PathBuf::from(project_runtime_layout(repo_root).directory_name()).join(file_name)
}

/// Get the path to the current global config file.
pub fn global_config_path() -> Option<PathBuf> {
    config_base_dir().map(|base| base.join(GLOBAL_CONFIG_DIR).join(CONFIG_FILE_NAME))
}

/// Get the path to the legacy global config file.
pub fn legacy_global_config_path() -> Option<PathBuf> {
    config_base_dir().map(|base| base.join(LEGACY_GLOBAL_CONFIG_DIR).join(CONFIG_FILE_NAME))
}

fn global_config_layer_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = legacy_global_config_path() {
        paths.push(path);
    }
    if let Some(path) = global_config_path()
        && !paths.iter().any(|existing| existing == &path)
    {
        paths.push(path);
    }
    paths
}

fn config_base_dir() -> Option<PathBuf> {
    if let Some(value) = env::var_os("XDG_CONFIG_HOME") {
        Some(PathBuf::from(value))
    } else {
        let home = env::var_os("HOME")?;
        Some(PathBuf::from(home).join(".config"))
    }
}

/// Get the path to the project config file for a given repo root.
pub fn project_config_path(repo_root: &Path) -> PathBuf {
    project_runtime_dir(repo_root).join(CONFIG_FILE_NAME)
}

/// Get the active repo-local runtime directory for a repository root.
pub fn project_runtime_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(project_runtime_layout(repo_root).directory_name())
}

/// Detect the active repo-local runtime layout.
pub fn project_runtime_layout(repo_root: &Path) -> ProjectRuntimeLayout {
    let current_dir = repo_root.join(PROJECT_RUNTIME_DIR);
    let legacy_dir = repo_root.join(LEGACY_PROJECT_RUNTIME_DIR);

    if has_runtime_marker(&current_dir) {
        ProjectRuntimeLayout::Current
    } else if has_runtime_marker(&legacy_dir) {
        ProjectRuntimeLayout::Legacy
    } else {
        ProjectRuntimeLayout::Uninitialized
    }
}

fn has_runtime_marker(runtime_dir: &Path) -> bool {
    runtime_dir.is_dir()
        && RUNTIME_MARKER_FILES
            .iter()
            .any(|name| runtime_dir.join(name).is_file())
}

/// Find the repository root starting from a given path.
///
/// Searches upward for current `.cueloop/` marker files, then legacy `.ralph/` marker files,
/// then a `.git/` directory.
pub fn find_repo_root(start: &Path) -> PathBuf {
    log::debug!("searching for repo root starting from: {}", start.display());
    for dir in start.ancestors() {
        log::debug!("checking directory: {}", dir.display());
        let current_dir = dir.join(PROJECT_RUNTIME_DIR);
        if has_runtime_marker(&current_dir) {
            log::debug!(
                "found repo root at: {} (via {PROJECT_RUNTIME_DIR}/)",
                dir.display()
            );
            return dir.to_path_buf();
        }

        let legacy_dir = dir.join(LEGACY_PROJECT_RUNTIME_DIR);
        if has_runtime_marker(&legacy_dir) {
            log::debug!(
                "found repo root at: {} (via {LEGACY_PROJECT_RUNTIME_DIR}/)",
                dir.display()
            );
            return dir.to_path_buf();
        }

        if dir.join(".git").exists() {
            log::debug!("found repo root at: {} (via .git/)", dir.display());
            return dir.to_path_buf();
        }
    }
    log::debug!(
        "no repo root found, using start directory: {}",
        start.display()
    );
    start.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_runtime_relative_path_matches_current_default_queue_constant() {
        let dir = TempDir::new().expect("temp dir");
        assert_eq!(
            default_runtime_relative_path(dir.path(), QUEUE_FILE_NAME),
            PathBuf::from(crate::constants::queue::DEFAULT_QUEUE_FILE)
        );
    }
}
