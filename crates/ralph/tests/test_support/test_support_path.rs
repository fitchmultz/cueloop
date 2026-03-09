//! Portable path and environment helpers for integration tests.
//!
//! Responsibilities:
//! - Resolve temp roots that stay outside repo markers and work across platforms.
//! - Provide a shared environment lock for tests that mutate process-wide variables.
//! - Centralize path derivation used by both Rust and CLI integration fixtures.
//!
//! Does not handle:
//! - Queue fixtures, command execution, or synchronization primitives.
//!
//! Invariants/assumptions callers must respect:
//! - Returned portable paths may not exist yet; callers create directories when needed.
//! - Environment mutations must hold `env_lock()` for the full mutation scope.

use ralph::config;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

pub fn path_has_repo_markers(path: &Path) -> bool {
    path.ancestors()
        .any(|dir| dir.join(".git").exists() || dir.join(".ralph").is_dir())
}

pub fn find_non_repo_temp_base() -> PathBuf {
    let cwd = std::env::current_dir().expect("resolve current dir");
    let repo_root = config::find_repo_root(&cwd);

    let temp_base = ralph::fsutil::ralph_temp_root().join("integration-tests");
    if !path_has_repo_markers(&temp_base) {
        return temp_base;
    }

    if let Some(parent) = repo_root.parent()
        && !path_has_repo_markers(parent)
    {
        return parent.join(".ralph-integration-tests");
    }

    panic!(
        "failed to find a portable temp base outside repo markers for {}",
        repo_root.display()
    );
}

pub fn temp_dir_outside_repo() -> TempDir {
    let base = find_non_repo_temp_base();
    std::fs::create_dir_all(&base).expect("ensure temp base exists");
    TempDir::new_in(&base).expect("create temp dir outside repo")
}

pub fn portable_abs_path(label: impl AsRef<Path>) -> PathBuf {
    find_non_repo_temp_base().join(label)
}

pub fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
