//! Purpose: Persist webhook delivery failures safely and load bounded failure history.
//!
//! Responsibilities:
//! - Store failed delivery records in the repo-local webhook diagnostics cache.
//! - Enforce bounded retention, replay-count updates, and serialized failure-store access.
//! - Redact destinations, persisted errors, and freeform payload text so secrets never reach disk.
//!
//! Scope:
//! - Failure-history file paths, locking, serialization, redaction, and retention only.
//!
//! Usage:
//! - Called by metrics, replay, and test helper companions behind the diagnostics facade.
//!
//! Invariants/Assumptions:
//! - Failure records never include raw freeform payload text, secrets, or token-bearing destination URLs.
//! - Stored history is bounded to the newest 200 failure records.
//! - Store writes are best-effort for runtime delivery failures and serialized by
//!   cross-process locking around each read-modify-write mutation.

use super::super::{WebhookMessage, WebhookPayload};
use crate::{fsutil, redaction};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const WEBHOOK_FAILURE_STORE_RELATIVE_PATH: &str = "cache/webhooks/failures.json";
const MAX_WEBHOOK_FAILURE_RECORDS: usize = 200;
const MAX_FAILURE_ERROR_CHARS: usize = 400;
const FAILURE_STORE_LOCK_LABEL: &str = "webhook failure store";
const FAILURE_STORE_LOCK_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const FAILURE_STORE_LOCK_WAIT_SLICE: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookFailureRecord {
    pub id: String,
    pub failed_at: String,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    pub error: String,
    pub attempts: u32,
    pub replay_count: u32,
    pub payload: WebhookPayload,
}

static FAILURE_STORE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static NEXT_FAILURE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

fn failure_store_lock() -> &'static Mutex<()> {
    FAILURE_STORE_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn failure_store_path(repo_root: &Path) -> PathBuf {
    crate::config::project_runtime_dir(repo_root).join(WEBHOOK_FAILURE_STORE_RELATIVE_PATH)
}

fn failure_store_lock_dir(path: &Path) -> PathBuf {
    path.with_extension("lock")
}

pub(super) fn persist_failed_delivery(
    msg: &WebhookMessage,
    err: &anyhow::Error,
    attempts: u32,
) -> Result<()> {
    let repo_root = match resolve_repo_root_from_runtime(msg) {
        Some(path) => path,
        None => {
            log::debug!("Unable to resolve repo root for webhook failure persistence");
            return Ok(());
        }
    };

    let path = failure_store_path(&repo_root);
    persist_failed_delivery_at_path(&path, msg, err, attempts)
}

pub(super) fn persist_failed_delivery_at_path(
    path: &Path,
    msg: &WebhookMessage,
    err: &anyhow::Error,
    attempts: u32,
) -> Result<()> {
    let _guard = failure_store_lock()
        .lock()
        .map_err(|_| anyhow!("failed to acquire webhook failure store lock"))?;
    let _file_lock = acquire_failure_store_lock(path)?;

    let mut records = load_failure_records_unlocked(path)?;
    maybe_pause_failure_store_mutation_for_tests();
    records.push(WebhookFailureRecord {
        id: next_failure_id(),
        failed_at: crate::timeutil::now_utc_rfc3339_or_fallback(),
        event: msg.payload.event.clone(),
        task_id: msg.payload.task_id.clone(),
        destination: msg
            .config
            .url
            .as_deref()
            .map(super::super::worker::redact_webhook_destination),
        error: sanitize_error(err, msg.config.url.as_deref()),
        attempts,
        replay_count: 0,
        payload: sanitize_payload_for_persistence(&msg.payload),
    });

    if records.len() > MAX_WEBHOOK_FAILURE_RECORDS {
        let retain_from = records.len().saturating_sub(MAX_WEBHOOK_FAILURE_RECORDS);
        records.drain(..retain_from);
    }

    write_failure_records_unlocked(path, &records)
}

pub(super) fn load_failure_records(path: &Path) -> Result<Vec<WebhookFailureRecord>> {
    let _guard = failure_store_lock()
        .lock()
        .map_err(|_| anyhow!("failed to acquire webhook failure store lock"))?;
    load_failure_records_unlocked(path)
}

#[cfg(test)]
pub(super) fn write_failure_records(path: &Path, records: &[WebhookFailureRecord]) -> Result<()> {
    let _guard = failure_store_lock()
        .lock()
        .map_err(|_| anyhow!("failed to acquire webhook failure store lock"))?;
    let _file_lock = acquire_failure_store_lock(path)?;
    write_failure_records_unlocked(path, records)
}

pub(super) fn update_replay_counts(path: &Path, replayed_ids: &[String]) -> Result<()> {
    let replayed_set = replayed_ids
        .iter()
        .map(std::string::String::as_str)
        .collect::<HashSet<_>>();

    let _guard = failure_store_lock()
        .lock()
        .map_err(|_| anyhow!("failed to acquire webhook failure store lock"))?;
    let _file_lock = acquire_failure_store_lock(path)?;
    let mut records = load_failure_records_unlocked(path)?;
    maybe_pause_failure_store_mutation_for_tests();
    for record in &mut records {
        if replayed_set.contains(record.id.as_str()) {
            record.replay_count = record.replay_count.saturating_add(1);
        }
    }
    write_failure_records_unlocked(path, &records)
}

fn load_failure_records_unlocked(path: &Path) -> Result<Vec<WebhookFailureRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("read webhook failure store {}", path.display()))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str::<Vec<WebhookFailureRecord>>(&content)
        .with_context(|| format!("parse webhook failure store {}", path.display()))
}

fn write_failure_records_unlocked(path: &Path, records: &[WebhookFailureRecord]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "create webhook failure store directory {}",
                parent.display()
            )
        })?;
    }

    let rendered = serde_json::to_string_pretty(records).context("serialize webhook failures")?;
    fsutil::write_atomic(path, rendered.as_bytes())
        .with_context(|| format!("write webhook failure store {}", path.display()))
}

fn acquire_failure_store_lock(path: &Path) -> Result<crate::lock::DirLock> {
    let lock_dir = failure_store_lock_dir(path);
    let deadline = Instant::now() + FAILURE_STORE_LOCK_WAIT_TIMEOUT;

    loop {
        match crate::lock::acquire_dir_lock(&lock_dir, FAILURE_STORE_LOCK_LABEL, true) {
            Ok(lock) => return Ok(lock),
            Err(error) if Instant::now() < deadline => {
                log::debug!(
                    "waiting for webhook failure store lock {}: {}",
                    lock_dir.display(),
                    error
                );
                std::thread::sleep(FAILURE_STORE_LOCK_WAIT_SLICE);
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "acquire webhook failure store lock {} within {}ms",
                        lock_dir.display(),
                        FAILURE_STORE_LOCK_WAIT_TIMEOUT.as_millis()
                    )
                });
            }
        }
    }
}

fn resolve_repo_root_from_runtime(msg: &WebhookMessage) -> Option<PathBuf> {
    let Some(repo_root) = msg.payload.context.repo_root.as_deref() else {
        log::debug!(
            "webhook payload missing repo_root; skipping failure persistence instead of falling back to current directory"
        );
        return None;
    };

    let repo_root = PathBuf::from(repo_root);
    if repo_root.exists() {
        return Some(crate::config::find_repo_root(&repo_root));
    }

    log::debug!(
        "webhook payload repo_root does not exist; skipping failure persistence: {}",
        repo_root.display()
    );
    None
}

fn next_failure_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let sequence = NEXT_FAILURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("wf-{nanos}-{sequence}")
}

fn sanitize_payload_for_persistence(payload: &WebhookPayload) -> WebhookPayload {
    let mut sanitized = payload.clone();

    sanitized.task_title = redact_optional_payload_text(sanitized.task_title);
    sanitized.previous_status = redact_optional_payload_text(sanitized.previous_status);
    sanitized.current_status = redact_optional_payload_text(sanitized.current_status);
    sanitized.note = redact_optional_payload_text(sanitized.note);

    sanitized.context.runner = redact_optional_payload_text(sanitized.context.runner);
    sanitized.context.model = redact_optional_payload_text(sanitized.context.model);
    sanitized.context.branch = redact_optional_payload_text(sanitized.context.branch);
    sanitized.context.commit = redact_optional_payload_text(sanitized.context.commit);
    sanitized.context.ci_gate = redact_optional_payload_text(sanitized.context.ci_gate);

    if sanitized.context.repo_root.is_some() {
        sanitized.context.repo_root = Some(crate::constants::defaults::REDACTED.to_string());
    }

    sanitized
}

fn redact_optional_payload_text(value: Option<String>) -> Option<String> {
    value
        .filter(|text| !text.trim().is_empty())
        .map(|_| crate::constants::defaults::REDACTED.to_string())
}

fn sanitize_error(err: &anyhow::Error, destination_url: Option<&str>) -> String {
    let mut rendered = err.to_string();
    if let Some(url) = destination_url {
        rendered = rendered.replace(url, &super::super::worker::redact_webhook_destination(url));
    }

    let redacted = redaction::redact_text(&rendered);
    let trimmed = redacted.trim();
    if trimmed.chars().count() <= MAX_FAILURE_ERROR_CHARS {
        return trimmed.to_string();
    }

    let truncated = trimmed
        .chars()
        .take(MAX_FAILURE_ERROR_CHARS)
        .collect::<String>();
    format!("{truncated}…")
}

#[cfg(test)]
fn maybe_pause_failure_store_mutation_for_tests() {
    let delay_ms = std::env::var("CUELOOP_TEST_WEBHOOK_FAILURE_STORE_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|delay_ms| *delay_ms > 0);

    if let Some(delay_ms) = delay_ms {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
}

#[cfg(not(test))]
fn maybe_pause_failure_store_mutation_for_tests() {}
