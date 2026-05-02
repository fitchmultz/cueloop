/**
 CueLoopLoggerTests

 Purpose:
 - Keep CueLoopLoggerTests behavior scoped to its owning CueLoopMac feature.

 Responsibilities:
 - Provide focused app, core, or test behavior for its owning feature.

 Scope:
 - Limited to this file's owning CueLoopMac feature boundary.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/Assumptions:
 - Keep behavior aligned with CueLoop's machine-contract and queue semantics.
 */

/*
 Purpose:
 - Exercise CueLoopLogger initialization, category coverage, and basic exported capabilities.

 Responsibilities:
 - Verify the shared logger is available for every declared category.
 - Verify subsystem and category metadata stay stable.
 - Smoke-test the public logging entrypoints and log-export availability contract.

 Scope:
 - CueLoopLogger XCTest coverage only.

 Usage:
 - Runs as part of the CueLoopCore XCTest suite.

 Invariants/Assumptions:
 - Logging entrypoints should remain safe to call during tests without additional fixture setup.
 - Category descriptions and subsystem identifiers are part of the user-visible diagnostics contract.
 */

import XCTest
@testable import CueLoopCore

@MainActor
final class CueLoopLoggerTests: CueLoopCoreTestCase {
    func testLoggerInitialization() {
        let logger = CueLoopLogger.shared
        XCTAssertNotNil(logger)

        // Test all categories have loggers
        for category in CueLoopLogger.Category.allCases {
            let categoryLogger = logger.logger(for: category)
            XCTAssertNotNil(categoryLogger)
        }
    }

    func testSubsystem() {
        XCTAssertEqual(CueLoopLogger.subsystem, "com.mitchfultz.cueloop")
    }

    func testLogLevels() {
        // These should not crash
        CueLoopLogger.shared.debug("Test debug message", category: .fileWatching)
        CueLoopLogger.shared.info("Test info message", category: .cli)
        CueLoopLogger.shared.error("Test error message", category: .workspace)
        CueLoopLogger.shared.fault("Test fault message", category: .lifecycle)
    }

    @available(macOS 12.0, *)
    func testExportLogsAvailability() {
        // Export should be available on macOS 12+
        XCTAssertTrue(CueLoopLogger.shared.canExportLogs)
    }

    func testCategoryDescriptions() {
        XCTAssertEqual(CueLoopLogger.Category.fileWatching.description, "FileWatching")
        XCTAssertEqual(CueLoopLogger.Category.cli.description, "CLI")
        XCTAssertEqual(CueLoopLogger.Category.workspace.description, "Workspace")
        XCTAssertEqual(CueLoopLogger.Category.ui.description, "UI")
        XCTAssertEqual(CueLoopLogger.Category.lifecycle.description, "Lifecycle")
        XCTAssertEqual(CueLoopLogger.Category.crashReporting.description, "CrashReporting")
    }
}
