//! Small logging helpers for the outer supervisor (`run_cmd`).
//!
//! Goal: consistent, human-readable lifecycle logs for supervisor scopes:
//! - "<Scope>: start"
//! - "<Scope>: end"
//! - "<Scope>: error: <message>"

use anyhow::Result;

/// Run `f` while logging a consistent start/end/error envelope.
///
/// NOTE: Keep messages short/human-readable. Full error context is still surfaced
/// by the CLI error printer; this log line is about boundary visibility.
pub(crate) fn with_scope<T>(label: &str, f: impl FnOnce() -> Result<T>) -> Result<T> {
    log::info!("{label}: start");
    match f() {
        Ok(value) => {
            log::info!("{label}: end");
            Ok(value)
        }
        Err(err) => {
            log::error!("{label}: error: {}", err);
            Err(err)
        }
    }
}

pub(crate) fn phase_label(phase: u8, total: u8, name: &str, task_id: &str) -> String {
    format!("Phase {phase}/{total} ({name}) for {}", task_id.trim())
}

pub(crate) fn single_phase_label(name: &str, task_id: &str) -> String {
    format!("{name} for {}", task_id.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use log::{LevelFilter, Log, Metadata, Record};
    use std::sync::{Mutex, Once, OnceLock};

    struct TestLogger;

    static LOGGER: TestLogger = TestLogger;
    static INIT: Once = Once::new();
    static LOGS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

    impl Log for TestLogger {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &Record<'_>) {
            let logs = LOGS.get_or_init(|| Mutex::new(Vec::new()));
            let mut guard = logs.lock().expect("log mutex");
            guard.push(record.args().to_string());
        }

        fn flush(&self) {}
    }

    fn init_logger() -> &'static Mutex<Vec<String>> {
        INIT.call_once(|| {
            let _ = log::set_logger(&LOGGER);
            log::set_max_level(LevelFilter::Info);
        });
        LOGS.get_or_init(|| Mutex::new(Vec::new()))
    }

    fn take_logs() -> Vec<String> {
        let logs = init_logger();
        let mut guard = logs.lock().expect("log mutex");
        let drained = guard.drain(..).collect::<Vec<_>>();
        drained
    }

    #[test]
    fn with_scope_logs_start_and_end_on_success() -> Result<()> {
        let _ = take_logs();

        with_scope("ScopeA", || Ok(()))?;

        let logs = take_logs();
        assert_eq!(logs, vec!["ScopeA: start", "ScopeA: end"]);
        Ok(())
    }

    #[test]
    fn with_scope_logs_error_on_failure() {
        let _ = take_logs();

        let err = with_scope::<()>("ScopeB", || Err(anyhow!("boom"))).unwrap_err();
        assert_eq!(err.to_string(), "boom");

        let logs = take_logs();
        assert_eq!(logs, vec!["ScopeB: start", "ScopeB: error: boom"]);
    }

    #[test]
    fn labels_trim_task_ids() {
        assert_eq!(
            phase_label(2, 3, "Implementation", " RQ-1 "),
            "Phase 2/3 (Implementation) for RQ-1"
        );
        assert_eq!(
            single_phase_label("SinglePhase (Execution)", " RQ-2 "),
            "SinglePhase (Execution) for RQ-2"
        );
    }
}
