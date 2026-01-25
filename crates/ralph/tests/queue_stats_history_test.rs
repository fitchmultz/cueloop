//! Integration tests for queue stats/history/burndown report output.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

mod test_support;

fn ralph_bin() -> PathBuf {
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

fn run_in_dir(dir: &Path, args: &[&str]) -> (ExitStatus, String, String) {
    let output = Command::new(ralph_bin())
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to execute ralph binary");
    (
        output.status,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn init_repo(dir: &Path) -> Result<()> {
    let (status, stdout, stderr) = run_in_dir(dir, &["init", "--force"]);
    anyhow::ensure!(
        status.success(),
        "ralph init failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    Ok(())
}

fn write_empty_queue(dir: &Path) -> Result<()> {
    let queue = r#"{
  "version": 1,
  "tasks": []
}"#;
    let done = r#"{
  "version": 1,
  "tasks": []
}"#;

    std::fs::write(dir.join(".ralph/queue.json"), queue).context("write queue.json")?;
    std::fs::write(dir.join(".ralph/done.json"), done).context("write done.json")?;
    Ok(())
}

fn format_date_key(dt: OffsetDateTime) -> String {
    format!("{:04}-{:02}-{:02}", dt.year(), dt.month() as u8, dt.day())
}

#[test]
fn burndown_empty_window_reports_no_remaining_tasks() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    init_repo(dir.path())?;
    write_empty_queue(dir.path())?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["queue", "burndown", "--days", "2"]);
    anyhow::ensure!(
        status.success(),
        "expected burndown to succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    anyhow::ensure!(
        stdout.contains("No remaining tasks in the last 2 days."),
        "expected empty-window message\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    anyhow::ensure!(
        !stdout.contains("Remaining Tasks"),
        "expected no Remaining Tasks header when empty\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    anyhow::ensure!(
        !stdout.contains("█ ="),
        "expected no legend when empty\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    Ok(())
}

#[test]
fn burndown_zero_count_day_renders_empty_bar() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    init_repo(dir.path())?;

    let now = OffsetDateTime::now_utc();
    let created_at = now.format(&Rfc3339).context("format created_at")?;
    let queue = format!(
        r#"{{
  "version": 1,
  "tasks": [
    {{
      "id": "RQ-0001",
      "status": "todo",
      "title": "Open task",
      "priority": "medium",
      "tags": ["reports"],
      "scope": ["crates/ralph"],
      "evidence": ["test"],
      "plan": ["verify"],
      "request": "burndown",
      "created_at": "{created_at}",
      "updated_at": "{created_at}"
    }}
  ]
}}"#
    );
    let done = r#"{
  "version": 1,
  "tasks": []
}"#;

    std::fs::write(dir.path().join(".ralph/queue.json"), queue).context("write queue.json")?;
    std::fs::write(dir.path().join(".ralph/done.json"), done).context("write done.json")?;

    let (status, stdout, stderr) = run_in_dir(dir.path(), &["queue", "burndown", "--days", "2"]);
    anyhow::ensure!(
        status.success(),
        "expected burndown to succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let today_key = format_date_key(now);
    let yesterday_key = format_date_key(now - Duration::days(1));
    let today_prefix = format!("  {today_key} | ");
    let yesterday_prefix = format!("  {yesterday_key} | ");

    let yesterday_line = stdout
        .lines()
        .find(|line| line.starts_with(&yesterday_prefix))
        .context("find yesterday burndown line")?;
    anyhow::ensure!(
        !yesterday_line.contains('█'),
        "expected no bar for zero-count day\nline: {yesterday_line}\nstdout:\n{stdout}"
    );
    anyhow::ensure!(
        yesterday_line.trim_end().ends_with(" 0"),
        "expected yesterday count to be 0\nline: {yesterday_line}\nstdout:\n{stdout}"
    );

    let today_line = stdout
        .lines()
        .find(|line| line.starts_with(&today_prefix))
        .context("find today burndown line")?;
    anyhow::ensure!(
        today_line.contains('█'),
        "expected bar for non-zero day\nline: {today_line}\nstdout:\n{stdout}"
    );
    anyhow::ensure!(
        today_line.trim_end().ends_with(" 1"),
        "expected today count to be 1\nline: {today_line}\nstdout:\n{stdout}"
    );

    Ok(())
}
