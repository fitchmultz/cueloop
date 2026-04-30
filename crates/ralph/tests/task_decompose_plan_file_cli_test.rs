//! Plan-file task decomposition CLI smoke tests.
//!
//! Purpose:
//! - Prove `ralph task decompose --from-file` handles complete ordered plans end to end.
//!
//! Responsibilities:
//! - Exercise preview and write JSON output through a stubbed runner.
//! - Validate queue/navigation commands after writing a decomposed ordered plan.
//! - Assert materialized queue order, source provenance, and dependency edges.
//!
//! Scope:
//! - Integration coverage for CLI plumbing, runner parsing, queue writes, and navigation.
//! - Not live-model quality or planner creativity.
//!
//! Usage:
//! - Run with `cargo test -p ralph-agent-loop --test task_decompose_plan_file_cli_test`.
//!
//! Invariants/assumptions:
//! - The fake runner emits a deterministic complete planner response.
//! - Plan-file dependency inference remains sibling-only and preserves planner child order.

mod test_support;

use anyhow::{Context, Result};
use ralph::contracts::{QueueFile, TaskKind};
use serde_json::Value;
use tempfile::tempdir;

const ORDERED_PLAN_PATH: &str = "docs/plans/full-plan-ordering.md";
const ORDERED_PLAN_CONTENT: &str = r#"# Full plan ordering fixture

## Phase 1: Inventory current behavior
Capture current CLI, queue, and prompt behavior.

## Phase 2: Implement prompt and planner guardrails
Tighten plan-file decomposition rules.

## Phase 3: Add regression coverage
Cover normalization, materialization, preview, and write behavior.

## Phase 4: Document acceptance workflow
Update PRD and CLI docs with queue validation/navigation checks.
"#;

const PHASE_TITLES: [&str; 4] = [
    "Phase 1: Inventory current behavior",
    "Phase 2: Implement prompt and planner guardrails",
    "Phase 3: Add regression coverage",
    "Phase 4: Document acceptance workflow",
];

#[test]
fn plan_file_decompose_preview_write_validate_and_navigate_ordered_plan() -> Result<()> {
    let temp = tempdir()?;
    test_support::ralph_init(temp.path())?;
    write_ordered_plan(temp.path())?;
    configure_fake_planner(temp.path())?;

    let (status, stdout, stderr) = test_support::run_in_dir(
        temp.path(),
        &[
            "task",
            "decompose",
            "--from-file",
            ORDERED_PLAN_PATH,
            "--with-dependencies",
            "--format",
            "json",
        ],
    );
    assert!(
        status.success(),
        "preview failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let preview: Value = parse_json_output(&stdout, "preview")?;
    assert_eq!(preview["result"]["mode"], "preview");
    assert_eq!(
        preview["result"]["preview"]["source"]["path"],
        ORDERED_PLAN_PATH
    );
    assert_eq!(preview["result"]["preview"]["plan"]["total_nodes"], 5);
    assert_eq!(phase_titles_from_preview(&preview), PHASE_TITLES.to_vec());
    assert_eq!(
        preview["result"]["preview"]["plan"]["dependency_edges"]
            .as_array()
            .expect("dependency edge array")
            .len(),
        3
    );
    assert_eq!(
        preview["result"]["preview"]["plan"]["actionability"]["root_group"]["kind"],
        "group"
    );
    assert_eq!(
        preview["result"]["preview"]["plan"]["actionability"]["first_actionable_leaf"]["planner_key"],
        "phase-1"
    );

    let (status, stdout, stderr) = test_support::run_in_dir(
        temp.path(),
        &[
            "task",
            "decompose",
            "--from-file",
            ORDERED_PLAN_PATH,
            "--with-dependencies",
            "--write",
            "--format",
            "json",
        ],
    );
    assert!(
        status.success(),
        "write failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let written: Value = parse_json_output(&stdout, "write")?;
    assert_eq!(written["result"]["mode"], "write");
    let created_ids = written["result"]["write"]["created_ids"]
        .as_array()
        .expect("created ids");
    assert_eq!(created_ids.len(), 5);
    let root_id = created_ids[0].as_str().expect("root id").to_string();
    assert_eq!(written["result"]["write"]["root_group_task_id"], root_id);
    assert_eq!(
        written["result"]["write"]["first_actionable_leaf_task_id"],
        created_ids[1]
    );

    assert_command_success(temp.path(), &["queue", "validate"]);
    let tree = assert_command_success(temp.path(), &["queue", "tree"]);
    assert_titles_in_order(
        &tree,
        &[
            "Full plan ordering fixture",
            PHASE_TITLES[0],
            PHASE_TITLES[1],
            PHASE_TITLES[2],
            PHASE_TITLES[3],
        ],
    );
    let children = assert_command_success(temp.path(), &["task", "children", &root_id]);
    assert_titles_in_order(&children, &PHASE_TITLES);

    let queue_file: QueueFile = serde_json::from_str(
        &std::fs::read_to_string(temp.path().join(".ralph/queue.jsonc")).context("read queue")?,
    )
    .context("parse queue")?;
    assert_eq!(
        queue_file
            .tasks
            .iter()
            .map(|task| task.title.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Full plan ordering fixture",
            PHASE_TITLES[0],
            PHASE_TITLES[1],
            PHASE_TITLES[2],
            PHASE_TITLES[3],
        ]
    );
    assert_eq!(
        queue_file.tasks[2].depends_on,
        vec![queue_file.tasks[1].id.clone()]
    );
    assert_eq!(
        queue_file.tasks[3].depends_on,
        vec![queue_file.tasks[2].id.clone()]
    );
    assert_eq!(
        queue_file.tasks[4].depends_on,
        vec![queue_file.tasks[3].id.clone()]
    );
    assert_eq!(queue_file.tasks[0].kind, TaskKind::Group);
    for task in queue_file.tasks.iter().skip(1) {
        assert_eq!(task.kind, TaskKind::WorkItem);
    }
    for task in &queue_file.tasks {
        assert_eq!(
            task.request.as_deref(),
            Some("Plan file docs/plans/full-plan-ordering.md")
        );
        assert!(
            task.scope.iter().any(|scope| scope == ORDERED_PLAN_PATH),
            "{} missing plan-file scope provenance",
            task.title
        );
        assert_eq!(
            task.evidence,
            vec![format!(
                "path: {} :: {} :: source plan for this decomposed task",
                ORDERED_PLAN_PATH, task.title
            )]
        );
    }

    Ok(())
}

fn write_ordered_plan(repo: &std::path::Path) -> Result<()> {
    let path = repo.join(ORDERED_PLAN_PATH);
    std::fs::create_dir_all(path.parent().expect("plan parent"))?;
    std::fs::write(path, ORDERED_PLAN_CONTENT)?;
    Ok(())
}

fn configure_fake_planner(repo: &std::path::Path) -> Result<()> {
    let planner_response = serde_json::json!({
        "warnings": [],
        "tree": {
            "key": "root",
            "title": "Full plan ordering fixture",
            "description": "Decompose the complete ordered fixture plan.",
            "plan": ["Represent the whole ordered source plan"],
            "tags": ["ordering"],
            "scope": [ORDERED_PLAN_PATH],
            "depends_on": [],
            "children": [
                raw_phase("phase-1", PHASE_TITLES[0], &[]),
                raw_phase("phase-2", PHASE_TITLES[1], &["phase-1"]),
                raw_phase("phase-3", PHASE_TITLES[2], &["phase-2"]),
                raw_phase("phase-4", PHASE_TITLES[3], &["phase-3"]),
            ]
        }
    });
    let planner_text = serde_json::to_string(&planner_response)?;
    let jsonl = serde_json::json!({
        "type": "item.completed",
        "item": {"type": "agent_message", "text": planner_text}
    })
    .to_string();
    let script = format!("#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' '{}'\n", jsonl);
    let runner_path = test_support::create_fake_runner(repo, "codex", &script)?;
    test_support::configure_runner(repo, "codex", "gpt-5.3-codex", Some(&runner_path))?;
    Ok(())
}

fn raw_phase(key: &str, title: &str, depends_on: &[&str]) -> Value {
    serde_json::json!({
        "key": key,
        "title": title,
        "description": format!("Cover source section {title}."),
        "plan": [format!("Cover source section: {title}")],
        "tags": ["ordering"],
        "scope": [ORDERED_PLAN_PATH, title],
        "depends_on": depends_on,
        "children": []
    })
}

fn parse_json_output(stdout: &str, label: &str) -> Result<Value> {
    let json_start = stdout.rfind("\n{\n").map(|index| index + 1).unwrap_or(0);
    serde_json::from_str(&stdout[json_start..])
        .with_context(|| format!("parse {label} JSON from stdout:\n{stdout}"))
}

fn phase_titles_from_preview(document: &Value) -> Vec<&str> {
    document["result"]["preview"]["plan"]["root"]["children"]
        .as_array()
        .expect("preview children")
        .iter()
        .map(|child| child["title"].as_str().expect("child title"))
        .collect()
}

fn assert_command_success(repo: &std::path::Path, args: &[&str]) -> String {
    let (status, stdout, stderr) = test_support::run_in_dir(repo, args);
    assert!(
        status.success(),
        "command {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        stdout,
        stderr
    );
    stdout
}

fn assert_titles_in_order(output: &str, titles: &[&str]) {
    let mut previous = 0;
    for title in titles {
        let offset = output[previous..]
            .find(title)
            .unwrap_or_else(|| panic!("missing title {title} in output:\n{output}"));
        previous += offset + title.len();
    }
}
