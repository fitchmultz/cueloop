use anyhow::{Context, Result};
use std::fs;
use tempfile::TempDir;

#[test]
fn done_repair_handles_mapping_like_notes() -> Result<()> {
    let dir = TempDir::new()?;
    let done_path = dir.path().join("done.yaml");

    let raw = r#"version: 1
tasks:
  - id: RQ-9999
    status: done
    title: Done task
    tags:
      - test
    scope:
      - crates/ralph
    evidence:
      - done evidence
    plan:
      - done plan
    notes:
      - key: value
"#;

    fs::write(&done_path, raw).context("write done yaml")?;

    let (done, repaired) = ralph::queue::load_queue_or_default_with_repair(&done_path, "RQ", 4)?;
    assert!(repaired, "expected done.yaml repair");
    assert_eq!(done.tasks.len(), 1);
    assert_eq!(done.tasks[0].notes, vec!["key: value".to_string()]);

    let repaired_raw = fs::read_to_string(&done_path)?;
    assert!(
        repaired_raw.contains("- 'key: value'") || repaired_raw.contains("- \"key: value\""),
        "done.yaml should quote mapping-like list items"
    );

    Ok(())
}
