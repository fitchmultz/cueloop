//! Managed preview checkpoints for task decomposition.
//!
//! Purpose:
//! - Persist exact task-decomposition previews for later replay.
//!
//! Responsibilities:
//! - Save preview-only decomposition plans under the repo runtime cache.
//! - Load a saved preview by opaque checkpoint ID for exact write replay.
//! - Validate checkpoint IDs and repository paths to prevent traversal or cross-repo replay.
//! - Prune stale checkpoint cache files on best-effort save.
//!
//! Not handled here:
//! - Planner invocation or queue mutation.
//! - Undo snapshot creation for writes.
//!
//! Usage:
//! - Preview handlers call `save_decomposition_preview_checkpoint` after successful planning.
//! - Write handlers call `load_decomposition_preview_checkpoint` and pass the preview to the canonical writer.
//!
//! Invariants/assumptions:
//! - Checkpoints are runtime cache artifacts, not undo snapshots.
//! - Successful checkpoint replay writes still flow through `write_task_decomposition` safeguards.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::types::DecompositionPreview;
use crate::{config, timeutil};
use time::Duration as TimeDuration;

const CHECKPOINT_VERSION: u32 = 1;
const CHECKPOINT_DIR: &str = ".cueloop/cache/decompose-previews";
const CHECKPOINT_TTL_DAYS: u64 = 7;
const CHECKPOINT_TTL_SECONDS: u64 = CHECKPOINT_TTL_DAYS * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionPreviewCheckpointRef {
    pub id: String,
    pub path: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DecompositionPreviewCheckpoint {
    version: u32,
    id: String,
    created_at: String,
    expires_at: String,
    repo_root: String,
    queue_path: String,
    done_path: String,
    preview: DecompositionPreview,
}

pub fn save_decomposition_preview_checkpoint(
    resolved: &config::Resolved,
    preview: &DecompositionPreview,
) -> Result<DecompositionPreviewCheckpointRef> {
    let dir = checkpoint_dir(&resolved.repo_root);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    prune_stale_checkpoints(&dir);

    let now = time::OffsetDateTime::now_utc();
    let created_at = timeutil::format_rfc3339(now)?;
    let expires_at =
        timeutil::format_rfc3339(now + TimeDuration::seconds(CHECKPOINT_TTL_SECONDS as i64))?;
    let id = checkpoint_id(&created_at, preview)?;
    let checkpoint = DecompositionPreviewCheckpoint {
        version: CHECKPOINT_VERSION,
        id: id.clone(),
        created_at: created_at.clone(),
        expires_at: expires_at.clone(),
        repo_root: path_string(&resolved.repo_root),
        queue_path: path_string(&resolved.queue_path),
        done_path: path_string(&resolved.done_path),
        preview: preview.clone(),
    };
    let path = checkpoint_path(&dir, &id)?;
    let data = serde_json::to_vec_pretty(&checkpoint)
        .context("serialize decomposition preview checkpoint")?;
    fs::write(&path, data).with_context(|| format!("write {}", path.display()))?;

    Ok(DecompositionPreviewCheckpointRef {
        id,
        path: relative_checkpoint_path(&resolved.repo_root, &path),
        created_at,
        expires_at,
    })
}

pub fn load_decomposition_preview_checkpoint(
    resolved: &config::Resolved,
    id: &str,
) -> Result<(DecompositionPreview, DecompositionPreviewCheckpointRef)> {
    let dir = checkpoint_dir(&resolved.repo_root);
    let path = checkpoint_path(&dir, id)?;
    let data =
        fs::read(&path).with_context(|| format!("read decomposition preview checkpoint {id}"))?;
    let checkpoint: DecompositionPreviewCheckpoint = serde_json::from_slice(&data)
        .with_context(|| format!("parse decomposition preview checkpoint {id}"))?;
    if checkpoint.version != CHECKPOINT_VERSION {
        bail!(
            "Unsupported decomposition preview checkpoint version {} for {}",
            checkpoint.version,
            id
        );
    }
    if checkpoint.id != id {
        bail!("Decomposition preview checkpoint id mismatch for {id}");
    }
    ensure_path_matches("repo root", &checkpoint.repo_root, &resolved.repo_root)?;
    ensure_path_matches("queue path", &checkpoint.queue_path, &resolved.queue_path)?;
    ensure_path_matches("done path", &checkpoint.done_path, &resolved.done_path)?;

    let reference = DecompositionPreviewCheckpointRef {
        id: checkpoint.id,
        path: relative_checkpoint_path(&resolved.repo_root, &path),
        created_at: checkpoint.created_at,
        expires_at: checkpoint.expires_at,
    };
    Ok((checkpoint.preview, reference))
}

fn checkpoint_id(created_at: &str, preview: &DecompositionPreview) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(created_at.as_bytes());
    hasher.update(serde_json::to_vec(preview).context("hash decomposition preview checkpoint")?);
    let digest = hasher.finalize();
    let suffix = hex::encode(&digest[..6]);
    let compact_time = created_at
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    Ok(format!("dp-{compact_time}-{suffix}"))
}

fn checkpoint_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(CHECKPOINT_DIR)
}

fn checkpoint_path(dir: &Path, id: &str) -> Result<PathBuf> {
    validate_checkpoint_id(id)?;
    Ok(dir.join(format!("{id}.json")))
}

fn validate_checkpoint_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        bail!("Invalid decomposition preview checkpoint id '{id}'");
    }
    Ok(())
}

fn prune_stale_checkpoints(dir: &Path) {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(CHECKPOINT_TTL_SECONDS))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified < cutoff {
            let _ = fs::remove_file(path);
        }
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn relative_checkpoint_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn ensure_path_matches(label: &str, stored: &str, current: &Path) -> Result<()> {
    let current = path_string(current);
    if stored != current {
        bail!(
            "Decomposition preview checkpoint belongs to a different {label}: expected {current}, found {stored}"
        );
    }
    Ok(())
}
