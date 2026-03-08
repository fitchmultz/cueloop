//! JSONC preference and tilde-expansion tests.

use super::*;

#[test]
#[serial]
fn test_resolve_queue_path_expands_tilde_to_home() {
    let _guard = env_lock().lock().expect("env lock");
    let original_home = env::var("HOME").ok();

    unsafe { env::set_var("HOME", "/custom/home") };

    let repo_root = PathBuf::from("/repo/root");
    let mut cfg = Config::default();
    cfg.queue.file = Some(PathBuf::from("~/myqueue.json"));

    let queue_path = config::resolve_queue_path(&repo_root, &cfg).unwrap();
    assert_eq!(queue_path, PathBuf::from("/custom/home/myqueue.json"));

    // Restore HOME
    match original_home {
        Some(v) => unsafe { env::set_var("HOME", v) },
        None => unsafe { env::remove_var("HOME") },
    }
}

// Tests for .jsonc file format support (RQ-0807)

#[test]
fn test_find_repo_root_via_ralph_queue_jsonc() {
    let dir = TempDir::new().expect("create temp dir");
    create_queue_jsonc(&dir, r#"{"version":1,"tasks":[]}"#);

    let repo_root = config::find_repo_root(dir.path());
    assert_eq!(repo_root, dir.path());
}

#[test]
fn test_find_repo_root_via_ralph_config_jsonc() {
    let dir = TempDir::new().expect("create temp dir");
    create_config_jsonc(&dir, r#"{"version":1}"#);

    let repo_root = config::find_repo_root(dir.path());
    assert_eq!(repo_root, dir.path());
}

#[test]
#[serial]
fn test_resolve_queue_path_prefers_jsonc_over_json() {
    let _guard = env_lock().lock().expect("env lock");

    let dir = TempDir::new().expect("create temp dir");
    let ralph_dir = setup_ralph_dir(&dir);

    // Create both .json and .jsonc files
    fs::write(ralph_dir.join("queue.json"), r#"{"version":1,"tasks":[]}"#).unwrap();
    fs::write(ralph_dir.join("queue.jsonc"), r#"{"version":1,"tasks":[]}"#).unwrap();

    // Use explicit config with the new default path to ensure is_default=true
    let cfg = Config {
        queue: QueueConfig {
            file: Some(PathBuf::from(".ralph/queue.jsonc")),
            ..Default::default()
        },
        ..Config::default()
    };
    let queue_path = config::resolve_queue_path(dir.path(), &cfg).unwrap();

    // Should prefer .jsonc over .json
    assert_eq!(queue_path, ralph_dir.join("queue.jsonc"));
}

#[test]
#[serial]
fn test_resolve_queue_path_falls_back_to_json() {
    let _guard = env_lock().lock().expect("env lock");

    let dir = TempDir::new().expect("create temp dir");
    let ralph_dir = setup_ralph_dir(&dir);

    // Create only .json file
    fs::write(ralph_dir.join("queue.json"), r#"{"version":1,"tasks":[]}"#).unwrap();

    let cfg = Config::default();
    let queue_path = config::resolve_queue_path(dir.path(), &cfg).unwrap();

    // Should fall back to .json when .jsonc doesn't exist
    assert_eq!(queue_path, ralph_dir.join("queue.json"));
}

#[test]
#[serial]
fn test_resolve_done_path_prefers_jsonc_over_json() {
    let _guard = env_lock().lock().expect("env lock");

    let dir = TempDir::new().expect("create temp dir");
    let ralph_dir = setup_ralph_dir(&dir);

    // Create both .json and .jsonc files at the default paths
    // The default is now .jsonc, so we need to create both files to test preference
    fs::write(ralph_dir.join("done.jsonc"), r#"{"version":1,"tasks":[]}"#).unwrap();
    fs::write(ralph_dir.join("done.json"), r#"{"version":1,"tasks":[]}"#).unwrap();

    // Use explicit config with the new default path to ensure is_default=true
    let cfg = Config {
        queue: QueueConfig {
            done_file: Some(PathBuf::from(".ralph/done.jsonc")),
            ..Default::default()
        },
        ..Config::default()
    };
    let done_path = config::resolve_done_path(dir.path(), &cfg).unwrap();

    // Should prefer .jsonc over .json
    assert_eq!(done_path, ralph_dir.join("done.jsonc"));
}

#[test]
#[serial]
fn test_resolve_done_path_falls_back_to_json() {
    let _guard = env_lock().lock().expect("env lock");

    let dir = TempDir::new().expect("create temp dir");
    let ralph_dir = setup_ralph_dir(&dir);

    // Create only .json file
    fs::write(ralph_dir.join("done.json"), r#"{"version":1,"tasks":[]}"#).unwrap();

    let cfg = Config::default();
    let done_path = config::resolve_done_path(dir.path(), &cfg).unwrap();

    // Should fall back to .json when .jsonc doesn't exist
    assert_eq!(done_path, ralph_dir.join("done.json"));
}

#[test]
fn test_project_config_path_prefers_jsonc_over_json() {
    let dir = TempDir::new().expect("create temp dir");
    let ralph_dir = setup_ralph_dir(&dir);

    // Create both .json and .jsonc files
    fs::write(ralph_dir.join("config.json"), r#"{"version":1}"#).unwrap();
    fs::write(ralph_dir.join("config.jsonc"), r#"{"version":1}"#).unwrap();

    let config_path = config::project_config_path(dir.path());

    // Should prefer .jsonc over .json
    assert_eq!(config_path, ralph_dir.join("config.jsonc"));
}

#[test]
fn test_project_config_path_falls_back_to_jsonc() {
    let dir = TempDir::new().expect("create temp dir");
    let ralph_dir = setup_ralph_dir(&dir);

    // Create only .jsonc file
    fs::write(ralph_dir.join("config.jsonc"), r#"{"version":1}"#).unwrap();

    let config_path = config::project_config_path(dir.path());

    // Should fall back to .jsonc when .json doesn't exist
    assert_eq!(config_path, ralph_dir.join("config.jsonc"));
}

#[test]
#[serial]
fn test_global_config_path_falls_back_to_jsonc() {
    let _guard = env_lock().lock().expect("env lock");
    let dir = TempDir::new().expect("create temp dir");
    let xdg_config = dir.path().join(".config");
    let ralph_dir = xdg_config.join("ralph");
    fs::create_dir_all(&ralph_dir).expect("create xdg config dir");

    // Create only config.jsonc (no config.json)
    fs::write(ralph_dir.join("config.jsonc"), r#"{"version":1}"#).unwrap();

    unsafe { env::set_var("XDG_CONFIG_HOME", &xdg_config) };
    let config_path = config::global_config_path();
    unsafe { env::remove_var("XDG_CONFIG_HOME") };

    assert!(config_path.is_some());
    assert_eq!(config_path.unwrap(), ralph_dir.join("config.jsonc"));
}

#[test]
#[serial]
fn test_global_config_path_prefers_jsonc_over_json() {
    let _guard = env_lock().lock().expect("env lock");
    let dir = TempDir::new().expect("create temp dir");
    let xdg_config = dir.path().join(".config");
    let ralph_dir = xdg_config.join("ralph");
    fs::create_dir_all(&ralph_dir).expect("create xdg config dir");

    // Create both .json and .jsonc files
    fs::write(ralph_dir.join("config.json"), r#"{"version":1}"#).unwrap();
    fs::write(ralph_dir.join("config.jsonc"), r#"{"version":1}"#).unwrap();

    unsafe { env::set_var("XDG_CONFIG_HOME", &xdg_config) };
    let config_path = config::global_config_path();
    unsafe { env::remove_var("XDG_CONFIG_HOME") };

    assert!(config_path.is_some());
    // Should prefer .jsonc over .json
    assert_eq!(config_path.unwrap(), ralph_dir.join("config.jsonc"));
}

#[test]
fn test_load_layer_accepts_jsonc_with_comments() {
    let dir = TempDir::new().expect("create temp dir");
    let config_path = dir.path().join("config.jsonc");

    // Write JSONC with comments
    let jsonc_content = r#"{
        // This is a single-line comment
        "version": 1,
        "agent": {
            /* Multi-line
               comment */
            "runner": "claude"
        }
    }"#;
    fs::write(&config_path, jsonc_content).expect("write config.jsonc");

    let layer = config::load_layer(&config_path).unwrap();
    assert_eq!(layer.version, Some(1));
    assert_eq!(layer.agent.runner, Some(Runner::Claude));
}

#[test]
fn test_load_queue_accepts_jsonc_with_comments() {
    let dir = TempDir::new().expect("create temp dir");
    let ralph_dir = setup_ralph_dir(&dir);
    let queue_path = ralph_dir.join("queue.jsonc");

    // Write JSONC with comments
    let jsonc_content = r#"{
        // Queue file with comments
        "version": 1,
        "tasks": [
            /* Task entry */
            {
                "id": "RQ-0001",
                "title": "Test task",
                "status": "todo",
                "tags": [],
                "scope": [],
                "evidence": [],
                "plan": [],
                "created_at": "2026-01-18T00:00:00Z",
                "updated_at": "2026-01-18T00:00:00Z"
            }
        ]
    }"#;
    fs::write(&queue_path, jsonc_content).expect("write queue.jsonc");

    let queue = ralph::queue::load_queue(&queue_path).unwrap();
    assert_eq!(queue.tasks.len(), 1);
    assert_eq!(queue.tasks[0].id, "RQ-0001");
}

#[test]
#[serial]
fn test_resolve_done_path_expands_tilde_to_home() {
    let _guard = env_lock().lock().expect("env lock");
    let original_home = env::var("HOME").ok();

    unsafe { env::set_var("HOME", "/custom/home") };

    let repo_root = PathBuf::from("/repo/root");
    let mut cfg = Config::default();
    cfg.queue.done_file = Some(PathBuf::from("~/mydone.json"));

    let done_path = config::resolve_done_path(&repo_root, &cfg).unwrap();
    assert_eq!(done_path, PathBuf::from("/custom/home/mydone.json"));

    // Restore HOME
    match original_home {
        Some(v) => unsafe { env::set_var("HOME", v) },
        None => unsafe { env::remove_var("HOME") },
    }
}

#[test]
#[serial]
fn test_resolve_queue_path_expands_tilde_alone_to_home() {
    let _guard = env_lock().lock().expect("env lock");
    let original_home = env::var("HOME").ok();

    unsafe { env::set_var("HOME", "/custom/home") };

    let repo_root = PathBuf::from("/repo/root");
    let mut cfg = Config::default();
    cfg.queue.file = Some(PathBuf::from("~"));

    let queue_path = config::resolve_queue_path(&repo_root, &cfg).unwrap();
    assert_eq!(queue_path, PathBuf::from("/custom/home"));

    // Restore HOME
    match original_home {
        Some(v) => unsafe { env::set_var("HOME", v) },
        None => unsafe { env::remove_var("HOME") },
    }
}

#[test]
#[serial]
fn test_resolve_queue_path_does_not_join_when_tilde_expands() {
    let _guard = env_lock().lock().expect("env lock");
    let original_home = env::var("HOME").ok();

    unsafe { env::set_var("HOME", "/custom/home") };

    // When ~ expands to an absolute path, it should NOT be joined to repo_root
    let repo_root = PathBuf::from("/repo/root");
    let mut cfg = Config::default();
    cfg.queue.file = Some(PathBuf::from("~/queue.json"));

    let queue_path = config::resolve_queue_path(&repo_root, &cfg).unwrap();
    // Should be /custom/home/queue.json, NOT /repo/root/custom/home/queue.json
    assert_eq!(queue_path, PathBuf::from("/custom/home/queue.json"));
    assert!(!queue_path.to_string_lossy().contains("/repo/root"));

    // Restore HOME
    match original_home {
        Some(v) => unsafe { env::set_var("HOME", v) },
        None => unsafe { env::remove_var("HOME") },
    }
}

#[test]
#[serial]
fn test_resolve_queue_path_relative_when_home_unset() {
    let _guard = env_lock().lock().expect("env lock");
    let original_home = env::var("HOME").ok();

    // Remove HOME - tilde should not expand, path treated as relative
    unsafe { env::remove_var("HOME") };

    let dir = TempDir::new().expect("create temp dir");
    let repo_root = dir.path();
    let mut cfg = Config::default();
    cfg.queue.file = Some(PathBuf::from("~/queue.json"));

    // When HOME is unset, ~/queue.json is treated as a relative path
    let queue_path = config::resolve_queue_path(repo_root, &cfg).unwrap();
    assert_eq!(queue_path, repo_root.join("~/queue.json"));

    // Restore HOME
    if let Some(v) = original_home {
        unsafe { env::set_var("HOME", v) }
    }
}
