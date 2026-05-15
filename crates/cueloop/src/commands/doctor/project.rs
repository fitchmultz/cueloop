//! Project environment checks for the doctor command.
//!
//! Purpose:
//! - Project environment checks for the doctor command.
//!
//! Responsibilities:
//! - Verify CI-gate prerequisites in the current repository.
//! - Check `.gitignore` for sensitive log entries.
//! - Apply safe auto-fixes for gitignore issues.
//!
//! Not handled here:
//! - Build system validation beyond the configured/local CI entrypoint.
//! - Dependency management.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - `.cueloop/logs/` should always be gitignored to prevent secret leakage.
//! - Auto-fixes are conservative and idempotent.
//! - Make-based CI gates should narrate CI-blocking reasons through the canonical blocking contract.

use crate::commands::doctor::types::{CheckResult, DoctorReport};
use crate::config;
use crate::contracts::{BlockingReason, BlockingState, BlockingStatus};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

fn doctor_ci_blocked(
    pattern: &str,
    message: impl Into<String>,
    detail: impl Into<String>,
) -> BlockingState {
    BlockingState::new(
        BlockingStatus::Stalled,
        BlockingReason::CiBlocked {
            exit_code: None,
            pattern: Some(pattern.to_string()),
        },
        None,
        message,
        detail,
    )
    .with_observed_at(crate::timeutil::now_utc_rfc3339_or_fallback())
}

pub(crate) fn check_project(
    report: &mut DoctorReport,
    resolved: &config::Resolved,
    auto_fix: bool,
) {
    check_ci_gate_prerequisites(report, resolved);
    check_gitignore_runtime_logs(report, resolved, auto_fix);
}

fn check_ci_gate_prerequisites(report: &mut DoctorReport, resolved: &config::Resolved) {
    let makefile_path = resolved.repo_root.join("Makefile");
    let make_target = ci_gate_make_target(resolved);

    if let Some(target) = make_target.as_deref() {
        if !makefile_path.exists() {
            report.add(
                CheckResult::error(
                    "project",
                    "makefile",
                    "Makefile missing in repo root",
                    false,
                    Some(&format!("Create a Makefile with a '{target}' target")),
                )
                .with_blocking(doctor_ci_blocked(
                    "makefile_missing",
                    "CueLoop is stalled because the project CI gate is unavailable.",
                    format!(
                        "The configured CI gate expects a Makefile target '{target}', but {} is missing.",
                        makefile_path.display()
                    ),
                )),
            );
            return;
        }

        report.add(CheckResult::success(
            "project",
            "makefile",
            "Makefile found",
        ));
        match fs::read_to_string(&makefile_path) {
            Ok(content) => {
                if make_target_exists(&makefile_path, &content, target) {
                    report.add(CheckResult::success(
                        "project",
                        "ci_target",
                        &format!("Makefile has '{target}' target"),
                    ));
                } else {
                    report.add(
                        CheckResult::error(
                            "project",
                            "ci_target",
                            &format!("Makefile exists but missing '{target}' target"),
                            false,
                            Some(&format!(
                                "Add a '{target}' target to your Makefile for automated checks"
                            )),
                        )
                        .with_blocking(doctor_ci_blocked(
                            "ci_target_missing",
                            "CueLoop is stalled because the project CI gate is unavailable.",
                            format!(
                                "The repository Makefile does not define the configured CI target '{target}'."
                            ),
                        )),
                    );
                }
            }
            Err(e) => {
                report.add(CheckResult::error(
                    "project",
                    "makefile_read",
                    &format!("failed to read Makefile: {}", e),
                    false,
                    Some("Check file permissions"),
                ));
            }
        }
        return;
    }

    if makefile_path.exists() {
        report.add(CheckResult::success(
            "project",
            "makefile",
            "Makefile found",
        ));
    } else {
        report.add(CheckResult::warning(
            "project",
            "makefile",
            "Makefile missing in repo root, but the configured CI gate does not require it",
            false,
            Some("Add a Makefile if you want a local make-based CI entrypoint"),
        ));
    }
}

fn ci_gate_make_target(resolved: &config::Resolved) -> Option<String> {
    let ci_gate = resolved.config.agent.ci_gate.as_ref();
    if ci_gate.is_some_and(|ci_gate| !ci_gate.is_enabled()) {
        return None;
    }

    let argv = ci_gate.and_then(|ci_gate| ci_gate.argv.as_ref());
    let Some(argv) = argv else {
        return Some("ci".to_string());
    };
    if argv.first().map(String::as_str) != Some("make") {
        return None;
    }

    argv.iter()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .cloned()
        .or_else(|| Some("ci".to_string()))
}

fn make_target_exists(makefile_path: &Path, content: &str, target: &str) -> bool {
    let mut visited = HashSet::new();
    if let Ok(canonical_path) = fs::canonicalize(makefile_path) {
        visited.insert(canonical_path);
    }
    make_target_exists_in_content(makefile_path, content, target, &mut visited)
}

fn make_target_exists_in_file(
    makefile_path: &Path,
    target: &str,
    visited: &mut HashSet<PathBuf>,
) -> bool {
    let Ok(canonical_path) = fs::canonicalize(makefile_path) else {
        return false;
    };
    if !visited.insert(canonical_path) {
        return false;
    }
    let Ok(content) = fs::read_to_string(makefile_path) else {
        return false;
    };
    make_target_exists_in_content(makefile_path, &content, target, visited)
}

fn make_target_exists_in_content(
    makefile_path: &Path,
    content: &str,
    target: &str,
    visited: &mut HashSet<PathBuf>,
) -> bool {
    content.lines().any(|line| {
        let line = line.trim_start();
        make_line_defines_target(line, target)
            || make_include_paths(line).iter().any(|include_path| {
                let include_path = makefile_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(include_path);
                make_target_exists_in_file(&include_path, target, visited)
            })
    })
}

fn make_line_defines_target(line: &str, target: &str) -> bool {
    if line.starts_with('#') {
        return false;
    }
    let Some((target_list, after_colon)) = line.split_once(':') else {
        return false;
    };
    if after_colon.trim_start().starts_with('=') || target_list.contains('=') {
        return false;
    }
    target_list
        .split_whitespace()
        .any(|candidate| candidate == target)
}

fn make_include_paths(line: &str) -> Vec<PathBuf> {
    if line.starts_with('#') {
        return Vec::new();
    }
    let Some(rest) = line
        .strip_prefix("include ")
        .or_else(|| line.strip_prefix("-include "))
        .or_else(|| line.strip_prefix("sinclude "))
    else {
        return Vec::new();
    };

    rest.split_whitespace()
        .take_while(|part| !part.starts_with('#'))
        .filter(|part| !part.contains('$'))
        .map(PathBuf::from)
        .collect()
}

/// Check if `.cueloop/logs/` is covered by the repo root `.gitignore`.
///
/// This check inspects the repo-local `.gitignore` file content directly
/// (not using `git check-ignore`, which would incorrectly pass on machines
/// with global excludes).
pub(crate) fn check_gitignore_runtime_logs(
    report: &mut DoctorReport,
    resolved: &config::Resolved,
    auto_fix: bool,
) {
    let gitignore_path = resolved.repo_root.join(".gitignore");

    let content = if gitignore_path.exists() {
        match fs::read_to_string(&gitignore_path) {
            Ok(c) => c,
            Err(e) => {
                report.add(CheckResult::error(
                    "project",
                    "gitignore_cueloop_logs",
                    &format!("failed to read .gitignore: {}", e),
                    false,
                    Some("Check file permissions"),
                ));
                return;
            }
        }
    } else {
        String::new()
    };

    if gitignore_covers_cueloop_logs(&content) {
        report.add(CheckResult::success(
            "project",
            "gitignore_cueloop_logs",
            ".gitignore contains .cueloop/logs/ (debug logs will not be committed)",
        ));
        return;
    }

    let fix_available = true;
    let mut result = CheckResult::error(
        "project",
        "gitignore_cueloop_logs",
        ".gitignore missing ignore rule for .cueloop/logs/ (debug logs may contain secrets)",
        fix_available,
        Some("Add this to your repo root .gitignore:\n\n.cueloop/logs/\n"),
    );

    if auto_fix && fix_available {
        match crate::commands::init::gitignore::ensure_cueloop_gitignore_entries(
            &resolved.repo_root,
        ) {
            Ok(()) => match fs::read_to_string(&gitignore_path) {
                Ok(new_content) => {
                    if gitignore_covers_cueloop_logs(&new_content) {
                        log::info!("Auto-fixed: added .cueloop/logs/ to .gitignore");
                        result = CheckResult::success(
                            "project",
                            "gitignore_cueloop_logs",
                            ".gitignore now contains .cueloop/logs/ (auto-fixed)",
                        )
                        .with_fix_applied(true);
                    } else {
                        result = result.with_fix_applied(false);
                    }
                }
                Err(_) => {
                    result = result.with_fix_applied(false);
                }
            },
            Err(e) => {
                log::error!("Failed to auto-fix .gitignore: {}", e);
                result = result.with_fix_applied(false);
            }
        }
    }

    report.add(result);
}

fn gitignore_covers_cueloop_logs(content: &str) -> bool {
    content.lines().any(|line| {
        matches!(
            line.trim(),
            ".cueloop" | ".cueloop/" | ".cueloop/*" | ".cueloop/logs" | ".cueloop/logs/"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gitignore_covers_cueloop_logs_accepts_specific_and_broad_runtime_entries() {
        assert!(gitignore_covers_cueloop_logs(".cueloop/logs/\n"));
        assert!(gitignore_covers_cueloop_logs(".cueloop/logs\n"));
        assert!(gitignore_covers_cueloop_logs(".cueloop/\n"));
        assert!(gitignore_covers_cueloop_logs(".cueloop\n"));
        assert!(gitignore_covers_cueloop_logs(".cueloop/*\n"));
        assert!(!gitignore_covers_cueloop_logs("# .cueloop/logs/\n"));
        assert!(!gitignore_covers_cueloop_logs(".cueloop/cache/\n"));
    }

    #[test]
    fn make_target_exists_reads_included_makefiles() -> anyhow::Result<()> {
        let temp = tempfile::TempDir::new()?;
        let makefile_path = temp.path().join("Makefile");
        let mk_dir = temp.path().join("mk");
        fs::create_dir_all(&mk_dir)?;
        fs::write(&makefile_path, "include mk/ci.mk\n")?;
        fs::write(
            mk_dir.join("ci.mk"),
            ".PHONY: agent-ci\nagent-ci:\n\t@echo ok\n",
        )?;

        let content = fs::read_to_string(&makefile_path)?;
        assert!(make_target_exists(&makefile_path, &content, "agent-ci"));
        Ok(())
    }

    #[test]
    fn make_target_exists_ignores_variable_assignments() -> anyhow::Result<()> {
        let temp = tempfile::TempDir::new()?;
        let makefile_path = temp.path().join("Makefile");
        fs::write(&makefile_path, "agent-ci := not-a-target\n")?;

        let content = fs::read_to_string(&makefile_path)?;
        assert!(!make_target_exists(&makefile_path, &content, "agent-ci"));
        Ok(())
    }
}
