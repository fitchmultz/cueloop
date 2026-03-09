/**
 ErrorRecoveryTests

 Responsibilities:
 - Provide shared imports and low-level test helpers for recovery-oriented RalphCore tests.
 - Centralize temporary-directory and process-exit helpers used by split recovery suites.

 Does not handle:
 - Defining the actual recovery, CLI health, timeout, or workspace caching assertions.

 Invariants/assumptions callers must respect:
 - These helpers are test-only and may assume temporary filesystem access.
 - Callers are responsible for cleaning up created directories.
 */

import XCTest
@testable import RalphCore

enum ErrorRecoveryTestSupport {
    static func makeTempDir(prefix: String) throws -> URL {
        try RalphCoreTestSupport.makeTemporaryDirectory(prefix: prefix)
    }

    static func waitForProcessExit(_ pid: pid_t, timeout: TimeInterval) async -> Bool {
        await RalphCoreTestSupport.waitForProcessExit(pid, timeout: .seconds(timeout))
    }
}
