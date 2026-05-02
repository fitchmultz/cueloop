/**
 WorkspaceManagerCLIOverrideTests

 Purpose:
 - Validate workspace-manager CLI override adoption rejects insecure or invalid URL-driven overrides.

 Responsibilities:
 - Validate workspace-manager CLI override adoption rejects insecure or invalid URL-driven overrides.

 Does not handle:
 - Runner-configuration loading or retargeting behavior.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Tests assert current hard-cutover policy: URL-driven CLI overrides are not adopted.
 */

import XCTest

@testable import CueLoopCore

@MainActor
final class WorkspaceManagerCLIOverrideTests: CueLoopCoreTestCase {
    func test_workspaceManager_adoptCLIExecutable_rejectsValidPathOverride() async throws {
        let manager = WorkspaceManager.shared
        let baselinePath = manager.client?.executableURL.standardizedFileURL.resolvingSymlinksInPath().path
        let tempDir = try WorkspacePerformanceTestSupport.makeTempDir(prefix: "cueloop-workspace-manager-cli")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }
        let overrideURL = try WorkspacePerformanceTestSupport.makeVersionAwareMockCLI(in: tempDir, name: "mock-cueloop-version-ok")

        manager.adoptCLIExecutable(path: overrideURL.path)

        if let baselinePath {
            XCTAssertEqual(
                manager.client?.executableURL.standardizedFileURL.resolvingSymlinksInPath().path,
                baselinePath
            )
        } else {
            XCTAssertNil(manager.client)
        }
    }

    func test_workspaceManager_adoptCLIExecutable_preservesClientOnInvalidPath() {
        let manager = WorkspaceManager.shared
        let baselinePath = manager.client?.executableURL.standardizedFileURL.resolvingSymlinksInPath().path

        manager.adoptCLIExecutable(path: "/definitely/not/a/real/cueloop-binary")

        if let baselinePath {
            XCTAssertEqual(
                manager.client?.executableURL.standardizedFileURL.resolvingSymlinksInPath().path,
                baselinePath
            )
        } else {
            XCTAssertNotNil(manager.errorMessage)
        }
    }
}
