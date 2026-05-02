//! Portable path and environment helpers for integration tests.
//!
//! Temp roots stay outside repo markers, and process-wide environment mutations must hold
//! `env_lock()`.
//! Tests that spawn nested parallel workers should hold `parallel_run_lock()` only for the
//! overlapping run window.

use cueloop::config;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

pub fn path_has_repo_markers(path: &Path) -> bool {
    path.ancestors()
        .any(|dir| dir.join(".git").exists() || dir.join(".cueloop").is_dir())
}

pub fn find_non_repo_temp_base() -> PathBuf {
    let cwd = std::env::current_dir().expect("resolve current dir");
    let repo_root = config::find_repo_root(&cwd);

    let temp_base = cueloop::fsutil::cueloop_temp_root().join("integration-tests");
    if !path_has_repo_markers(&temp_base) {
        return temp_base;
    }

    if let Some(parent) = repo_root.parent()
        && !path_has_repo_markers(parent)
    {
        return parent.join(".cueloop-integration-tests");
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

pub fn parallel_run_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
