//! Bookkeeping path sync tests for parallel workspace state synchronization.
//!
//! Purpose:
//! - Bookkeeping path sync tests for parallel workspace state synchronization.
//!
//! Responsibilities:
//! - Verify custom queue/done paths are mapped into worker workspaces.
//! - Verify missing bookkeeping files remain a no-op when allowed.
//! - Verify `.jsonc` migration seeding behavior for uncommitted local config states.
//!
//! Non-scope:
//! - `.cueloop` runtime-tree traversal coverage.
//! - Gitignored allowlist filtering rules.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants:
//! - Test names and assertions match the prior flat suite exactly.
//! - Queue/done expectations are asserted from on-disk workspace state.

use super::*;

#[test]
fn sync_cueloop_state_custom_queue_done_paths_are_synced() -> Result<()> {
    let temp = TempDir::new()?;
    let repo_root = temp.path().join("repo");
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&repo_root)?;
    git_test::init_repo(&repo_root)?;

    let queue_path = repo_root.join("queue/active.json");
    let done_path = repo_root.join("archive/done.json");
    fs::create_dir_all(queue_path.parent().unwrap())?;
    fs::create_dir_all(done_path.parent().unwrap())?;
    fs::write(&queue_path, "{custom_queue}")?;
    fs::write(&done_path, "{custom_done}")?;

    fs::create_dir_all(repo_root.join(".cueloop/prompts"))?;
    fs::write(repo_root.join(".cueloop/config.json"), "{config}")?;
    fs::write(repo_root.join(".cueloop/prompts/override.md"), "prompt")?;
    fs::create_dir_all(&workspace_root)?;

    let resolved = build_test_resolved(&repo_root, Some(queue_path), Some(done_path));
    sync_cueloop_state(&resolved, &workspace_root)?;

    assert_eq!(
        fs::read_to_string(workspace_root.join("queue/active.json"))?,
        "{custom_queue}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("archive/done.json"))?,
        "{custom_done}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join(".cueloop/config.json"))?,
        "{config}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join(".cueloop/prompts/override.md"))?,
        "prompt"
    );
    Ok(())
}

#[test]
fn sync_cueloop_state_custom_queue_done_paths_inside_cueloop_are_synced() -> Result<()> {
    let temp = TempDir::new()?;
    let repo_root = temp.path().join("repo");
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&repo_root)?;
    git_test::init_repo(&repo_root)?;

    let queue_path = repo_root.join(".cueloop/data/queue.jsonc");
    let done_path = repo_root.join(".cueloop/data/done.json");
    fs::create_dir_all(queue_path.parent().unwrap())?;
    fs::write(&queue_path, "{custom_queue}")?;
    fs::write(&done_path, "{custom_done}")?;

    fs::create_dir_all(repo_root.join(".cueloop/prompts"))?;
    fs::write(repo_root.join(".cueloop/config.jsonc"), "{config}")?;
    fs::write(repo_root.join(".cueloop/prompts/override.md"), "prompt")?;
    fs::create_dir_all(&workspace_root)?;

    let resolved = build_test_resolved(&repo_root, Some(queue_path), Some(done_path));
    sync_cueloop_state(&resolved, &workspace_root)?;

    assert_eq!(
        fs::read_to_string(workspace_root.join(".cueloop/data/queue.jsonc"))?,
        "{custom_queue}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join(".cueloop/data/done.json"))?,
        "{custom_done}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join(".cueloop/config.jsonc"))?,
        "{config}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join(".cueloop/prompts/override.md"))?,
        "prompt"
    );
    Ok(())
}

#[test]
fn sync_cueloop_state_missing_done_file_allowed() -> Result<()> {
    let temp = TempDir::new()?;
    let repo_root = temp.path().join("repo");
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&repo_root)?;
    git_test::init_repo(&repo_root)?;

    let queue_path = repo_root.join("queue/active.json");
    fs::create_dir_all(queue_path.parent().unwrap())?;
    fs::write(&queue_path, "{queue}")?;

    fs::create_dir_all(repo_root.join(".cueloop"))?;
    fs::write(repo_root.join(".cueloop/config.json"), "{config}")?;
    fs::create_dir_all(&workspace_root)?;

    let done_path = repo_root.join("archive/done.json");
    let resolved = build_test_resolved(&repo_root, Some(queue_path), Some(done_path));
    sync_cueloop_state(&resolved, &workspace_root)?;

    assert_eq!(
        fs::read_to_string(workspace_root.join("queue/active.json"))?,
        "{queue}"
    );
    assert!(!workspace_root.join("archive/done.json").exists());
    Ok(())
}

#[test]
fn sync_cueloop_state_seeds_jsonc_bookkeeping_for_migrated_uncommitted_config() -> Result<()> {
    let temp = TempDir::new()?;
    let repo_root = temp.path().join("repo");
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&repo_root)?;
    git_test::init_repo(&repo_root)?;

    fs::create_dir_all(repo_root.join(".cueloop"))?;
    fs::write(repo_root.join(".cueloop/queue.json"), "{legacy_queue}")?;
    fs::write(repo_root.join(".cueloop/done.json"), "{legacy_done}")?;
    fs::write(repo_root.join(".cueloop/queue.jsonc"), "{migrated_queue}")?;
    fs::write(repo_root.join(".cueloop/done.jsonc"), "{migrated_done}")?;
    fs::write(repo_root.join(".cueloop/config.jsonc"), "{config}")?;

    fs::create_dir_all(workspace_root.join(".cueloop"))?;
    fs::write(
        workspace_root.join(".cueloop/queue.json"),
        "{legacy_workspace_queue}",
    )?;
    fs::write(
        workspace_root.join(".cueloop/done.json"),
        "{legacy_workspace_done}",
    )?;

    let resolved = build_test_resolved(
        &repo_root,
        Some(repo_root.join(".cueloop/queue.jsonc")),
        Some(repo_root.join(".cueloop/done.jsonc")),
    );
    sync_cueloop_state(&resolved, &workspace_root)?;

    assert_eq!(
        fs::read_to_string(workspace_root.join(".cueloop/queue.jsonc"))?,
        "{migrated_queue}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join(".cueloop/done.jsonc"))?,
        "{migrated_done}"
    );
    assert!(workspace_root.join(".cueloop/queue.json").exists());
    assert!(workspace_root.join(".cueloop/done.json").exists());

    Ok(())
}
