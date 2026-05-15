//! Parallel ignored-file setup helpers for `cueloop init`.
//!
//! Purpose:
//! - Discover small gitignored repository-local files that may be useful in parallel workers.
//!
//! Responsibilities:
//! - Query Git for ignored untracked files during interactive initialization.
//! - Filter recommendations to explicit, safe file paths suitable for `parallel.ignored_file_allowlist`.
//! - Keep non-interactive initialization deterministic by providing discovery only when called.
//!
//! Scope:
//! - Recommendation discovery only; runtime worker syncing lives under `commands::run::parallel::sync`.
//!
//! Usage:
//! - The interactive init wizard calls `discover_parallel_sync_candidates` and lets the user choose entries.
//!
//! Invariants/Assumptions:
//! - `.env` and `.env.*` are intentionally excluded because parallel runtime syncs them by default.
//! - Directories, heavy/runtime paths, absolute paths, and parent traversal are never recommended.

use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::Result;

use crate::runutil::{ManagedCommand, TimeoutClass, execute_checked_command};

const MAX_RECOMMENDED_FILE_BYTES: u64 = 256 * 1024;

/// Return deterministic repo-relative ignored-file candidates for explicit parallel sync.
pub fn discover_parallel_sync_candidates(repo_root: &Path) -> Result<Vec<String>> {
    let mut command = Command::new("git");
    command.current_dir(repo_root).args([
        "ls-files",
        "--others",
        "--ignored",
        "--exclude-standard",
        "-z",
    ]);
    let output = match execute_checked_command(ManagedCommand::new(
        command,
        "git ls-files ignored local files for init parallel sync recommendations",
        TimeoutClass::Git,
    )) {
        Ok(output) => output,
        Err(error) => {
            log::debug!(
                "Skipping parallel ignored-file recommendations because git ignored-file scan failed: {error:#}"
            );
            return Ok(Vec::new());
        }
    };

    let mut candidates = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
        .filter_map(|raw| std::str::from_utf8(raw).ok())
        .filter_map(|path| normalize_candidate(repo_root, path))
        .collect::<Vec<_>>();

    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

fn normalize_candidate(repo_root: &Path, raw: &str) -> Option<String> {
    let normalized = raw.trim().trim_start_matches("./");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains('\\')
        || normalized.contains('\0')
        || normalized.contains("**")
        || is_default_env_sync(normalized)
        || is_denied_candidate(normalized)
        || crate::commands::run::parallel::sync::is_denied_parallel_ignored_sync_path(normalized)
        || has_parent_or_prefix_component(normalized)
    {
        return None;
    }

    let path = repo_root.join(normalized);
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_RECOMMENDED_FILE_BYTES {
        return None;
    }

    Some(normalized.to_string())
}

fn is_default_env_sync(path: &str) -> bool {
    path == ".env" || path.starts_with(".env.")
}

fn has_parent_or_prefix_component(path: &str) -> bool {
    PathBuf::from(path).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

fn is_denied_candidate(path: &str) -> bool {
    path.starts_with(".git/")
        || path.starts_with(".cueloop/cache/")
        || path.starts_with(".cueloop/logs/")
        || path.starts_with(".cueloop/workspaces/")
        || path.starts_with("target/")
        || path.starts_with("node_modules/")
        || path.starts_with(".venv/")
        || path.starts_with("__pycache__/")
        || path.starts_with(".ruff_cache/")
        || path.starts_with(".pytest_cache/")
        || path.starts_with(".ty_cache/")
        || path.starts_with("build/")
        || path.starts_with("dist/")
        || path.starts_with("out/")
        || path.starts_with("coverage/")
        || path.ends_with(".db")
        || path.ends_with(".sqlite")
        || path.ends_with(".sqlite3")
        || path.ends_with(".pem")
        || path.ends_with(".key")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn normalize_candidate_filters_default_env_and_unsafe_paths() -> Result<()> {
        let temp = TempDir::new()?;
        std::fs::write(temp.path().join("local-tool.json"), "{}")?;
        std::fs::write(temp.path().join(".env.local"), "TOKEN=x")?;
        std::fs::create_dir_all(temp.path().join("node_modules/zod"))?;
        std::fs::write(temp.path().join("node_modules/zod/index.js"), "module")?;
        std::fs::create_dir_all(temp.path().join(".venv/bin"))?;
        std::fs::write(temp.path().join(".venv/bin/python"), "python")?;

        assert_eq!(
            normalize_candidate(temp.path(), "local-tool.json"),
            Some("local-tool.json".to_string())
        );
        assert_eq!(normalize_candidate(temp.path(), ".env.local"), None);
        assert_eq!(normalize_candidate(temp.path(), "../secret"), None);
        assert_eq!(normalize_candidate(temp.path(), "target/cache.json"), None);
        assert_eq!(
            normalize_candidate(temp.path(), "node_modules/zod/index.js"),
            None
        );
        assert_eq!(normalize_candidate(temp.path(), ".venv/bin/python"), None);
        Ok(())
    }

    #[test]
    fn normalize_candidate_rejects_directories_and_large_files() -> Result<()> {
        let temp = TempDir::new()?;
        std::fs::create_dir(temp.path().join("local-dir"))?;
        std::fs::write(
            temp.path().join("large.local"),
            vec![b'x'; (MAX_RECOMMENDED_FILE_BYTES + 1) as usize],
        )?;

        assert_eq!(normalize_candidate(temp.path(), "local-dir"), None);
        assert_eq!(normalize_candidate(temp.path(), "large.local"), None);
        Ok(())
    }
}
