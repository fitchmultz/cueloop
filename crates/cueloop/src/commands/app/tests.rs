//! App command regression tests.
//!
//! Purpose:
//! - Verify `crate::commands::app` planning and launch behavior stays stable.
//!
//! Responsibilities:
//! - Exercise launch-target planning, URL handoff construction, encoding, and workspace resolution.
//! - Cover CLI path propagation and launcher failure reporting.
//!
//! Scope:
//! - Unit tests for app-command helpers only.
//!
//! Usage:
//! - Compiled via `cargo test` when the app command module is exercised.
//!
//! Invariants/assumptions:
//! - Tests avoid real app launches by inspecting planned command specs.
//! - URL planning must remain deterministic across environments.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use crate::cli::app::AppOpenArgs;

use super::launch_plan::{
    env_assignment_for_path, installed_app_candidates_for_home, plan_open_command,
    plan_open_command_with_installed_path,
};
use super::model::{DEFAULT_APP_NAME, GUI_CLI_BIN_ENV, OpenCommandSpec};
use super::runtime::execute_launch_command;
use super::url_plan::{
    percent_encode, percent_encode_path, plan_url_command_with_installed_path,
    resolve_workspace_path,
};

#[test]
fn plan_open_command_non_macos_errors() {
    let args = AppOpenArgs {
        bundle_id: None,
        path: None,
        workspace: None,
    };

    let err = plan_open_command(false, &args, None).expect_err("expected error");
    assert!(
        err.to_string().to_lowercase().contains("macos-only"),
        "unexpected error: {err:#}"
    );
}

#[test]
fn installed_app_candidates_prioritize_system_then_home() {
    let home = PathBuf::from("/Users/tester");
    let candidates = installed_app_candidates_for_home(Some(home.clone()));

    assert_eq!(
        candidates,
        vec![
            PathBuf::from("/Applications").join(DEFAULT_APP_NAME),
            home.join("Applications").join(DEFAULT_APP_NAME),
        ]
    );
}

#[test]
fn plan_open_command_bundle_id_override_uses_open_b_when_no_installed_app() -> anyhow::Result<()> {
    let args = AppOpenArgs {
        bundle_id: Some("com.example.override".to_string()),
        path: None,
        workspace: None,
    };

    let spec = plan_open_command(true, &args, None)?;
    assert_eq!(spec.program, OsString::from("open"));
    assert_eq!(
        spec.args,
        vec![
            OsStr::new("-b").to_os_string(),
            OsStr::new("com.example.override").to_os_string()
        ]
    );
    Ok(())
}

#[test]
fn plan_open_command_path_uses_open_a() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let app_dir = temp.path().join("CueLoopMac.app");
    std::fs::create_dir_all(&app_dir)?;

    let args = AppOpenArgs {
        bundle_id: None,
        path: Some(app_dir.clone()),
        workspace: None,
    };

    let spec = plan_open_command(true, &args, None)?;
    assert_eq!(spec.program, OsString::from("open"));
    assert_eq!(
        spec.args,
        vec![
            OsStr::new("-a").to_os_string(),
            app_dir.as_os_str().to_os_string()
        ]
    );
    Ok(())
}

#[test]
fn plan_open_command_default_prefers_injected_installed_app_path() -> anyhow::Result<()> {
    let app_dir =
        crate::testsupport::path::portable_abs_path("test/Applications").join(DEFAULT_APP_NAME);
    let args = AppOpenArgs {
        bundle_id: None,
        path: None,
        workspace: None,
    };

    let spec = plan_open_command_with_installed_path(true, &args, None, Some(app_dir.clone()))?;

    assert_eq!(spec.program, OsString::from("open"));
    assert_eq!(
        spec.args,
        vec![
            OsStr::new("-a").to_os_string(),
            app_dir.as_os_str().to_os_string()
        ]
    );
    Ok(())
}

#[test]
fn plan_open_command_path_missing_errors() {
    let args = AppOpenArgs {
        bundle_id: None,
        path: Some(PathBuf::from("/definitely/not/a/real/path/CueLoopMac.app")),
        workspace: None,
    };

    let err = plan_open_command(true, &args, None).expect_err("expected error");
    assert!(
        err.to_string().to_lowercase().contains("does not exist"),
        "unexpected error: {err:#}"
    );
}

#[test]
fn plan_url_command_encodes_workspace() -> anyhow::Result<()> {
    let workspace = PathBuf::from("/Users/test/my project");
    let spec = plan_url_command_with_installed_path(
        &workspace,
        &AppOpenArgs {
            bundle_id: None,
            path: None,
            workspace: None,
        },
        None,
        Some(PathBuf::from("/Applications").join(DEFAULT_APP_NAME)),
    )?;

    assert_eq!(spec.program, OsString::from("open"));
    assert_eq!(spec.args.len(), 3);
    assert_eq!(spec.args[0], OsString::from("-a"));

    let url = spec.args[2].to_str().unwrap();
    assert!(url.starts_with("cueloop://open?workspace="));
    assert!(
        url.contains("my%20project"),
        "space should be percent-encoded"
    );
    Ok(())
}

#[test]
fn plan_url_command_handles_special_chars() -> anyhow::Result<()> {
    let workspace = PathBuf::from("/path/with&special=chars");
    let spec = plan_url_command_with_installed_path(
        &workspace,
        &AppOpenArgs {
            bundle_id: None,
            path: None,
            workspace: None,
        },
        None,
        Some(PathBuf::from("/Applications").join(DEFAULT_APP_NAME)),
    )?;

    let url = spec.args.last().unwrap().to_str().unwrap();
    assert!(url.contains("%26"), "& should be encoded as %26");
    assert!(url.contains("%3D"), "= should be encoded as %3D");
    Ok(())
}

#[test]
fn percent_encode_preserves_unreserved_chars() {
    let input = b"abc-_.~/123";
    let encoded = percent_encode(input);
    assert_eq!(encoded, "abc-_.~/123");
}

#[test]
fn percent_encode_encodes_reserved_chars() {
    let input = b"hello world";
    let encoded = percent_encode(input);
    assert_eq!(encoded, "hello%20world");
}

#[test]
fn percent_encode_encodes_unicode() {
    let input = "test/文件".as_bytes();
    let encoded = percent_encode(input);
    assert!(encoded.starts_with("test/"));
    assert!(encoded.len() > "test/文件".len());
}

#[test]
fn percent_encode_path_handles_spaces() {
    let path = PathBuf::from("/Users/test/my project");
    let encoded = percent_encode_path(&path);
    assert!(encoded.contains("%20"), "spaces should be encoded as %20");
    assert!(
        !encoded.contains(' '),
        "result should not contain literal spaces"
    );
}

#[test]
fn percent_encode_path_preserves_path_structure() {
    let path = PathBuf::from("/path/to/directory");
    let encoded = percent_encode_path(&path);
    assert!(encoded.starts_with("/path/to/"));
    assert!(encoded.contains('/'));
}

#[test]
fn plan_open_command_includes_cli_env_when_provided() -> anyhow::Result<()> {
    let args = AppOpenArgs {
        bundle_id: None,
        path: None,
        workspace: None,
    };
    let cli = crate::testsupport::path::portable_abs_path("cueloop-bin");

    let spec = plan_open_command(true, &args, Some(&cli))?;
    assert_eq!(spec.program, OsString::from("open"));
    assert!(spec.args.len() >= 4);
    assert_eq!(spec.args[0], OsString::from("--env"));
    assert_eq!(spec.args[1], env_assignment_for_path(GUI_CLI_BIN_ENV, &cli));
    assert!(
        spec.args[2] == "-a" || spec.args[2] == "-b",
        "unexpected launch args: {:?}",
        spec.args
    );
    Ok(())
}

#[test]
fn plan_url_command_never_includes_cli_param() -> anyhow::Result<()> {
    let workspace = PathBuf::from("/Users/test/workspace");
    let spec = plan_url_command_with_installed_path(
        &workspace,
        &AppOpenArgs {
            bundle_id: None,
            path: None,
            workspace: None,
        },
        None,
        Some(PathBuf::from("/Applications").join(DEFAULT_APP_NAME)),
    )?;

    let url = spec.args.last().unwrap().to_string_lossy();
    assert!(url.starts_with("cueloop://open?workspace="));
    assert!(!url.contains("&cli="));
    Ok(())
}

#[test]
fn plan_url_command_prefers_installed_app_path_over_bundle_lookup() -> anyhow::Result<()> {
    let app_dir =
        crate::testsupport::path::portable_abs_path("test/Applications").join(DEFAULT_APP_NAME);
    let workspace = PathBuf::from("/Users/test/workspace");
    let spec = plan_url_command_with_installed_path(
        &workspace,
        &AppOpenArgs {
            bundle_id: None,
            path: None,
            workspace: None,
        },
        None,
        Some(app_dir.clone()),
    )?;

    assert_eq!(spec.program, OsString::from("open"));
    assert_eq!(spec.args[0], OsString::from("-a"));
    assert_eq!(spec.args[1], app_dir.as_os_str().to_os_string());
    assert!(
        spec.args[2]
            .to_string_lossy()
            .starts_with("cueloop://open?workspace=")
    );
    Ok(())
}

#[test]
fn plan_url_command_bundle_id_uses_open_launcher() -> anyhow::Result<()> {
    let workspace = PathBuf::from("/Users/test/workspace");
    let spec = plan_url_command_with_installed_path(
        &workspace,
        &AppOpenArgs {
            bundle_id: Some("com.example.override".to_string()),
            path: None,
            workspace: None,
        },
        None,
        Some(PathBuf::from("/Applications").join(DEFAULT_APP_NAME)),
    )?;

    assert_eq!(spec.program, OsString::from("open"));
    assert_eq!(spec.args[0], OsString::from("-b"));
    assert_eq!(spec.args[1], OsString::from("com.example.override"));
    assert!(
        spec.args[2]
            .to_string_lossy()
            .starts_with("cueloop://open?workspace=")
    );
    Ok(())
}

#[test]
fn plan_url_command_includes_cli_env_when_provided() -> anyhow::Result<()> {
    let workspace = PathBuf::from("/Users/test/workspace");
    let cli = crate::testsupport::path::portable_abs_path("cueloop-bin");
    let spec = plan_url_command_with_installed_path(
        &workspace,
        &AppOpenArgs {
            bundle_id: Some("com.example.override".to_string()),
            path: None,
            workspace: None,
        },
        Some(&cli),
        Some(PathBuf::from("/Applications").join(DEFAULT_APP_NAME)),
    )?;

    assert_eq!(spec.program, OsString::from("open"));
    assert_eq!(spec.args[0], OsString::from("--env"));
    assert_eq!(spec.args[1], env_assignment_for_path(GUI_CLI_BIN_ENV, &cli));
    assert_eq!(spec.args[2], OsString::from("-b"));
    assert_eq!(spec.args[3], OsString::from("com.example.override"));
    assert!(
        spec.args[4]
            .to_string_lossy()
            .starts_with("cueloop://open?workspace=")
    );
    Ok(())
}

#[test]
fn env_assignment_prefixes_variable_name() {
    let cli = crate::testsupport::path::portable_abs_path("cueloop");
    let assignment = env_assignment_for_path(GUI_CLI_BIN_ENV, &cli);
    let text = assignment.to_string_lossy();
    assert!(text.starts_with(&format!("{GUI_CLI_BIN_ENV}=")));
    assert!(text.ends_with(&*cli.to_string_lossy()));
}

#[cfg(unix)]
#[test]
fn execute_launch_command_surfaces_launcher_failure() {
    let spec = OpenCommandSpec {
        program: OsString::from("/bin/sh"),
        args: vec![
            OsString::from("-c"),
            OsString::from("printf 'launch failed' >&2; exit 9"),
        ],
    };

    let err = execute_launch_command(&spec).expect_err("expected launcher failure");
    let text = format!("{err:#}");
    assert!(text.contains("spawn macOS app launch command"));
    assert!(text.contains("launch failed"));
}

#[test]
fn resolve_workspace_path_prefers_explicit_workspace() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let args = AppOpenArgs {
        bundle_id: None,
        path: None,
        workspace: Some(temp.path().to_path_buf()),
    };

    let resolved = resolve_workspace_path(&args)?;
    assert_eq!(resolved.as_deref(), Some(temp.path()));
    Ok(())
}

#[test]
fn resolve_workspace_path_errors_for_missing_workspace() {
    let args = AppOpenArgs {
        bundle_id: None,
        path: None,
        workspace: Some(PathBuf::from("/definitely/not/a/real/workspace")),
    };

    let err = resolve_workspace_path(&args).expect_err("expected error");
    assert!(
        err.to_string().contains("Workspace path does not exist"),
        "unexpected error: {err:#}"
    );
}
