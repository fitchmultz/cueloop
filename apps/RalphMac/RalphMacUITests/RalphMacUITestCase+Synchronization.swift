/**
 Purpose:
 - Provide deterministic synchronization helpers for Ralph macOS UI tests.

 Responsibilities:
 - Poll arbitrary UI conditions without hard-coded sleeps in scenario tests.
 - Standardize existence/disappearance assertions for XCUI elements.

 Scope:
 - Waiting and assertion helpers only.

 Usage:
 - Call `assertExists`, `assertDoesNotExist`, or `assertEventually` from UI suites and harness extensions.

 Invariants/Assumptions:
 - Helpers run on the main actor and drain the run loop between checks.
 */

import XCTest

@MainActor
extension RalphMacUITestCase {
    @discardableResult
    func waitUntil(
        timeout: TimeInterval = 5,
        interval: TimeInterval = 0.05,
        condition: () -> Bool
    ) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        var currentInterval = max(interval, 0.01)
        while Date() < deadline {
            if condition() {
                return true
            }
            let nextDeadline = min(deadline, Date().addingTimeInterval(currentInterval))
            RunLoop.current.run(mode: .default, before: nextDeadline)
            currentInterval = min(currentInterval * 1.5, 0.25)
        }
        return condition()
    }

    func assertEventually(
        _ message: @autoclosure () -> String,
        timeout: TimeInterval = 5,
        interval: TimeInterval = 0.05,
        file: StaticString = #filePath,
        line: UInt = #line,
        condition: () -> Bool
    ) {
        XCTAssertTrue(
            waitUntil(timeout: timeout, interval: interval, condition: condition),
            message(),
            file: file,
            line: line
        )
    }

    func assertExists(
        _ element: XCUIElement,
        timeout: TimeInterval = 5,
        message: String,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        assertEventually(
            message,
            timeout: timeout,
            file: file,
            line: line
        ) {
            element.exists
        }
    }

    func assertDoesNotExist(
        _ element: XCUIElement,
        timeout: TimeInterval = 5,
        message: String,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        assertEventually(
            message,
            timeout: timeout,
            file: file,
            line: line
        ) {
            !element.exists
        }
    }
}
