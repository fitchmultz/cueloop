//! AGENTS.md template rendering helpers.
//!
//! Purpose:
//! - AGENTS.md template rendering helpers.
//!
//! Responsibilities:
//! - Select the correct embedded template for the detected project type.
//! - Build repository-map placeholders from repo structure.
//! - Fill template placeholders using resolved config and wizard hints.
//!
//! Not handled here:
//! - Interactive prompting or file writes.
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/Assumptions:
//! - Keep behavior aligned with CueLoop's canonical CLI, machine-contract, and queue semantics.

use super::types::DetectedProjectType;
use super::wizard;
use crate::config;
use crate::constants::versions::TEMPLATE_VERSION;
use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const TEMPLATE_GENERIC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/agents_templates/generic.md"
));
const TEMPLATE_RUST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/agents_templates/rust.md"
));
const TEMPLATE_PYTHON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/agents_templates/python.md"
));
const TEMPLATE_TYPESCRIPT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/agents_templates/typescript.md"
));
const TEMPLATE_GO: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/agents_templates/go.md"
));

const DOC_STYLE_RULE: &str = "Documentation style: preserve the repo's existing file/module documentation style. Add or update file-level docs only when the repo convention, public API surface, or implementation complexity warrants it.";
const TODO_CI: &str = "TODO: record this repo's CI command.";
const TODO_BUILD: &str = "TODO: record this repo's build command.";
const TODO_TEST: &str = "TODO: record this repo's test command.";
const TODO_LINT: &str = "TODO: record this repo's lint command.";
const TODO_FORMAT: &str = "TODO: record this repo's format command.";

fn format_rfc3339_now() -> String {
    let now = time::OffsetDateTime::now_utc();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommandKind {
    Ci,
    Build,
    Test,
    Lint,
    Format,
}

impl CommandKind {
    fn todo(self) -> &'static str {
        match self {
            Self::Ci => TODO_CI,
            Self::Build => TODO_BUILD,
            Self::Test => TODO_TEST,
            Self::Lint => TODO_LINT,
            Self::Format => TODO_FORMAT,
        }
    }
}

#[derive(Clone, Debug)]
enum CommandEvidence {
    UserHint,
    Detected(&'static str),
    Default(&'static str),
    Todo,
}

#[derive(Clone, Debug)]
struct RenderCommand {
    command: String,
    evidence: CommandEvidence,
}

impl RenderCommand {
    fn user_hint(kind: CommandKind, command: String) -> Self {
        if command.trim().is_empty() || command.trim() == kind.todo() {
            return Self::todo(kind);
        }

        Self {
            command,
            evidence: CommandEvidence::UserHint,
        }
    }

    fn detected(kind: CommandKind, command: impl Into<String>, source: &'static str) -> Self {
        let _ = kind;
        Self {
            command: command.into(),
            evidence: CommandEvidence::Detected(source),
        }
    }

    fn default(kind: CommandKind, command: impl Into<String>, note: &'static str) -> Self {
        let _ = kind;
        Self {
            command: command.into(),
            evidence: CommandEvidence::Default(note),
        }
    }

    fn todo(kind: CommandKind) -> Self {
        Self {
            command: kind.todo().to_string(),
            evidence: CommandEvidence::Todo,
        }
    }

    fn line(&self) -> String {
        match &self.evidence {
            CommandEvidence::Todo => self.command.clone(),
            CommandEvidence::UserHint => {
                format!("`{}` — provided during interactive setup.", self.command)
            }
            CommandEvidence::Detected(source) => {
                format!("`{}` — detected from {source}.", self.command)
            }
            CommandEvidence::Default(note) => {
                format!("`{}` — default: {note}", self.command)
            }
        }
    }

    fn ci_gate_rule(&self) -> String {
        match &self.evidence {
            CommandEvidence::Todo => format!(
                "CI gate: verify and record this repo's local verification command before claiming completion, committing, or merging. Current placeholder: {}",
                self.command
            ),
            CommandEvidence::Default(_) => format!(
                "CI gate: run `{}` before claiming completion, committing, or merging, but verify it against the repo's actual workflow before treating it as the contract.",
                self.command
            ),
            CommandEvidence::UserHint | CommandEvidence::Detected(_) => format!(
                "CI gate: run `{}` before claiming completion, committing, or merging.",
                self.command
            ),
        }
    }

    fn git_rule(&self) -> String {
        match &self.evidence {
            CommandEvidence::Todo => {
                "Do not commit until this repo's local verification command is recorded and passing."
                    .to_string()
            }
            CommandEvidence::Default(_) => format!(
                "Do not commit if the current verification default (`{}`) is failing; replace it if the repo documents a different contract.",
                self.command
            ),
            CommandEvidence::UserHint | CommandEvidence::Detected(_) => {
                format!("Do not commit if `{}` is failing.", self.command)
            }
        }
    }

    fn review_expectation(&self) -> String {
        match &self.evidence {
            CommandEvidence::Todo => {
                "document this repo's actual verification command first".to_string()
            }
            _ => format!("`{}`", self.command),
        }
    }

    fn troubleshooting_line(&self) -> String {
        match &self.evidence {
            CommandEvidence::Todo => {
                "CI failing: first replace the TODO command entry in this file with the repo's real verification command."
                    .to_string()
            }
            CommandEvidence::Default(_) => format!(
                "CI failing: start with `{}` and confirm whether the repo documents a stricter local gate.",
                self.command
            ),
            CommandEvidence::UserHint | CommandEvidence::Detected(_) => {
                format!("CI failing: run `{}`.", self.command)
            }
        }
    }
}

#[derive(Clone, Debug)]
struct RenderCommandSet {
    source_note: String,
    ci: RenderCommand,
    build: RenderCommand,
    test: RenderCommand,
    lint: RenderCommand,
    format: RenderCommand,
}

impl RenderCommandSet {
    fn detected_for_repo(repo_root: &Path, project_type: DetectedProjectType) -> Self {
        let mut commands = base_command_set(project_type);
        let mut sources = BTreeSet::new();

        if let Some((package_manager, scripts)) = package_json_scripts(repo_root)
            && overlay_package_scripts(&mut commands, &package_manager, &scripts)
        {
            sources.insert("package.json scripts");
        }

        if let Some(targets) = makefile_targets(repo_root)
            && overlay_makefile_targets(&mut commands, &targets)
        {
            sources.insert("Makefile targets");
        }

        commands.source_note = build_source_note(&commands, &sources);
        commands
    }

    fn from_hints(hints: &wizard::ConfigHints) -> Self {
        Self {
            source_note: "Provided during `cueloop context init --interactive`.".to_string(),
            ci: RenderCommand::user_hint(CommandKind::Ci, hints.ci_command.clone()),
            build: RenderCommand::user_hint(CommandKind::Build, hints.build_command.clone()),
            test: RenderCommand::user_hint(CommandKind::Test, hints.test_command.clone()),
            lint: RenderCommand::user_hint(CommandKind::Lint, hints.lint_command.clone()),
            format: RenderCommand::user_hint(CommandKind::Format, hints.format_command.clone()),
        }
    }
}

pub(super) fn suggested_config_hints(
    repo_root: &Path,
    project_type: DetectedProjectType,
) -> wizard::ConfigHints {
    let commands = RenderCommandSet::detected_for_repo(repo_root, project_type);
    wizard::ConfigHints {
        project_description: None,
        ci_command: commands.ci.command,
        build_command: commands.build.command,
        test_command: commands.test.command,
        lint_command: commands.lint.command,
        format_command: commands.format.command,
        customized_commands: false,
    }
}

pub(super) fn generate_agents_md(
    resolved: &config::Resolved,
    project_type: DetectedProjectType,
) -> Result<String> {
    generate_agents_md_with_hints(resolved, project_type, None)
}

pub(super) fn generate_agents_md_with_hints(
    resolved: &config::Resolved,
    project_type: DetectedProjectType,
    hints: Option<&wizard::ConfigHints>,
) -> Result<String> {
    let template = match project_type {
        DetectedProjectType::Rust => TEMPLATE_RUST,
        DetectedProjectType::Python => TEMPLATE_PYTHON,
        DetectedProjectType::TypeScript => TEMPLATE_TYPESCRIPT,
        DetectedProjectType::Go => TEMPLATE_GO,
        DetectedProjectType::Generic => TEMPLATE_GENERIC,
    };

    let repo_map = build_repository_map(resolved)?;
    let project_name = resolved
        .repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Project")
        .to_string();
    let id_prefix = resolved.id_prefix.clone();

    let project_description = hints
        .and_then(|h| h.project_description.as_deref())
        .unwrap_or("Add a brief description of your project here.");
    let commands = hints
        .filter(|h| h.customized_commands)
        .map(RenderCommandSet::from_hints)
        .unwrap_or_else(|| RenderCommandSet::detected_for_repo(&resolved.repo_root, project_type));

    Ok(template
        .replace("{project_name}", &project_name)
        .replace("{project_description}", project_description)
        .replace("{documentation_style_rule}", DOC_STYLE_RULE)
        .replace("{ci_gate_rule}", &commands.ci.ci_gate_rule())
        .replace("{repository_map}", &repo_map)
        .replace("{command_source_note}", &commands.source_note)
        .replace("{ci_command_line}", &commands.ci.line())
        .replace("{build_command_line}", &commands.build.line())
        .replace("{test_command_line}", &commands.test.line())
        .replace("{lint_command_line}", &commands.lint.line())
        .replace("{format_command_line}", &commands.format.line())
        .replace("{git_ci_rule}", &commands.ci.git_rule())
        .replace("{ci_review_expectation}", &commands.ci.review_expectation())
        .replace("{ci_troubleshooting}", &commands.ci.troubleshooting_line())
        .replace(
            "{package_name}",
            &project_name.to_lowercase().replace(" ", "-"),
        )
        .replace(
            "{module_name}",
            &project_name.to_lowercase().replace(" ", "_"),
        )
        .replace("{id_prefix}", &id_prefix)
        .replace("{version}", env!("CARGO_PKG_VERSION"))
        .replace("{timestamp}", &format_rfc3339_now())
        .replace("{template_version}", TEMPLATE_VERSION))
}

fn base_command_set(project_type: DetectedProjectType) -> RenderCommandSet {
    let (ci, build, test, lint, format, source_note) = match project_type {
        DetectedProjectType::Rust => (
            RenderCommand::default(
                CommandKind::Ci,
                "cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace",
                "common Rust verification suite; verify before treating it as the repo contract.",
            ),
            RenderCommand::default(
                CommandKind::Build,
                "cargo build --workspace",
                "common Rust build command; verify before treating it as the repo contract.",
            ),
            RenderCommand::default(
                CommandKind::Test,
                "cargo test --workspace",
                "common Rust test command; verify before treating it as the repo contract.",
            ),
            RenderCommand::default(
                CommandKind::Lint,
                "cargo clippy --workspace --all-targets -- -D warnings",
                "common Rust lint command; verify before treating it as the repo contract.",
            ),
            RenderCommand::default(
                CommandKind::Format,
                "cargo fmt --all",
                "common Rust format command; verify before treating it as the repo contract.",
            ),
            "No repo-specific command contract detected; entries below start from common Rust defaults and still need repo verification.",
        ),
        DetectedProjectType::Go => (
            RenderCommand::default(
                CommandKind::Ci,
                "go test ./... && go vet ./...",
                "common Go verification suite; verify before treating it as the repo contract.",
            ),
            RenderCommand::default(
                CommandKind::Build,
                "go build ./...",
                "common Go build command; verify before treating it as the repo contract.",
            ),
            RenderCommand::default(
                CommandKind::Test,
                "go test ./...",
                "common Go test command; verify before treating it as the repo contract.",
            ),
            RenderCommand::default(
                CommandKind::Lint,
                "go vet ./...",
                "common Go lint command; verify before treating it as the repo contract.",
            ),
            RenderCommand::todo(CommandKind::Format),
            "No repo-specific command contract detected; detected Go repos get common defaults where they are broadly safe and TODOs where the tool invocation is repo-specific.",
        ),
        DetectedProjectType::Python
        | DetectedProjectType::TypeScript
        | DetectedProjectType::Generic => (
            RenderCommand::todo(CommandKind::Ci),
            RenderCommand::todo(CommandKind::Build),
            RenderCommand::todo(CommandKind::Test),
            RenderCommand::todo(CommandKind::Lint),
            RenderCommand::todo(CommandKind::Format),
            "No repo-specific command contract detected; replace the TODO entries below with the repo's actual commands.",
        ),
    };

    RenderCommandSet {
        source_note: source_note.to_string(),
        ci,
        build,
        test,
        lint,
        format,
    }
}

fn build_source_note(commands: &RenderCommandSet, sources: &BTreeSet<&'static str>) -> String {
    let detected_count = [
        &commands.ci,
        &commands.build,
        &commands.test,
        &commands.lint,
        &commands.format,
    ]
    .into_iter()
    .filter(|command| matches!(command.evidence, CommandEvidence::Detected(_)))
    .count();
    let has_default = [
        &commands.ci,
        &commands.build,
        &commands.test,
        &commands.lint,
        &commands.format,
    ]
    .into_iter()
    .any(|command| matches!(command.evidence, CommandEvidence::Default(_)));
    let has_todo = [
        &commands.ci,
        &commands.build,
        &commands.test,
        &commands.lint,
        &commands.format,
    ]
    .into_iter()
    .any(|command| matches!(command.evidence, CommandEvidence::Todo));

    if detected_count == 0 {
        return commands.source_note.clone();
    }

    let sources = sources.iter().copied().collect::<Vec<_>>().join(" and ");
    if has_default || has_todo {
        format!(
            "Detected from {sources} where possible; remaining entries stay marked as defaults or TODOs until the repo documents them."
        )
    } else {
        format!("Detected from {sources}.")
    }
}

fn overlay_package_scripts(
    commands: &mut RenderCommandSet,
    package_manager: &str,
    scripts: &serde_json::Map<String, Value>,
) -> bool {
    let mut applied = false;
    for (kind, script_names) in [
        (CommandKind::Ci, &["ci"][..]),
        (CommandKind::Build, &["build"][..]),
        (CommandKind::Test, &["test"][..]),
        (CommandKind::Lint, &["lint"][..]),
        (CommandKind::Format, &["format", "fmt"][..]),
    ] {
        if let Some(script_name) = script_names
            .iter()
            .find(|name| scripts.contains_key(**name))
            .copied()
        {
            let command = package_script_command(package_manager, script_name);
            set_command(
                commands,
                kind,
                RenderCommand::detected(kind, command, "`package.json` scripts"),
            );
            applied = true;
        }
    }
    applied
}

fn overlay_makefile_targets(commands: &mut RenderCommandSet, targets: &BTreeSet<String>) -> bool {
    let mut applied = false;
    for (kind, target_names) in [
        (CommandKind::Ci, &["ci", "check", "verify"][..]),
        (CommandKind::Build, &["build"][..]),
        (CommandKind::Test, &["test"][..]),
        (CommandKind::Lint, &["lint", "clippy"][..]),
        (CommandKind::Format, &["format", "fmt"][..]),
    ] {
        if let Some(target_name) = target_names
            .iter()
            .find(|name| targets.contains(**name))
            .copied()
        {
            let command = format!("make {target_name}");
            set_command(
                commands,
                kind,
                RenderCommand::detected(kind, command, "`Makefile` targets"),
            );
            applied = true;
        }
    }
    applied
}

fn set_command(commands: &mut RenderCommandSet, kind: CommandKind, command: RenderCommand) {
    match kind {
        CommandKind::Ci => commands.ci = command,
        CommandKind::Build => commands.build = command,
        CommandKind::Test => commands.test = command,
        CommandKind::Lint => commands.lint = command,
        CommandKind::Format => commands.format = command,
    }
}

fn package_script_command(package_manager: &str, script_name: &str) -> String {
    match package_manager {
        "npm" => format!("npm run {script_name}"),
        "yarn" => format!("yarn run {script_name}"),
        "pnpm" => format!("pnpm run {script_name}"),
        "bun" => format!("bun run {script_name}"),
        _ => format!("npm run {script_name}"),
    }
}

fn package_json_scripts(repo_root: &Path) -> Option<(String, serde_json::Map<String, Value>)> {
    let package_json_path = repo_root.join("package.json");
    let content = fs::read_to_string(package_json_path).ok()?;
    let value: Value = serde_json::from_str(&content).ok()?;
    let scripts = value.get("scripts")?.as_object()?.clone();
    if scripts.is_empty() {
        return None;
    }

    Some((detect_package_manager(repo_root).to_string(), scripts))
}

fn detect_package_manager(repo_root: &Path) -> &'static str {
    if repo_root.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if repo_root.join("yarn.lock").exists() {
        "yarn"
    } else if repo_root.join("bun.lock").exists() || repo_root.join("bun.lockb").exists() {
        "bun"
    } else {
        "npm"
    }
}

fn makefile_targets(repo_root: &Path) -> Option<BTreeSet<String>> {
    let makefile_path = repo_root.join("Makefile");
    let content = fs::read_to_string(makefile_path).ok()?;
    let mut targets = BTreeSet::new();

    for line in content.lines() {
        if line.starts_with('\t') {
            continue;
        }

        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('.') {
            continue;
        }

        let Some((target, _rest)) = line.split_once(':') else {
            continue;
        };
        let target = target.trim();
        if target.is_empty()
            || target.contains(' ')
            || target.contains('\t')
            || target.contains('%')
            || target.contains('/')
            || target.contains('$')
        {
            continue;
        }

        targets.insert(target.to_string());
    }

    if targets.is_empty() {
        None
    } else {
        Some(targets)
    }
}

fn build_repository_map(resolved: &config::Resolved) -> Result<String> {
    let mut entries = Vec::new();

    let dirs_to_check = [
        ("src", "Source code"),
        ("lib", "Library code"),
        ("bin", "Binary/executable code"),
        ("tests", "Tests"),
        ("docs", "Documentation"),
        ("crates", "Rust workspace crates"),
        ("packages", "Package subdirectories"),
        ("scripts", "Utility scripts"),
        (".cueloop", "CueLoop runtime state (queue, config)"),
    ];

    for (dir, desc) in &dirs_to_check {
        if resolved.repo_root.join(dir).exists() {
            entries.push(format!("- `{}/`: {}", dir, desc));
        }
    }

    let files_to_check = [
        ("README.md", "Project overview"),
        ("Makefile", "Build automation"),
        ("Cargo.toml", "Rust package manifest"),
        ("pyproject.toml", "Python package manifest"),
        ("package.json", "Node.js package manifest"),
        ("go.mod", "Go module definition"),
    ];

    for (file, desc) in &files_to_check {
        if resolved.repo_root.join(file).exists() {
            entries.push(format!("- `{}`: {}", file, desc));
        }
    }

    if entries.is_empty() {
        entries.push("- Add your repository structure here".to_string());
    }

    Ok(entries.join("\n"))
}
