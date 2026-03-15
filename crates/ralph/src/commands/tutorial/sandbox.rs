//! Tutorial sandbox creation helpers.
//!
//! Purpose:
//! - Build the disposable git-backed project used by the interactive tutorial.
//!
//! Responsibilities:
//! - Create a temporary directory outside the current repository.
//! - Seed the sandbox with sample Rust project files and `.gitignore`.
//! - Initialize and configure git using managed subprocess execution.
//! - Support automatic cleanup or explicit preservation via `--keep-sandbox`.
//!
//! Scope:
//! - Tutorial sandbox filesystem and git bootstrap only.
//!
//! Usage:
//! - Called by the tutorial workflow before guided CLI steps begin.
//!
//! Invariants/assumptions:
//! - The sandbox must be a valid git repository before the tutorial continues.
//! - Git command failures must surface as hard errors instead of being ignored.
//! - Preserved sandboxes must not be deleted on drop.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::runutil::{ManagedCommand, TimeoutClass, execute_checked_command};

/// Sample Rust project files for tutorial sandbox.
const SAMPLE_CARGO_TOML: &str = r#"[package]
name = "tutorial-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;

const SAMPLE_LIB_RS: &str = r#"//! Tutorial project for Ralph onboarding.
//!
//! Add your code here.

/// Returns a greeting message.
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("World"), "Hello, World!");
    }
}
"#;

fn run_tutorial_git(path: &Path, description: &str, args: &[&str]) -> Result<()> {
    let mut command = std::process::Command::new("git");
    command.current_dir(path).args(args);

    execute_checked_command(ManagedCommand::new(command, description, TimeoutClass::Git))
        .with_context(|| format!("{description} in tutorial sandbox {}", path.display()))?;
    Ok(())
}

/// A tutorial sandbox with automatic or manual cleanup.
pub struct TutorialSandbox {
    /// The temp directory (None if preserved).
    temp_dir: Option<TempDir>,
    /// The sandbox path (always available).
    pub path: PathBuf,
}

impl TutorialSandbox {
    /// Create a new tutorial sandbox with git init and sample files.
    pub fn create() -> Result<Self> {
        let temp_dir =
            TempDir::new().context("failed to create temp directory for tutorial sandbox")?;
        let path = temp_dir.path().to_path_buf();

        run_tutorial_git(
            &path,
            "initialize tutorial git repository",
            &["init", "--quiet"],
        )?;
        run_tutorial_git(
            &path,
            "configure tutorial git user.name",
            &["config", "user.name", "Ralph Tutorial"],
        )?;
        run_tutorial_git(
            &path,
            "configure tutorial git user.email",
            &["config", "user.email", "tutorial@ralph.invalid"],
        )?;

        // Create sample project files
        std::fs::write(path.join("Cargo.toml"), SAMPLE_CARGO_TOML)?;
        std::fs::create_dir_all(path.join("src"))?;
        std::fs::write(path.join("src/lib.rs"), SAMPLE_LIB_RS)?;

        // Create .gitignore
        std::fs::write(
            path.join(".gitignore"),
            "/target\n.ralph/lock\n.ralph/cache/\n.ralph/logs/\n",
        )?;

        run_tutorial_git(&path, "stage tutorial sandbox files", &["add", "."])?;
        run_tutorial_git(
            &path,
            "create tutorial sandbox initial commit",
            &["commit", "--quiet", "-m", "Initial commit"],
        )?;

        Ok(Self {
            temp_dir: Some(temp_dir),
            path,
        })
    }

    /// Keep the sandbox directory (don't delete on drop).
    pub fn preserve(mut self) -> PathBuf {
        let path = self.path.clone();
        // Take the temp_dir out and keep it to prevent cleanup
        if let Some(temp_dir) = self.temp_dir.take() {
            // Keep the directory (ignoring the Result)
            let _ = temp_dir.keep();
        }
        path
    }
}

impl Drop for TutorialSandbox {
    fn drop(&mut self) {
        // temp_dir auto-cleans when dropped (if not None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn write_fake_git(bin_dir: &Path, script: &str) {
        use std::os::unix::fs::PermissionsExt;

        let path = bin_dir.join("git");
        std::fs::write(&path, script).expect("write fake git");
        let mut perms = std::fs::metadata(&path)
            .expect("fake git metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod fake git");
    }

    #[test]
    fn sandbox_creates_files() {
        let sandbox = TutorialSandbox::create().unwrap();

        assert!(sandbox.path.join("Cargo.toml").exists());
        assert!(sandbox.path.join("src/lib.rs").exists());
        assert!(sandbox.path.join(".gitignore").exists());
        assert!(sandbox.path.join(".git").exists());
    }

    #[cfg(unix)]
    #[test]
    fn sandbox_create_fails_when_git_configuration_fails() {
        let temp = TempDir::new().unwrap();
        let bin_dir = temp.path().join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        write_fake_git(
            &bin_dir,
            r#"#!/bin/sh
if [ "$1" = "init" ]; then
  exit 0
fi
if [ "$1" = "config" ] && [ "$2" = "user.name" ]; then
  echo "fake git failing: config" >&2
  exit 7
fi
exit 0
"#,
        );

        let err =
            match crate::testsupport::path::with_prepend_path(&bin_dir, TutorialSandbox::create) {
                Ok(_) => panic!("sandbox creation should fail when git config fails"),
                Err(err) => err,
            };
        let text = format!("{err:#}");
        assert!(text.contains("configure tutorial git user.name"));
        assert!(text.contains("fake git failing: config"));
    }

    #[test]
    fn sandbox_preserve_prevents_cleanup() {
        let sandbox = TutorialSandbox::create().unwrap();
        let path = sandbox.preserve();

        // Path should still exist after preserve
        assert!(path.exists());

        // Clean up manually
        let _ = std::fs::remove_dir_all(&path);
    }
}
