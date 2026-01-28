//! Desktop notification system for task completion.
//!
//! Responsibilities:
//! - Send cross-platform desktop notifications via notify-rust.
//! - Play optional sound alerts using platform-specific mechanisms.
//! - Provide graceful degradation when notification systems are unavailable.
//!
//! Does NOT handle:
//! - Notification scheduling or queuing (callers trigger explicitly).
//! - Persistent notification history or logging.
//! - TUI mode detection (callers should suppress if desired).
//!
//! Invariants:
//! - Sound playback failures don't fail the notification.
//! - Notification failures are logged but don't fail the calling operation.
//! - All platform-specific code is isolated per target OS.

use std::path::Path;

/// Configuration for desktop notifications.
#[derive(Debug, Clone, Default)]
pub struct NotificationConfig {
    /// Enable desktop notifications on task completion.
    pub enabled: bool,
    /// Enable sound alerts with notifications.
    pub sound_enabled: bool,
    /// Custom sound file path (platform-specific format).
    /// If not set, uses platform default sounds.
    pub sound_path: Option<String>,
    /// Notification timeout in milliseconds (default: 8000).
    pub timeout_ms: u32,
}

impl NotificationConfig {
    /// Create a new config with sensible defaults.
    pub fn new() -> Self {
        Self {
            enabled: true,
            sound_enabled: false,
            sound_path: None,
            timeout_ms: 8000,
        }
    }
}

/// Send task completion notification.
/// Silently logs errors but never fails the calling operation.
pub fn notify_task_complete(task_id: &str, task_title: &str, config: &NotificationConfig) {
    if !config.enabled {
        log::debug!("Notification disabled; skipping completion notification");
        return;
    }

    // Build and show notification
    if let Err(e) = show_notification(task_id, task_title, config.timeout_ms) {
        log::debug!("Failed to show notification: {}", e);
    }

    // Play sound if enabled
    if config.sound_enabled {
        if let Err(e) = play_completion_sound(config.sound_path.as_deref()) {
            log::debug!("Failed to play sound: {}", e);
        }
    }
}

#[cfg(feature = "notifications")]
fn show_notification(task_id: &str, task_title: &str, timeout_ms: u32) -> anyhow::Result<()> {
    use notify_rust::{Notification, Timeout};

    Notification::new()
        .summary("Ralph: Task Complete")
        .body(&format!("{} - {}", task_id, task_title))
        .timeout(Timeout::Milliseconds(timeout_ms))
        .show()
        .map_err(|e| anyhow::anyhow!("Failed to show notification: {}", e))?;

    Ok(())
}

#[cfg(not(feature = "notifications"))]
fn show_notification(_task_id: &str, _task_title: &str, _timeout_ms: u32) -> anyhow::Result<()> {
    log::debug!("Notifications feature not compiled in; skipping notification display");
    Ok(())
}

/// Play completion sound using platform-specific mechanisms.
fn play_completion_sound(custom_path: Option<&str>) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        play_macos_sound(custom_path)
    }

    #[cfg(target_os = "linux")]
    {
        play_linux_sound(custom_path)
    }

    #[cfg(target_os = "windows")]
    {
        play_windows_sound(custom_path)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        log::debug!("Sound playback not supported on this platform");
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn play_macos_sound(custom_path: Option<&str>) -> anyhow::Result<()> {
    let sound_path = if let Some(path) = custom_path {
        path.to_string()
    } else {
        "/System/Library/Sounds/Glass.aiff".to_string()
    };

    if !Path::new(&sound_path).exists() {
        return Err(anyhow::anyhow!("Sound file not found: {}", sound_path));
    }

    let output = std::process::Command::new("afplay")
        .arg(&sound_path)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute afplay: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("afplay failed: {}", stderr));
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn play_linux_sound(custom_path: Option<&str>) -> anyhow::Result<()> {
    if let Some(path) = custom_path {
        // Try paplay first (PulseAudio), fall back to aplay (ALSA)
        if Path::new(path).exists() {
            let result = std::process::Command::new("paplay").arg(path).output();
            if let Ok(output) = result {
                if output.status.success() {
                    return Ok(());
                }
            }

            // Fall back to aplay
            let output = std::process::Command::new("aplay")
                .arg(path)
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to execute aplay: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("aplay failed: {}", stderr));
            }
            return Ok(());
        } else {
            return Err(anyhow::anyhow!("Sound file not found: {}", path));
        }
    }

    // No custom path - try to play default notification sound via canberra-gtk-play
    let result = std::process::Command::new("canberra-gtk-play")
        .arg("--id=message")
        .output();

    if let Ok(output) = result {
        if output.status.success() {
            return Ok(());
        }
    }

    // If canberra-gtk-play fails or isn't available, that's okay - just log it
    log::debug!(
        "Could not play default notification sound (canberra-gtk-play not available or failed)"
    );
    Ok(())
}

#[cfg(target_os = "windows")]
fn play_windows_sound(_custom_path: Option<&str>) -> anyhow::Result<()> {
    // Windows notification sound is typically handled by the toast notification itself
    // For custom sounds, we'd need windows-specific APIs which are complex to add
    // The notify-rust library handles Windows toast sounds internally
    if _custom_path.is_some() {
        log::debug!("Custom sounds not yet supported on Windows via this API");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_config_default_values() {
        let config = NotificationConfig::new();
        assert!(config.enabled);
        assert!(!config.sound_enabled);
        assert!(config.sound_path.is_none());
        assert_eq!(config.timeout_ms, 8000);
    }

    #[test]
    fn notify_task_complete_disabled_does_nothing() {
        let config = NotificationConfig {
            enabled: false,
            sound_enabled: true,
            ..Default::default()
        };
        // Should not panic or fail
        notify_task_complete("RQ-0001", "Test task", &config);
    }

    #[test]
    fn notification_config_can_be_customized() {
        let config = NotificationConfig {
            enabled: true,
            sound_enabled: true,
            sound_path: Some("/path/to/sound.wav".to_string()),
            timeout_ms: 5000,
        };
        assert!(config.enabled);
        assert!(config.sound_enabled);
        assert_eq!(config.sound_path, Some("/path/to/sound.wav".to_string()));
        assert_eq!(config.timeout_ms, 5000);
    }
}
