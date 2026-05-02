/**
 WorkspaceOfflineCachingTests

 Purpose:
 - Validate workspace offline-banner and cached-task presentation behavior.

 Responsibilities:
 - Validate workspace offline-banner and cached-task presentation behavior.

 Does not handle:
 - CLI health probe execution or recovery category formatting.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Tests mutate in-memory workspace state only.
 */

import Foundation
import XCTest
@testable import CueLoopCore

@MainActor
final class WorkspaceOfflineCachingTests: CueLoopCoreTestCase {
    func testShowOfflineBannerWhenUnavailable() {
        let workspaceURL = CueLoopCoreTestSupport.workspaceURL(label: "offline-banner-unavailable")
        let workspace = Workspace(workingDirectoryURL: workspaceURL)
        XCTAssertFalse(workspace.showOfflineBanner)

        workspace.cliHealthStatus = CLIHealthStatus(
            availability: .unavailable(reason: .cliNotFound),
            lastChecked: Date(),
            workspaceURL: workspaceURL
        )

        XCTAssertTrue(workspace.showOfflineBanner)
    }

    func testShowOfflineBannerWhenAvailable() {
        let workspaceURL = CueLoopCoreTestSupport.workspaceURL(label: "offline-banner-available")
        let workspace = Workspace(workingDirectoryURL: workspaceURL)
        workspace.cliHealthStatus = CLIHealthStatus(
            availability: .available,
            lastChecked: Date(),
            workspaceURL: workspaceURL
        )
        XCTAssertFalse(workspace.showOfflineBanner)
    }

    func testIsShowingCachedTasks() {
        let workspaceURL = CueLoopCoreTestSupport.workspaceURL(label: "offline-cached-tasks")
        let workspace = Workspace(workingDirectoryURL: workspaceURL)
        XCTAssertFalse(workspace.isShowingCachedTasks)

        workspace.cliHealthStatus = CLIHealthStatus(
            availability: .unavailable(reason: .cliNotFound),
            lastChecked: Date(),
            workspaceURL: workspaceURL
        )
        workspace.cachedTasks = [
            CueLoopTask(id: "RQ-TEST", status: .todo, title: "Test", priority: .medium)
        ]

        XCTAssertTrue(workspace.isShowingCachedTasks)
    }

    func testDisplayTasksWhenOffline() {
        let workspaceURL = CueLoopCoreTestSupport.workspaceURL(label: "offline-display")
        let workspace = Workspace(workingDirectoryURL: workspaceURL)
        let onlineTask = CueLoopTask(id: "RQ-ONLINE", status: .todo, title: "Online", priority: .medium)
        let cachedTask = CueLoopTask(id: "RQ-CACHED", status: .done, title: "Cached", priority: .low)

        workspace.tasks = [onlineTask]
        workspace.cachedTasks = [cachedTask]
        workspace.cliHealthStatus = CLIHealthStatus(
            availability: .unavailable(reason: .cliNotFound),
            lastChecked: Date(),
            workspaceURL: workspaceURL
        )

        let displayTasks = workspace.displayTasks()
        XCTAssertEqual(displayTasks.count, 1)
        XCTAssertEqual(displayTasks.first?.id, "RQ-CACHED")
    }

    func testDisplayTasksWhenOnline() {
        let workspaceURL = CueLoopCoreTestSupport.workspaceURL(label: "online-display")
        let workspace = Workspace(workingDirectoryURL: workspaceURL)
        let onlineTask = CueLoopTask(id: "RQ-ONLINE", status: .todo, title: "Online", priority: .medium)

        workspace.tasks = [onlineTask]
        workspace.cachedTasks = []
        workspace.cliHealthStatus = CLIHealthStatus(
            availability: .available,
            lastChecked: Date(),
            workspaceURL: workspaceURL
        )

        let displayTasks = workspace.displayTasks()
        XCTAssertEqual(displayTasks.count, 1)
        XCTAssertEqual(displayTasks.first?.id, "RQ-ONLINE")
    }

    func testClearCachedTasks() {
        let workspace = Workspace(workingDirectoryURL: CueLoopCoreTestSupport.workspaceURL(label: "clear-cached"))
        workspace.cachedTasks = [
            CueLoopTask(id: "RQ-TEST", status: .todo, title: "Test", priority: .medium)
        ]

        workspace.clearCachedTasks()
        XCTAssertTrue(workspace.cachedTasks.isEmpty)
    }

    func testCachedTaskRoundTripPreservesExpandedContractFields() throws {
        let workspaceURL = CueLoopCoreTestSupport.workspaceURL(label: "offline-cache-round-trip")
        let workspace = Workspace(workingDirectoryURL: workspaceURL)
        let scheduledStart = ISO8601DateFormatter().date(from: "2026-04-23T12:00:00Z")

        workspace.tasks = [
            CueLoopTask(
                id: "RQ-TEST",
                status: .todo,
                title: "Test",
                priority: .medium,
                scheduledStart: scheduledStart,
                duplicates: "RQ-0009",
                parentID: "RQ-0001"
            )
        ]

        workspace.refreshCachedTasks()
        workspace.cachedTasks = []
        workspace.loadCachedTasks()

        XCTAssertEqual(workspace.cachedTasks.count, 1)
        XCTAssertEqual(workspace.cachedTasks.first?.scheduledStart, scheduledStart)
        XCTAssertEqual(workspace.cachedTasks.first?.duplicates, "RQ-0009")
        XCTAssertEqual(workspace.cachedTasks.first?.parentID, "RQ-0001")
    }
}
