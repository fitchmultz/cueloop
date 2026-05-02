/**
 WorkspaceManagerVersioningTests

 Purpose:
 - Verify WorkspaceManager version-check flow enforces machine-contract versions for `machine system info`.

 Responsibilities:
 - Verify matching `machine system info` payloads continue through semantic version validation.
 - Verify unsupported `machine system info` versions fail fast with version-mismatch recovery messaging.
 - Keep shared WorkspaceManager singleton state isolated between tests.

 Scope:
 - WorkspaceManager version-check behavior only.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/Assumptions:
 - Tests run on the main actor because `WorkspaceManager` is main-actor isolated.
 - Tests must restore shared singleton state before returning.
 */

import Foundation
import XCTest

@testable import CueLoopCore

@MainActor
final class WorkspaceManagerVersioningTests: CueLoopCoreTestCase {
    private struct CachedVersionResultFixture: Codable {
        let timestamp: Date
        let isCompatible: Bool
        let versionString: String
    }

    private func resetManagerVersioningState(_ manager: WorkspaceManager) {
        manager.versionCheckTask?.cancel()
        manager.versionCheckTask = nil
        manager.versionCheckResult = nil
        manager.errorMessage = nil
        CueLoopAppDefaults.userDefaults.removeObject(forKey: manager.versionCheckCacheKey)
    }

    func testExecuteVersionCheck_acceptsMatchingSystemInfoVersion() async throws {
        let manager = WorkspaceManager.shared
        let originalClient = manager.client
        resetManagerVersioningState(manager)
        defer {
            manager.client = originalClient
            resetManagerVersioningState(manager)
        }

        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-manager-version-check-ok")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let script = """
        #!/bin/sh
        if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
          echo '{"version":1,"cli_version":"\(VersionCompatibility.minimumCLIVersion)"}'
          exit 0
        fi
        exit 64
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)
        manager.client = try CueLoopCLIClient(executableURL: scriptURL)

        let result = await manager.executeVersionCheck()

        XCTAssertEqual(result?.status, .compatible)
        XCTAssertEqual(result?.rawVersion, VersionCompatibility.minimumCLIVersion)
        XCTAssertNil(manager.errorMessage)
    }

    func testExecuteVersionCheck_rejectsUnsupportedSystemInfoVersion() async throws {
        let manager = WorkspaceManager.shared
        let originalClient = manager.client
        resetManagerVersioningState(manager)
        defer {
            manager.client = originalClient
            resetManagerVersioningState(manager)
        }

        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-manager-version-check-mismatch")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let script = """
        #!/bin/sh
        if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
          echo '{"version":999,"cli_version":"\(VersionCompatibility.minimumCLIVersion)"}'
          exit 0
        fi
        exit 64
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)
        manager.client = try CueLoopCLIClient(executableURL: scriptURL)

        let result = await manager.executeVersionCheck()

        XCTAssertNil(result)
        XCTAssertTrue(manager.errorMessage?.contains("Unsupported machine system info version 999") == true)
        XCTAssertTrue(manager.errorMessage?.contains("Rebuild the macOS app and the bundled CueLoop CLI from the same revision.") == true)
        XCTAssertFalse(manager.errorMessage?.contains("Failed to check CLI version:") == true)
    }

    func testPerformVersionCheck_usesCachedCompatibleResultWithoutInvokingCLI() async throws {
        let manager = WorkspaceManager.shared
        let originalClient = manager.client
        resetManagerVersioningState(manager)
        defer {
            manager.client = originalClient
            resetManagerVersioningState(manager)
        }

        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-manager-version-check-cache-hit")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }
        let commandLogURL = tempDir.appendingPathComponent("command-log.txt", isDirectory: false)

        let script = """
        #!/bin/sh
        set -eu
        printf '%s\\n' "$*" >> "\(commandLogURL.path)"
        exit 70
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)
        manager.client = try CueLoopCLIClient(executableURL: scriptURL)

        let cached = CachedVersionResultFixture(
            timestamp: Date(),
            isCompatible: true,
            versionString: VersionCompatibility.minimumCLIVersion
        )
        let encoded = try JSONEncoder().encode(cached)
        CueLoopAppDefaults.userDefaults.set(encoded, forKey: manager.versionCheckCacheKey)

        await manager.performVersionCheck()

        XCTAssertEqual(manager.versionCheckResult?.status, .compatible)
        XCTAssertEqual(manager.versionCheckResult?.rawVersion, VersionCompatibility.minimumCLIVersion)
        XCTAssertNil(manager.errorMessage)
        XCTAssertFalse(FileManager.default.fileExists(atPath: commandLogURL.path))
    }

    func testExecuteVersionCheck_surfacesNonZeroExitCode() async throws {
        let manager = WorkspaceManager.shared
        let originalClient = manager.client
        resetManagerVersioningState(manager)
        defer {
            manager.client = originalClient
            resetManagerVersioningState(manager)
        }

        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-manager-version-check-exit-code")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let script = """
        #!/bin/sh
        if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
          exit 70
        fi
        exit 64
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)
        manager.client = try CueLoopCLIClient(executableURL: scriptURL)

        let result = await manager.executeVersionCheck()

        XCTAssertNil(result)
        XCTAssertEqual(manager.errorMessage, "CLI version check failed with exit code 70")
    }

    func testPerformVersionCheck_reportsIncompatibleSemanticVersionWithGuidance() async throws {
        let manager = WorkspaceManager.shared
        let originalClient = manager.client
        resetManagerVersioningState(manager)
        defer {
            manager.client = originalClient
            resetManagerVersioningState(manager)
        }

        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-manager-version-check-incompatible")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let script = """
        #!/bin/sh
        if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
          echo '{"version":1,"cli_version":"0.3.9"}'
          exit 0
        fi
        exit 64
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)
        manager.client = try CueLoopCLIClient(executableURL: scriptURL)

        await manager.performVersionCheck()

        guard let result = manager.versionCheckResult else {
            return XCTFail("expected version-check result")
        }
        guard case .tooOld(let found, let minimum) = result.status else {
            return XCTFail("expected tooOld status, got \(result.status)")
        }
        XCTAssertEqual(found.description, "0.3.9")
        XCTAssertEqual(minimum.description, VersionCompatibility.minimumCLIVersion)
        XCTAssertTrue(manager.errorMessage?.contains("is too old") == true)
        XCTAssertTrue(manager.errorMessage?.contains("Please reinstall CueLoop") == true)
        XCTAssertNil(CueLoopAppDefaults.userDefaults.data(forKey: manager.versionCheckCacheKey))
    }

    func testCheckCachedVersionResult_discardsCorruptCacheAndRecordsPersistenceIssue() {
        let manager = WorkspaceManager.shared
        resetManagerVersioningState(manager)
        defer { resetManagerVersioningState(manager) }

        CueLoopAppDefaults.userDefaults.set(Data("not-valid-json".utf8), forKey: manager.versionCheckCacheKey)

        let result = manager.checkCachedVersionResult()

        XCTAssertNil(result)
        XCTAssertEqual(manager.persistenceIssue?.domain, .versionCache)
        XCTAssertEqual(manager.persistenceIssue?.operation, .load)
        XCTAssertNil(CueLoopAppDefaults.userDefaults.data(forKey: manager.versionCheckCacheKey))
    }
}
