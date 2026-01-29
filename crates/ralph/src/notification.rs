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
fn play_windows_sound(custom_path: Option<&str>) -> anyhow::Result<()> {
    if let Some(path) = custom_path {
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(anyhow::anyhow!("Sound file not found: {}", path));
        }

        // Try winmm PlaySound first for .wav files
        if path.ends_with(".wav") || path.ends_with(".WAV") {
            if let Ok(()) = play_sound_winmm(path) {
                return Ok(());
            }
        }

        // Fall back to PowerShell MediaPlayer for other formats or if winmm fails
        if let Ok(()) = play_sound_powershell(path) {
            return Ok(());
        }

        return Err(anyhow::anyhow!(
            "Failed to play sound with all available methods"
        ));
    }

    // No custom path - Windows toast notification handles default sound
    Ok(())
}

#[cfg(target_os = "windows")]
fn play_sound_winmm(path: &str) -> anyhow::Result<()> {
    use std::ffi::CString;
    use windows_sys::Win32::Media::Audio::{PlaySoundA, SND_FILENAME, SND_SYNC};

    let c_path = CString::new(path).map_err(|e| anyhow::anyhow!("Invalid path encoding: {}", e))?;

    let result = unsafe {
        PlaySoundA(
            c_path.as_ptr(),
            std::ptr::null_mut(),
            SND_FILENAME | SND_SYNC,
        )
    };

    if result == 0 {
        return Err(anyhow::anyhow!("PlaySoundA failed"));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn play_sound_powershell(path: &str) -> anyhow::Result<()> {
    let script = format!(
        "$player = New-Object System.Media.SoundPlayer '{}'; $player.PlaySync()",
        path.replace('\'', "''")
    );

    let output = std::process::Command::new("powershell.exe")
        .arg("-Command")
        .arg(&script)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute PowerShell: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "PowerShell sound playback failed: {}",
            stderr
        ));
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

    #[cfg(target_os = "windows")]
    mod windows_tests {
        use super::*;
        use std::io::Write;
        use tempfile::NamedTempFile;

        #[test]
        fn play_windows_sound_missing_file() {
            let result = play_windows_sound(Some("/nonexistent/path/sound.wav"));
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not found"));
        }

        #[test]
        fn play_windows_sound_none_path() {
            // Should succeed (no custom sound requested)
            let result = play_windows_sound(None);
            assert!(result.is_ok());
        }

        #[test]
        fn play_windows_sound_wav_file_exists() {
            // Create a minimal valid WAV file header
            let mut temp_file = NamedTempFile::with_suffix(".wav").unwrap();
            // RIFF WAV header (44 bytes minimum)
            let wav_header: Vec<u8> = vec![
                // RIFF chunk
                0x52, 0x49, 0x46, 0x46, // "RIFF"
                0x24, 0x00, 0x00, 0x00, // file size - 8
                0x57, 0x41, 0x56, 0x45, // "WAVE"
                // fmt chunk
                0x66, 0x6D, 0x74, 0x20, // "fmt "
                0x10, 0x00, 0x00, 0x00, // chunk size (16)
                0x01, 0x00, // audio format (PCM)
                0x01, 0x00, // num channels (1)
                0x44, 0xAC, 0x00, 0x00, // sample rate (44100)
                0x88, 0x58, 0x01, 0x00, // byte rate
                0x02, 0x00, // block align
                0x10, 0x00, // bits per sample (16)
                // data chunk
                0x64, 0x61, 0x74, 0x61, // "data"
                0x00, 0x00, 0x00, 0x00, // data size
            ];
            temp_file.write_all(&wav_header).unwrap();
            temp_file.flush().unwrap();

            let path = temp_file.path().to_str().unwrap();
            // Should not error on file existence check
            // Actual playback may fail in CI without audio subsystem
            let _ = play_windows_sound(Some(path));
        }

        #[test]
        fn play_windows_sound_non_wav_uses_powershell() {
            // Create a dummy mp3 file (just a header, not a real mp3)
            let mut temp_file = NamedTempFile::with_suffix(".mp3").unwrap();
            // MP3 sync word (not a full valid header, but enough for path validation)
            let mp3_header: Vec<u8> = vec![0xFF, 0xFB, 0x90, 0x00];
            temp_file.write_all(&mp3_header).unwrap();
            temp_file.flush().unwrap();

            let path = temp_file.path().to_str().unwrap();
            // Should attempt PowerShell fallback for non-WAV files
            // Result depends on whether PowerShell is available
            let _ = play_windows_sound(Some(path));
        }
    }
}
