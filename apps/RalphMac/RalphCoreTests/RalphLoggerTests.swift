/*
 Purpose:
 - Exercise RalphLogger initialization, category coverage, and basic exported capabilities.

 Responsibilities:
 - Verify the shared logger is available for every declared category.
 - Verify subsystem and category metadata stay stable.
 - Smoke-test the public logging entrypoints and log-export availability contract.

 Scope:
 - RalphLogger XCTest coverage only.

 Usage:
 - Runs as part of the RalphCore XCTest suite.

 Invariants/Assumptions:
 - Logging entrypoints should remain safe to call during tests without additional fixture setup.
 - Category descriptions and subsystem identifiers are part of the user-visible diagnostics contract.
 */

import XCTest
@testable import RalphCore

@MainActor
final class RalphLoggerTests: RalphCoreTestCase {
    func testLoggerInitialization() {
        let logger = RalphLogger.shared
        XCTAssertNotNil(logger)

        // Test all categories have loggers
        for category in RalphLogger.Category.allCases {
            let categoryLogger = logger.logger(for: category)
            XCTAssertNotNil(categoryLogger)
        }
    }

    func testSubsystem() {
        XCTAssertEqual(RalphLogger.subsystem, "com.mitchfultz.ralph")
    }

    func testLogLevels() {
        // These should not crash
        RalphLogger.shared.debug("Test debug message", category: .fileWatching)
        RalphLogger.shared.info("Test info message", category: .cli)
        RalphLogger.shared.error("Test error message", category: .workspace)
        RalphLogger.shared.fault("Test fault message", category: .lifecycle)
    }

    @available(macOS 12.0, *)
    func testExportLogsAvailability() {
        // Export should be available on macOS 12+
        XCTAssertTrue(RalphLogger.shared.canExportLogs)
    }

    func testCategoryDescriptions() {
        XCTAssertEqual(RalphLogger.Category.fileWatching.description, "FileWatching")
        XCTAssertEqual(RalphLogger.Category.cli.description, "CLI")
        XCTAssertEqual(RalphLogger.Category.workspace.description, "Workspace")
        XCTAssertEqual(RalphLogger.Category.ui.description, "UI")
        XCTAssertEqual(RalphLogger.Category.lifecycle.description, "Lifecycle")
        XCTAssertEqual(RalphLogger.Category.crashReporting.description, "CrashReporting")
    }
}
