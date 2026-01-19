use ralph::queue;
use std::fs;
use tempfile::TempDir;

#[test]
fn repair_handles_nested_colons_in_list_items() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("queue.yaml");
    let broken = r#"
version: 1
tasks:
  - id: RQ-0001
    status: todo
    title: Fix bug
    tags: [rust]
    scope: [crates]
    evidence:
      - error: invalid type
      - nested: colon: value
    plan: []
    notes: []
    request: fix it
    created_at: 2026-01-18T00:00:00Z
    updated_at: 2026-01-18T00:00:00Z
"#;
    fs::write(&path, broken)?;

    let report = queue::repair_queue(&path, "RQ", 4)?;
    assert!(report.repaired);

    let (queue, _) = queue::load_queue_with_repair(&path, "RQ", 4)?;
    assert_eq!(queue.tasks.len(), 1);
    assert_eq!(queue.tasks[0].evidence.len(), 2);
    assert_eq!(queue.tasks[0].evidence[0], "error: invalid type");
    assert_eq!(queue.tasks[0].evidence[1], "nested: colon: value");

    Ok(())
}

#[test]
fn repair_handles_colons_in_mapping_values() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("queue.yaml");
    let broken = r#"
version: 1
tasks:
  - id: RQ-0001
    status: todo
    title: Fix: the title
    tags: []
    scope: []
    evidence: []
    plan:
      - step 1: do this
    notes:
      - note: value
    request: req: value
    created_at: 2026-01-18T00:00:00Z
    updated_at: 2026-01-18T00:00:00Z
"#;
    fs::write(&path, broken)?;

    let report = queue::repair_queue(&path, "RQ", 4)?;
    assert!(report.repaired);

    let (queue, _) = queue::load_queue_with_repair(&path, "RQ", 4)?;
    assert_eq!(queue.tasks[0].title, "Fix: the title");
    assert_eq!(queue.tasks[0].plan[0], "step 1: do this");
    assert_eq!(queue.tasks[0].notes[0], "note: value");
    assert_eq!(queue.tasks[0].request.as_deref(), Some("req: value"));

    Ok(())
}

#[test]
fn repair_handles_comments() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("queue.yaml");
    let broken = r#"
version: 1
tasks:
  - id: RQ-0001
    status: todo
    title: Title # comment
    tags: []
    scope: []
    evidence:
      - evidence # comment
    plan: []
    notes: []
    request: req
    created_at: 2026-01-18T00:00:00Z
    updated_at: 2026-01-18T00:00:00Z
"#;
    fs::write(&path, broken)?;

    let (queue, _) = queue::load_queue_with_repair(&path, "RQ", 4)?;
    // serde_yaml handles comments, so no repair needed and title should be just "Title" (if serde handles it correctly)
    // Actually, "Title # comment" unquoted in YAML:
    // If # is preceded by space, it is a comment.
    // So title is "Title".
    assert_eq!(queue.tasks[0].title, "Title");

    Ok(())
}

#[test]
fn repair_handles_truncated_yaml_structure() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("queue.yaml");
    // Indentation error
    let really_broken = r#"
version: 1
  tasks:
  - id: RQ-0001
    status: todo
    title: T
    tags: []
    scope: []
    evidence: []
    plan: []
    notes: []
    request: r
    created_at: 2026-01-18T00:00:00Z
    updated_at: 2026-01-18T00:00:00Z
"#;
    fs::write(&path, really_broken)?;

    let report = queue::repair_queue(&path, "RQ", 4)?;
    assert!(report.repaired);

    let raw = fs::read_to_string(&path)?;
    assert!(raw.contains("tasks:\n"));

    Ok(())
}

#[test]
fn repair_preserves_nested_objects_structure() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("queue.yaml");
    // Indented tasks (4 spaces) - valid YAML but triggers indent > 2 logic in repair
    // We introduce a colon error in 'title' to force repair execution.
    let broken_and_indented = r#"
version: 1
tasks:
    - id: RQ-0001
      status: todo
      title: Broken: title
      tags: []
      scope: []
      evidence: []
      plan: []
      notes: []
      request: r
      created_at: 2026-01-18T00:00:00Z
      updated_at: 2026-01-18T00:00:00Z
"#;
    fs::write(&path, broken_and_indented)?;

    let report = queue::repair_queue(&path, "RQ", 4)?;
    assert!(report.repaired);

    let (queue, _) = queue::load_queue_with_repair(&path, "RQ", 4)?;
    assert_eq!(queue.tasks[0].title, "Broken: title");
    assert_eq!(queue.tasks[0].id, "RQ-0001");

    Ok(())
}

#[test]
fn repair_handles_numeric_ids_and_missing_fields() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("queue.yaml");
    let broken = r#"
version: 1
tasks:
  - id: 1
    status: todo
    title: Add logging
    tags: [logging]
    scope: [internal/cli]
    evidence: []
    plan: []
  - id: "RQ-0001"
    status: todo
    title: Duplicate id
    tags: []
    scope: []
    evidence: []
    plan: []
"#;
    fs::write(&path, broken)?;

    let report = queue::repair_queue(&path, "RQ", 4)?;
    assert!(report.repaired);

    let (queue, repaired) = queue::load_queue_with_repair(&path, "RQ", 4)?;
    assert!(!repaired, "repaired queue should parse cleanly");
    assert_eq!(queue.tasks.len(), 2);
    assert!(queue.tasks[0].id.starts_with("RQ-"));
    assert!(queue.tasks[1].id.starts_with("RQ-"));
    assert_ne!(queue.tasks[0].id, queue.tasks[1].id);
    assert!(!queue.tasks[0].tags.is_empty());
    assert!(!queue.tasks[0].scope.is_empty());
    assert!(!queue.tasks[0].evidence.is_empty());
    assert!(!queue.tasks[0].plan.is_empty());
    assert!(queue.tasks[0].request.as_ref().is_some());
    Ok(())
}

#[test]
fn repair_converts_block_scalar_list_fields() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("queue.yaml");
    let broken = r#"
version: 1
tasks:
  - id: 1
    status: todo
    title: Evidence is a block scalar
    tags: [rust]
    scope: [crates]
    evidence: |
      Evidence line one
      Evidence line two
    plan:
      - step one
    request: scan finding
    created_at: 2026-01-18T00:00:00Z
    updated_at: 2026-01-18T00:00:00Z
"#;
    fs::write(&path, broken)?;

    let report = queue::repair_queue(&path, "RQ", 4)?;
    assert!(report.repaired);

    let (queue, repaired) = queue::load_queue_with_repair(&path, "RQ", 4)?;
    assert!(!repaired);
    assert_eq!(queue.tasks.len(), 1);
    assert_eq!(queue.tasks[0].evidence.len(), 1);
    assert!(queue.tasks[0].evidence[0].contains("Evidence line one"));
    assert!(queue.tasks[0].evidence[0].contains("Evidence line two"));
    Ok(())
}

#[test]
fn repair_converts_multiline_plain_scalar_list_fields() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("queue.yaml");
    let broken = r#"
version: 1
tasks:
  - id: 2
    status: todo
    title: Evidence is a multiline plain scalar
    tags: [rust]
    scope: [crates]
    evidence: First line
      Second line continues
    plan:
      - step one
    request: scan finding
    created_at: 2026-01-18T00:00:00Z
    updated_at: 2026-01-18T00:00:00Z
"#;
    fs::write(&path, broken)?;

    let report = queue::repair_queue(&path, "RQ", 4)?;
    assert!(report.repaired);

    let (queue, repaired) = queue::load_queue_with_repair(&path, "RQ", 4)?;
    assert!(!repaired);
    assert_eq!(queue.tasks.len(), 1);
    assert_eq!(queue.tasks[0].evidence.len(), 1);
    assert!(queue.tasks[0].evidence[0].contains("First line"));
    assert!(queue.tasks[0].evidence[0].contains("Second line continues"));
    Ok(())
}
