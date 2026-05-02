/**
 CueLoopCoreTestCase

 Purpose:
 - Reset unit-test persistence state around every CueLoopCore XCTest case.

 Responsibilities:
 - Reset unit-test persistence state around every CueLoopCore XCTest case.
 - Provide one shared base class for deterministic test isolation.

 Does not handle:
 - Temp-directory or async wait helpers.
 - Production workspace behavior.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Tests inheriting from this base class must keep all persistent state scoped to test helpers.
 */

import XCTest

@testable import CueLoopCore

class CueLoopCoreTestCase: XCTestCase {
    override func setUpWithError() throws {
        try super.setUpWithError()
        CueLoopCoreTestSupport.resetPersistentTestState()
    }

    override func tearDownWithError() throws {
        CueLoopCoreTestSupport.resetPersistentTestState()
        try super.tearDownWithError()
    }
}
