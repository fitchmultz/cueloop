//! Purpose: Provide redaction-aware string wrappers, logger plumbing, and
//! redacted logging macros.
//!
//! Responsibilities:
//! - Wrap strings so display/debug output is redacted automatically.
//! - Wrap a `log::Log` implementation and redact emitted messages.
//! - Export convenience macros for explicit redacted logging.
//!
//! Scope:
//! - Logging and display behavior only; redaction pattern matching lives in
//!   `patterns.rs`.
//!
//! Usage:
//! - Used by runner/error surfaces and logger bootstrap code.
//!
//! Invariants/Assumptions:
//! - User-visible log/display output always flows through `redact_text`.
//! - Raw debug-log capture still happens before terminal redaction.

use std::fmt;

use super::patterns::redact_text;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct RedactedString(pub String);

impl From<String> for RedactedString {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for RedactedString {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl fmt::Display for RedactedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", redact_text(&self.0))
    }
}

impl fmt::Debug for RedactedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "RedactedString({:#?})", redact_text(&self.0))
        } else {
            write!(f, "RedactedString({:?})", redact_text(&self.0))
        }
    }
}

pub struct RedactedLogger {
    inner: Box<dyn log::Log>,
}

impl RedactedLogger {
    pub fn new(inner: Box<dyn log::Log>) -> Self {
        Self { inner }
    }

    pub fn init(
        inner: Box<dyn log::Log>,
        max_level: log::LevelFilter,
    ) -> Result<(), log::SetLoggerError> {
        log::set_boxed_logger(Box::new(Self::new(inner)))?;
        log::set_max_level(max_level);
        Ok(())
    }
}

impl log::Log for RedactedLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            crate::debuglog::write_log_record(record);
            let redacted_msg = redact_text(&format!("{}", record.args()));
            self.inner.log(
                &log::Record::builder()
                    .args(format_args!("{}", redacted_msg))
                    .level(record.level())
                    .target(record.target())
                    .file(record.file())
                    .line(record.line())
                    .module_path(record.module_path())
                    .build(),
            );
        }
    }

    fn flush(&self) {
        self.inner.flush();
    }
}

#[macro_export]
macro_rules! rinfo {
    ($($arg:tt)+) => {
        log::info!("{}", $crate::redaction::redact_text(&format!($($arg)+)))
    }
}

#[macro_export]
macro_rules! rwarn {
    ($($arg:tt)+) => {
        log::warn!("{}", $crate::redaction::redact_text(&format!($($arg)+)))
    }
}

#[macro_export]
macro_rules! rerror {
    ($($arg:tt)+) => {
        log::error!("{}", $crate::redaction::redact_text(&format!($($arg)+)))
    }
}

#[macro_export]
macro_rules! rdebug {
    ($($arg:tt)+) => {
        log::debug!("{}", $crate::redaction::redact_text(&format!($($arg)+)))
    }
}

#[macro_export]
macro_rules! rtrace {
    ($($arg:tt)+) => {
        log::trace!("{}", $crate::redaction::redact_text(&format!($($arg)+)))
    }
}
