//! Parallel run state persistence for crash recovery.
//!
//! Responsibilities:
//! - Define the parallel state file format and helpers.
//! - Persist and reload state for in-flight tasks, PRs, pending merges, and finished-without-PR blockers.
//!
//! Not handled here:
//! - Worker orchestration or process management (see `parallel/mod.rs`).
//! - PR merge logic (see `merge_agent`).
//!
//! Invariants/assumptions:
//! - State file lives at `.ralph/cache/parallel/state.json`.
//! - Callers update and persist state after each significant transition.
//! - Deserialization is tolerant of missing/unknown fields; callers normalize and persist the canonical shape.

use crate::contracts::{ParallelMergeMethod, ParallelMergeWhen};
use crate::fsutil;
use crate::git::WorkspaceSpec;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

// =============================================================================
// Pending Merge Job (new architecture - merge-agent subprocess)
// =============================================================================

/// Lifecycle states for a pending merge job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PendingMergeLifecycle {
    #[default]
    Queued,
    InProgress,
    RetryableFailed,
    TerminalFailed,
}

/// A merge job waiting to be processed or currently in progress.
///
/// This struct tracks merge jobs that are queued for the merge-agent subprocess.
/// The coordinator enqueues these after a worker succeeds and creates a PR,
/// then processes them via subprocess invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingMergeJob {
    /// Task ID associated with this merge job.
    pub task_id: String,
    /// PR number to merge.
    pub pr_number: u32,
    /// Optional path to the workspace (for cleanup after merge).
    pub workspace_path: Option<PathBuf>,
    /// Current lifecycle state.
    #[serde(default)]
    pub lifecycle: PendingMergeLifecycle,
    /// Number of merge attempts (for retry policy).
    #[serde(default)]
    pub attempts: u8,
    /// Timestamp when queued (RFC3339).
    pub queued_at: String,
    /// Last error message if failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelStateFile {
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub base_branch: String,
    #[serde(default)]
    pub merge_method: ParallelMergeMethod,
    #[serde(default)]
    pub merge_when: ParallelMergeWhen,
    #[serde(default)]
    pub tasks_in_flight: Vec<ParallelTaskRecord>,
    #[serde(default)]
    pub prs: Vec<ParallelPrRecord>,
    #[serde(default)]
    pub finished_without_pr: Vec<ParallelFinishedWithoutPrRecord>,
    /// Merge jobs queued or in-progress (new architecture using merge-agent subprocess).
    #[serde(default)]
    pub pending_merges: Vec<PendingMergeJob>,
}

impl ParallelStateFile {
    pub fn new(
        started_at: String,
        base_branch: String,
        merge_method: ParallelMergeMethod,
        merge_when: ParallelMergeWhen,
    ) -> Self {
        Self {
            started_at,
            base_branch,
            merge_method,
            merge_when,
            tasks_in_flight: Vec::new(),
            prs: Vec::new(),
            finished_without_pr: Vec::new(),
            pending_merges: Vec::new(),
        }
    }

    pub fn upsert_task(&mut self, record: ParallelTaskRecord) {
        if let Some(existing) = self
            .tasks_in_flight
            .iter_mut()
            .find(|item| item.task_id == record.task_id)
        {
            *existing = record;
        } else {
            self.tasks_in_flight.push(record);
        }
    }

    pub fn remove_task(&mut self, task_id: &str) {
        self.tasks_in_flight.retain(|item| item.task_id != task_id);
    }

    pub fn upsert_pr(&mut self, record: ParallelPrRecord) {
        self.remove_finished_without_pr(&record.task_id);
        if let Some(existing) = self
            .prs
            .iter_mut()
            .find(|item| item.task_id == record.task_id)
        {
            *existing = record;
        } else {
            self.prs.push(record);
        }
    }

    pub fn mark_pr_merged(&mut self, task_id: &str) {
        if let Some(existing) = self.prs.iter_mut().find(|item| item.task_id == task_id) {
            existing.merged = true;
            existing.lifecycle = ParallelPrLifecycle::Merged;
        }
    }

    pub fn has_pr_record(&self, task_id: &str) -> bool {
        self.prs.iter().any(|item| item.task_id == task_id)
    }

    pub fn upsert_finished_without_pr(&mut self, record: ParallelFinishedWithoutPrRecord) {
        if let Some(existing) = self
            .finished_without_pr
            .iter_mut()
            .find(|item| item.task_id == record.task_id)
        {
            *existing = record;
        } else {
            self.finished_without_pr.push(record);
        }
    }

    pub fn remove_finished_without_pr(&mut self, task_id: &str) -> bool {
        let before = self.finished_without_pr.len();
        self.finished_without_pr
            .retain(|item| item.task_id != task_id);
        before != self.finished_without_pr.len()
    }

    /// Remove finished-without-PR records that are no longer blocking under the current policy.
    pub(crate) fn prune_finished_without_pr(
        &mut self,
        now: OffsetDateTime,
        auto_pr_enabled: bool,
        draft_on_failure: bool,
    ) -> Vec<String> {
        let mut dropped = Vec::new();
        self.finished_without_pr.retain(|record| {
            let keep = record.is_blocking(now, auto_pr_enabled, draft_on_failure);
            if !keep {
                dropped.push(record.task_id.clone());
            }
            keep
        });
        dropped
    }

    // =========================================================================
    // Pending Merge Job Management (new architecture - merge-agent subprocess)
    // =========================================================================

    /// Queue a new merge job after worker success.
    ///
    /// If a job for this task already exists, it is replaced with the new one.
    pub fn enqueue_merge(&mut self, job: PendingMergeJob) {
        // Remove any existing entry for this task
        self.pending_merges.retain(|j| j.task_id != job.task_id);
        self.pending_merges.push(job);
    }

    /// Get the next queued merge job (FIFO order).
    pub fn next_queued_merge(&self) -> Option<&PendingMergeJob> {
        self.pending_merges
            .iter()
            .find(|j| j.lifecycle == PendingMergeLifecycle::Queued)
    }

    /// Get the next queued merge job mutably (FIFO order).
    pub fn next_queued_merge_mut(&mut self) -> Option<&mut PendingMergeJob> {
        self.pending_merges
            .iter_mut()
            .find(|j| j.lifecycle == PendingMergeLifecycle::Queued)
    }

    /// Mark a merge job as in-progress.
    pub fn mark_merge_in_progress(&mut self, task_id: &str) {
        if let Some(job) = self
            .pending_merges
            .iter_mut()
            .find(|j| j.task_id == task_id)
        {
            job.lifecycle = PendingMergeLifecycle::InProgress;
        }
    }

    /// Update merge job after completion or failure.
    ///
    /// On success, the job will be marked for removal (lifecycle set to a marker).
    /// On failure, attempts are incremented and lifecycle is set appropriately.
    pub fn update_merge_result(
        &mut self,
        task_id: &str,
        success: bool,
        error: Option<String>,
        retryable: bool,
    ) {
        if let Some(job) = self
            .pending_merges
            .iter_mut()
            .find(|j| j.task_id == task_id)
        {
            if success {
                // Mark for removal - caller should call remove_pending_merge
                job.lifecycle = PendingMergeLifecycle::Queued; // marker for removal
            } else {
                job.attempts += 1;
                job.lifecycle = if retryable {
                    PendingMergeLifecycle::RetryableFailed
                } else {
                    PendingMergeLifecycle::TerminalFailed
                };
                job.last_error = error;
            }
        }
    }

    /// Set a pending merge back to queued state (for retry).
    pub fn requeue_merge(&mut self, task_id: &str) {
        if let Some(job) = self
            .pending_merges
            .iter_mut()
            .find(|j| j.task_id == task_id)
        {
            job.lifecycle = PendingMergeLifecycle::Queued;
        }
    }

    /// Mark a merge job as terminally failed.
    pub fn mark_merge_terminal_failed(&mut self, task_id: &str, error: String) {
        if let Some(job) = self
            .pending_merges
            .iter_mut()
            .find(|j| j.task_id == task_id)
        {
            job.lifecycle = PendingMergeLifecycle::TerminalFailed;
            job.last_error = Some(error);
        }
    }

    /// Remove a completed merge job.
    pub fn remove_pending_merge(&mut self, task_id: &str) {
        self.pending_merges.retain(|j| j.task_id != task_id);
    }

    /// Count pending merges (for capacity tracking).
    pub fn pending_merge_count(&self) -> usize {
        self.pending_merges.len()
    }

    /// Check if there are any queued merges waiting to be processed.
    pub fn has_queued_merges(&self) -> bool {
        self.pending_merges
            .iter()
            .any(|j| j.lifecycle == PendingMergeLifecycle::Queued)
    }

    /// Get a pending merge job by task_id.
    pub fn get_pending_merge(&self, task_id: &str) -> Option<&PendingMergeJob> {
        self.pending_merges.iter().find(|j| j.task_id == task_id)
    }

    /// Get a mutable pending merge job by task_id.
    pub fn get_pending_merge_mut(&mut self, task_id: &str) -> Option<&mut PendingMergeJob> {
        self.pending_merges
            .iter_mut()
            .find(|j| j.task_id == task_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTaskRecord {
    pub task_id: String,
    #[serde(alias = "worktree_path")]
    pub workspace_path: String,
    pub branch: String,
    pub pid: Option<u32>,

    /// Timestamp when the task was started (RFC3339).
    /// Backward compatible: legacy state files may omit this field.
    #[serde(default)]
    pub started_at: String,
}

impl ParallelTaskRecord {
    pub(crate) fn new(
        task_id: &str,
        workspace: &WorkspaceSpec,
        pid: u32,
        started_at: Option<String>,
    ) -> Self {
        Self {
            task_id: task_id.to_string(),
            workspace_path: workspace.path.to_string_lossy().to_string(),
            branch: workspace.branch.clone(),
            pid: Some(pid),
            started_at: started_at.unwrap_or_else(crate::timeutil::now_utc_rfc3339_or_fallback),
        }
    }
}

/// PR lifecycle state for persisted parallel PR records.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ParallelPrLifecycle {
    #[default]
    Open,
    Closed,
    Merged,
}

/// Reason a parallel task finished without a PR record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ParallelNoPrReason {
    #[default]
    Unknown,
    AutoPrDisabled,
    PrCreateFailed,
    DraftPrDisabled,
    DraftPrSkippedNoChanges,
}

impl ParallelNoPrReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            ParallelNoPrReason::Unknown => "unknown",
            ParallelNoPrReason::AutoPrDisabled => "auto_pr_disabled",
            ParallelNoPrReason::PrCreateFailed => "pr_create_failed",
            ParallelNoPrReason::DraftPrDisabled => "draft_pr_disabled",
            ParallelNoPrReason::DraftPrSkippedNoChanges => "draft_pr_skipped_no_changes",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelPrRecord {
    pub task_id: String,
    pub pr_number: u32,
    pub pr_url: String,
    #[serde(default)]
    pub head: Option<String>,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default, alias = "worktree_path")]
    pub workspace_path: Option<String>,
    pub merged: bool,
    #[serde(default)]
    pub lifecycle: ParallelPrLifecycle,
    /// Human-readable reason this PR is blocked from auto-merge.
    /// Set when the PR head doesn't match the expected branch naming convention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_blocker: Option<String>,
}

impl ParallelPrRecord {
    pub(crate) fn new(
        task_id: &str,
        pr: &crate::git::PrInfo,
        workspace_path: Option<&Path>,
    ) -> Self {
        Self {
            task_id: task_id.to_string(),
            pr_number: pr.number,
            pr_url: pr.url.clone(),
            head: Some(pr.head.clone()),
            base: Some(pr.base.clone()),
            workspace_path: workspace_path.map(|p| p.to_string_lossy().to_string()),
            merged: false,
            lifecycle: ParallelPrLifecycle::Open,
            merge_blocker: None,
        }
    }

    /// Returns true if the PR is open (not merged/closed) and not yet merged.
    /// These represent work already in flight from a prior run that should
    /// count toward max_tasks limits on resume.
    pub fn is_open_unmerged(&self) -> bool {
        matches!(self.lifecycle, ParallelPrLifecycle::Open) && !self.merged
    }

    /// Create a PrInfo from this record.
    /// Note: Kept for backward compatibility with merge-runner tests.
    #[allow(dead_code)]
    pub(crate) fn pr_info(&self, fallback_head: &str, fallback_base: &str) -> crate::git::PrInfo {
        let head = self
            .head
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or(fallback_head)
            .to_string();
        let base = self
            .base
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or(fallback_base)
            .to_string();
        crate::git::PrInfo {
            number: self.pr_number,
            url: self.pr_url.clone(),
            head,
            base,
        }
    }

    pub fn workspace_path(&self) -> Option<PathBuf> {
        self.workspace_path.as_ref().map(PathBuf::from)
    }
}

/// Record for a task that finished without a PR record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelFinishedWithoutPrRecord {
    pub task_id: String,
    #[serde(alias = "worktree_path")]
    pub workspace_path: String,
    pub branch: String,
    pub success: bool,
    pub finished_at: String,
    #[serde(default)]
    pub reason: ParallelNoPrReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ParallelFinishedWithoutPrRecord {
    pub(crate) fn new(
        task_id: &str,
        workspace: &WorkspaceSpec,
        success: bool,
        finished_at: String,
        reason: ParallelNoPrReason,
        message: Option<String>,
    ) -> Self {
        Self {
            task_id: task_id.to_string(),
            workspace_path: workspace.path.to_string_lossy().to_string(),
            branch: workspace.branch.clone(),
            success,
            finished_at,
            reason,
            message,
        }
    }

    /// Returns true if this record should currently block task selection.
    ///
    /// Policy:
    /// - Never block if the recorded workspace no longer exists (stale recovery state).
    /// - AutoPrDisabled blocks only while auto_pr is still disabled.
    /// - DraftPrDisabled blocks only while we still cannot create draft PRs on failure.
    /// - PrCreateFailed / Unknown / DraftPrSkippedNoChanges block only within a TTL window.
    pub(crate) fn is_blocking(
        &self,
        now: OffsetDateTime,
        auto_pr_enabled: bool,
        draft_on_failure: bool,
    ) -> bool {
        if !std::path::Path::new(self.workspace_path.trim()).exists() {
            return false;
        }

        match &self.reason {
            ParallelNoPrReason::AutoPrDisabled => !auto_pr_enabled,
            ParallelNoPrReason::DraftPrDisabled => !(auto_pr_enabled && draft_on_failure),
            ParallelNoPrReason::PrCreateFailed
            | ParallelNoPrReason::Unknown
            | ParallelNoPrReason::DraftPrSkippedNoChanges => {
                let Some(finished_at) = crate::timeutil::parse_rfc3339_opt(&self.finished_at)
                else {
                    // Bad timestamp must not become a permanent blocker.
                    return false;
                };
                now - finished_at < finished_without_pr_blocker_ttl()
            }
        }
    }
}

fn finished_without_pr_blocker_ttl() -> time::Duration {
    let secs: i64 = crate::constants::timeouts::PARALLEL_FINISHED_WITHOUT_PR_BLOCKER_TTL
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX);
    time::Duration::seconds(secs)
}

pub fn state_file_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".ralph/cache/parallel/state.json")
}

pub fn load_state(path: &Path) -> Result<Option<ParallelStateFile>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read parallel state {}", path.display()))?;
    let state = crate::jsonc::parse_jsonc::<ParallelStateFile>(&raw, "parallel state")?;
    Ok(Some(state))
}

pub(crate) fn save_state(path: &Path, state: &ParallelStateFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create parallel state dir {}", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(state).context("serialize parallel state")?;
    fsutil::write_atomic(path, rendered.as_bytes())
        .with_context(|| format!("write parallel state {}", path.display()))?;
    Ok(())
}

/// Summary of PR reconciliation results.
#[derive(Debug, Clone, Default)]
pub(crate) struct ReconcileSummary {
    pub open_count: usize,
    pub closed_count: usize,
    pub merged_count: usize,
    pub unknown_count: usize,
    pub error_count: usize,
    pub affected_task_ids: Vec<String>,
}

impl ReconcileSummary {
    pub fn has_changes(&self) -> bool {
        !self.affected_task_ids.is_empty()
    }
}

/// Reconcile persisted PR records against current GitHub state.
///
/// For each PR record where `merged == false` and `lifecycle == Open`,
/// queries GitHub to determine if the PR is still open. Updates the
/// record's lifecycle and merged flag based on the current state.
///
/// Errors during individual PR lookups are logged as warnings and do not
/// abort the reconciliation process.
pub(crate) fn reconcile_pr_records(
    repo_root: &Path,
    state_file: &mut ParallelStateFile,
) -> Result<ReconcileSummary> {
    use crate::git;

    let mut summary = ReconcileSummary::default();

    for record in state_file.prs.iter_mut() {
        // Skip already merged records
        if record.merged || !matches!(record.lifecycle, ParallelPrLifecycle::Open) {
            match record.lifecycle {
                ParallelPrLifecycle::Open => summary.open_count += 1,
                ParallelPrLifecycle::Closed => summary.closed_count += 1,
                ParallelPrLifecycle::Merged => summary.merged_count += 1,
            }
            continue;
        }

        match git::pr_lifecycle_status(repo_root, record.pr_number) {
            Ok(status) => {
                match status.lifecycle {
                    git::PrLifecycle::Open => {
                        record.lifecycle = ParallelPrLifecycle::Open;
                        summary.open_count += 1;
                    }
                    git::PrLifecycle::Closed => {
                        record.lifecycle = ParallelPrLifecycle::Closed;
                        summary.closed_count += 1;
                        summary.affected_task_ids.push(record.task_id.clone());
                    }
                    git::PrLifecycle::Merged => {
                        record.lifecycle = ParallelPrLifecycle::Merged;
                        record.merged = true;
                        summary.merged_count += 1;
                        summary.affected_task_ids.push(record.task_id.clone());
                    }
                    git::PrLifecycle::Unknown(ref s) => {
                        // Treat unknown as blocking (keep as Open)
                        log::warn!(
                            "PR {} for task {} has unknown lifecycle state '{}'; treating as blocking",
                            record.pr_number,
                            record.task_id,
                            s
                        );
                        record.lifecycle = ParallelPrLifecycle::Open;
                        summary.unknown_count += 1;
                    }
                }
            }
            Err(err) => {
                // Log warning and keep record as blocking
                log::warn!(
                    "Failed to query PR {} for task {}: {}; keeping as blocking",
                    record.pr_number,
                    record.task_id,
                    err
                );
                summary.error_count += 1;
            }
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{ParallelMergeMethod, ParallelMergeWhen};
    use tempfile::TempDir;

    #[test]
    fn state_round_trips() -> Result<()> {
        let temp = TempDir::new()?;
        let path = temp.path().join("state.json");
        let mut state = ParallelStateFile::new(
            "2026-02-01T00:00:00Z".to_string(),
            "main".to_string(),
            ParallelMergeMethod::Squash,
            ParallelMergeWhen::AsCreated,
        );
        state.upsert_finished_without_pr(ParallelFinishedWithoutPrRecord {
            task_id: "RQ-0009".to_string(),
            workspace_path: "/tmp/workspace/RQ-0009".to_string(),
            branch: "ralph/RQ-0009".to_string(),
            success: true,
            finished_at: "2026-02-01T01:00:00Z".to_string(),
            reason: ParallelNoPrReason::AutoPrDisabled,
            message: Some("auto_pr disabled".to_string()),
        });
        state.upsert_pr(ParallelPrRecord {
            task_id: "RQ-0001".to_string(),
            pr_number: 5,
            pr_url: "https://example.com/pr/5".to_string(),
            head: Some("ralph/RQ-0001".to_string()),
            base: Some("main".to_string()),
            workspace_path: Some("/tmp/workspace".to_string()),
            merged: false,
            lifecycle: ParallelPrLifecycle::Open,
            merge_blocker: None,
        });

        save_state(&path, &state)?;
        let loaded = load_state(&path)?.expect("state");
        assert_eq!(loaded.base_branch, "main");
        assert_eq!(loaded.prs.len(), 1);
        assert_eq!(loaded.finished_without_pr.len(), 1);
        Ok(())
    }

    #[test]
    fn state_deserialization_accepts_legacy_worktree_path_in_tasks() -> Result<()> {
        let raw = r#"{
            "started_at":"2026-02-01T00:00:00Z",
            "base_branch":"main",
            "merge_method":"squash",
            "merge_when":"as_created",
            "tasks_in_flight":[{"task_id":"RQ-0001","worktree_path":"/tmp/wt","branch":"b","pid":1}],
            "prs":[]
        }"#;
        let state: ParallelStateFile = serde_json::from_str(raw)?;
        assert_eq!(state.tasks_in_flight.len(), 1);
        assert_eq!(state.tasks_in_flight[0].workspace_path, "/tmp/wt");
        assert!(state.finished_without_pr.is_empty());
        Ok(())
    }

    #[test]
    fn state_deserialization_accepts_legacy_worktree_path_in_prs() -> Result<()> {
        let raw = r#"{
            "started_at":"2026-02-01T00:00:00Z",
            "base_branch":"main",
            "merge_method":"squash",
            "merge_when":"as_created",
            "tasks_in_flight":[],
            "prs":[{"task_id":"RQ-0001","pr_number":5,"pr_url":"https://example.com/pr/5","worktree_path":"/tmp/wt","merged":false}]
        }"#;
        let state: ParallelStateFile = serde_json::from_str(raw)?;
        assert_eq!(state.prs.len(), 1);
        assert_eq!(state.prs[0].workspace_path.as_deref(), Some("/tmp/wt"));
        Ok(())
    }

    #[test]
    fn state_deserialization_ignores_unknown_fields() -> Result<()> {
        let raw = r#"{
            "started_at":"2026-02-01T00:00:00Z",
            "base_branch":"main",
            "merge_method":"squash",
            "merge_when":"as_created",
            "extra_top":"ignored",
            "tasks_in_flight":[{"task_id":"RQ-0001","workspace_path":"/tmp/wt","branch":"b","pid":1,"extra_task":true}],
            "prs":[{"task_id":"RQ-0002","pr_number":5,"pr_url":"https://example.com/pr/5","merged":false,"extra_pr":"ignored"}],
            "finished_without_pr":[{"task_id":"RQ-0003","workspace_path":"/tmp/wt","branch":"b","success":true,"finished_at":"2026-02-01T00:00:00Z","extra_blocker":"ignored"}]
        }"#;
        let state: ParallelStateFile = serde_json::from_str(raw)?;
        assert_eq!(state.tasks_in_flight.len(), 1);
        assert_eq!(state.prs.len(), 1);
        assert_eq!(state.finished_without_pr.len(), 1);
        Ok(())
    }

    #[test]
    fn state_deserialization_allows_missing_base_branch() -> Result<()> {
        let raw = r#"{
            "merge_method":"squash",
            "merge_when":"as_created",
            "tasks_in_flight":[],
            "prs":[]
        }"#;
        let state: ParallelStateFile = serde_json::from_str(raw)?;
        assert!(state.base_branch.is_empty());
        assert!(state.started_at.is_empty());
        assert!(state.finished_without_pr.is_empty());
        Ok(())
    }

    #[test]
    fn finished_without_pr_reason_defaults_to_unknown() {
        let raw = r#"{
            "task_id":"RQ-0010",
            "workspace_path":"/tmp/ws/RQ-0010",
            "branch":"ralph/RQ-0010",
            "success":true,
            "finished_at":"2026-02-01T02:00:00Z"
        }"#;
        let record: ParallelFinishedWithoutPrRecord = serde_json::from_str(raw).unwrap();
        assert!(matches!(record.reason, ParallelNoPrReason::Unknown));
    }

    #[test]
    fn pr_record_uses_fallbacks_when_missing() {
        let record = ParallelPrRecord {
            task_id: "RQ-0002".to_string(),
            pr_number: 9,
            pr_url: "https://example.com/pr/9".to_string(),
            head: None,
            base: None,
            workspace_path: None,
            merged: false,
            lifecycle: ParallelPrLifecycle::Open,
            merge_blocker: None,
        };
        let info = record.pr_info("ralph/RQ-0002", "main");
        assert_eq!(info.head, "ralph/RQ-0002");
        assert_eq!(info.base, "main");
    }

    #[test]
    fn pr_lifecycle_defaults_to_open() {
        // Verify backward compatibility: old state files without lifecycle default to Open
        let raw = r#"{
            "task_id":"RQ-0001",
            "pr_number":5,
            "pr_url":"https://example.com/pr/5",
            "head":"ralph/RQ-0001",
            "base":"main",
            "workspace_path":"/tmp/ws",
            "merged":false
        }"#;
        let record: ParallelPrRecord = serde_json::from_str(raw).unwrap();
        assert!(matches!(record.lifecycle, ParallelPrLifecycle::Open));
        assert!(!record.merged);
    }

    #[test]
    fn pr_lifecycle_round_trips() {
        let record = ParallelPrRecord {
            task_id: "RQ-0003".to_string(),
            pr_number: 10,
            pr_url: "https://example.com/pr/10".to_string(),
            head: Some("ralph/RQ-0003".to_string()),
            base: Some("main".to_string()),
            workspace_path: None,
            merged: true,
            lifecycle: ParallelPrLifecycle::Merged,
            merge_blocker: None,
        };
        let json = serde_json::to_string(&record).unwrap();
        let parsed: ParallelPrRecord = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed.lifecycle, ParallelPrLifecycle::Merged));
        assert!(parsed.merged);
    }

    #[test]
    fn prune_finished_without_pr_drops_non_blocking_and_expired() -> Result<()> {
        use crate::timeutil;

        let temp = TempDir::new()?;
        let ws_keep = temp.path().join("ws_keep");
        let ws_drop = temp.path().join("ws_drop");
        std::fs::create_dir_all(&ws_keep)?;
        std::fs::create_dir_all(&ws_drop)?;

        let now = timeutil::parse_rfc3339("2026-02-03T00:00:00Z")?;

        let mut state = ParallelStateFile::new(
            "2026-02-03T00:00:00Z".to_string(),
            "main".to_string(),
            ParallelMergeMethod::Squash,
            ParallelMergeWhen::AsCreated,
        );

        // Should become non-blocking when auto_pr is enabled (rehydration).
        state.upsert_finished_without_pr(ParallelFinishedWithoutPrRecord {
            task_id: "RQ-AUTO".to_string(),
            workspace_path: ws_drop.to_string_lossy().to_string(),
            branch: "ralph/RQ-AUTO".to_string(),
            success: true,
            finished_at: "2026-02-02T00:00:00Z".to_string(),
            reason: ParallelNoPrReason::AutoPrDisabled,
            message: None,
        });

        // Should remain blocking (PrCreateFailed within TTL window).
        state.upsert_finished_without_pr(ParallelFinishedWithoutPrRecord {
            task_id: "RQ-KEEP".to_string(),
            workspace_path: ws_keep.to_string_lossy().to_string(),
            branch: "ralph/RQ-KEEP".to_string(),
            success: true,
            finished_at: "2026-02-02T23:30:00Z".to_string(),
            reason: ParallelNoPrReason::PrCreateFailed,
            message: Some("rate limited".to_string()),
        });

        // Should be expired (very old).
        state.upsert_finished_without_pr(ParallelFinishedWithoutPrRecord {
            task_id: "RQ-OLD".to_string(),
            workspace_path: ws_drop.to_string_lossy().to_string(),
            branch: "ralph/RQ-OLD".to_string(),
            success: true,
            finished_at: "2020-01-01T00:00:00Z".to_string(),
            reason: ParallelNoPrReason::PrCreateFailed,
            message: None,
        });

        let dropped = state.prune_finished_without_pr(now, true, true);

        assert!(dropped.contains(&"RQ-AUTO".to_string()));
        assert!(dropped.contains(&"RQ-OLD".to_string()));
        assert!(!dropped.contains(&"RQ-KEEP".to_string()));

        assert!(
            state
                .finished_without_pr
                .iter()
                .any(|r| r.task_id == "RQ-KEEP")
        );
        assert!(
            !state
                .finished_without_pr
                .iter()
                .any(|r| r.task_id == "RQ-AUTO")
        );
        assert!(
            !state
                .finished_without_pr
                .iter()
                .any(|r| r.task_id == "RQ-OLD")
        );

        Ok(())
    }

    #[test]
    fn finished_without_pr_never_blocks_when_workspace_missing() -> Result<()> {
        use crate::timeutil;

        let now = timeutil::parse_rfc3339("2026-02-03T00:00:00Z")?;
        let record = ParallelFinishedWithoutPrRecord {
            task_id: "RQ-MISSING".to_string(),
            workspace_path: "/nonexistent/path".to_string(),
            branch: "ralph/RQ-MISSING".to_string(),
            success: true,
            finished_at: "2026-02-02T23:30:00Z".to_string(),
            reason: ParallelNoPrReason::AutoPrDisabled,
            message: None,
        };

        assert!(!record.is_blocking(now, false, false));
        Ok(())
    }

    // Tests for reconcile_pr_records with stubbed gh binary
    use crate::testsupport::path::with_prepend_path;
    use std::io::Write;

    fn create_fake_gh(tmp_dir: &TempDir, pr_responses: &[(u32, &str)]) -> PathBuf {
        let bin_dir = tmp_dir.path().join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let gh_path = bin_dir.join("gh");

        let mut script = String::from(
            r#"#!/bin/bash
# Fake gh script for testing
if [ "$1" = "pr" ] && [ "$2" = "view" ]; then
    PR_NUM="$3"
"#,
        );

        for (pr_num, response) in pr_responses {
            script.push_str(&format!(
                r#"
    if [ "$PR_NUM" = "{}" ]; then
        echo '{}'
        exit 0
    fi
"#,
                pr_num, response
            ));
        }

        script.push_str(
            r#"
fi
echo "Unknown PR or command" >&2
exit 1
"#,
        );

        let mut file = std::fs::File::create(&gh_path).unwrap();
        file.write_all(script.as_bytes()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata().unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&gh_path, perms).unwrap();
        }
        bin_dir
    }

    #[test]
    fn reconcile_pr_records_updates_open_closed_merged() -> Result<()> {
        let temp = TempDir::new()?;

        // PR 1 stays OPEN
        // PR 2 is CLOSED (not merged)
        // PR 3 is MERGED
        let responses = vec![
            (
                1,
                r#"{"state":"OPEN","merged":false,"mergeStateStatus":"CLEAN","number":1,"url":"https://example.com/pr/1","headRefName":"ralph/RQ-0001","baseRefName":"main","isDraft":false}"#,
            ),
            (
                2,
                r#"{"state":"CLOSED","merged":false,"mergeStateStatus":"CLEAN","number":2,"url":"https://example.com/pr/2","headRefName":"ralph/RQ-0002","baseRefName":"main","isDraft":false}"#,
            ),
            (
                3,
                r#"{"state":"CLOSED","merged":true,"mergeStateStatus":"CLEAN","number":3,"url":"https://example.com/pr/3","headRefName":"ralph/RQ-0003","baseRefName":"main","isDraft":false}"#,
            ),
        ];
        let bin_dir = create_fake_gh(&temp, &responses);

        let result = with_prepend_path(&bin_dir, || {
            let mut state_file = ParallelStateFile::new(
                "2026-02-01T00:00:00Z".to_string(),
                "main".to_string(),
                ParallelMergeMethod::Squash,
                ParallelMergeWhen::AsCreated,
            );

            // Add 3 PR records, all initially Open and unmerged
            state_file.upsert_pr(ParallelPrRecord {
                task_id: "RQ-0001".to_string(),
                pr_number: 1,
                pr_url: "https://example.com/pr/1".to_string(),
                head: Some("ralph/RQ-0001".to_string()),
                base: Some("main".to_string()),
                workspace_path: None,
                merged: false,
                lifecycle: ParallelPrLifecycle::Open,
                merge_blocker: None,
            });
            state_file.upsert_pr(ParallelPrRecord {
                task_id: "RQ-0002".to_string(),
                pr_number: 2,
                pr_url: "https://example.com/pr/2".to_string(),
                head: Some("ralph/RQ-0002".to_string()),
                base: Some("main".to_string()),
                workspace_path: None,
                merged: false,
                lifecycle: ParallelPrLifecycle::Open,
                merge_blocker: None,
            });
            state_file.upsert_pr(ParallelPrRecord {
                task_id: "RQ-0003".to_string(),
                pr_number: 3,
                pr_url: "https://example.com/pr/3".to_string(),
                head: Some("ralph/RQ-0003".to_string()),
                base: Some("main".to_string()),
                workspace_path: None,
                merged: false,
                lifecycle: ParallelPrLifecycle::Open,
                merge_blocker: None,
            });

            reconcile_pr_records(temp.path(), &mut state_file).map(|s| (s, state_file))
        });

        let (summary, state_file) = result?;

        // Assert summary
        assert!(summary.has_changes());
        assert_eq!(summary.open_count, 1);
        assert_eq!(summary.closed_count, 1);
        assert_eq!(summary.merged_count, 1);
        assert_eq!(summary.affected_task_ids.len(), 2); // RQ-0002 and RQ-0003
        assert!(summary.affected_task_ids.contains(&"RQ-0002".to_string()));
        assert!(summary.affected_task_ids.contains(&"RQ-0003".to_string()));

        // Assert state file updates
        let pr1 = state_file
            .prs
            .iter()
            .find(|p| p.task_id == "RQ-0001")
            .unwrap();
        let pr2 = state_file
            .prs
            .iter()
            .find(|p| p.task_id == "RQ-0002")
            .unwrap();
        let pr3 = state_file
            .prs
            .iter()
            .find(|p| p.task_id == "RQ-0003")
            .unwrap();

        assert!(matches!(pr1.lifecycle, ParallelPrLifecycle::Open));
        assert!(!pr1.merged);

        assert!(matches!(pr2.lifecycle, ParallelPrLifecycle::Closed));
        assert!(!pr2.merged);

        assert!(matches!(pr3.lifecycle, ParallelPrLifecycle::Merged));
        assert!(pr3.merged);

        Ok(())
    }

    #[test]
    fn reconcile_pr_records_handles_gh_errors_gracefully() -> Result<()> {
        let temp = TempDir::new()?;

        // Fake gh that fails for PR 2
        let responses = vec![
            (
                1,
                r#"{"state":"OPEN","merged":false,"mergeStateStatus":"CLEAN","number":1,"url":"https://example.com/pr/1","headRefName":"ralph/RQ-0001","baseRefName":"main","isDraft":false}"#,
            ),
            // PR 2 will fail (not in the response list)
        ];
        let bin_dir = create_fake_gh(&temp, &responses);

        let result = with_prepend_path(&bin_dir, || {
            let mut state_file = ParallelStateFile::new(
                "2026-02-01T00:00:00Z".to_string(),
                "main".to_string(),
                ParallelMergeMethod::Squash,
                ParallelMergeWhen::AsCreated,
            );

            state_file.upsert_pr(ParallelPrRecord {
                task_id: "RQ-0001".to_string(),
                pr_number: 1,
                pr_url: "https://example.com/pr/1".to_string(),
                head: Some("ralph/RQ-0001".to_string()),
                base: Some("main".to_string()),
                workspace_path: None,
                merged: false,
                lifecycle: ParallelPrLifecycle::Open,
                merge_blocker: None,
            });
            state_file.upsert_pr(ParallelPrRecord {
                task_id: "RQ-0002".to_string(),
                pr_number: 2,
                pr_url: "https://example.com/pr/2".to_string(),
                head: Some("ralph/RQ-0002".to_string()),
                base: Some("main".to_string()),
                workspace_path: None,
                merged: false,
                lifecycle: ParallelPrLifecycle::Open,
                merge_blocker: None,
            });

            reconcile_pr_records(temp.path(), &mut state_file).map(|s| (s, state_file))
        });
        let (summary, state_file) = result?;

        // Should not fail, but should report error for PR 2
        assert_eq!(summary.error_count, 1);
        assert_eq!(summary.open_count, 1); // PR 1 stayed open

        // PR 2 should remain unchanged (still blocking)
        let pr2 = state_file
            .prs
            .iter()
            .find(|p| p.task_id == "RQ-0002")
            .unwrap();
        assert!(matches!(pr2.lifecycle, ParallelPrLifecycle::Open));
        assert!(!pr2.merged);

        Ok(())
    }

    // =========================================================================
    // Pending Merge Job Tests
    // =========================================================================

    #[test]
    fn pending_merge_job_serialization() {
        let job = PendingMergeJob {
            task_id: "RQ-0001".to_string(),
            pr_number: 42,
            workspace_path: Some(PathBuf::from("/tmp/ws")),
            lifecycle: PendingMergeLifecycle::Queued,
            attempts: 0,
            queued_at: "2026-02-17T00:00:00Z".to_string(),
            last_error: None,
        };
        let json = serde_json::to_string(&job).unwrap();
        let parsed: PendingMergeJob = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.task_id, "RQ-0001");
        assert_eq!(parsed.pr_number, 42);
        assert_eq!(parsed.workspace_path, Some(PathBuf::from("/tmp/ws")));
        assert_eq!(parsed.lifecycle, PendingMergeLifecycle::Queued);
    }

    #[test]
    fn pending_merge_job_lifecycle_defaults_to_queued() {
        let raw = r#"{
            "task_id": "RQ-0002",
            "pr_number": 1,
            "queued_at": "2026-02-17T00:00:00Z"
        }"#;
        let job: PendingMergeJob = serde_json::from_str(raw).unwrap();
        assert_eq!(job.lifecycle, PendingMergeLifecycle::Queued);
        assert_eq!(job.attempts, 0);
        assert!(job.last_error.is_none());
    }

    #[test]
    fn state_file_enqueue_merge() {
        let mut state = ParallelStateFile::new(
            "2026-02-17T00:00:00Z".into(),
            "main".into(),
            ParallelMergeMethod::Squash,
            ParallelMergeWhen::AsCreated,
        );

        state.enqueue_merge(PendingMergeJob {
            task_id: "RQ-0001".into(),
            pr_number: 1,
            workspace_path: None,
            lifecycle: PendingMergeLifecycle::Queued,
            attempts: 0,
            queued_at: "2026-02-17T00:00:00Z".into(),
            last_error: None,
        });

        assert_eq!(state.pending_merges.len(), 1);
        assert!(state.next_queued_merge().is_some());
    }

    #[test]
    fn state_file_enqueue_replaces_existing() {
        let mut state = ParallelStateFile::new(
            "2026-02-17T00:00:00Z".into(),
            "main".into(),
            ParallelMergeMethod::Squash,
            ParallelMergeWhen::AsCreated,
        );

        state.enqueue_merge(PendingMergeJob {
            task_id: "RQ-0001".into(),
            pr_number: 1,
            workspace_path: None,
            lifecycle: PendingMergeLifecycle::Queued,
            attempts: 0,
            queued_at: "2026-02-17T00:00:00Z".into(),
            last_error: None,
        });

        state.enqueue_merge(PendingMergeJob {
            task_id: "RQ-0001".into(),
            pr_number: 2, // Updated PR number
            workspace_path: Some(PathBuf::from("/tmp/ws")),
            lifecycle: PendingMergeLifecycle::Queued,
            attempts: 1,
            queued_at: "2026-02-17T01:00:00Z".into(),
            last_error: Some("previous error".into()),
        });

        assert_eq!(state.pending_merges.len(), 1);
        let job = state.next_queued_merge().unwrap();
        assert_eq!(job.pr_number, 2);
        assert_eq!(job.attempts, 1);
    }

    #[test]
    fn state_file_merge_lifecycle_transitions() {
        let mut state = ParallelStateFile::new(
            "2026-02-17T00:00:00Z".into(),
            "main".into(),
            ParallelMergeMethod::Squash,
            ParallelMergeWhen::AsCreated,
        );

        state.enqueue_merge(PendingMergeJob {
            task_id: "RQ-0001".into(),
            pr_number: 1,
            workspace_path: None,
            lifecycle: PendingMergeLifecycle::Queued,
            attempts: 0,
            queued_at: "2026-02-17T00:00:00Z".into(),
            last_error: None,
        });

        // Mark in-progress
        state.mark_merge_in_progress("RQ-0001");
        assert_eq!(
            state.pending_merges[0].lifecycle,
            PendingMergeLifecycle::InProgress
        );

        // Update with retryable failure
        state.update_merge_result("RQ-0001", false, Some("conflict".into()), true);
        assert_eq!(
            state.pending_merges[0].lifecycle,
            PendingMergeLifecycle::RetryableFailed
        );
        assert_eq!(state.pending_merges[0].attempts, 1);
        assert_eq!(state.pending_merges[0].last_error, Some("conflict".into()));

        // Requeue for retry
        state.requeue_merge("RQ-0001");
        assert_eq!(
            state.pending_merges[0].lifecycle,
            PendingMergeLifecycle::Queued
        );

        // Remove on success
        state.remove_pending_merge("RQ-0001");
        assert!(state.pending_merges.is_empty());
    }

    #[test]
    fn state_file_update_merge_result_success() {
        let mut state = ParallelStateFile::new(
            "2026-02-17T00:00:00Z".into(),
            "main".into(),
            ParallelMergeMethod::Squash,
            ParallelMergeWhen::AsCreated,
        );

        state.enqueue_merge(PendingMergeJob {
            task_id: "RQ-0001".into(),
            pr_number: 1,
            workspace_path: None,
            lifecycle: PendingMergeLifecycle::InProgress,
            attempts: 2,
            queued_at: "2026-02-17T00:00:00Z".into(),
            last_error: Some("previous error".into()),
        });

        // Update with success
        state.update_merge_result("RQ-0001", true, None, false);

        // On success, lifecycle is set to Queued as a marker for removal
        let job = state.get_pending_merge("RQ-0001").unwrap();
        assert_eq!(job.lifecycle, PendingMergeLifecycle::Queued);
    }

    #[test]
    fn state_file_update_merge_result_terminal_failure() {
        let mut state = ParallelStateFile::new(
            "2026-02-17T00:00:00Z".into(),
            "main".into(),
            ParallelMergeMethod::Squash,
            ParallelMergeWhen::AsCreated,
        );

        state.enqueue_merge(PendingMergeJob {
            task_id: "RQ-0001".into(),
            pr_number: 1,
            workspace_path: None,
            lifecycle: PendingMergeLifecycle::InProgress,
            attempts: 0,
            queued_at: "2026-02-17T00:00:00Z".into(),
            last_error: None,
        });

        // Update with terminal failure
        state.update_merge_result("RQ-0001", false, Some("PR closed".into()), false);

        let job = state.get_pending_merge("RQ-0001").unwrap();
        assert_eq!(job.lifecycle, PendingMergeLifecycle::TerminalFailed);
        assert_eq!(job.last_error, Some("PR closed".into()));
    }

    #[test]
    fn state_file_pending_merge_count() {
        let mut state = ParallelStateFile::new(
            "2026-02-17T00:00:00Z".into(),
            "main".into(),
            ParallelMergeMethod::Squash,
            ParallelMergeWhen::AsCreated,
        );

        assert_eq!(state.pending_merge_count(), 0);

        state.enqueue_merge(PendingMergeJob {
            task_id: "RQ-0001".into(),
            pr_number: 1,
            workspace_path: None,
            lifecycle: PendingMergeLifecycle::Queued,
            attempts: 0,
            queued_at: "2026-02-17T00:00:00Z".into(),
            last_error: None,
        });

        assert_eq!(state.pending_merge_count(), 1);

        state.enqueue_merge(PendingMergeJob {
            task_id: "RQ-0002".into(),
            pr_number: 2,
            workspace_path: None,
            lifecycle: PendingMergeLifecycle::Queued,
            attempts: 0,
            queued_at: "2026-02-17T00:00:00Z".into(),
            last_error: None,
        });

        assert_eq!(state.pending_merge_count(), 2);
    }

    #[test]
    fn state_file_has_queued_merges() {
        let mut state = ParallelStateFile::new(
            "2026-02-17T00:00:00Z".into(),
            "main".into(),
            ParallelMergeMethod::Squash,
            ParallelMergeWhen::AsCreated,
        );

        assert!(!state.has_queued_merges());

        state.enqueue_merge(PendingMergeJob {
            task_id: "RQ-0001".into(),
            pr_number: 1,
            workspace_path: None,
            lifecycle: PendingMergeLifecycle::InProgress,
            attempts: 0,
            queued_at: "2026-02-17T00:00:00Z".into(),
            last_error: None,
        });

        // InProgress should not count as queued
        assert!(!state.has_queued_merges());

        state.requeue_merge("RQ-0001");
        assert!(state.has_queued_merges());
    }

    #[test]
    fn state_file_round_trips_with_pending_merges() -> Result<()> {
        let temp = TempDir::new()?;
        let path = temp.path().join("state.json");
        let mut state = ParallelStateFile::new(
            "2026-02-17T00:00:00Z".to_string(),
            "main".to_string(),
            ParallelMergeMethod::Squash,
            ParallelMergeWhen::AsCreated,
        );

        state.enqueue_merge(PendingMergeJob {
            task_id: "RQ-0001".into(),
            pr_number: 42,
            workspace_path: Some(PathBuf::from("/tmp/ws/RQ-0001")),
            lifecycle: PendingMergeLifecycle::Queued,
            attempts: 1,
            queued_at: "2026-02-17T00:00:00Z".into(),
            last_error: Some("previous attempt failed".into()),
        });

        save_state(&path, &state)?;
        let loaded = load_state(&path)?.expect("state should exist");

        assert_eq!(loaded.pending_merges.len(), 1);
        let job = &loaded.pending_merges[0];
        assert_eq!(job.task_id, "RQ-0001");
        assert_eq!(job.pr_number, 42);
        assert_eq!(job.attempts, 1);
        assert_eq!(job.last_error, Some("previous attempt failed".into()));
        Ok(())
    }
}
