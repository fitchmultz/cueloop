use anyhow::{Context, Result};
use std::fs;
use tempfile::TempDir;

#[test]
fn scan_uses_load_queue_with_repair() -> Result<()> {
    let dir = TempDir::new()?;
    let root = dir.path();

    // Create a queue with colon scalars that need repair
    let queue_path = root.join(".ralph").join("queue.yaml");
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
      - Test scan behavior
"#;

    // Create .ralph directory and write the queue
    fs::create_dir_all(root.join(".ralph"))?;
    fs::write(&queue_path, queue_with_colon_scalars).context("write queue with colon scalars")?;

    // Load the queue with repair
    let (queue, repaired) =
        ralph::queue::load_queue_with_repair(&queue_path).context("load queue with repair")?;

    // Verify repair happened
    assert!(
        repaired,
        "load_queue_with_repair should have repaired colon scalars"
    );

    // Verify the queue content is correct after repair
    assert_eq!(queue.tasks.len(), 1);
    assert_eq!(queue.tasks[0].id, "RQ-0001");
    assert_eq!(
        queue.tasks[0].title, "Fix colon: in this title",
        "title with colon space should be preserved correctly"
    );

    // Verify the file was actually repaired on disk
    let file_content = fs::read_to_string(&queue_path)?;
    assert!(
        file_content.contains("title: 'Fix colon: in this title'")
            || file_content.contains("title: \"Fix colon: in this title\""),
        "file on disk should have quoted colon scalar"
    );

    Ok(())
}
