//! Tests for supervising-process detection of TUI lock owners.

use anyhow::Result;
use ralph::fsutil;
use tempfile::TempDir;

#[test]
fn supervising_process_detects_tui_label() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let lock_dir = temp_dir.path().join("lock");
    std::fs::create_dir_all(&lock_dir)?;
    let owner_path = lock_dir.join("owner");
    let owner = "pid: 123\nstarted_at: now\ncommand: ralph tui\nlabel: tui\n";
    std::fs::write(&owner_path, owner)?;

    assert!(fsutil::is_supervising_process(&lock_dir)?);
    Ok(())
}
