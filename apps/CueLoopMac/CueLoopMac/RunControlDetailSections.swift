//!
//! RunControlDetailSections
//!
//! Purpose:
//! - Keep the Run Control detail column split into focused section files so `RunControlSection.swift`
//!   remains orchestration-only and each detail card lives beside related helpers.
//!
//! Responsibilities:
//! - Provide the shared import/docs facade for the decomposed detail-column sections.
//!
//! Scope:
//! - Detail-column section implementations live in `RunControlDetailSections+...` companion files.
//!
//!
//! Usage:
//! - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.
//! Invariants/Assumptions:
//! - Callers keep usage within the documented responsibilities and owning feature contracts.

import CueLoopCore
import SwiftUI
