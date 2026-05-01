//! Webhook dispatcher, retry, and redaction tests.
//!
//! Purpose:
//! - Webhook dispatcher, retry, and redaction tests.
//!
//! Responsibilities:
//! - Verify dispatcher rebuilds, destination redaction, and failure-record redaction.
//! - Assert retry scheduling and worker-pool concurrency behavior with the test transport.
//!
//! Non-scope:
//! - Payload/config serialization behavior.
//! - Diagnostics replay filtering.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants:
//! - Serial tests reset dispatcher state before installing test transports.
//! - Transport closures run entirely in-process without real network I/O.

use super::super::*;
use super::support::{reset_webhook_test_state, webhook_test_config};
use crate::contracts::WebhookConfig;
use crate::webhook::types::{WebhookContext, WebhookMessage, WebhookPayload};
use crate::webhook::worker::{
    current_dispatcher_settings_for_tests, install_test_transport_for_tests,
    redact_webhook_destination,
};
use crossbeam_channel::bounded;
use serial_test::serial;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[test]
#[serial]
fn parallel_init_rebuilds_dispatcher_with_deterministic_capacity() {
    reset_webhook_test_state();

    let mut config = webhook_test_config();
    config.queue_capacity = Some(50);
    config.parallel_queue_multiplier = Some(2.0);

    let standard = current_dispatcher_settings_for_tests(&config);
    assert_eq!(standard, Some((50, 4)));

    init_worker_for_parallel(&config, 5);
    let parallel = current_dispatcher_settings_for_tests(&config);
    assert_eq!(parallel, Some((500, 5)));
}

#[test]
fn webhook_destination_redaction_hides_sensitive_url_components() {
    let redacted = redact_webhook_destination(
        "https://user:secret@example.com/hooks/tenant/token-123?sig=abc#frag",
    );

    assert_eq!(redacted, "https://example.com/…");
    assert!(!redacted.contains("user"));
    assert!(!redacted.contains("secret"));
    assert!(!redacted.contains("token-123"));
    assert!(!redacted.contains("sig=abc"));
}

#[test]
#[serial]
fn failed_delivery_records_store_redacted_destination() {
    reset_webhook_test_state();
    let repo_root = tempfile::tempdir().expect("tempdir");

    let msg = WebhookMessage {
        payload: WebhookPayload {
            event: "task_failed".to_string(),
            timestamp: "2026-02-13T00:00:00Z".to_string(),
            task_id: Some("RQ-0814".to_string()),
            task_title: Some("Secret-safe failure".to_string()),
            previous_status: None,
            current_status: None,
            note: None,
            context: WebhookContext::default(),
        },
        config: ResolvedWebhookConfig {
            enabled: true,
            url: Some("https://user:secret@example.com/hooks/private-token?sig=abc123".to_string()),
            secret: None,
            timeout: Duration::from_secs(1),
            retry_count: 0,
            retry_backoff: Duration::from_millis(10),
            allow_insecure_http: false,
            allow_private_targets: false,
        },
    };

    crate::webhook::diagnostics::persist_failed_delivery_for_tests(
        repo_root.path(),
        &msg,
        &anyhow::anyhow!(
            "delivery to https://user:secret@example.com/hooks/private-token?sig=abc123 failed"
        ),
        1,
    )
    .expect("persist failed delivery");

    let records = crate::webhook::diagnostics::load_failure_records_for_tests(repo_root.path())
        .expect("load failure records");
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].destination.as_deref(),
        Some("https://example.com/…")
    );
    assert!(!records[0].error.contains("secret"));
    assert!(!records[0].error.contains("private-token"));
    assert!(!records[0].error.contains("sig=abc123"));
}

#[test]
#[serial]
fn failed_delivery_records_store_sanitized_payload_text() {
    reset_webhook_test_state();
    let repo_root = tempfile::tempdir().expect("tempdir");
    let sensitive_repo_root = repo_root.path().join("customer-token=path-secret");

    let msg = WebhookMessage {
        payload: WebhookPayload {
            event: "task_failed".to_string(),
            timestamp: "2026-02-13T00:00:00Z".to_string(),
            task_id: Some("RQ-0814".to_string()),
            task_title: Some("Customer Jane Doe token=title-secret".to_string()),
            previous_status: Some("doing token=prev-secret".to_string()),
            current_status: Some("rejected token=current-secret".to_string()),
            note: Some(
                "operator note includes customer data token=supersecret bearer abcdef".to_string(),
            ),
            context: WebhookContext {
                repo_root: Some(sensitive_repo_root.display().to_string()),
                branch: Some("feature/token=branch-secret".to_string()),
                ..WebhookContext::default()
            },
        },
        config: ResolvedWebhookConfig {
            enabled: true,
            url: Some("https://hooks.example.com/delivery".to_string()),
            secret: None,
            timeout: Duration::from_secs(1),
            retry_count: 0,
            retry_backoff: Duration::from_millis(10),
            allow_insecure_http: false,
            allow_private_targets: false,
        },
    };

    crate::webhook::diagnostics::persist_failed_delivery_for_tests(
        repo_root.path(),
        &msg,
        &anyhow::anyhow!("simulated failure"),
        1,
    )
    .expect("persist failed delivery");

    let rendered = std::fs::read_to_string(crate::webhook::diagnostics::failure_store_path(
        repo_root.path(),
    ))
    .expect("read failure store");
    assert!(!rendered.contains("operator note"), "{rendered}");
    assert!(!rendered.contains("customer data"), "{rendered}");
    assert!(!rendered.contains("token=supersecret"), "{rendered}");
    assert!(!rendered.contains("Customer Jane Doe"), "{rendered}");
    assert!(!rendered.contains("title-secret"), "{rendered}");
    assert!(!rendered.contains("prev-secret"), "{rendered}");
    assert!(!rendered.contains("current-secret"), "{rendered}");
    assert!(!rendered.contains("path-secret"), "{rendered}");
    assert!(!rendered.contains("branch-secret"), "{rendered}");

    let records = crate::webhook::diagnostics::load_failure_records_for_tests(repo_root.path())
        .expect("load failure records");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].payload.task_id.as_deref(), Some("RQ-0814"));
    assert_eq!(records[0].payload.event, "task_failed");
    assert_eq!(records[0].payload.task_title.as_deref(), Some("[REDACTED]"));
    assert_eq!(
        records[0].payload.previous_status.as_deref(),
        Some("[REDACTED]")
    );
    assert_eq!(
        records[0].payload.current_status.as_deref(),
        Some("[REDACTED]")
    );
    assert_eq!(records[0].payload.note.as_deref(), Some("[REDACTED]"));
    assert_eq!(
        records[0].payload.context.branch.as_deref(),
        Some("[REDACTED]")
    );
    assert_eq!(
        records[0].payload.context.repo_root.as_deref(),
        Some("[REDACTED]")
    );

    let (replay_tx, replay_rx) = bounded::<String>(1);
    install_test_transport_for_tests(Some(Arc::new(move |request| {
        replay_tx
            .send(request.body.clone())
            .expect("record replay request body");
        Ok(())
    })));

    let replay_report = crate::webhook::diagnostics::replay_failed_deliveries(
        repo_root.path(),
        &webhook_test_config(),
        &crate::webhook::diagnostics::ReplaySelector {
            ids: Vec::new(),
            event: Some("task_failed".to_string()),
            task_id: Some("RQ-0814".to_string()),
            limit: 10,
            max_replay_attempts: 3,
        },
        false,
    )
    .expect("replay sanitized failure record");
    assert_eq!(replay_report.matched_count, 1);
    assert_eq!(replay_report.replayed_count, 1);

    let replay_body = replay_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("replayed webhook request");
    assert!(replay_body.contains("RQ-0814"), "{replay_body}");
    assert!(replay_body.contains("[REDACTED]"), "{replay_body}");
    assert!(!replay_body.contains("operator note"), "{replay_body}");
    assert!(!replay_body.contains("Customer Jane Doe"), "{replay_body}");
}

#[test]
#[serial]
fn retry_backoff_is_scheduled_off_the_hot_worker_path() {
    reset_webhook_test_state();

    let attempts = Arc::new(AtomicUsize::new(0));
    let (events_tx, events_rx) = bounded::<String>(8);
    let attempts_for_transport = Arc::clone(&attempts);
    let events_for_transport = events_tx.clone();

    install_test_transport_for_tests(Some(Arc::new(move |request| {
        let _ = request.body.len();
        let _ = request.signature.clone();
        let _ = request.timeout;

        if request.url.contains("slow.test") {
            let attempt = attempts_for_transport.fetch_add(1, Ordering::SeqCst) + 1;
            let _ = events_for_transport.send(format!("slow-attempt-{attempt}"));
            anyhow::bail!("simulated failure");
        }

        let _ = events_for_transport.send("fast-success".to_string());
        Ok(())
    })));

    let slow_config = WebhookConfig {
        url: Some("https://slow.test/hook".to_string()),
        retry_count: Some(2),
        retry_backoff_ms: Some(150),
        ..webhook_test_config()
    };
    let fast_config = WebhookConfig {
        url: Some("https://fast.test/hook".to_string()),
        retry_count: Some(0),
        ..webhook_test_config()
    };

    send_webhook_payload(
        WebhookPayload {
            event: "task_failed".to_string(),
            timestamp: "2026-03-07T00:00:00Z".to_string(),
            task_id: Some("RQ-SLOW".to_string()),
            task_title: Some("Slow endpoint".to_string()),
            previous_status: None,
            current_status: None,
            note: None,
            context: WebhookContext::default(),
        },
        &slow_config,
    );
    send_webhook_payload(
        WebhookPayload {
            event: "task_completed".to_string(),
            timestamp: "2026-03-07T00:00:00Z".to_string(),
            task_id: Some("RQ-FAST".to_string()),
            task_title: Some("Fast endpoint".to_string()),
            previous_status: None,
            current_status: None,
            note: None,
            context: WebhookContext::default(),
        },
        &fast_config,
    );

    let first = events_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("first event");
    let second = events_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("second event");
    let third = events_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("third event");

    let first_two = [first.as_str(), second.as_str()];
    assert!(first_two.contains(&"slow-attempt-1"));
    assert!(first_two.contains(&"fast-success"));
    assert_eq!(third, "slow-attempt-2");
}

#[test]
#[serial]
fn worker_pool_prevents_one_blocked_destination_from_serializing_all_deliveries() {
    reset_webhook_test_state();

    let (blocked_entered_tx, blocked_entered_rx) = bounded::<()>(1);
    let (release_tx, release_rx) = bounded::<()>(1);
    let (events_tx, events_rx) = bounded::<String>(8);

    install_test_transport_for_tests(Some(Arc::new(move |request| {
        if request.url.contains("blocked.test") {
            blocked_entered_tx
                .send(())
                .expect("blocked request entered");
            release_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("release blocked request");
            anyhow::bail!("blocked request released");
        }

        events_tx
            .send("fast-success".to_string())
            .expect("record fast success");
        Ok(())
    })));

    let blocked_config = WebhookConfig {
        url: Some("https://blocked.test/hook".to_string()),
        retry_count: Some(0),
        ..webhook_test_config()
    };
    let fast_config = WebhookConfig {
        url: Some("https://fast.test/hook".to_string()),
        retry_count: Some(0),
        ..webhook_test_config()
    };

    send_webhook_payload(
        WebhookPayload {
            event: "task_failed".to_string(),
            timestamp: "2026-03-07T00:00:00Z".to_string(),
            task_id: Some("RQ-BLOCKED".to_string()),
            task_title: Some("Blocked endpoint".to_string()),
            previous_status: None,
            current_status: None,
            note: None,
            context: WebhookContext::default(),
        },
        &blocked_config,
    );

    blocked_entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("blocked request should start");

    send_webhook_payload(
        WebhookPayload {
            event: "task_completed".to_string(),
            timestamp: "2026-03-07T00:00:00Z".to_string(),
            task_id: Some("RQ-FAST".to_string()),
            task_title: Some("Independent delivery".to_string()),
            previous_status: None,
            current_status: None,
            note: None,
            context: WebhookContext::default(),
        },
        &fast_config,
    );

    let fast_event = events_rx
        .recv_timeout(Duration::from_millis(250))
        .expect("fast delivery should not wait for blocked destination");
    assert_eq!(fast_event, "fast-success");

    release_tx.send(()).expect("release blocked request");
}

#[test]
#[serial]
fn invalid_numeric_webhook_settings_are_rejected_before_enqueue() {
    reset_webhook_test_state();

    let deliveries = Arc::new(AtomicUsize::new(0));
    let deliveries_for_transport = Arc::clone(&deliveries);
    install_test_transport_for_tests(Some(Arc::new(move |_| {
        deliveries_for_transport.fetch_add(1, Ordering::SeqCst);
        Ok(())
    })));

    let invalid_config = WebhookConfig {
        retry_backoff_ms: Some(99),
        ..webhook_test_config()
    };

    let enqueued = crate::webhook::worker::send_webhook_payload_internal(
        WebhookPayload {
            event: "task_completed".to_string(),
            timestamp: "2026-03-07T00:00:00Z".to_string(),
            task_id: Some("RQ-INVALID".to_string()),
            task_title: Some("Invalid config".to_string()),
            previous_status: None,
            current_status: None,
            note: None,
            context: WebhookContext::default(),
        },
        &invalid_config,
        false,
    );

    assert!(!enqueued);
    assert_eq!(deliveries.load(Ordering::SeqCst), 0);
}
