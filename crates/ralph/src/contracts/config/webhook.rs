//! Webhook configuration for HTTP task event notifications.
//!
//! Responsibilities:
//! - Define webhook config structs and backpressure policy enum.
//! - Provide merge behavior and event filtering.
//! - Define valid webhook event subscription types for config validation.
//!
//! Not handled here:
//! - Actual webhook delivery (see `crate::webhook` module).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Webhook event subscription type for config.
/// Each variant corresponds to a WebhookEventType, plus Wildcard for "all events".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventSubscription {
    /// Task was created/added to queue.
    TaskCreated,
    /// Task status changed to Doing (execution started).
    TaskStarted,
    /// Task completed successfully (status Done).
    TaskCompleted,
    /// Task failed or was rejected.
    TaskFailed,
    /// Generic status change.
    TaskStatusChanged,
    /// Run loop started.
    LoopStarted,
    /// Run loop stopped.
    LoopStopped,
    /// Phase started for a task.
    PhaseStarted,
    /// Phase completed for a task.
    PhaseCompleted,
    /// Queue became unblocked.
    QueueUnblocked,
    /// Wildcard: subscribe to all events.
    #[serde(rename = "*")]
    Wildcard,
}

impl WebhookEventSubscription {
    /// Convert to the string representation used in event matching.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TaskCreated => "task_created",
            Self::TaskStarted => "task_started",
            Self::TaskCompleted => "task_completed",
            Self::TaskFailed => "task_failed",
            Self::TaskStatusChanged => "task_status_changed",
            Self::LoopStarted => "loop_started",
            Self::LoopStopped => "loop_stopped",
            Self::PhaseStarted => "phase_started",
            Self::PhaseCompleted => "phase_completed",
            Self::QueueUnblocked => "queue_unblocked",
            Self::Wildcard => "*",
        }
    }
}

/// Backpressure policy for webhook delivery queue.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebhookQueuePolicy {
    /// Drop new webhooks when queue is full, preserving existing queue contents.
    /// This is functionally equivalent to `drop_new` due to channel constraints.
    #[default]
    DropOldest,
    /// Drop the new webhook if queue is full.
    DropNew,
    /// Block sender briefly, then drop if queue is still full.
    BlockWithTimeout,
}

/// Webhook configuration for HTTP task event notifications.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct WebhookConfig {
    /// Enable webhook notifications (default: false).
    pub enabled: Option<bool>,

    /// Webhook endpoint URL (required when enabled).
    pub url: Option<String>,

    /// Secret key for HMAC-SHA256 signature generation.
    /// When set, webhooks include an X-Ralph-Signature header.
    pub secret: Option<String>,

    /// Events to subscribe to (default: legacy task events only).
    pub events: Option<Vec<WebhookEventSubscription>>,

    /// Request timeout in seconds (default: 30, max: 300).
    #[schemars(range(min = 1, max = 300))]
    pub timeout_secs: Option<u32>,

    /// Number of retry attempts for failed deliveries (default: 3, max: 10).
    #[schemars(range(min = 0, max = 10))]
    pub retry_count: Option<u32>,

    /// Retry backoff base in milliseconds (default: 1000, max: 30000).
    #[schemars(range(min = 100, max = 30000))]
    pub retry_backoff_ms: Option<u32>,

    /// Maximum number of pending webhooks in the delivery queue (default: 100, range: 10-10000).
    #[schemars(range(min = 10, max = 10000))]
    pub queue_capacity: Option<u32>,

    /// Backpressure policy when queue is full (default: drop_oldest).
    /// - drop_oldest: Drop new webhooks when full (preserves existing queue contents)
    /// - drop_new: Drop the new webhook if queue is full
    /// - block_with_timeout: Block sender briefly (100ms), then drop if still full
    pub queue_policy: Option<WebhookQueuePolicy>,
}

impl WebhookConfig {
    pub fn merge_from(&mut self, other: Self) {
        if other.enabled.is_some() {
            self.enabled = other.enabled;
        }
        if other.url.is_some() {
            self.url = other.url;
        }
        if other.secret.is_some() {
            self.secret = other.secret;
        }
        if other.events.is_some() {
            self.events = other.events;
        }
        if other.timeout_secs.is_some() {
            self.timeout_secs = other.timeout_secs;
        }
        if other.retry_count.is_some() {
            self.retry_count = other.retry_count;
        }
        if other.retry_backoff_ms.is_some() {
            self.retry_backoff_ms = other.retry_backoff_ms;
        }
        if other.queue_capacity.is_some() {
            self.queue_capacity = other.queue_capacity;
        }
        if other.queue_policy.is_some() {
            self.queue_policy = other.queue_policy;
        }
    }

    /// Legacy default events that are enabled when `events` is not specified.
    /// New events (loop_*, phase_*) are opt-in and require explicit configuration.
    const DEFAULT_EVENTS_V1: [&'static str; 5] = [
        "task_created",
        "task_started",
        "task_completed",
        "task_failed",
        "task_status_changed",
    ];

    /// Check if a specific event type is enabled.
    ///
    /// Event filtering behavior:
    /// - If webhooks are disabled, no events are sent.
    /// - If `events` is `None`: only legacy task events are enabled (backward compatible).
    /// - If `events` is `Some([...])`: only those events are enabled; use `["*"]` to enable all.
    pub fn is_event_enabled(&self, event: &str) -> bool {
        if !self.enabled.unwrap_or(false) {
            return false;
        }
        match &self.events {
            None => Self::DEFAULT_EVENTS_V1.contains(&event),
            Some(events) => events
                .iter()
                .any(|e| e.as_str() == event || e.as_str() == "*"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_subscription_serialization() {
        // Test snake_case serialization
        let sub = WebhookEventSubscription::TaskCreated;
        assert_eq!(serde_json::to_string(&sub).unwrap(), "\"task_created\"");

        // Test wildcard serialization
        let wild = WebhookEventSubscription::Wildcard;
        assert_eq!(serde_json::to_string(&wild).unwrap(), "\"*\"");
    }

    #[test]
    fn test_event_subscription_deserialization() {
        let sub: WebhookEventSubscription = serde_json::from_str("\"task_created\"").unwrap();
        assert_eq!(sub, WebhookEventSubscription::TaskCreated);

        let wild: WebhookEventSubscription = serde_json::from_str("\"*\"").unwrap();
        assert_eq!(wild, WebhookEventSubscription::Wildcard);
    }

    #[test]
    fn test_invalid_event_rejected() {
        let result: Result<WebhookEventSubscription, _> = serde_json::from_str("\"task_creatd\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_event_enabled_with_subscription_type() {
        let config = WebhookConfig {
            enabled: Some(true),
            events: Some(vec![
                WebhookEventSubscription::TaskCreated,
                WebhookEventSubscription::Wildcard,
            ]),
            ..Default::default()
        };
        assert!(config.is_event_enabled("task_created"));
        assert!(config.is_event_enabled("loop_started")); // via wildcard
    }

    #[test]
    fn test_is_event_enabled_default_events_when_none() {
        let config = WebhookConfig {
            enabled: Some(true),
            events: None,
            ..Default::default()
        };
        assert!(config.is_event_enabled("task_created"));
        assert!(config.is_event_enabled("task_started"));
        assert!(!config.is_event_enabled("loop_started")); // not in default set
    }

    #[test]
    fn test_is_event_enabled_disabled_when_not_enabled() {
        let config = WebhookConfig {
            enabled: Some(false),
            events: Some(vec![WebhookEventSubscription::TaskCreated]),
            ..Default::default()
        };
        assert!(!config.is_event_enabled("task_created"));
    }
}
