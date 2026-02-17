//! Merge-agent CLI handler for parallel coordinator subprocess invocation.
//!
//! Responsibilities:
//! - Validate --task and --pr argument formats
//! - Emit structured JSON result to stdout
//! - Write user-facing diagnostics to stderr
//! - Return appropriate exit codes (0/1/2/>=3)
//!
//! Not handled here (RQ-0943):
//! - Actual merge execution
//! - Task finalization in queue/done
//! - Conflict resolution
//! - GitHub API validation
//!
//! Invariants/assumptions:
//! - This command runs in the coordinator repo context (CWD is repo root)
//! - The coordinator has already verified PR existence before invoking

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

/// Result payload emitted to stdout on success.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeAgentResult {
    /// Task ID that was finalized
    pub task_id: String,
    /// PR number that was merged
    pub pr_number: u32,
    /// Whether the merge was successful
    pub merged: bool,
    /// Optional message (success details or error description)
    pub message: Option<String>,
}

/// Error classification for exit code determination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeAgentError {
    /// Validation failure (exit code 2)
    Validation(String),
    /// Runtime failure (exit code 1)
    Runtime(String),
    /// Domain-specific failure (exit code >= 3)
    Domain { code: u8, message: String },
}

impl MergeAgentError {
    pub fn exit_code(&self) -> i32 {
        match self {
            MergeAgentError::Validation(_) => 2,
            MergeAgentError::Runtime(_) => 1,
            MergeAgentError::Domain { code, .. } => (*code as i32).max(3),
        }
    }

    pub fn message(&self) -> &str {
        match self {
            MergeAgentError::Validation(msg) => msg,
            MergeAgentError::Runtime(msg) => msg,
            MergeAgentError::Domain { message, .. } => message,
        }
    }
}

/// Validate task ID format.
/// Task IDs must be non-empty and match the expected pattern (e.g., "RQ-0942").
pub fn validate_task_id(task_id: &str) -> Result<(), MergeAgentError> {
    let trimmed = task_id.trim();
    if trimmed.is_empty() {
        return Err(MergeAgentError::Validation(
            "Task ID cannot be empty".to_string(),
        ));
    }
    // Basic format check: should contain alphanumeric, hyphens, underscores
    if !trimmed
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(MergeAgentError::Validation(format!(
            "Invalid task ID format: '{}'. Expected alphanumeric with hyphens/underscores.",
            trimmed
        )));
    }
    Ok(())
}

/// Validate PR number.
/// PR numbers must be positive integers.
pub fn validate_pr_number(pr: u32) -> Result<(), MergeAgentError> {
    if pr == 0 {
        return Err(MergeAgentError::Validation(
            "PR number must be a positive integer".to_string(),
        ));
    }
    Ok(())
}

/// Emit a successful result to stdout as JSON.
pub fn emit_success(task_id: &str, pr_number: u32, message: Option<String>) -> Result<()> {
    let result = MergeAgentResult {
        task_id: task_id.to_string(),
        pr_number,
        merged: true,
        message,
    };
    emit_result(&result)
}

/// Emit an error result to stdout as JSON (for structured error consumption).
pub fn emit_error(task_id: &str, pr_number: u32, error: &MergeAgentError) -> Result<()> {
    let result = MergeAgentResult {
        task_id: task_id.to_string(),
        pr_number,
        merged: false,
        message: Some(error.message().to_string()),
    };
    emit_result(&result)
}

/// Write result to stdout as JSON.
fn emit_result(result: &MergeAgentResult) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{}", json)?;
    Ok(())
}

/// Write diagnostic message to stderr.
pub fn emit_diagnostic(message: &str) {
    eprintln!("{}", message);
}

/// Handle the merge-agent command.
/// For RQ-0942, this only validates inputs and emits the result contract.
/// The actual execution logic is added in RQ-0943.
pub fn handle_merge_agent(task_id: &str, pr_number: u32) -> Result<i32> {
    // Validate inputs
    if let Err(err) = validate_task_id(task_id) {
        emit_diagnostic(&format!("Validation error: {}", err.message()));
        emit_error(task_id, pr_number, &err)?;
        return Ok(err.exit_code());
    }

    if let Err(err) = validate_pr_number(pr_number) {
        emit_diagnostic(&format!("Validation error: {}", err.message()));
        emit_error(task_id, pr_number, &err)?;
        return Ok(err.exit_code());
    }

    // RQ-0943 will add the actual merge execution here.
    // For now, emit a placeholder success result to establish the contract.
    emit_diagnostic(&format!(
        "merge-agent: task={}, pr={} - CLI contract validated (execution in RQ-0943)",
        task_id, pr_number
    ));

    emit_success(
        task_id,
        pr_number,
        Some("CLI contract validated - execution not yet implemented".to_string()),
    )?;

    // Return 0 for successful validation (RQ-0943 will change this based on actual merge result)
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_task_id_accepts_valid_format() {
        assert!(validate_task_id("RQ-0942").is_ok());
        assert!(validate_task_id("TASK-001").is_ok());
        assert!(validate_task_id("feature_123").is_ok());
    }

    #[test]
    fn validate_task_id_rejects_empty() {
        let err = validate_task_id("").unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert!(err.message().contains("empty"));
    }

    #[test]
    fn validate_task_id_rejects_special_chars() {
        let err = validate_task_id("RQ/0942").unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert!(err.message().contains("Invalid task ID format"));
    }

    #[test]
    fn validate_pr_number_accepts_positive() {
        assert!(validate_pr_number(1).is_ok());
        assert!(validate_pr_number(42).is_ok());
        assert!(validate_pr_number(999999).is_ok());
    }

    #[test]
    fn validate_pr_number_rejects_zero() {
        let err = validate_pr_number(0).unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert!(err.message().contains("positive integer"));
    }

    #[test]
    fn merge_agent_error_exit_codes() {
        assert_eq!(MergeAgentError::Validation("test".into()).exit_code(), 2);
        assert_eq!(MergeAgentError::Runtime("test".into()).exit_code(), 1);
        assert_eq!(
            MergeAgentError::Domain {
                code: 3,
                message: "test".into()
            }
            .exit_code(),
            3
        );
        assert_eq!(
            MergeAgentError::Domain {
                code: 5,
                message: "test".into()
            }
            .exit_code(),
            5
        );
    }

    #[test]
    fn emit_success_produces_valid_json() {
        let result = MergeAgentResult {
            task_id: "RQ-0942".to_string(),
            pr_number: 42,
            merged: true,
            message: Some("Success".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("RQ-0942"));
        assert!(json.contains("42"));
        assert!(json.contains("true"));
    }

    #[test]
    fn merge_agent_result_serialization_roundtrip() {
        let original = MergeAgentResult {
            task_id: "RQ-0942".to_string(),
            pr_number: 42,
            merged: true,
            message: Some("Test message".to_string()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: MergeAgentResult = serde_json::from_str(&json).unwrap();
        assert_eq!(original.task_id, deserialized.task_id);
        assert_eq!(original.pr_number, deserialized.pr_number);
        assert_eq!(original.merged, deserialized.merged);
        assert_eq!(original.message, deserialized.message);
    }

    #[test]
    fn merge_agent_result_without_message() {
        let result = MergeAgentResult {
            task_id: "RQ-0942".to_string(),
            pr_number: 42,
            merged: false,
            message: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("null"));
    }
}
