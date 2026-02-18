//! README auto-update logic for sanity checks.
//!
//! Responsibilities:
//! - Check if README.md is outdated compared to embedded template
//! - Auto-update README without prompting (automatic operation)
//!
//! Not handled here:
//! - User prompts (automatic operation only)
//! - Migration handling (see migrations.rs)
//! - Unknown key detection (see unknown_keys.rs)
//!
//! Invariants:
//! - README auto-update is always automatic, never prompts user
//! - Missing README is not an error (optional file)

use crate::config::Resolved;
use anyhow::{Context, Result};

/// Check and auto-update README if needed.
///
/// Returns `Ok(Some(message))` if README was updated.
/// Returns `Ok(None)` if README is current or not applicable.
pub(crate) fn check_and_update_readme(resolved: &Resolved) -> Result<Option<String>> {
    use crate::commands::init::readme;

    match readme::check_readme_current(resolved)? {
        readme::ReadmeCheckResult::Current(version) => {
            log::debug!("README is current (version {})", version);
            Ok(None)
        }
        readme::ReadmeCheckResult::Outdated {
            current_version,
            embedded_version,
        } => {
            let readme_path = resolved.repo_root.join(".ralph/README.md");
            log::info!(
                "README is outdated (version {} < {}), updating...",
                current_version,
                embedded_version
            );

            let (status, _) =
                readme::write_readme(&readme_path, false, true).context("write updated README")?;

            if status == crate::commands::init::FileInitStatus::Updated {
                let msg = format!(
                    "Updated README from version {} to {}",
                    current_version, embedded_version
                );
                log::info!("{}", msg);
                Ok(Some(msg))
            } else {
                log::debug!("README write returned status: {:?}", status);
                Ok(None)
            }
        }
        readme::ReadmeCheckResult::Missing => {
            log::debug!("README.md is missing (optional)");
            Ok(None)
        }
        readme::ReadmeCheckResult::NotApplicable => {
            log::debug!("README.md is not applicable");
            Ok(None)
        }
    }
}
