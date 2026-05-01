//! Purpose: Define ETA result data models and confidence presentation helpers.
//!
//! Responsibilities:
//! - Define `EtaEstimate`.
//! - Define `EtaConfidence` and its display-oriented helper methods.
//!
//! Scope:
//! - Data modeling only; ETA calculations and duration formatting live in
//!   sibling modules.
//!
//! Usage:
//! - Used by queue, report, and calculator code through `crate::eta_calculator`.
//!
//! Invariants/Assumptions:
//! - `EtaEstimate.remaining` is always a non-negative `Duration`.
//! - Confidence indicator and color mappings remain stable for callers.

use std::time::Duration;

/// ETA estimate with confidence level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtaEstimate {
    /// Estimated time remaining.
    pub remaining: Duration,
    /// Confidence level based on historical data availability.
    pub confidence: EtaConfidence,
    /// Whether the estimate is based on historical data.
    pub based_on_history: bool,
}

/// Confidence level for ETA estimates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtaConfidence {
    /// High confidence (>5 historical entries).
    High,
    /// Medium confidence (2-5 entries).
    Medium,
    /// Low confidence (<2 entries or fallback).
    Low,
}

impl EtaConfidence {
    /// Returns a visual indicator for the confidence level.
    pub fn indicator(&self) -> &'static str {
        match self {
            EtaConfidence::High => "+++",
            EtaConfidence::Medium => "++",
            EtaConfidence::Low => "+",
        }
    }

    /// Returns a color name for the confidence level (for UI styling).
    pub fn color_name(&self) -> &'static str {
        match self {
            EtaConfidence::High => "green",
            EtaConfidence::Medium => "yellow",
            EtaConfidence::Low => "gray",
        }
    }
}
