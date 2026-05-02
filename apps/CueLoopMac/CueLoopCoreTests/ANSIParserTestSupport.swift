/**
 ANSIParserTestSupport

 Purpose:
 - Provide a fresh Workspace test fixture for split ANSI parser suites.

 Responsibilities:
 - Provide a fresh Workspace test fixture for split ANSI parser suites.

 Does not handle:
 - Defining parser assertions for specific escape-sequence behaviors.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Tests are main-actor isolated because Workspace is main-actor isolated.
 - Each test receives a fresh Workspace with empty attributed output.
 */

import Foundation
import XCTest
@testable import CueLoopCore

@MainActor
class ANSIParserTestCase: CueLoopCoreTestCase {
    var workspace: Workspace!

    override func setUp() async throws {
        try await super.setUp()
        workspace = Workspace(workingDirectoryURL: CueLoopCoreTestSupport.workspaceURL(label: "ansi-parser"))
    }

    override func tearDown() async throws {
        workspace = nil
        try await super.tearDown()
    }
}
