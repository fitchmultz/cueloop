//! Sandbox creation utilities for tutorial.
//!
//! Responsibilities:
//! - Create temporary directory outside repo.
//! - Initialize git repository with sample files.
//! - Provide cleanup handling with --keep-sandbox support.

use anyhow::{Context, Result};
use std::path::PathBuf;
use tempfile::TempDir;

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

        // Initialize git repo
        let status = std::process::Command::new("git")
            .current_dir(&path)
            .args(["init", "--quiet"])
            .status()
            .context("run git init")?;
        anyhow::ensure!(status.success(), "git init failed");

        // Configure git user
        std::process::Command::new("git")
            .current_dir(&path)
            .args(["config", "user.name", "Ralph Tutorial"])
            .status()
            .context("set git user.name")?;
        std::process::Command::new("git")
            .current_dir(&path)
            .args(["config", "user.email", "tutorial@ralph.invalid"])
            .status()
            .context("set git user.email")?;

        // Create sample project files
        std::fs::write(path.join("Cargo.toml"), SAMPLE_CARGO_TOML)?;
        std::fs::create_dir_all(path.join("src"))?;
        std::fs::write(path.join("src/lib.rs"), SAMPLE_LIB_RS)?;

        // Create .gitignore
        std::fs::write(
            path.join(".gitignore"),
            "/target\n.ralph/lock\n.ralph/cache/\n.ralph/logs/\n",
        )?;

        // Initial commit
        std::process::Command::new("git")
            .current_dir(&path)
            .args(["add", "."])
            .status()
            .context("git add")?;
        std::process::Command::new("git")
            .current_dir(&path)
            .args(["commit", "--quiet", "-m", "Initial commit"])
            .status()
            .context("git commit")?;

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

    #[test]
    fn sandbox_creates_files() {
        let sandbox = TutorialSandbox::create().unwrap();

        assert!(sandbox.path.join("Cargo.toml").exists());
        assert!(sandbox.path.join("src/lib.rs").exists());
        assert!(sandbox.path.join(".gitignore").exists());
        assert!(sandbox.path.join(".git").exists());
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
