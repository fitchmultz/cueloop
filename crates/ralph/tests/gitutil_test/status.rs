//! Purpose: status, porcelain, path parsing, and LFS-related integration coverage for `ralph::git`.
//!
//! Responsibilities:
//! - Verify porcelain output for clean, dirty, and non-repo cases.
//! - Verify `status_paths()` path extraction across tracked, untracked, rename, and special-character cases.
//! - Verify LFS helper behavior covered by this suite.
//!
//! Scope:
//! - Public status/LFS APIs exercised through disposable test repositories.
//!
//! Usage:
//! - Uses `use super::*;` for shared imports and suite-local repo helpers.
//!
//! Invariants/Assumptions:
//! - Test names and assertions remain unchanged from the original monolithic suite.
//! - The non-git-directory case stays grouped here because it validates status API behavior.

use super::*;

#[test]
fn test_status_porcelain_clean_repo() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let status = git::status_porcelain(dir.path()).unwrap();
    assert!(status.trim().is_empty());
}

#[test]
fn test_status_porcelain_with_changes() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "content").expect("failed to write file");
    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(dir.path())
        .output()
        .expect("git add failed");

    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()
        .expect("git commit failed");

    fs::write(&file_path, "modified content").expect("failed to modify file");

    let status = git::status_porcelain(dir.path()).unwrap();
    assert!(!status.trim().is_empty());
    assert!(status.contains("M"));
}

#[test]
fn test_status_porcelain_with_untracked_files() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let file_path = dir.path().join("untracked.txt");
    fs::write(&file_path, "content").expect("failed to write file");

    let status = git::status_porcelain(dir.path()).unwrap();
    assert!(!status.trim().is_empty());
    assert!(status.contains("??"));
}

#[test]
fn test_status_paths_includes_tracked_and_untracked() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "tracked.txt", "content", "initial");
    fs::write(dir.path().join("tracked.txt"), "modified").expect("modify tracked");
    fs::write(dir.path().join("untracked.txt"), "new").expect("create untracked");

    let paths = git::status_paths(dir.path()).expect("status paths");
    assert!(paths.contains(&"tracked.txt".to_string()));
    assert!(paths.contains(&"untracked.txt".to_string()));
}

#[test]
fn test_status_paths_handles_paths_with_spaces() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "file with spaces.txt", "content", "initial");
    fs::write(dir.path().join("file with spaces.txt"), "modified").expect("modify tracked");
    fs::write(dir.path().join("untracked file.txt"), "new").expect("create untracked");

    let paths = git::status_paths(dir.path()).expect("status paths");
    assert!(
        paths.contains(&"file with spaces.txt".to_string()),
        "expected modified tracked file with spaces"
    );
    assert!(
        paths.contains(&"untracked file.txt".to_string()),
        "expected untracked file with spaces"
    );
}

#[test]
fn test_status_paths_returns_new_path_for_renames_with_spaces() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "old name.txt", "content", "initial");

    let mv_status = Command::new("git")
        .args(["mv", "old name.txt", "new name.txt"])
        .current_dir(dir.path())
        .status()
        .expect("git mv failed");
    assert!(mv_status.success(), "git mv should succeed");

    let paths = git::status_paths(dir.path()).expect("status paths");
    assert!(
        paths.contains(&"new name.txt".to_string()),
        "expected new rename destination path"
    );
    assert!(
        !paths.contains(&"old name.txt".to_string()),
        "should not return old rename source path (API compatibility requirement)"
    );
}

#[cfg(unix)]
#[test]
fn test_status_paths_handles_paths_with_newlines_and_tabs() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let newline_name = "line1\nline2.txt";
    let tab_name = "tab\tname.txt";

    fs::write(dir.path().join(newline_name), "content").expect("write newline file");
    fs::write(dir.path().join(tab_name), "content").expect("write tab file");

    let paths = git::status_paths(dir.path()).expect("status paths");
    assert!(
        paths.contains(&newline_name.to_string()),
        "expected newline-containing path to be parsed via -z"
    );
    assert!(
        paths.contains(&tab_name.to_string()),
        "expected tab-containing path to be parsed via -z"
    );
}

#[test]
fn test_filter_modified_lfs_files_intersects_lists() {
    let status_paths = vec![
        "assets/large.bin".to_string(),
        "notes.txt".to_string(),
        "media/video.mov".to_string(),
    ];
    let lfs_files = vec![
        "assets/large.bin".to_string(),
        "media/video.mov".to_string(),
    ];
    let modified = git::filter_modified_lfs_files(&status_paths, &lfs_files);
    assert_eq!(
        modified,
        vec![
            "assets/large.bin".to_string(),
            "media/video.mov".to_string()
        ]
    );
}

#[test]
fn test_has_lfs_detects_gitattributes_filter() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let attrs = dir.path().join(".gitattributes");
    fs::write(&attrs, "*.bin filter=lfs diff=lfs merge=lfs -text\n").expect("write gitattributes");

    let has_lfs = git::has_lfs(dir.path()).expect("has lfs");
    assert!(has_lfs);
}

#[test]
fn test_status_porcelain_non_git_directory() {
    let dir = super::test_support::temp_dir_outside_repo();

    let result = git::status_porcelain(dir.path());
    assert!(result.is_err());
}
