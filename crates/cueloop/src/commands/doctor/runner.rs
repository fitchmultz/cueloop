//! Runner configuration and binary checks for the doctor command.

use super::cursor_sdk_probe::{
    check_cursor_sdk_node_version, check_cursor_sdk_package, cursor_sdk_blocking_reason,
};
use crate::commands::doctor::types::{CheckResult, DoctorReport};
use crate::config;
use crate::constants::versions::CURSOR_SDK_VERSION;
use crate::contracts::{BlockingState, Runner};
use crate::prompts;
use crate::runner;
use crate::runutil::{ManagedCommand, TimeoutClass, execute_managed_command};
use std::process::Command;

fn runner_blocking_state(
    scope: &str,
    reason: &str,
    message: impl Into<String>,
    detail: impl Into<String>,
) -> BlockingState {
    BlockingState::runner_recovery(scope, reason, None, message, detail)
        .with_observed_at(crate::timeutil::now_utc_rfc3339_or_fallback())
}

pub(crate) fn check_runner(report: &mut DoctorReport, resolved: &config::Resolved) {
    let runner = resolved.config.agent.runner.clone().unwrap_or_default();
    let runner_configured = runner_configured(resolved);
    let bin_name = match runner {
        Runner::Codex => resolved
            .config
            .agent
            .codex_bin
            .as_deref()
            .unwrap_or("codex"),
        Runner::Opencode => resolved
            .config
            .agent
            .opencode_bin
            .as_deref()
            .unwrap_or("opencode"),
        Runner::Gemini => resolved
            .config
            .agent
            .gemini_bin
            .as_deref()
            .unwrap_or("gemini"),
        Runner::Claude => resolved
            .config
            .agent
            .claude_bin
            .as_deref()
            .unwrap_or("claude"),
        Runner::Cursor => resolved
            .config
            .agent
            .cursor_sdk_node_bin
            .as_deref()
            .unwrap_or("node"),
        Runner::Kimi => resolved.config.agent.kimi_bin.as_deref().unwrap_or("kimi"),
        Runner::Pi => resolved.config.agent.pi_bin.as_deref().unwrap_or("pi"),
        Runner::Plugin(_plugin_id) => {
            // For plugin runners, we can't determine the binary name from config
            // The plugin registry would need to be consulted
            return;
        }
    };

    if let Some((config_key, config_path)) = blocked_project_runner_override(resolved, &runner) {
        let message = format!(
            "project config defines execution-sensitive runner override '{}', but this repo is not trusted",
            config_key
        );
        let guidance = format!(
            "Move agent.{config_key} to trusted global config or create .cueloop/trust.jsonc before running doctor checks that execute runner binaries. Config file: {}",
            config_path.display()
        );
        report.add(
            CheckResult::error(
                "runner",
                "runner_binary",
                &message,
                false,
                Some(&guidance),
            )
            .with_blocking(runner_blocking_state(
                "runner",
                "project_runner_override_untrusted",
                "CueLoop is stalled because project runner overrides are blocked until the repo is trusted.",
                guidance.clone(),
            )),
        );
        log::error!("{message}");
        log::error!("{guidance}");
        return;
    }

    if let Err(e) = check_runner_binary(bin_name) {
        let config_key = get_runner_config_key(&runner);
        let message = format!(
            "runner binary '{}' ({:?}) check failed: {}",
            bin_name, runner, e
        );

        let guidance = if runner_configured {
            format!(
                "Install the runner binary, or configure a custom path in .cueloop/config.jsonc: {{ \"agent\": {{ \"{}\": \"/path/to/{}\" }} }}",
                config_key, bin_name
            )
        } else {
            format!(
                "Install the default runner binary, or configure agent.runner plus agent.{config_key} in .cueloop/config.jsonc before running CueLoop."
            )
        };
        let blocking = runner_blocking_state(
            "runner",
            "runner_binary_missing",
            format!("CueLoop is stalled because runner binary '{bin_name}' is unavailable."),
            format!(
                "Configured/default runner {:?} cannot execute because '{}' is not on PATH or not executable.",
                runner, bin_name
            ),
        );
        let result =
            CheckResult::error("runner", "runner_binary", &message, false, Some(&guidance))
                .with_blocking(blocking);
        report.add(result);
        log::error!("");
        log::error!("To fix this issue:");
        log::error!("  1. Install the runner binary, or");
        log::error!("  2. Configure a custom path in .cueloop/config.jsonc:");
        log::error!("     {{");
        log::error!("       \"agent\": {{");
        log::error!("         \"{}\": \"/path/to/{}\"", config_key, bin_name);
        log::error!("       }}");
        log::error!("     }}");
        log::error!("  3. Run 'cueloop doctor' to verify the fix");
    } else if runner == Runner::Cursor
        && let Err(e) = check_cursor_sdk_node_version(bin_name)
    {
        let message = format!(
            "Cursor SDK Node runtime check failed for '{}': {}",
            bin_name, e
        );
        let guidance =
            "Configure agent.cursor_sdk_node_bin to Node 18 or newer before running Cursor.";
        let blocking = runner_blocking_state(
            "runner",
            "cursor_sdk_node_unsupported",
            "CueLoop is stalled because the Cursor SDK requires Node 18 or newer.",
            guidance,
        );
        report.add(
            CheckResult::error(
                "runner",
                "cursor_sdk_node_version",
                &message,
                false,
                Some(guidance),
            )
            .with_blocking(blocking),
        );
        log::error!("{message}");
        log::error!("{guidance}");
    } else if runner == Runner::Cursor {
        match check_cursor_sdk_package(bin_name, &resolved.repo_root) {
            Ok(check) if check.version_mismatch() => {
                let selected = check
                    .selected
                    .as_ref()
                    .expect("version mismatch requires selected Cursor SDK");
                let detected = selected.sdk_version.as_deref().unwrap_or("unknown");
                let best_effort = if check.proceeded_best_effort {
                    "Cursor runner will try it best-effort"
                } else {
                    "Cursor runner can proceed best-effort when the SDK API shape is compatible"
                };
                let mut details = format!(
                    "SDK entrypoint: {}; package: {}; global root: {}; fatal cause: {}; tried: {}",
                    selected.entrypoint,
                    selected.package_json.as_deref().unwrap_or("unknown"),
                    selected.global_root.as_deref().unwrap_or("n/a"),
                    check.fatal_cause.as_deref().unwrap_or("none"),
                    check.attempted_sources_summary()
                );
                if !check.warnings.is_empty() {
                    details.push_str(&format!("; warnings: {}", check.warnings.join("; ")));
                }
                report.add(CheckResult::warning(
                    "runner",
                    "cursor_sdk_package",
                    &format!(
                        "@cursor/sdk {detected} from {} differs from CueLoop's preferred/tested {}; {best_effort}",
                        selected.source, check.preferred_sdk_version
                    ),
                    false,
                    Some(&details),
                ));
            }
            Ok(_) => {}
            Err(e) => {
                let message = format!("Cursor SDK package check failed for '{}': {}", bin_name, e);
                let reason = cursor_sdk_blocking_reason(&e.to_string());
                let guidance = format!(
                    "Install @cursor/sdk in this workspace (preferred/tested: `npm install --save-exact @cursor/sdk@{CURSOR_SDK_VERSION}`), \
                        install it globally, or set CUELOOP_CURSOR_SDK_MODULE_PATH to a trusted SDK entrypoint. Version drift is warning-only when the SDK exposes Agent."
                );
                let blocking = runner_blocking_state(
                    "runner",
                    reason,
                    "CueLoop is stalled because the Cursor SDK package is unavailable or unusable.",
                    guidance.clone(),
                );
                report.add(
                    CheckResult::error(
                        "runner",
                        "cursor_sdk_package",
                        &message,
                        false,
                        Some(&guidance),
                    )
                    .with_blocking(blocking),
                );
                log::error!("{message}");
                log::error!("{guidance}");
                return;
            }
        }
        if !cursor_api_key_configured() {
            let message = "Cursor SDK API key is not configured";
            let guidance = "Export CURSOR_API_KEY before running CueLoop with the Cursor runner.";
            let blocking = runner_blocking_state(
                "runner",
                "cursor_api_key_missing",
                "CueLoop is stalled because CURSOR_API_KEY is required for Cursor SDK runs.",
                guidance,
            );
            report.add(
                CheckResult::error("runner", "cursor_api_key", message, false, Some(guidance))
                    .with_blocking(blocking),
            );
            log::error!("{message}");
            log::error!("{guidance}");
        } else {
            report.add(CheckResult::success(
                "runner",
                "runner_binary",
                &format!("runner binary '{}' ({:?}) found", bin_name, runner),
            ));
        }
    } else {
        report.add(CheckResult::success(
            "runner",
            "runner_binary",
            &format!("runner binary '{}' ({:?}) found", bin_name, runner),
        ));
    }

    // Model Compatibility Check
    let model = runner::resolve_model_for_runner(
        &runner,
        None,
        None,
        resolved.config.agent.model.clone(),
        false,
    );
    if let Err(e) = runner::validate_model_for_runner(&runner, &model) {
        report.add(
            CheckResult::error(
                "runner",
                "model_compatibility",
                &format!("config model/runner mismatch: {}", e),
                false,
                Some("Check the model is compatible with the selected runner in config"),
            )
            .with_blocking(runner_blocking_state(
                "runner",
                "model_incompatible",
                "CueLoop is stalled because the selected runner/model combination is invalid.",
                e.to_string(),
            )),
        );
    } else {
        report.add(CheckResult::success(
            "runner",
            "model_compatibility",
            &format!(
                "model '{}' compatible with runner '{:?}'",
                model.as_str(),
                runner
            ),
        ));
    }

    // Instruction file injection checks
    let instruction_warnings =
        prompts::instruction_file_warnings(&resolved.repo_root, &resolved.config);

    // Check if repo AGENTS.md is explicitly configured
    let repo_agents_configured = resolved
        .config
        .agent
        .instruction_files
        .as_ref()
        .map(|files| {
            files.iter().any(|p| {
                let resolved = resolved.repo_root.join(p);
                resolved.ends_with("AGENTS.md")
            })
        })
        .unwrap_or(false);
    let repo_agents_path = resolved.repo_root.join("AGENTS.md");
    let repo_agents_exists = repo_agents_path.exists();

    if instruction_warnings.is_empty() {
        if let Some(files) = resolved.config.agent.instruction_files.as_ref()
            && !files.is_empty()
        {
            report.add(CheckResult::success(
                "runner",
                "instruction_files",
                &format!(
                    "instruction_files valid ({} configured file(s))",
                    files.len()
                ),
            ));
        }
        // Report status of repo AGENTS.md based on configuration
        if repo_agents_configured && repo_agents_exists {
            report.add(CheckResult::success(
                "runner",
                "agents_md",
                "AGENTS.md configured and readable",
            ));
        } else if repo_agents_exists && !repo_agents_configured {
            report.add(CheckResult::warning(
                "runner",
                "agents_md",
                "AGENTS.md exists at repo root but is not configured for injection. \
                 To enable, add 'AGENTS.md' to agent.instruction_files in your config.",
                false,
                Some("Add 'AGENTS.md' to agent.instruction_files in .cueloop/config.jsonc"),
            ));
        }
    } else {
        for warning in instruction_warnings {
            report.add(CheckResult::warning(
                "runner",
                "instruction_files",
                &warning,
                false,
                Some("Check instruction file paths in config"),
            ));
        }
    }
}

fn blocked_project_runner_override(
    resolved: &config::Resolved,
    runner: &Runner,
) -> Option<(&'static str, std::path::PathBuf)> {
    let config_key = get_runner_config_key(runner);
    if config_key == "plugin_bin" {
        return None;
    }

    let repo_trust = config::load_repo_trust(&resolved.repo_root).ok()?;
    if repo_trust.is_trusted() {
        return None;
    }

    let project_path = resolved.project_config_path.as_ref()?;
    if !project_path.exists() {
        return None;
    }

    let layer = config::load_layer(project_path).ok()?;
    if runner_override_is_configured(&layer.agent, runner) {
        return Some((config_key, project_path.clone()));
    }

    None
}

fn runner_override_is_configured(agent: &crate::contracts::AgentConfig, runner: &Runner) -> bool {
    match runner {
        Runner::Codex => agent.codex_bin.is_some(),
        Runner::Opencode => agent.opencode_bin.is_some(),
        Runner::Gemini => agent.gemini_bin.is_some(),
        Runner::Claude => agent.claude_bin.is_some(),
        Runner::Cursor => agent.cursor_sdk_node_bin.is_some(),
        Runner::Kimi => agent.kimi_bin.is_some(),
        Runner::Pi => agent.pi_bin.is_some(),
        Runner::Plugin(_) => false,
    }
}

pub(crate) fn runner_configured(resolved: &config::Resolved) -> bool {
    let mut configured = false;
    let mut consider_layer = |path: &std::path::Path| {
        if configured {
            return;
        }
        let layer = match config::load_layer(path) {
            Ok(layer) => layer,
            Err(err) => {
                log::warn!("Unable to load config layer at {}: {}", path.display(), err);
                return;
            }
        };
        configured = layer.agent.runner.is_some()
            || layer.agent.codex_bin.is_some()
            || layer.agent.opencode_bin.is_some()
            || layer.agent.gemini_bin.is_some()
            || layer.agent.claude_bin.is_some()
            || layer.agent.cursor_sdk_node_bin.is_some()
            || layer.agent.kimi_bin.is_some()
            || layer.agent.pi_bin.is_some();
    };

    if let Some(path) = resolved.global_config_path.as_ref()
        && path.exists()
    {
        consider_layer(path);
    }
    if let Some(path) = resolved.project_config_path.as_ref()
        && path.exists()
    {
        consider_layer(path);
    }

    configured
}

/// Check if a runner binary is executable by trying multiple common flags.
///
/// Tries the following in order:
/// 1. `--version`
/// 2. `-V`
/// 3. `--help`
/// 4. `help`
///
/// Returns Ok if any invocation succeeds.
pub(crate) fn check_runner_binary(bin: &str) -> anyhow::Result<()> {
    let fallbacks: &[&[&str]] = &[&["--version"], &["-V"], &["--help"], &["help"]];

    for args in fallbacks {
        match check_command(bin, args) {
            Ok(()) => return Ok(()),
            Err(_) => continue,
        }
    }

    Err(anyhow::anyhow!(
        "tried: {}",
        fallbacks
            .iter()
            .map(|a| a.join(" "))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn cursor_api_key_configured() -> bool {
    cursor_api_key_value_configured(std::env::var_os("CURSOR_API_KEY"))
}

fn cursor_api_key_value_configured(value: Option<std::ffi::OsString>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}

/// Get the config key for a runner's binary path override.
pub(crate) fn get_runner_config_key(runner: &Runner) -> &'static str {
    match runner {
        Runner::Codex => "codex_bin",
        Runner::Opencode => "opencode_bin",
        Runner::Gemini => "gemini_bin",
        Runner::Claude => "claude_bin",
        Runner::Cursor => "cursor_sdk_node_bin",
        Runner::Kimi => "kimi_bin",
        Runner::Pi => "pi_bin",
        Runner::Plugin(_) => "plugin_bin",
    }
}

fn check_command(bin: &str, args: &[&str]) -> anyhow::Result<()> {
    let mut command = Command::new(bin);
    command
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    let output = execute_managed_command(ManagedCommand::new(
        command,
        format!("doctor runner probe: {} {}", bin, args.join(" ")),
        TimeoutClass::Probe,
    ))?
    .into_output();

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr_msg = if stderr.trim().is_empty() {
            format!(
                "command '{}' {:?} failed with exit status: {}",
                bin, args, output.status
            )
        } else {
            format!(
                "command '{}' {:?} failed with exit status {}: {}",
                bin,
                args,
                output.status,
                stderr.trim()
            )
        };
        Err(anyhow::anyhow!(stderr_msg))
    }
}

#[cfg(test)]
mod tests {
    use super::cursor_api_key_value_configured;
    use crate::commands::doctor::cursor_sdk_probe::{
        check_cursor_sdk_package, cursor_sdk_blocking_reason,
        ensure_cursor_sdk_node_version_supported,
    };
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::Mutex;

    static CURSOR_SDK_ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }

        fn unset(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::remove_var(key) };
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.previous.as_ref() {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    fn node_bin() -> Option<PathBuf> {
        let output = Command::new("node")
            .args(["-p", "process.execPath"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8(output.stdout).ok()?;
        Some(PathBuf::from(path.trim()))
    }

    fn write_workspace_sdk(temp: &tempfile::TempDir, version: &str) -> anyhow::Result<PathBuf> {
        std::fs::write(temp.path().join("package.json"), r#"{"type":"module"}"#)?;
        let sdk_dir = temp.path().join("node_modules/@cursor/sdk");
        std::fs::create_dir_all(&sdk_dir)?;
        std::fs::write(
            sdk_dir.join("package.json"),
            format!(r#"{{"name":"@cursor/sdk","version":"{version}","main":"index.js"}}"#),
        )?;
        std::fs::write(
            sdk_dir.join("index.js"),
            "import fs from 'node:fs'; fs.writeFileSync('sdk-imported', 'yes'); export class Agent {}",
        )?;
        Ok(sdk_dir.join("index.js"))
    }

    fn write_importable_sdk(root: &std::path::Path, version: &str) -> anyhow::Result<PathBuf> {
        std::fs::create_dir_all(root)?;
        std::fs::write(root.join("package.json"), r#"{}"#)?;
        let sdk_dir = root.join("@cursor/sdk");
        std::fs::create_dir_all(&sdk_dir)?;
        std::fs::write(
            sdk_dir.join("package.json"),
            format!(
                r#"{{"name":"@cursor/sdk","version":"{version}","type":"module","main":"index.js"}}"#
            ),
        )?;
        std::fs::write(sdk_dir.join("index.js"), "export class Agent {}")?;
        Ok(sdk_dir.join("index.js"))
    }

    #[test]
    fn cursor_sdk_workspace_probe_resolves_without_importing_package() -> anyhow::Result<()> {
        let Some(node) = node_bin() else {
            return Ok(());
        };
        let _lock = CURSOR_SDK_ENV_LOCK
            .lock()
            .expect("cursor SDK env lock poisoned");
        let _module_path = EnvGuard::unset("CUELOOP_CURSOR_SDK_MODULE_PATH");
        let _global_root = EnvGuard::unset("CUELOOP_CURSOR_SDK_GLOBAL_ROOT");

        let temp = tempfile::TempDir::new()?;
        write_workspace_sdk(&temp, "1.0.12")?;

        let check = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())?;
        let selected = check.selected.as_ref().expect("selected SDK");
        assert_eq!(selected.source, "workspace");
        assert_eq!(selected.sdk_version.as_deref(), Some("1.0.12"));
        assert!(!check.version_mismatch());

        assert!(
            !temp.path().join("sdk-imported").exists(),
            "doctor workspace SDK probe must not import repo-local package code"
        );
        Ok(())
    }

    #[test]
    fn cursor_sdk_workspace_probe_warns_wrong_version_without_importing() -> anyhow::Result<()> {
        let Some(node) = node_bin() else {
            return Ok(());
        };
        let _lock = CURSOR_SDK_ENV_LOCK
            .lock()
            .expect("cursor SDK env lock poisoned");
        let _module_path = EnvGuard::unset("CUELOOP_CURSOR_SDK_MODULE_PATH");
        let _global_root = EnvGuard::unset("CUELOOP_CURSOR_SDK_GLOBAL_ROOT");

        let temp = tempfile::TempDir::new()?;
        write_workspace_sdk(&temp, "1.0.10")?;

        let check = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())?;

        let selected = check.selected.as_ref().expect("selected SDK");
        assert_eq!(selected.source, "workspace");
        assert_eq!(selected.sdk_version.as_deref(), Some("1.0.10"));
        assert!(check.version_mismatch());
        assert!(check.proceeded_best_effort);
        assert!(
            check
                .warnings
                .iter()
                .any(|warning| warning.contains("preferred/tested"))
        );
        assert!(check.attempted_sources_summary().contains("workspace"));
        assert!(
            !temp.path().join("sdk-imported").exists(),
            "doctor workspace SDK version probe must not import repo-local package code"
        );
        Ok(())
    }

    #[test]
    fn cursor_sdk_probe_honors_env_override_before_workspace() -> anyhow::Result<()> {
        let Some(node) = node_bin() else {
            return Ok(());
        };
        let _lock = CURSOR_SDK_ENV_LOCK
            .lock()
            .expect("cursor SDK env lock poisoned");
        let _global_root = EnvGuard::unset("CUELOOP_CURSOR_SDK_GLOBAL_ROOT");

        let temp = tempfile::TempDir::new()?;
        write_workspace_sdk(&temp, "1.0.12")?;
        let env_root = temp.path().join("env_node_modules");
        let env_entrypoint = write_importable_sdk(&env_root, "1.0.13")?;
        let _module_path = EnvGuard::set("CUELOOP_CURSOR_SDK_MODULE_PATH", &env_entrypoint);

        let check = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())?;

        let selected = check.selected.as_ref().expect("selected SDK");
        assert_eq!(selected.source, "env");
        assert_eq!(
            selected.entrypoint,
            env_entrypoint.to_string_lossy().as_ref()
        );
        assert_eq!(selected.sdk_version.as_deref(), Some("1.0.13"));
        assert!(check.version_mismatch());
        assert!(check.proceeded_best_effort);
        assert!(check.attempted_sources_summary().contains("env"));
        Ok(())
    }

    #[test]
    fn cursor_sdk_probe_falls_back_to_global_root() -> anyhow::Result<()> {
        let Some(node) = node_bin() else {
            return Ok(());
        };
        let _lock = CURSOR_SDK_ENV_LOCK
            .lock()
            .expect("cursor SDK env lock poisoned");
        let _module_path = EnvGuard::unset("CUELOOP_CURSOR_SDK_MODULE_PATH");

        let temp = tempfile::TempDir::new()?;
        std::fs::write(temp.path().join("package.json"), r#"{"type":"module"}"#)?;
        let global_root = temp.path().join("global_node_modules");
        let global_entrypoint = write_importable_sdk(&global_root, "1.0.13")?;
        let _global_root = EnvGuard::set("CUELOOP_CURSOR_SDK_GLOBAL_ROOT", &global_root);

        let check = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())?;

        let selected = check.selected.as_ref().expect("selected SDK");
        assert_eq!(selected.source, "global");
        assert_eq!(
            selected.entrypoint,
            global_entrypoint.canonicalize()?.to_string_lossy().as_ref()
        );
        assert_eq!(selected.sdk_version.as_deref(), Some("1.0.13"));
        assert_eq!(
            selected.global_root.as_deref(),
            Some(global_root.to_string_lossy().as_ref())
        );
        assert!(check.version_mismatch());
        assert!(check.proceeded_best_effort);
        assert!(check.attempted_sources_summary().contains("global"));
        Ok(())
    }

    #[test]
    fn cursor_sdk_probe_errors_when_sdk_missing() -> anyhow::Result<()> {
        let Some(node) = node_bin() else {
            return Ok(());
        };
        let _lock = CURSOR_SDK_ENV_LOCK
            .lock()
            .expect("cursor SDK env lock poisoned");
        let _module_path = EnvGuard::unset("CUELOOP_CURSOR_SDK_MODULE_PATH");
        let _global_root = EnvGuard::unset("CUELOOP_CURSOR_SDK_GLOBAL_ROOT");
        let empty_bin = tempfile::TempDir::new()?;
        let _path = EnvGuard::set("PATH", empty_bin.path());

        let temp = tempfile::TempDir::new()?;
        std::fs::write(temp.path().join("package.json"), r#"{"type":"module"}"#)?;

        let err = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())
            .expect_err("missing Cursor SDK should fail doctor");
        assert!(
            err.to_string().contains("missing_sdk")
                && err.to_string().contains("attempted_sources"),
            "unexpected error: {err}"
        );
        Ok(())
    }

    #[test]
    fn cursor_sdk_probe_errors_for_structurally_invalid_env_override() -> anyhow::Result<()> {
        let Some(node) = node_bin() else {
            return Ok(());
        };
        let _lock = CURSOR_SDK_ENV_LOCK
            .lock()
            .expect("cursor SDK env lock poisoned");
        let _global_root = EnvGuard::unset("CUELOOP_CURSOR_SDK_GLOBAL_ROOT");

        let temp = tempfile::TempDir::new()?;
        std::fs::write(temp.path().join("package.json"), r#"{"type":"module"}"#)?;
        let sdk_dir = temp.path().join("env_node_modules/@cursor/sdk");
        std::fs::create_dir_all(&sdk_dir)?;
        std::fs::write(
            sdk_dir.join("package.json"),
            r#"{"name":"@cursor/sdk","version":"1.0.12","type":"module","main":"index.js"}"#,
        )?;
        std::fs::write(sdk_dir.join("index.js"), "export const NotAgent = true;")?;
        let _module_path =
            EnvGuard::set("CUELOOP_CURSOR_SDK_MODULE_PATH", sdk_dir.join("index.js"));

        let err = check_cursor_sdk_package(&node.to_string_lossy(), temp.path())
            .expect_err("invalid Cursor SDK override should fail doctor");
        assert!(
            err.to_string()
                .contains("does not expose required export Agent")
                && err.to_string().contains("invalid_module_path")
                && err.to_string().contains("attempted_sources"),
            "unexpected error: {err}"
        );
        Ok(())
    }

    #[test]
    fn cursor_api_key_check_rejects_missing_or_empty_values() {
        assert!(!cursor_api_key_value_configured(None));
        assert!(!cursor_api_key_value_configured(Some(
            std::ffi::OsString::new()
        )));
        assert!(cursor_api_key_value_configured(Some(
            std::ffi::OsString::from("cursor-key")
        )));
    }

    #[test]
    fn cursor_sdk_blocking_reason_uses_fatal_cause_before_tried_location_text() {
        for (message, reason) in [
            (
                r#"{"fatal_cause":"missing_sdk","message":"tried CUELOOP_CURSOR_SDK_MODULE_PATH, workspace, and global npm roots"}"#,
                "cursor_sdk_missing",
            ),
            (
                r#"{"fatal_cause":"invalid_module_path"}"#,
                "cursor_sdk_invalid_module_path",
            ),
            (
                r#"{"fatal_cause":"incompatible_api"}"#,
                "cursor_sdk_incompatible_api",
            ),
            (
                r#"{"fatal_cause":"import_failed"}"#,
                "cursor_sdk_import_failed",
            ),
        ] {
            assert_eq!(cursor_sdk_blocking_reason(message), reason);
        }
    }

    #[test]
    fn cursor_sdk_node_version_requires_node_18_or_newer() {
        ensure_cursor_sdk_node_version_supported("18.0.0").expect("node 18 should pass");
        ensure_cursor_sdk_node_version_supported("v20.11.1").expect("node 20 should pass");

        let err =
            ensure_cursor_sdk_node_version_supported("17.9.1").expect_err("node 17 should fail");
        assert!(err.to_string().contains("requires Node 18 or newer"));

        let err = ensure_cursor_sdk_node_version_supported("not-a-version")
            .expect_err("invalid versions should fail");
        assert!(err.to_string().contains("could not parse Node version"));
    }
}
