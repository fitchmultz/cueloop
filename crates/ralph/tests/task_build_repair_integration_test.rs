use anyhow::{Context, Result};
use std::fs;
use tempfile::TempDir;

#[test]
fn task_build_repairs_preexisting_colon_scalars() -> Result<()> {
    let dir = TempDir::new()?;
    let root = dir.path();

    // Create minimal required structure
    let ralph_dir = root.join(".ralph");
    let queue_path = ralph_dir.join("queue.yaml");

    fs::create_dir_all(&ralph_dir)?;

    // Create a queue with colon scalars that need repair
    let queue_with_colon_scalars = r#"version: 1
tasks:
  - id: RQ-0001
    status: todo
    title: Fix colon: in this title
    tags:
      - test
    scope:
      - file:rs
    evidence:
      - contains colon: in evidence
    plan:
      - Repair YAML scalars with colons
      - Test task build behavior
"#;

    fs::write(&queue_path, queue_with_colon_scalars).context("write queue with colon scalars")?;

    // Load queue with repair (simulating what task build does)
    let (before, repaired_before) =
        ralph::queue::load_queue_with_repair(&queue_path).context("load queue with repair")?;

    // Verify repair happened on the initial load
    assert!(
        repaired_before,
        "task build should repair colon scalars on initial queue load"
    );

    // Verify queue content is correct after repair
    assert_eq!(before.tasks.len(), 1);
    assert_eq!(before.tasks[0].id, "RQ-0001");
    assert_eq!(
        before.tasks[0].title, "Fix colon: in this title",
        "title with colon space should be preserved correctly"
    );

    // Verify the file was actually repaired on disk
    let file_content = fs::read_to_string(&queue_path)?;
    assert!(
        file_content.contains("title: 'Fix colon: in this title'")
            || file_content.contains("title: \"Fix colon: in this title\""),
        "file on disk should have quoted colon scalar"
    );
    assert!(
        file_content.contains("- 'contains colon: in evidence'")
            || file_content.contains("- \"contains colon: in evidence\""),
        "evidence list item with colon should be quoted"
    );

    Ok(())
}
