use super::*;

#[test]
fn plan_task_decomposition_rejects_stray_mutations() -> Result<()> {
    let (_temp, mut resolved) = test_resolved()?;
    git_test::init_repo(&resolved.repo_root)?;
    std::fs::write(resolved.repo_root.join("README.md"), "# decompose test\n")?;
    queue::save_queue(&resolved.queue_path, &QueueFile::default())?;
    queue::save_queue(&resolved.done_path, &QueueFile::default())?;
    git_test::commit_all(&resolved.repo_root, "init")?;

    let readme_path = resolved.repo_root.join("README.md");
    let runner_script = format!(
        r#"#!/bin/sh
set -e
cat >/dev/null
printf '\nstray edit\n' >> "{readme_path}"
echo '{{"type":"item.completed","item":{{"type":"agent_message","text":"{{\"tree\":{{\"title\":\"Ship auth\",\"children\":[]}}}}"}}}}'
"#,
        readme_path = readme_path.display(),
    );
    let runner_dir = TempDir::new()?;
    let runner_path = create_fake_runner(runner_dir.path(), "codex", &runner_script)?;
    resolved.config.agent.codex_bin = Some(runner_path.to_string_lossy().to_string());
    resolved.config.agent.git_revert_mode = Some(crate::contracts::GitRevertMode::Enabled);

    let opts = TaskDecomposeOptions {
        source: TaskDecomposeSourceInput::Inline("Ship auth".to_string()),
        attach_to_task_id: None,
        max_depth: 3,
        max_children: 5,
        max_nodes: 10,
        status: TaskStatus::Draft,
        parent_status: TaskStatus::Draft,
        leaf_status: TaskStatus::Draft,
        child_policy: DecompositionChildPolicy::Fail,
        with_dependencies: false,
        runner_override: Some(crate::contracts::Runner::Codex),
        model_override: Some(crate::contracts::Model::Gpt53Codex),
        reasoning_effort_override: None,
        runner_cli_overrides: crate::contracts::RunnerCliOptionsPatch::default(),
        repoprompt_tool_injection: false,
        stream_planner_output: false,
        force: false,
    };

    let err = super::super::plan_task_decomposition(&resolved, &opts)
        .expect_err("planner should fail on stray mutation");
    let message = format!("{err:#}");
    assert!(message.contains("Queue-only mutation boundary violated."));
    assert!(message.contains("README.md"));
    assert_eq!(std::fs::read_to_string(&readme_path)?, "# decompose test\n");
    Ok(())
}
