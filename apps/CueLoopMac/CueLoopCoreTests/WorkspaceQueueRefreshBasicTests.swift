/**
 WorkspaceQueueRefreshBasicTests

 Purpose:
 - Keep foundational queue refresh tests in a focused companion file.

 Responsibilities:
 - Validate initial queue hydration and watcher-triggered refresh behavior.
 - Preserve existing WorkspaceQueueRefreshTests coverage while reducing per-file size.

 Does not handle:
 - Retargeting and workspace-overview fallback scenarios.
 - Queue-path resolution edge-case coverage.

 Usage:
 - Executed by the CueLoopCore test target alongside WorkspaceQueueRefreshTests.

 Invariants/assumptions callers must respect:
 - Tests initialize isolated temp workspaces and wait for deterministic state convergence.
 */

import Foundation
import XCTest

@testable import CueLoopCore

@MainActor
extension WorkspaceQueueRefreshTests {
    func test_workspaceInitialRefresh_populatesQueueWithoutEagerGraphOrAnalytics() async throws {
        var seedingWorkspace: Workspace!
        var workspace: Workspace!
        let workspaceURL = try WorkspaceTaskCreationTestSupport.makeTempDir(prefix: "cueloop-workspace-refresh-initial-")
        defer { CueLoopCoreTestSupport.shutdownAndRemove(workspaceURL, seedingWorkspace, workspace) }

        let client = try CueLoopCLIClient(executableURL: try WorkspaceTaskCreationTestSupport.resolveCueLoopBinaryURL())
        try await WorkspaceTaskCreationTestSupport.runChecked(
            client: client,
            arguments: ["--no-color", "init", "--non-interactive"],
            currentDirectoryURL: workspaceURL
        )

        seedingWorkspace = Workspace(workingDirectoryURL: workspaceURL, client: client)
        try await seedingWorkspace.createTask(
            title: "Seed queue state",
            priority: .medium
        )
        try await seedingWorkspace.createTask(
            title: "Render analytics state",
            priority: .medium
        )

        workspace = Workspace(workingDirectoryURL: workspaceURL, client: client)

        let loaded = await CueLoopCoreTestSupport.waitUntil(timeout: .seconds(10)) {
            await MainActor.run {
                workspace.taskState.tasks.count == 2
                    && workspace.insightsState.graphData == nil
                    && workspace.insightsState.analytics.lastRefreshedAt == nil
            }
        }

        XCTAssertTrue(loaded)
        XCTAssertEqual(workspace.taskState.tasks.count, 2)
        XCTAssertNil(workspace.insightsState.graphData)
        XCTAssertNil(workspace.insightsState.analytics.lastRefreshedAt)
    }

    func test_workspaceWatcherExternalMutation_refreshesQueueGraphAndAnalytics() async throws {
        var workspace: Workspace!
        var writerWorkspace: Workspace!
        let workspaceURL = try WorkspaceTaskCreationTestSupport.makeTempDir(prefix: "cueloop-workspace-refresh-watch-")
        defer { CueLoopCoreTestSupport.shutdownAndRemove(workspaceURL, workspace, writerWorkspace) }

        let client = try CueLoopCLIClient(executableURL: try WorkspaceTaskCreationTestSupport.resolveCueLoopBinaryURL())
        try await WorkspaceTaskCreationTestSupport.runChecked(
            client: client,
            arguments: ["--no-color", "init", "--non-interactive"],
            currentDirectoryURL: workspaceURL
        )

        workspace = Workspace(workingDirectoryURL: workspaceURL, client: client)
        writerWorkspace = Workspace(workingDirectoryURL: workspaceURL, client: client)

        let loadedEmptyState = await CueLoopCoreTestSupport.waitUntil(timeout: .seconds(10)) {
            await MainActor.run {
                workspace.taskState.tasks.isEmpty
                    && workspace.insightsState.graphData == nil
                    && workspace.insightsState.analytics.lastRefreshedAt == nil
                    && workspace.diagnosticsState.watcherHealth.isWatching
            }
        }

        XCTAssertTrue(loadedEmptyState)

        try await writerWorkspace.createTask(
            title: "Observe watcher refresh",
            priority: .medium
        )
        try await writerWorkspace.createTask(
            title: "Update analytics after mutation",
            priority: .medium
        )

        let refreshed = await CueLoopCoreTestSupport.waitUntil(timeout: .seconds(10)) {
            await MainActor.run {
                workspace.taskState.tasks.count == 2
                    && workspace.insightsState.graphData?.summary.totalTasks == 2
                    && workspace.insightsState.analytics.queueStatsValue?.summary.active == 2
                    && workspace.taskState.lastQueueRefreshEvent?.source == .externalFileChange
            }
        }

        XCTAssertTrue(refreshed)
        XCTAssertEqual(workspace.taskState.lastQueueRefreshEvent?.highlightedTaskIDs.count, 2)
    }
}
