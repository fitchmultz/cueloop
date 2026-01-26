//! Queue unlock subcommand.

use anyhow::{Context, Result};

use crate::config::Resolved;
use crate::fsutil;

pub(crate) fn handle(resolved: &Resolved) -> Result<()> {
    let lock_dir = fsutil::queue_lock_dir(&resolved.repo_root);
    if lock_dir.exists() {
        std::fs::remove_dir_all(&lock_dir)
            .with_context(|| format!("remove lock dir {}", lock_dir.display()))?;
        log::info!("Queue unlocked (removed {}).", lock_dir.display());
    } else {
        log::info!("Queue is not locked.");
    }
    Ok(())
}
