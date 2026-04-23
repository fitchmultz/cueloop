//! Purpose: Preserve webhook diagnostics test helpers behind the split diagnostics facade.
//!
//! Responsibilities:
//! - Bridge test code to failure-store helpers without widening production APIs.
//! - Keep metrics reset helpers out of production-facing modules.
//! - Preserve historical `*_for_tests` entrypoints used by webhook test suites.
//!
//! Scope:
//! - Test-only wrapper functions for diagnostics companions.
//!
//! Usage:
//! - Called by webhook unit/integration-style crate tests through `crate::webhook::diagnostics`.
//!
//! Invariants/Assumptions:
//! - These helpers compile only for tests.
//! - Test wrappers preserve existing behavior and names after the diagnostics split.

use super::super::WebhookMessage;
use super::failure_store::{self, WebhookFailureRecord};
use super::metrics;
use anyhow::Result;
use std::path::Path;

pub(crate) fn write_failure_records_for_tests(
    repo_root: &Path,
    records: &[WebhookFailureRecord],
) -> Result<()> {
    let path = failure_store::failure_store_path(repo_root);
    failure_store::write_failure_records(&path, records)
}

pub(crate) fn load_failure_records_for_tests(
    repo_root: &Path,
) -> Result<Vec<WebhookFailureRecord>> {
    let path = failure_store::failure_store_path(repo_root);
    failure_store::load_failure_records(&path)
}

pub(crate) fn persist_failed_delivery_for_tests(
    repo_root: &Path,
    msg: &WebhookMessage,
    err: &anyhow::Error,
    attempts: u32,
) -> Result<()> {
    let path = failure_store::failure_store_path(repo_root);
    failure_store::persist_failed_delivery_at_path(&path, msg, err, attempts)
}

pub(crate) fn update_replay_counts_for_tests(
    repo_root: &Path,
    replayed_ids: &[String],
) -> Result<()> {
    let path = failure_store::failure_store_path(repo_root);
    failure_store::update_replay_counts(&path, replayed_ids)
}

pub(crate) fn reset_webhook_metrics_for_tests() {
    metrics::reset_metrics_for_tests();
}
