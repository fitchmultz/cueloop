//! Purpose: shared command and repo helpers for integration tests.
//!
//! Responsibilities:
//! - Resolve the built `ralph` binary and run isolated subprocesses for tests.
//! - Initialize disposable git repos and executable fixtures.
//! - Seed reusable runtime scaffolding and cached git+runtime repo templates.
//! - Provide scoped PATH mutation utilities for fake toolchains.
//!
//! Scope:
//! - Test-only process, git, and fixture bootstrap helpers used by Rust integration suites.
//!
//! Usage:
//! - Prefer `seed_ralph_dir()` when a test needs legacy `.ralph/` runtime fixtures.
//! - Prefer `seed_git_repo_with_ralph()` when a suite repeatedly needs the same initialized git repo.
//! - Use `run_in_dir()`/`ralph_command()` for CLI execution and `create_fake_runner()` for fake runner binaries.
//!
//! Invariants/assumptions callers must respect:
//! - Callers that need cross-test PATH isolation must hold `env_lock()` while using `with_prepend_path`.
//! - Executable fixture helpers mark scripts executable only on Unix hosts.
//! - `ralph_init()` invokes the real CLI and may overwrite queue files; tests that need pre-seeded fixtures should call `seed_ralph_dir()` or `seed_git_repo_with_ralph()` instead.

use anyhow::{Context, Result};
use ralph::config::project_runtime_dir;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::OnceLock;

const TEST_GIT_USER_NAME: &str = "Ralph Test";
const TEST_GIT_USER_EMAIL: &str = "ralph-tests@example.invalid";

static RALPH_BIN_PATH: OnceLock<PathBuf> = OnceLock::new();
static EMPTY_GIT_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();
static RALPH_INIT_TEMPLATE_DIR: OnceLock<PathBuf> = OnceLock::new();
static SEEDED_GIT_RALPH_TEMPLATE_DIR: OnceLock<PathBuf> = OnceLock::new();

fn resolve_ralph_bin() -> PathBuf {
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_ralph") {
        return PathBuf::from(path);
    }

    let exe = std::env::current_exe().expect("resolve current test executable path");
    let exe_dir = exe
        .parent()
        .expect("test executable should have a parent directory");
    let profile_dir = if exe_dir.file_name() == Some(std::ffi::OsStr::new("deps")) {
        exe_dir
            .parent()
            .expect("deps directory should have a parent directory")
    } else {
        exe_dir
    };

    let bin_name = if cfg!(windows) { "ralph.exe" } else { "ralph" };
    let candidate = profile_dir.join(bin_name);
    if candidate.exists() {
        return candidate;
    }

    panic!(
        "CARGO_BIN_EXE_ralph was not set and fallback binary path does not exist: {}",
        candidate.display()
    );
}

pub fn ralph_bin() -> PathBuf {
    RALPH_BIN_PATH.get_or_init(resolve_ralph_bin).clone()
}

fn empty_git_config_path() -> &'static PathBuf {
    EMPTY_GIT_CONFIG_PATH.get_or_init(|| {
        let tempfile = tempfile::Builder::new()
            .prefix("ralph-empty-gitconfig.")
            .tempfile()
            .expect("create empty git config");
        let (_file, path) = tempfile.keep().expect("persist empty git config");
        std::fs::write(&path, "").expect("write empty git config");
        path
    })
}

fn git_command(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", empty_git_config_path())
        .env("GIT_AUTHOR_NAME", TEST_GIT_USER_NAME)
        .env("GIT_AUTHOR_EMAIL", TEST_GIT_USER_EMAIL)
        .env("GIT_COMMITTER_NAME", TEST_GIT_USER_NAME)
        .env("GIT_COMMITTER_EMAIL", TEST_GIT_USER_EMAIL);
    cmd
}

fn run_git(dir: &Path, context: &'static str, args: &[&str]) -> Result<()> {
    let status = git_command(dir).args(args).status().context(context)?;
    anyhow::ensure!(status.success(), "{context} failed");
    Ok(())
}

fn copy_dir_recursive_missing_only(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("create {}", dst.display()))?;

    for entry in std::fs::read_dir(src).with_context(|| format!("read {}", src.display()))? {
        let entry = entry?;
        let entry_path = entry.path();
        let target_path = dst.join(entry.file_name());
        let file_type = entry
            .file_type()
            .with_context(|| format!("read type for {}", entry_path.display()))?;

        if file_type.is_dir() {
            copy_dir_recursive_missing_only(&entry_path, &target_path)?;
            continue;
        }

        if file_type.is_file() && !target_path.exists() {
            std::fs::copy(&entry_path, &target_path).with_context(|| {
                format!("copy {} to {}", entry_path.display(), target_path.display())
            })?;
        }
    }

    Ok(())
}

fn run_ralph_init_cli(dir: &Path) -> Result<()> {
    let (status, stdout, stderr) = run_in_dir(dir, &["init", "--force", "--non-interactive"]);
    anyhow::ensure!(
        status.success(),
        "ralph init failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    Ok(())
}

fn ralph_init_template_dir() -> &'static PathBuf {
    RALPH_INIT_TEMPLATE_DIR.get_or_init(|| {
        let template_dir = tempfile::Builder::new()
            .prefix("ralph-init-template.")
            .tempdir()
            .expect("create ralph init template dir");
        let template_path = template_dir.keep();
        run_ralph_init_cli(&template_path).expect("seed ralph init template");
        template_path
    })
}

fn seeded_git_ralph_template_dir() -> &'static PathBuf {
    SEEDED_GIT_RALPH_TEMPLATE_DIR.get_or_init(|| {
        let template_dir = tempfile::Builder::new()
            .prefix("ralph-git-runtime-template.")
            .tempdir()
            .expect("create cached git + runtime template dir");
        let template_path = template_dir.keep();
        git_init(&template_path).expect("initialize cached git repo");
        seed_ralph_dir(&template_path).expect("seed cached runtime fixture");
        template_path
    })
}

fn mark_executable_if_unix(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)?;
    }

    Ok(())
}

pub fn run_in_dir(dir: &Path, args: &[&str]) -> (ExitStatus, String, String) {
    let output = Command::new(ralph_bin())
        .current_dir(dir)
        .env_remove("RUST_LOG")
        .args(args)
        .output()
        .expect("failed to execute ralph binary");
    (
        output.status,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// Create a ralph Command with proper environment isolation.
pub fn ralph_command(dir: &Path) -> Command {
    let mut cmd = Command::new(ralph_bin());
    cmd.current_dir(dir).env_remove("RUST_LOG");
    cmd
}

pub fn git_init(dir: &Path) -> Result<()> {
    run_git(dir, "run git init", &["init", "--quiet", "-b", "main"])?;

    let gitignore_path = dir.join(".gitignore");
    std::fs::write(
        &gitignore_path,
        ".cueloop/lock\n.cueloop/cache/\n.cueloop/logs/\n.ralph/lock\n.ralph/cache/\n.ralph/logs/\n",
    )
    .context("write .gitignore")?;

    run_git(dir, "git add .gitignore", &["add", ".gitignore"])?;
    run_git(
        dir,
        "git commit .gitignore",
        &["commit", "--quiet", "-m", "add gitignore"],
    )?;

    Ok(())
}

pub fn trust_project_commands(dir: &Path) -> Result<()> {
    let runtime_dir = project_runtime_dir(dir);
    std::fs::create_dir_all(&runtime_dir).context("create runtime dir")?;
    std::fs::write(
        runtime_dir.join("trust.jsonc"),
        r#"{
  "allow_project_commands": true,
  "trusted_at": "2026-03-07T00:00:00Z"
}
"#,
    )
    .context("write trust config")?;
    Ok(())
}

pub fn create_fake_runner(dir: &Path, runner: &str, script: &str) -> Result<PathBuf> {
    let bin_dir = dir.join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    let runner_path = bin_dir.join(runner);
    std::fs::write(&runner_path, script)?;
    mark_executable_if_unix(&runner_path)?;
    Ok(runner_path)
}

pub fn create_executable_script(dir: &Path, name: &str, script: &str) -> Result<PathBuf> {
    let path = dir.join(name);
    std::fs::write(&path, script)?;
    mark_executable_if_unix(&path)?;
    Ok(path)
}

pub fn run_in_dir_raw(dir: &Path, bin: &str, args: &[&str]) -> (ExitStatus, String, String) {
    let output = Command::new(bin)
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|_| panic!("failed to execute binary: {}", bin));
    (
        output.status,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

pub fn git_add_all_commit(dir: &Path, message: &str) -> Result<()> {
    run_git(dir, "git add all", &["add", "."])?;
    run_git(dir, "git commit", &["commit", "--quiet", "-m", message])?;
    Ok(())
}

pub fn git_status_porcelain(dir: &Path) -> Result<String> {
    let output = git_command(dir)
        .args(["status", "--porcelain"])
        .output()
        .context("git status --porcelain")?;
    anyhow::ensure!(output.status.success(), "git status --porcelain failed");
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Initialize a Ralph project by invoking the real CLI.
pub fn ralph_init(dir: &Path) -> Result<()> {
    run_ralph_init_cli(dir)
}

/// Initialize a Ralph project by invoking the real CLI.
pub fn ralph_init_cli(dir: &Path) -> Result<()> {
    run_ralph_init_cli(dir)
}

/// Seed legacy `.ralph/` from a cached template while preserving files already written by the test.
pub fn seed_ralph_dir(dir: &Path) -> Result<()> {
    let target = dir.join(".ralph");
    let template = ralph_init_template_dir().join(".cueloop");
    copy_dir_recursive_missing_only(&template, &target)
}

/// Seed an empty disposable directory from a cached git repo plus cached legacy runtime scaffold.
///
/// This avoids repeated `git init` subprocesses in command-heavy integration suites.
pub fn seed_git_repo_with_ralph(dir: &Path) -> Result<()> {
    copy_dir_recursive_missing_only(seeded_git_ralph_template_dir(), dir)
}

/// Run a closure with a prepended path segment.
///
/// The PATH is restored after the closure completes, even if it panics.
/// This is safe because we use env_lock to prevent concurrent access.
///
/// # Safety
/// This function uses unsafe to call `std::env::set_var`. The caller must ensure
/// that `env_lock()` is held to prevent concurrent modifications.
pub fn with_prepend_path<F, T>(prepend: &Path, f: F) -> T
where
    F: FnOnce() -> T,
{
    let original = std::env::var("PATH").unwrap_or_default();
    let new_path = if cfg!(windows) {
        format!("{};{}", prepend.display(), original)
    } else {
        format!("{}:{}", prepend.display(), original)
    };

    struct PathGuard(String);
    impl Drop for PathGuard {
        fn drop(&mut self) {
            #[allow(unused_unsafe)]
            unsafe {
                std::env::set_var("PATH", &self.0);
            }
        }
    }
    let _guard = PathGuard(original.clone());

    #[allow(unused_unsafe)]
    unsafe {
        std::env::set_var("PATH", &new_path);
    }
    f()
}
