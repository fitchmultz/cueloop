use crate::constants::versions::CURSOR_SDK_VERSION;
use crate::runutil::{ManagedCommand, TimeoutClass, execute_managed_command};
use serde::Deserialize;
use std::process::Command;

const MIN_CURSOR_SDK_NODE_MAJOR: u32 = 18;

#[derive(Debug, Deserialize)]
pub(crate) struct CursorSdkPackageCheck {
    pub(crate) selected: Option<CursorSdkSelected>,
    #[serde(default)]
    pub(crate) attempted_sources: Vec<CursorSdkAttempt>,
    #[serde(default)]
    pub(crate) warnings: Vec<String>,
    pub(crate) preferred_sdk_version: String,
    #[serde(default)]
    pub(crate) proceeded_best_effort: bool,
    pub(crate) fatal_cause: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CursorSdkSelected {
    pub(crate) source: String,
    pub(crate) entrypoint: String,
    pub(crate) package_json: Option<String>,
    pub(crate) sdk_version: Option<String>,
    pub(crate) global_root: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CursorSdkAttempt {
    pub(crate) source: String,
    pub(crate) location: Option<String>,
    pub(crate) entrypoint: Option<String>,
    pub(crate) package_json: Option<String>,
    pub(crate) sdk_version: Option<String>,
    pub(crate) global_root: Option<String>,
    pub(crate) status: String,
    pub(crate) error: Option<String>,
}

impl CursorSdkPackageCheck {
    pub(crate) fn version_mismatch(&self) -> bool {
        self.selected
            .as_ref()
            .and_then(|selected| selected.sdk_version.as_deref())
            .is_some_and(|version| version != self.preferred_sdk_version)
    }

    pub(crate) fn attempted_sources_summary(&self) -> String {
        self.attempted_sources
            .iter()
            .map(|attempt| {
                let location = attempt.location.as_deref().unwrap_or("unknown");
                let entrypoint = attempt
                    .entrypoint
                    .as_deref()
                    .map(|value| format!(" -> {value}"))
                    .unwrap_or_default();
                let version = attempt
                    .sdk_version
                    .as_deref()
                    .map(|value| format!(", version {value}"))
                    .unwrap_or_default();
                let package_json = attempt
                    .package_json
                    .as_deref()
                    .map(|value| format!(", package {value}"))
                    .unwrap_or_default();
                let global_root = attempt
                    .global_root
                    .as_deref()
                    .map(|value| format!(", global root {value}"))
                    .unwrap_or_default();
                let error = attempt
                    .error
                    .as_deref()
                    .map(|value| format!(", error: {value}"))
                    .unwrap_or_default();
                format!(
                    "{} {location}{entrypoint} [{}{version}{package_json}{global_root}{error}]",
                    attempt.source, attempt.status
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

pub(crate) fn check_cursor_sdk_package(
    node_bin: &str,
    cwd: &std::path::Path,
) -> anyhow::Result<CursorSdkPackageCheck> {
    let script = include_str!("../../../assets/cursor_sdk_doctor_probe.cjs")
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

pub(crate) fn ensure_cursor_sdk_node_version_supported(version: &str) -> anyhow::Result<()> {
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

pub(crate) fn cursor_sdk_blocking_reason(message: &str) -> &'static str {
    if message.contains("missing_sdk") {
        "cursor_sdk_missing"
    } else if message.contains("invalid_module_path")
        || message.contains("CUELOOP_CURSOR_SDK_MODULE_PATH is set but unusable")
    {
        "cursor_sdk_invalid_module_path"
    } else if message.contains("incompatible_api") || message.contains("does not expose") {
        "cursor_sdk_incompatible_api"
    } else if message.contains("import_failed") {
        "cursor_sdk_import_failed"
    } else {
        "cursor_sdk_missing"
    }
}
