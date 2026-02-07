//! Queue synchronization after PR merges.
//!
//! Responsibilities:
//! - Apply queue/done file updates from merged PRs.
//! - Parse and validate queue bytes with JSONC support.
//! - Run semantic validation before persistence.
//! - Update productivity data when provided.
//!
//! Not handled here:
//! - State file management (see `super::state`).
//! - Merge coordination (see `super::merge_runner`).
//!
//! Invariants/assumptions:
//! - All bytes are validated before any file writes occur.
//! - Invalid queue/done data fails fast without modifying disk.

use crate::config;
use crate::contracts::QueueFile;
use crate::queue;
use crate::{fsutil, jsonc};
use anyhow::{Context, Result, bail};

use super::merge_runner::MergeResult;

/// Apply queue/done updates from a merge result.
pub(crate) fn apply_merge_queue_sync(
    resolved: &config::Resolved,
    result: &MergeResult,
) -> Result<()> {
    let Some(queue_bytes) = result.queue_bytes.as_ref() else {
        bail!(
            "Merged PR for {} did not return queue bytes; refusing to update local queue.",
            result.task_id
        )
    };

    // Parse queue bytes into QueueFile (validates UTF-8 + JSONC)
    let queue_file = parse_bytes_to_queue_file(queue_bytes, &result.task_id, "queue")?;

    // Parse done bytes if present
    let done_file: Option<QueueFile> = if let Some(done_bytes) = result.done_bytes.as_ref() {
        Some(parse_bytes_to_queue_file(
            done_bytes,
            &result.task_id,
            "done",
        )?)
    } else {
        None
    };

    // Run semantic validation before any disk writes
    let max_depth = resolved.config.queue.max_dependency_depth.unwrap_or(10);
    if let Some(ref done) = done_file {
        let warnings = queue::validate_queue_set(
            &queue_file,
            Some(done),
            &resolved.id_prefix,
            resolved.id_width,
            max_depth,
        )
        .with_context(|| {
            format!(
                "[{}] semantic validation failed for queue/done set",
                result.task_id
            )
        })?;
        queue::log_warnings(&warnings);
    } else {
        queue::validate_queue(&queue_file, &resolved.id_prefix, resolved.id_width).with_context(
            || format!("[{}] semantic validation failed for queue", result.task_id),
        )?;
    }

    // Only persist after all validation succeeds
    queue::save_queue(&resolved.queue_path, &queue_file)
        .with_context(|| format!("[{}] persist validated queue", result.task_id))?;

    match done_file {
        Some(done) => {
            queue::save_queue(&resolved.done_path, &done)
                .with_context(|| format!("[{}] persist validated done", result.task_id))?;
        }
        None => {
            if let Err(err) = std::fs::remove_file(&resolved.done_path)
                && err.kind() != std::io::ErrorKind::NotFound
            {
                return Err(err.into());
            }
        }
    }

    // Productivity is written last (only if queue/done validation succeeded)
    if let Some(bytes) = result.productivity_bytes.as_ref() {
        let productivity_path = resolved
            .repo_root
            .join(".ralph")
            .join("cache")
            .join("productivity.json");
        fsutil::write_atomic(&productivity_path, bytes)
            .with_context(|| format!("write productivity bytes for {}", result.task_id))?;
    }

    Ok(())
}

/// Parse bytes into a QueueFile using JSONC parsing rules.
fn parse_bytes_to_queue_file(bytes: &[u8], task_id: &str, label: &str) -> Result<QueueFile> {
    let raw = std::str::from_utf8(bytes)
        .with_context(|| format!("[{}] {} bytes are not valid UTF-8", task_id, label))?;
    jsonc::parse_jsonc::<QueueFile>(raw, &format!("[{}] parse {} as JSONC", task_id, label))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::contracts::{Config, QueueFile};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn build_test_resolved_for_merge_tests(
        repo_root: &Path,
        queue_path: std::path::PathBuf,
        done_path: std::path::PathBuf,
    ) -> config::Resolved {
        config::Resolved {
            config: Config::default(),
            repo_root: repo_root.to_path_buf(),
            queue_path,
            done_path,
            id_prefix: "RQ".to_string(),
            id_width: 4,
            global_config_path: None,
            project_config_path: None,
        }
    }

    #[test]
    fn apply_merge_queue_sync_rejects_invalid_queue_bytes() {
        let temp = TempDir::new().unwrap();
        let ralph_dir = temp.path().join(".ralph");
        fs::create_dir_all(&ralph_dir).unwrap();
        let cache_dir = ralph_dir.join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        let queue_path = ralph_dir.join("queue.json");
        let done_path = ralph_dir.join("done.json");
        let productivity_path = cache_dir.join("productivity.json");

        // Create sentinel files
        let sentinel_queue = "{\"version\":1,\"tasks\":[]}";
        let sentinel_done = "{\"version\":1,\"tasks\":[]}";
        let sentinel_productivity = "{}";
        fs::write(&queue_path, sentinel_queue).unwrap();
        fs::write(&done_path, sentinel_done).unwrap();
        fs::write(&productivity_path, sentinel_productivity).unwrap();

        let resolved =
            build_test_resolved_for_merge_tests(temp.path(), queue_path.clone(), done_path.clone());

        let result = MergeResult {
            task_id: "RQ-0001".to_string(),
            merged: true,
            merge_blocker: None,
            queue_bytes: Some(b"not valid json".to_vec()),
            done_bytes: Some(sentinel_done.as_bytes().to_vec()),
            productivity_bytes: Some(sentinel_productivity.as_bytes().to_vec()),
        };

        // Should return error for invalid queue bytes
        let err = apply_merge_queue_sync(&resolved, &result).unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("parse queue as JSONC") || err_msg.contains("queue"),
            "Error should mention queue parsing: {}",
            err_msg
        );

        // Verify sentinel files unchanged
        assert_eq!(fs::read_to_string(&queue_path).unwrap(), sentinel_queue);
        assert_eq!(fs::read_to_string(&done_path).unwrap(), sentinel_done);
        assert_eq!(
            fs::read_to_string(&productivity_path).unwrap(),
            sentinel_productivity
        );
    }

    #[test]
    fn apply_merge_queue_sync_rejects_invalid_done_bytes() {
        let temp = TempDir::new().unwrap();
        let ralph_dir = temp.path().join(".ralph");
        fs::create_dir_all(&ralph_dir).unwrap();
        let cache_dir = ralph_dir.join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        let queue_path = ralph_dir.join("queue.json");
        let done_path = ralph_dir.join("done.json");
        let productivity_path = cache_dir.join("productivity.json");

        // Create sentinel files
        let sentinel_queue = "{\"version\":1,\"tasks\":[]}";
        let sentinel_done = "{\"version\":1,\"tasks\":[]}";
        let sentinel_productivity = "{}";
        fs::write(&queue_path, sentinel_queue).unwrap();
        fs::write(&done_path, sentinel_done).unwrap();
        fs::write(&productivity_path, sentinel_productivity).unwrap();

        let valid_queue = "{\"version\":1,\"tasks\":[{\"id\":\"RQ-0001\",\"status\":\"todo\",\"title\":\"Test\",\"tags\":[\"test\"],\"scope\":[\"file\"],\"evidence\":[\"obs\"],\"plan\":[\"do\"],\"created_at\":\"2026-01-18T00:00:00Z\",\"updated_at\":\"2026-01-18T00:00:00Z\"}]}";

        let resolved =
            build_test_resolved_for_merge_tests(temp.path(), queue_path.clone(), done_path.clone());

        let result = MergeResult {
            task_id: "RQ-0001".to_string(),
            merged: true,
            merge_blocker: None,
            queue_bytes: Some(valid_queue.as_bytes().to_vec()),
            done_bytes: Some(b"not valid json".to_vec()),
            productivity_bytes: Some(sentinel_productivity.as_bytes().to_vec()),
        };

        // Should return error for invalid done bytes
        let err = apply_merge_queue_sync(&resolved, &result).unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("parse done as JSONC") || err_msg.contains("done"),
            "Error should mention done parsing: {}",
            err_msg
        );

        // Verify sentinel files unchanged
        assert_eq!(fs::read_to_string(&queue_path).unwrap(), sentinel_queue);
        assert_eq!(fs::read_to_string(&done_path).unwrap(), sentinel_done);
        assert_eq!(
            fs::read_to_string(&productivity_path).unwrap(),
            sentinel_productivity
        );
    }

    #[test]
    fn apply_merge_queue_sync_rejects_invalid_utf8_in_queue_bytes() {
        let temp = TempDir::new().unwrap();
        let ralph_dir = temp.path().join(".ralph");
        fs::create_dir_all(&ralph_dir).unwrap();
        let cache_dir = ralph_dir.join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        let queue_path = ralph_dir.join("queue.json");
        let done_path = ralph_dir.join("done.json");
        let productivity_path = cache_dir.join("productivity.json");

        // Create sentinel files
        let sentinel_queue = "{\"version\":1,\"tasks\":[]}";
        let sentinel_done = "{\"version\":1,\"tasks\":[]}";
        let sentinel_productivity = "{}";
        fs::write(&queue_path, sentinel_queue).unwrap();
        fs::write(&done_path, sentinel_done).unwrap();
        fs::write(&productivity_path, sentinel_productivity).unwrap();

        let resolved =
            build_test_resolved_for_merge_tests(temp.path(), queue_path.clone(), done_path.clone());

        // Invalid UTF-8 bytes
        let invalid_utf8 = vec![0xff, 0xfe, 0xfd];
        let valid_done = "{\"version\":1,\"tasks\":[]}";

        let result = MergeResult {
            task_id: "RQ-0001".to_string(),
            merged: true,
            merge_blocker: None,
            queue_bytes: Some(invalid_utf8),
            done_bytes: Some(valid_done.as_bytes().to_vec()),
            productivity_bytes: Some(sentinel_productivity.as_bytes().to_vec()),
        };

        // Should return error for invalid UTF-8
        let err = apply_merge_queue_sync(&resolved, &result).unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("not valid UTF-8") || err_msg.contains("UTF-8"),
            "Error should mention UTF-8: {}",
            err_msg
        );

        // Verify sentinel files unchanged
        assert_eq!(fs::read_to_string(&queue_path).unwrap(), sentinel_queue);
        assert_eq!(fs::read_to_string(&done_path).unwrap(), sentinel_done);
        assert_eq!(
            fs::read_to_string(&productivity_path).unwrap(),
            sentinel_productivity
        );
    }

    #[test]
    fn apply_merge_queue_sync_rejects_semantic_validation_failure() {
        let temp = TempDir::new().unwrap();
        let ralph_dir = temp.path().join(".ralph");
        fs::create_dir_all(&ralph_dir).unwrap();
        let cache_dir = ralph_dir.join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        let queue_path = ralph_dir.join("queue.json");
        let done_path = ralph_dir.join("done.json");
        let productivity_path = cache_dir.join("productivity.json");

        // Create sentinel files
        let sentinel_queue = "{\"version\":1,\"tasks\":[]}";
        let sentinel_done = "{\"version\":1,\"tasks\":[]}";
        let sentinel_productivity = "{}";
        fs::write(&queue_path, sentinel_queue).unwrap();
        fs::write(&done_path, sentinel_done).unwrap();
        fs::write(&productivity_path, sentinel_productivity).unwrap();

        // Queue and done both contain the same task ID (duplicate - should fail validation)
        let valid_queue = "{\"version\":1,\"tasks\":[{\"id\":\"RQ-0001\",\"status\":\"todo\",\"title\":\"Test\",\"tags\":[\"test\"],\"scope\":[\"file\"],\"evidence\":[\"obs\"],\"plan\":[\"do\"],\"created_at\":\"2026-01-18T00:00:00Z\",\"updated_at\":\"2026-01-18T00:00:00Z\"}]}";
        let valid_done = "{\"version\":1,\"tasks\":[{\"id\":\"RQ-0001\",\"status\":\"done\",\"title\":\"Test Done\",\"tags\":[\"test\"],\"scope\":[\"file\"],\"evidence\":[\"obs\"],\"plan\":[\"do\"],\"created_at\":\"2026-01-18T00:00:00Z\",\"updated_at\":\"2026-01-18T00:00:00Z\",\"completed_at\":\"2026-01-18T00:00:00Z\"}]}";

        let resolved =
            build_test_resolved_for_merge_tests(temp.path(), queue_path.clone(), done_path.clone());

        let result = MergeResult {
            task_id: "RQ-0001".to_string(),
            merged: true,
            merge_blocker: None,
            queue_bytes: Some(valid_queue.as_bytes().to_vec()),
            done_bytes: Some(valid_done.as_bytes().to_vec()),
            productivity_bytes: Some(sentinel_productivity.as_bytes().to_vec()),
        };

        // Should return error for duplicate IDs
        let err = apply_merge_queue_sync(&resolved, &result).unwrap_err();
        let err_chain: Vec<String> = err.chain().map(|e| e.to_string()).collect();
        let full_error = err_chain.join(" | ");
        assert!(
            full_error.contains("Duplicate task ID detected across queue and done"),
            "Error should mention duplicate ID: {}",
            full_error
        );

        // Verify sentinel files unchanged
        assert_eq!(fs::read_to_string(&queue_path).unwrap(), sentinel_queue);
        assert_eq!(fs::read_to_string(&done_path).unwrap(), sentinel_done);
        assert_eq!(
            fs::read_to_string(&productivity_path).unwrap(),
            sentinel_productivity
        );
    }

    #[test]
    fn apply_merge_queue_sync_removes_done_file_when_done_bytes_none() {
        let temp = TempDir::new().unwrap();
        let ralph_dir = temp.path().join(".ralph");
        fs::create_dir_all(&ralph_dir).unwrap();
        let cache_dir = ralph_dir.join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        let queue_path = ralph_dir.join("queue.json");
        let done_path = ralph_dir.join("done.json");
        let productivity_path = cache_dir.join("productivity.json");

        // Create existing done file
        let existing_done = "{\"version\":1,\"tasks\":[]}";
        fs::write(&done_path, existing_done).unwrap();

        let valid_queue = "{\"version\":1,\"tasks\":[{\"id\":\"RQ-0001\",\"status\":\"todo\",\"title\":\"Test\",\"tags\":[\"test\"],\"scope\":[\"file\"],\"evidence\":[\"obs\"],\"plan\":[\"do\"],\"created_at\":\"2026-01-18T00:00:00Z\",\"updated_at\":\"2026-01-18T00:00:00Z\"}]}";
        let productivity = "{}";

        let resolved =
            build_test_resolved_for_merge_tests(temp.path(), queue_path.clone(), done_path.clone());

        let result = MergeResult {
            task_id: "RQ-0001".to_string(),
            merged: true,
            merge_blocker: None,
            queue_bytes: Some(valid_queue.as_bytes().to_vec()),
            done_bytes: None, // No done bytes - should remove done file
            productivity_bytes: Some(productivity.as_bytes().to_vec()),
        };

        // Should succeed
        apply_merge_queue_sync(&resolved, &result).unwrap();

        // Queue should be written
        assert!(queue_path.exists());
        // Done file should be removed
        assert!(!done_path.exists());
        // Productivity should be written
        assert!(productivity_path.exists());
    }

    #[test]
    fn apply_merge_queue_sync_accepts_jsonc_and_normalizes_output() {
        let temp = TempDir::new().unwrap();
        let ralph_dir = temp.path().join(".ralph");
        fs::create_dir_all(&ralph_dir).unwrap();
        let cache_dir = ralph_dir.join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        let queue_path = ralph_dir.join("queue.json");
        let done_path = ralph_dir.join("done.json");

        // JSONC with comment and trailing comma (should be accepted)
        let jsonc_queue = r#"{
            // This is a comment
            "version": 1,
            "tasks": [{
                "id": "RQ-0001",
                "status": "todo",
                "title": "Test",
                "tags": ["test"],
                "scope": ["file"],
                "evidence": ["obs"],
                "plan": ["do"],
                "created_at": "2026-01-18T00:00:00Z",
                "updated_at": "2026-01-18T00:00:00Z",
            }],
        }"#;

        let resolved =
            build_test_resolved_for_merge_tests(temp.path(), queue_path.clone(), done_path.clone());

        let result = MergeResult {
            task_id: "RQ-0001".to_string(),
            merged: true,
            merge_blocker: None,
            queue_bytes: Some(jsonc_queue.as_bytes().to_vec()),
            done_bytes: None,
            productivity_bytes: None,
        };

        // Should succeed (JSONC accepted)
        apply_merge_queue_sync(&resolved, &result).unwrap();

        // Read the written queue
        let written_queue = fs::read_to_string(&queue_path).unwrap();

        // Comment should be stripped (not present in normalized output)
        assert!(
            !written_queue.contains("// This is a comment"),
            "Comment should be stripped from normalized output"
        );

        // Should be valid JSON that parses
        let parsed: QueueFile = serde_json::from_str(&written_queue).unwrap();
        assert_eq!(parsed.tasks.len(), 1);
        assert_eq!(parsed.tasks[0].id, "RQ-0001");
    }

    #[test]
    fn apply_merge_queue_sync_preserves_files_on_validation_failure() {
        let temp = TempDir::new().unwrap();
        let ralph_dir = temp.path().join(".ralph");
        fs::create_dir_all(&ralph_dir).unwrap();
        let cache_dir = ralph_dir.join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        let queue_path = ralph_dir.join("queue.json");
        let done_path = ralph_dir.join("done.json");
        let productivity_path = cache_dir.join("productivity.json");

        // Create sentinel files with unique content
        let sentinel_queue = "SENTINEL_QUEUE_CONTENT";
        let sentinel_done = "SENTINEL_DONE_CONTENT";
        let sentinel_productivity = "SENTINEL_PRODUCTIVITY";
        fs::write(&queue_path, sentinel_queue).unwrap();
        fs::write(&done_path, sentinel_done).unwrap();
        fs::write(&productivity_path, sentinel_productivity).unwrap();

        // Invalid queue bytes (malformed JSON)
        let result = MergeResult {
            task_id: "RQ-0001".to_string(),
            merged: true,
            merge_blocker: None,
            queue_bytes: Some(b"{ invalid json".to_vec()),
            done_bytes: Some(b"also invalid".to_vec()),
            productivity_bytes: Some(b"should not be written".to_vec()),
        };

        let resolved =
            build_test_resolved_for_merge_tests(temp.path(), queue_path.clone(), done_path.clone());

        // Should fail
        let _ = apply_merge_queue_sync(&resolved, &result);

        // All sentinel files should be unchanged
        assert_eq!(fs::read_to_string(&queue_path).unwrap(), sentinel_queue);
        assert_eq!(fs::read_to_string(&done_path).unwrap(), sentinel_done);
        assert_eq!(
            fs::read_to_string(&productivity_path).unwrap(),
            sentinel_productivity
        );
    }
}
