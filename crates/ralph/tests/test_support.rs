//! Shared helpers for integration tests that need repo-aware temp paths.

use ralph::config;
use std::env;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub fn path_has_repo_markers(path: &Path) -> bool {
    path.ancestors()
        .any(|dir| dir.join(".git").exists() || dir.join(".ralph").is_dir())
}

pub fn find_non_repo_temp_base() -> PathBuf {
    let cwd = env::current_dir().expect("resolve current dir");
    let repo_root = config::find_repo_root(&cwd);
    let mut candidates = Vec::new();
    if let Some(parent) = repo_root.parent() {
        candidates.push(parent.to_path_buf());
    }
    candidates.push(env::temp_dir());
    candidates.push(PathBuf::from("/tmp"));

    for candidate in candidates {
        if candidate.as_os_str().is_empty() {
            continue;
        }
        if !path_has_repo_markers(&candidate) {
            return candidate;
        }
    }

    repo_root
}

pub fn temp_dir_outside_repo() -> TempDir {
    let base = find_non_repo_temp_base();
    std::fs::create_dir_all(&base).expect("ensure temp base exists");
    TempDir::new_in(&base).expect("create temp dir outside repo")
}
