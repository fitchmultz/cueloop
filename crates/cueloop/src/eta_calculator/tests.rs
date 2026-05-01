//! Purpose: Verify ETA calculation, confidence, and formatting behavior.
//!
//! Responsibilities:
//! - Cover history-backed and fallback ETA estimation flows.
//! - Cover confidence thresholds and formatting output.
//! - Cover new-task total estimation behavior across phase-count variants.
//!
//! Scope:
//! - Regression tests only; ETA implementation lives in sibling modules.
//!
//! Usage:
//! - Run via `cargo test -p cueloop-agent-loop eta_calculator` or the broader CI
//!   gates.
//!
//! Invariants/Assumptions:
//! - Test history samples stay deterministic.
//! - Confidence thresholds remain low=1, medium=2-4, high=5+ matching current behavior.

use std::collections::HashMap;
use std::time::Duration;

use crate::execution_history::{ExecutionEntry, ExecutionHistory};
use crate::progress::ExecutionPhase;

use super::{EtaCalculator, EtaConfidence, format_eta};

fn create_test_history() -> ExecutionHistory {
    let mut entries = Vec::new();

    for index in 0..3 {
        entries.push(ExecutionEntry {
            timestamp: format!("2026-01-{:02}T12:00:00Z", 31 - index),
            task_id: format!("RQ-{index:04}"),
            runner: "codex".to_string(),
            model: "sonnet".to_string(),
            phase_count: 3,
            phase_durations: {
                let mut durations = HashMap::new();
                durations.insert(ExecutionPhase::Planning, Duration::from_secs(60));
                durations.insert(ExecutionPhase::Implementation, Duration::from_secs(120));
                durations.insert(ExecutionPhase::Review, Duration::from_secs(30));
                durations
            },
            total_duration: Duration::from_secs(210),
        });
    }

    ExecutionHistory {
        version: 1,
        entries,
    }
}

#[test]
fn eta_calculator_empty() {
    let calculator = EtaCalculator::empty();
    let mut elapsed = HashMap::new();
    elapsed.insert(ExecutionPhase::Planning, Duration::from_secs(30));

    let eta = calculator.calculate_eta("codex", "sonnet", 3, ExecutionPhase::Planning, &elapsed);
    assert!(eta.is_some());
    let estimate = eta.unwrap();
    assert!(!estimate.based_on_history);
    assert_eq!(estimate.confidence, EtaConfidence::Low);
}

#[test]
fn eta_calculator_with_history() {
    let history = create_test_history();
    let calculator = EtaCalculator::new(history);
    let mut elapsed = HashMap::new();
    elapsed.insert(ExecutionPhase::Planning, Duration::from_secs(30));

    let eta = calculator.calculate_eta("codex", "sonnet", 3, ExecutionPhase::Planning, &elapsed);
    assert!(eta.is_some());
    let estimate = eta.unwrap();
    assert!(estimate.based_on_history);
    assert_eq!(estimate.confidence, EtaConfidence::Medium);
}

#[test]
fn eta_calculation_first_phase() {
    let history = create_test_history();
    let calculator = EtaCalculator::new(history);
    let mut elapsed = HashMap::new();
    elapsed.insert(ExecutionPhase::Planning, Duration::from_secs(30));

    let eta = calculator.calculate_eta("codex", "sonnet", 3, ExecutionPhase::Planning, &elapsed);
    assert!(eta.is_some());
    let estimate = eta.unwrap();

    assert!(estimate.remaining >= Duration::from_secs(150));
}

#[test]
fn eta_calculation_second_phase() {
    let history = create_test_history();
    let calculator = EtaCalculator::new(history);
    let mut elapsed = HashMap::new();
    elapsed.insert(ExecutionPhase::Planning, Duration::from_secs(60));
    elapsed.insert(ExecutionPhase::Implementation, Duration::from_secs(60));

    let eta = calculator.calculate_eta(
        "codex",
        "sonnet",
        3,
        ExecutionPhase::Implementation,
        &elapsed,
    );
    assert!(eta.is_some());
    let estimate = eta.unwrap();

    assert!(estimate.remaining >= Duration::from_secs(60));
    assert!(estimate.remaining <= Duration::from_secs(120));
}

#[test]
fn eta_calculation_final_phase() {
    let history = create_test_history();
    let calculator = EtaCalculator::new(history);
    let mut elapsed = HashMap::new();
    elapsed.insert(ExecutionPhase::Planning, Duration::from_secs(60));
    elapsed.insert(ExecutionPhase::Implementation, Duration::from_secs(120));
    elapsed.insert(ExecutionPhase::Review, Duration::from_secs(10));

    let eta = calculator.calculate_eta("codex", "sonnet", 3, ExecutionPhase::Review, &elapsed);
    assert!(eta.is_some());
    let estimate = eta.unwrap();

    assert!(estimate.remaining <= Duration::from_secs(30));
}

#[test]
fn eta_without_history() {
    let calculator = EtaCalculator::empty();
    let mut elapsed = HashMap::new();
    elapsed.insert(ExecutionPhase::Planning, Duration::from_secs(60));

    let eta = calculator.calculate_eta("codex", "sonnet", 3, ExecutionPhase::Planning, &elapsed);
    assert!(eta.is_some());
    let estimate = eta.unwrap();

    assert!(!estimate.based_on_history);
    assert!(estimate.remaining > Duration::ZERO);
}

#[test]
fn confidence_levels() {
    assert_eq!(EtaConfidence::High.indicator(), "+++");
    assert_eq!(EtaConfidence::Medium.indicator(), "++");
    assert_eq!(EtaConfidence::Low.indicator(), "+");

    assert_eq!(EtaConfidence::High.color_name(), "green");
    assert_eq!(EtaConfidence::Medium.color_name(), "yellow");
    assert_eq!(EtaConfidence::Low.color_name(), "gray");
}

#[test]
fn eta_formatting() {
    assert_eq!(format_eta(Duration::from_secs(30)), "30s");
    assert_eq!(format_eta(Duration::from_secs(90)), "1m 30s");
    assert_eq!(format_eta(Duration::from_secs(60)), "1m");
    assert_eq!(format_eta(Duration::from_secs(3665)), "1h 1m");
    assert_eq!(format_eta(Duration::from_secs(7200)), "2h");
}

#[test]
fn single_phase_eta() {
    let history = create_test_history();
    let calculator = EtaCalculator::new(history);
    let mut elapsed = HashMap::new();
    elapsed.insert(ExecutionPhase::Planning, Duration::from_secs(30));

    let eta = calculator.calculate_eta("codex", "sonnet", 1, ExecutionPhase::Planning, &elapsed);
    assert!(eta.is_some());
    let estimate = eta.unwrap();

    assert!(estimate.remaining <= Duration::from_secs(60));
}

#[test]
fn two_phase_eta() {
    let history = create_test_history();
    let calculator = EtaCalculator::new(history);
    let mut elapsed = HashMap::new();
    elapsed.insert(ExecutionPhase::Planning, Duration::from_secs(60));
    elapsed.insert(ExecutionPhase::Implementation, Duration::from_secs(60));

    let eta = calculator.calculate_eta(
        "codex",
        "sonnet",
        2,
        ExecutionPhase::Implementation,
        &elapsed,
    );
    assert!(eta.is_some());
    let estimate = eta.unwrap();

    assert!(estimate.remaining >= Duration::from_secs(30));
    assert!(estimate.remaining <= Duration::from_secs(120));
}

#[test]
fn estimate_new_task_total_no_history() {
    let calculator = EtaCalculator::empty();
    let result = calculator.estimate_new_task_total("codex", "sonnet", 3);
    assert!(
        result.is_none(),
        "Should return None when no history exists"
    );
}

#[test]
fn estimate_new_task_total_with_history() {
    let history = create_test_history();
    let calculator = EtaCalculator::new(history);
    let result = calculator.estimate_new_task_total("codex", "sonnet", 3);
    assert!(result.is_some(), "Should return Some when history exists");

    let estimate = result.unwrap();
    assert!(estimate.based_on_history);
    assert_eq!(estimate.remaining, Duration::from_secs(210));
}

#[test]
fn estimate_new_task_total_confidence_levels() {
    let mut history = ExecutionHistory::default();
    history.entries.push(ExecutionEntry {
        timestamp: "2026-01-31T12:00:00Z".to_string(),
        task_id: "RQ-0001".to_string(),
        runner: "codex".to_string(),
        model: "sonnet".to_string(),
        phase_count: 3,
        phase_durations: {
            let mut durations = HashMap::new();
            durations.insert(ExecutionPhase::Planning, Duration::from_secs(60));
            durations
        },
        total_duration: Duration::from_secs(60),
    });

    let calculator = EtaCalculator::new(history);
    let result = calculator.estimate_new_task_total("codex", "sonnet", 3);
    assert_eq!(result.unwrap().confidence, EtaConfidence::Low);

    let history = create_test_history();
    let calculator = EtaCalculator::new(history);
    let result = calculator.estimate_new_task_total("codex", "sonnet", 3);
    assert_eq!(result.unwrap().confidence, EtaConfidence::Medium);

    let mut history = ExecutionHistory::default();
    for index in 0..5 {
        history.entries.push(ExecutionEntry {
            timestamp: format!("2026-01-{:02}T12:00:00Z", 31 - index),
            task_id: format!("RQ-{index:04}"),
            runner: "codex".to_string(),
            model: "sonnet".to_string(),
            phase_count: 3,
            phase_durations: {
                let mut durations = HashMap::new();
                durations.insert(ExecutionPhase::Planning, Duration::from_secs(60));
                durations.insert(ExecutionPhase::Implementation, Duration::from_secs(120));
                durations.insert(ExecutionPhase::Review, Duration::from_secs(30));
                durations
            },
            total_duration: Duration::from_secs(210),
        });
    }

    let calculator = EtaCalculator::new(history);
    let result = calculator.estimate_new_task_total("codex", "sonnet", 3);
    assert_eq!(result.unwrap().confidence, EtaConfidence::High);
}

#[test]
fn estimate_new_task_total_wrong_key() {
    let history = create_test_history();
    let calculator = EtaCalculator::new(history);

    let result = calculator.estimate_new_task_total("claude", "sonnet", 3);
    assert!(result.is_none());

    let result = calculator.estimate_new_task_total("codex", "gpt-4", 3);
    assert!(result.is_none());

    let result = calculator.estimate_new_task_total("codex", "sonnet", 1);
    assert!(result.is_none());
}

#[test]
fn estimate_new_task_total_phase_variations() {
    let mut history = ExecutionHistory::default();
    history.entries.push(ExecutionEntry {
        timestamp: "2026-01-31T12:00:00Z".to_string(),
        task_id: "RQ-0001".to_string(),
        runner: "codex".to_string(),
        model: "sonnet".to_string(),
        phase_count: 1,
        phase_durations: {
            let mut durations = HashMap::new();
            durations.insert(ExecutionPhase::Planning, Duration::from_secs(60));
            durations
        },
        total_duration: Duration::from_secs(60),
    });
    history.entries.push(ExecutionEntry {
        timestamp: "2026-01-31T12:00:00Z".to_string(),
        task_id: "RQ-0002".to_string(),
        runner: "codex".to_string(),
        model: "sonnet".to_string(),
        phase_count: 2,
        phase_durations: {
            let mut durations = HashMap::new();
            durations.insert(ExecutionPhase::Planning, Duration::from_secs(60));
            durations.insert(ExecutionPhase::Implementation, Duration::from_secs(120));
            durations
        },
        total_duration: Duration::from_secs(180),
    });

    let calculator = EtaCalculator::new(history);

    let result = calculator.estimate_new_task_total("codex", "sonnet", 1);
    assert_eq!(result.unwrap().remaining, Duration::from_secs(60));

    let result = calculator.estimate_new_task_total("codex", "sonnet", 2);
    assert_eq!(result.unwrap().remaining, Duration::from_secs(180));

    let result = calculator.estimate_new_task_total("codex", "sonnet", 3);
    assert!(result.is_none());
}
