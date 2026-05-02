//! Prompt template source helpers.
//!
//! Purpose:
//! - Prompt template source helpers.
//!
//! Responsibilities:
//! - Describe whether a preview uses an embedded template or a repo override.
//! - Keep explain-header source selection separate from prompt assembly logic.
//!
//! Not handled here:
//! - Template file reading or diffing.
//! - Prompt rendering.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Override paths stay aligned with `crate::constants::paths`.

use std::path::Path;

use crate::constants::paths::{
    SCAN_OVERRIDE_PATH, TASK_BUILDER_OVERRIDE_PATH, WORKER_OVERRIDE_PATH,
};

const LEGACY_WORKER_OVERRIDE_PATH: &str = ".cueloop/prompts/worker.md";
const LEGACY_SCAN_OVERRIDE_PATH: &str = ".cueloop/prompts/scan.md";
const LEGACY_TASK_BUILDER_OVERRIDE_PATH: &str = ".cueloop/prompts/task_builder.md";

pub(super) fn worker_template_source(repo_root: &Path) -> &'static str {
    template_source(repo_root, WORKER_OVERRIDE_PATH, LEGACY_WORKER_OVERRIDE_PATH)
}

pub(super) fn scan_template_source(repo_root: &Path) -> &'static str {
    template_source(repo_root, SCAN_OVERRIDE_PATH, LEGACY_SCAN_OVERRIDE_PATH)
}

pub(super) fn task_builder_template_source(repo_root: &Path) -> &'static str {
    template_source(
        repo_root,
        TASK_BUILDER_OVERRIDE_PATH,
        LEGACY_TASK_BUILDER_OVERRIDE_PATH,
    )
}

fn template_source(repo_root: &Path, current: &'static str, legacy: &'static str) -> &'static str {
    if repo_root.join(current).exists() {
        current
    } else if repo_root.join(legacy).exists() {
        legacy
    } else {
        "(embedded default)"
    }
}
