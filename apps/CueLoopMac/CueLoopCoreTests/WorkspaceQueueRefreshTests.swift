/**
 WorkspaceQueueRefreshTests

 Purpose:
 - Define the shared queue-refresh test suite anchor for behavior-focused companion files.

 Responsibilities:
 - Provide the XCTest class that companion extension files attach queue refresh scenarios to.
 - Keep queue refresh test discovery centralized without accumulating large behavior blocks in one file.

 Does not handle:
 - Individual queue refresh, watcher, custom path, or overview fallback scenarios.
 - Low-level file-watcher retry behavior or direct task-creation path coverage.

 Usage:
 - Extended by WorkspaceQueueRefresh*Tests.swift companion files in the CueLoopCore test target.

 Invariants/assumptions callers must respect:
 - Companion files own executable test methods so each source file stays below enforced size limits.
 */

import Foundation
import XCTest

@testable import CueLoopCore

@MainActor
final class WorkspaceQueueRefreshTests: CueLoopCoreTestCase {}
