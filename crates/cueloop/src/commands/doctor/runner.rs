//! Runner configuration and binary checks for the doctor command.
//!
//! Purpose:
//! - Runner configuration and binary checks for the doctor command.
//!
//! Responsibilities:
//! - Verify runner binary availability
//! - Check model compatibility with selected runner
//! - Validate instruction file configuration
//!
//! Not handled here:
//! - Runner execution (see runner module)
//! - Git repository checks (see git.rs)
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Runner binaries may have different flag conventions
//! - Plugin runners require separate validation

use crate::commands::doctor::types::{CheckResult, DoctorReport};
use crate::config;
use crate::constants::versions::CURSOR_SDK_VERSION;
use crate::contracts::{BlockingState, Runner};
use crate::prompts;
use crate::runner;
use crate::runutil::{ManagedCommand, TimeoutClass, execute_managed_command};
use serde::Deserialize;
use std::process::Command;

const MIN_CURSOR_SDK_NODE_MAJOR: u32 = 18;

#[derive(Debug, Deserialize)]
pub(crate) struct CursorSdkPackageCheck {
    pub(crate) source: String,
    pub(crate) entrypoint: String,
    pub(crate) detected_version: Option<String>,
    pub(crate) preferred_version: String,
}

impl CursorSdkPackageCheck {
    pub(crate) fn version_mismatch(&self) -> bool {
        self.detected_version
            .as_deref()
            .is_some_and(|version| version != self.preferred_version)
    }
}

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
                let detected = check.detected_version.as_deref().unwrap_or("unknown");
                report.add(CheckResult::warning(
                    "runner",
                    "cursor_sdk_package",
                    &format!(
                        "@cursor/sdk {detected} from {} differs from CueLoop's preferred/tested {}; Cursor runner will try it best-effort",
                        check.source, check.preferred_version
                    ),
                    false,
                    Some(&format!("SDK entrypoint: {}", check.entrypoint)),
                ));
            }
            Ok(_) => {}
            Err(e) => {
                let message = format!("Cursor SDK package check failed for '{}': {}", bin_name, e);
                let guidance = format!(
                    "Install @cursor/sdk in this workspace (preferred/tested: `npm install --save-exact @cursor/sdk@{CURSOR_SDK_VERSION}`), \
                        install it globally, or set CUELOOP_CURSOR_SDK_MODULE_PATH to a trusted SDK entrypoint."
                );
                let blocking = runner_blocking_state(
                    "runner",
                    "cursor_sdk_missing",
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

pub(crate) fn check_cursor_sdk_package(
    node_bin: &str,
    cwd: &std::path::Path,
) -> anyhow::Result<CursorSdkPackageCheck> {
    let script = r#"
const fs = require('fs');
const path = require('path');
const { createRequire } = require('module');
const { pathToFileURL } = require('url');
const { execFileSync } = require('child_process');

const preferredVersion = '__CUELOOP_CURSOR_SDK_VERSION__';

function findCursorSdkPackageJson(entrypoint) {
  let current = path.resolve(entrypoint);
  if (!fs.existsSync(current)) return null;
  if (!fs.statSync(current).isDirectory()) current = path.dirname(current);
  while (true) {
    const candidate = path.join(current, 'package.json');
    if (fs.existsSync(candidate)) {
      try {
        const pkg = JSON.parse(fs.readFileSync(candidate, 'utf8'));
        if (pkg.name === '@cursor/sdk') return candidate;
      } catch { return null; }
    }
    const parent = path.dirname(current);
    if (parent === current) return null;
    current = parent;
  }
}

function metadata(entrypoint) {
  const packageJsonPath = findCursorSdkPackageJson(entrypoint);
  if (!packageJsonPath) return { version: null };
  const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  return { version: pkg.version || null };
}

function candidate(source, entrypoint, shouldImport) {
  return { source, entrypoint: path.resolve(entrypoint), shouldImport };
}

function globalRoots() {
  const roots = [];
  if (process.env.CUELOOP_CURSOR_SDK_GLOBAL_ROOT) return [path.resolve(process.env.CUELOOP_CURSOR_SDK_GLOBAL_ROOT)];
  try {
    const root = execFileSync('npm', ['root', '-g'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
    if (root) roots.push(root);
  } catch {}
  return [...new Set(roots.map((root) => path.resolve(root)))];
}

function resolveFromGlobalRoot(root) {
  const packageJson = path.join(root, '@cursor', 'sdk', 'package.json');
  if (fs.existsSync(packageJson)) return createRequire(packageJson).resolve('./');
  const requireFromGlobalRoot = createRequire(path.join(root, 'package.json'));
  return requireFromGlobalRoot.resolve('@cursor/sdk', { paths: [root] });
}

function resolveCandidate() {
  const configured = process.env.CUELOOP_CURSOR_SDK_MODULE_PATH;
  if (configured) return candidate('env', configured, true);

  try {
    const requireFromWorkspace = createRequire(path.join(process.cwd(), 'package.json'));
    const resolved = requireFromWorkspace.resolve('@cursor/sdk', { paths: [process.cwd()] });
    return candidate('workspace', resolved, false);
  } catch (error) {}

  for (const root of globalRoots()) {
    try {
      const resolved = resolveFromGlobalRoot(root);
      return candidate('global', resolved, true);
    } catch (error) {}
  }
  throw new Error('@cursor/sdk could not be resolved from CUELOOP_CURSOR_SDK_MODULE_PATH, the workspace, or global npm roots');
}

function normalizeSdkModule(moduleNamespace) {
  const candidates = [moduleNamespace, moduleNamespace && moduleNamespace.default, moduleNamespace && moduleNamespace.default && moduleNamespace.default.default];
  const sdk = candidates.find((candidate) => candidate && candidate.Agent);
  if (!sdk) throw new Error('Loaded @cursor/sdk module does not expose Agent');
}

(async () => {
  const resolved = resolveCandidate();
  if (resolved.shouldImport) normalizeSdkModule(await import(pathToFileURL(resolved.entrypoint).href));
  const meta = metadata(resolved.entrypoint);
  process.stdout.write(JSON.stringify({
    source: resolved.source,
    entrypoint: resolved.entrypoint,
    detected_version: meta.version,
    preferred_version: preferredVersion
  }));
})().catch((error) => {
  console.error(error && error.stack ? error.stack : String(error));
  process.exit(1);
});
"#
    .replace("__CUELOOP_CURSOR_SDK_VERSION__", CURSOR_SDK_VERSION);
    let mut command = Command::new(node_bin);
    command
        .current_dir(cwd)
        .arg("-e")
        .arg(script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let output = execute_managed_command(ManagedCommand::new(
        command,
        "doctor runner probe: Cursor SDK package".to_string(),
        TimeoutClass::Probe,
    ))?
    .into_output();

    if output.status.success() {
        Ok(serde_json::from_slice(&output.stdout)?)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(
            "{}",
            if stderr.trim().is_empty() {
                "@cursor/sdk could not be resolved from CUELOOP_CURSOR_SDK_MODULE_PATH, the workspace, or global npm roots".to_string()
            } else {
                stderr.trim().to_string()
            }
        ))
    }
}

pub(crate) fn check_cursor_sdk_node_version(node_bin: &str) -> anyhow::Result<()> {
    let mut command = Command::new(node_bin);
    command
        .args(["-p", "process.versions.node"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let output = execute_managed_command(ManagedCommand::new(
        command,
        format!("doctor runner probe: {node_bin} Cursor SDK Node version"),
        TimeoutClass::Probe,
    ))?
    .into_output();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "{}",
            if stderr.trim().is_empty() {
                format!(
                    "failed to read Node version, exit status: {}",
                    output.status
                )
            } else {
                stderr.trim().to_string()
            }
        );
    }

    let version = String::from_utf8_lossy(&output.stdout);
    ensure_cursor_sdk_node_version_supported(version.trim())
}

fn ensure_cursor_sdk_node_version_supported(version: &str) -> anyhow::Result<()> {
    let major = version
        .trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|part| part.parse::<u32>().ok());

    match major {
        Some(major) if major >= MIN_CURSOR_SDK_NODE_MAJOR => Ok(()),
        Some(major) => anyhow::bail!(
            "Node {major} is unsupported; Cursor SDK requires Node {MIN_CURSOR_SDK_NODE_MAJOR} or newer"
        ),
        None => anyhow::bail!(
            "could not parse Node version '{version}'; Cursor SDK requires Node {MIN_CURSOR_SDK_NODE_MAJOR} or newer"
        ),
    }
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
    use super::{
        check_cursor_sdk_package, cursor_api_key_value_configured,
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
        assert_eq!(check.source, "workspace");
        assert_eq!(check.detected_version.as_deref(), Some("1.0.12"));
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

        assert_eq!(check.source, "workspace");
        assert_eq!(check.detected_version.as_deref(), Some("1.0.10"));
        assert!(check.version_mismatch());
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

        assert_eq!(check.source, "env");
        assert_eq!(check.entrypoint, env_entrypoint.to_string_lossy().as_ref());
        assert_eq!(check.detected_version.as_deref(), Some("1.0.13"));
        assert!(check.version_mismatch());
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

        assert_eq!(check.source, "global");
        assert_eq!(
            check.entrypoint,
            global_entrypoint.canonicalize()?.to_string_lossy().as_ref()
        );
        assert_eq!(check.detected_version.as_deref(), Some("1.0.13"));
        assert!(check.version_mismatch());
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
            err.to_string().contains("could not be resolved"),
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
            err.to_string().contains("does not expose Agent"),
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
