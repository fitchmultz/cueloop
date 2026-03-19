//! Purpose: Calculate task ETA estimates from execution history and current
//! progress.
//!
//! Responsibilities:
//! - Load historical execution data for ETA use.
//! - Estimate remaining time for in-progress tasks.
//! - Estimate total time for not-started tasks using history-only inputs.
//!
//! Scope:
//! - ETA heuristics only; persistence lives in `crate::execution_history` and
//!   rendering lives in sibling formatting helpers.
//!
//! Usage:
//! - Construct through `EtaCalculator::new`, `empty`, or `load`, then call
//!   `calculate_eta` or `estimate_new_task_total`.
//!
//! Invariants/Assumptions:
//! - Matching history keys are `(runner, model, phase_count)`.
//! - Phase ordering follows `ExecutionPhase::phase_number()` semantics.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::execution_history::{ExecutionHistory, get_phase_averages, load_execution_history};
use crate::progress::ExecutionPhase;

use super::types::{EtaConfidence, EtaEstimate};

const ONE_PHASE: &[ExecutionPhase] = &[ExecutionPhase::Planning];
const TWO_PHASES: &[ExecutionPhase] = &[ExecutionPhase::Planning, ExecutionPhase::Implementation];
const THREE_PHASES: &[ExecutionPhase] = &[
    ExecutionPhase::Planning,
    ExecutionPhase::Implementation,
    ExecutionPhase::Review,
];

/// Calculator for ETA estimates based on historical execution data.
#[derive(Debug, Clone)]
pub struct EtaCalculator {
    history: ExecutionHistory,
}

impl EtaCalculator {
    /// Create a new ETA calculator with the given history.
    pub fn new(history: ExecutionHistory) -> Self {
        Self { history }
    }

    /// Create an empty calculator with no historical data.
    pub fn empty() -> Self {
        Self {
            history: ExecutionHistory::default(),
        }
    }

    /// Load the ETA calculator from cache directory.
    pub fn load(cache_dir: &Path) -> Self {
        match load_execution_history(cache_dir) {
            Ok(history) => Self::new(history),
            Err(_) => Self::empty(),
        }
    }

    /// Calculate ETA based on current progress and historical data.
    pub fn calculate_eta(
        &self,
        runner: &str,
        model: &str,
        phase_count: u8,
        current_phase: ExecutionPhase,
        phase_elapsed: &HashMap<ExecutionPhase, Duration>,
    ) -> Option<EtaEstimate> {
        if phase_count == 0 {
            return None;
        }

        let averages = get_phase_averages(&self.history, runner, model, phase_count);
        let entry_count = self.entry_count_for_key(runner, model, phase_count);
        let confidence = confidence_for_entry_count(entry_count);
        let based_on_history = !averages.is_empty();
        let remaining = if based_on_history {
            self.calculate_with_history(phase_count, current_phase, phase_elapsed, &averages)
        } else {
            self.calculate_without_history(phase_count, current_phase, phase_elapsed)
        };

        Some(EtaEstimate {
            remaining,
            confidence,
            based_on_history,
        })
    }

    /// Count history entries matching the given key.
    pub fn count_entries_for_key(&self, runner: &str, model: &str, phase_count: u8) -> usize {
        self.entry_count_for_key(runner, model, phase_count)
    }

    /// Estimate total time for a not-started task using execution history only.
    /// Returns None when there are zero relevant history samples.
    pub fn estimate_new_task_total(
        &self,
        runner: &str,
        model: &str,
        phase_count: u8,
    ) -> Option<EtaEstimate> {
        if phase_count == 0 {
            return None;
        }

        let averages = get_phase_averages(&self.history, runner, model, phase_count);
        let entry_count = self.entry_count_for_key(runner, model, phase_count);

        if entry_count == 0 {
            return None;
        }

        let fallback = self.calculate_fallback_average(&averages);
        let remaining = phases_for_count(phase_count)
            .iter()
            .map(|phase| averages.get(phase).copied().unwrap_or(fallback))
            .fold(Duration::ZERO, |acc, duration| acc + duration);

        Some(EtaEstimate {
            remaining,
            confidence: confidence_for_entry_count(entry_count),
            based_on_history: true,
        })
    }

    fn calculate_with_history(
        &self,
        phase_count: u8,
        current_phase: ExecutionPhase,
        phase_elapsed: &HashMap<ExecutionPhase, Duration>,
        averages: &HashMap<ExecutionPhase, Duration>,
    ) -> Duration {
        let mut total_remaining = Duration::ZERO;

        for &phase in phases_for_count(phase_count) {
            let elapsed = phase_elapsed.get(&phase).copied().unwrap_or(Duration::ZERO);

            if phase == current_phase {
                if let Some(&avg) = averages.get(&phase) {
                    if elapsed < avg {
                        total_remaining += avg - elapsed;
                    } else {
                        total_remaining += Duration::from_secs(elapsed.as_secs() / 10);
                    }
                } else {
                    total_remaining += elapsed;
                }
            } else if !self.is_phase_completed(phase, current_phase) {
                if let Some(&avg) = averages.get(&phase) {
                    total_remaining += avg;
                } else {
                    total_remaining += self.calculate_fallback_average(averages);
                }
            }
        }

        total_remaining
    }

    fn calculate_without_history(
        &self,
        phase_count: u8,
        current_phase: ExecutionPhase,
        phase_elapsed: &HashMap<ExecutionPhase, Duration>,
    ) -> Duration {
        let phases = phases_for_count(phase_count);
        let mut total_remaining = Duration::ZERO;
        let current_elapsed = phase_elapsed
            .get(&current_phase)
            .copied()
            .unwrap_or(Duration::ZERO);
        let remaining_phases = phases
            .iter()
            .filter(|&&phase| !self.is_phase_completed(phase, current_phase))
            .count();

        if remaining_phases == 0 {
            return total_remaining;
        }

        total_remaining += current_elapsed;

        let completed_count = phase_count as usize - remaining_phases;
        if completed_count > 0 {
            let completed_total: Duration = phases
                .iter()
                .filter(|&&phase| self.is_phase_completed(phase, current_phase))
                .filter_map(|phase| phase_elapsed.get(phase).copied())
                .fold(Duration::ZERO, |acc, duration| acc + duration);
            let avg_completed = completed_total / completed_count as u32;
            total_remaining += avg_completed * (remaining_phases.saturating_sub(1) as u32);
        } else {
            total_remaining += current_elapsed * (remaining_phases.saturating_sub(1) as u32);
        }

        total_remaining
    }

    fn is_phase_completed(&self, phase: ExecutionPhase, current_phase: ExecutionPhase) -> bool {
        phase.phase_number() < current_phase.phase_number()
    }

    fn calculate_fallback_average(&self, averages: &HashMap<ExecutionPhase, Duration>) -> Duration {
        if averages.is_empty() {
            return Duration::from_secs(60);
        }

        let total = averages
            .values()
            .copied()
            .fold(Duration::ZERO, |acc, duration| acc + duration);
        total / averages.len() as u32
    }

    fn entry_count_for_key(&self, runner: &str, model: &str, phase_count: u8) -> usize {
        self.history
            .entries
            .iter()
            .filter(|entry| {
                entry.runner == runner && entry.model == model && entry.phase_count == phase_count
            })
            .count()
    }
}

fn confidence_for_entry_count(entry_count: usize) -> EtaConfidence {
    if entry_count >= 5 {
        EtaConfidence::High
    } else if entry_count >= 2 {
        EtaConfidence::Medium
    } else {
        EtaConfidence::Low
    }
}

fn phases_for_count(phase_count: u8) -> &'static [ExecutionPhase] {
    match phase_count {
        1 => ONE_PHASE,
        2 => TWO_PHASES,
        _ => THREE_PHASES,
    }
}
