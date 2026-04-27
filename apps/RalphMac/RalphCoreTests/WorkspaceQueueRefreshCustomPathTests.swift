/**
 WorkspaceQueueRefreshCustomPathTests

 Purpose:
 - Validate custom queue path handling when workspace overview fallback paths are required.

 Responsibilities:
 - Cover custom queue and done path resolution when machine workspace overview is unsupported.
 - Validate missing configured queue preflight guidance avoids unsafe queue-read fallback.

 Does not handle:
 - Watcher-driven refresh and retargeting scenarios.
 - Workspace-overview fallback admission and denial matrix scenarios.

 Usage:
 - Executed by the RalphCore test target as companion methods on WorkspaceQueueRefreshTests.

 Invariants/assumptions callers must respect:
 - Tests initialize isolated temp workspaces and rely on deterministic queue, watcher, and analytics convergence checks.
 */

import Foundation
import XCTest

@testable import RalphCore

@MainActor
extension WorkspaceQueueRefreshTests {
    func test_loadTasks_resolvesCustomQueuePathWhenWorkspaceOverviewCapabilityIsUnsupported() async throws {
        var workspace: Workspace!
        let rootURL = try WorkspaceTaskCreationTestSupport.makeTempDir(prefix: "ralph-workspace-custom-queue-load-")
        let workspaceURL = rootURL.appendingPathComponent("workspace", isDirectory: true)
        let customStateURL = workspaceURL.appendingPathComponent("custom-state", isDirectory: true)
        defer { RalphCoreTestSupport.shutdownAndRemove(rootURL, workspace) }

        try FileManager.default.createDirectory(at: customStateURL, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(
            at: workspaceURL.appendingPathComponent(".ralph", isDirectory: true),
            withIntermediateDirectories: true
        )

        let customQueueURL = customStateURL.appendingPathComponent("queue.jsonc", isDirectory: false)
        let customDoneURL = customStateURL.appendingPathComponent("done.jsonc", isDirectory: false)
        let projectConfigURL = workspaceURL.appendingPathComponent(".ralph/config.jsonc", isDirectory: false)
        try "[]\n".write(to: customDoneURL, atomically: true, encoding: .utf8)
        try "{ \"version\": 2, \"queue\": { \"file\": \"custom-state/queue.jsonc\", \"done_file\": \"custom-state/done.jsonc\" } }\n"
            .write(to: projectConfigURL, atomically: true, encoding: .utf8)

        let initialTask = RalphMockCLITestSupport.task(
            id: "RQ-CUSTOM-1",
            status: .todo,
            title: "Initial custom queue task",
            priority: .medium,
            createdAt: "2026-04-25T00:00:00Z",
            updatedAt: "2026-04-25T00:00:00Z"
        )
        let updatedTask = RalphMockCLITestSupport.task(
            id: "RQ-CUSTOM-2",
            status: .todo,
            title: "Updated custom queue task",
            priority: .high,
            createdAt: "2026-04-25T01:00:00Z",
            updatedAt: "2026-04-25T01:00:00Z"
        )
        try WorkspaceTaskCreationTestSupport.writeQueueDocument(to: customQueueURL, tasks: [initialTask])

        let pathOverrides = RalphMockCLITestSupport.MockResolvedPathOverrides(
            queueURL: customQueueURL,
            doneURL: customDoneURL,
            projectConfigURL: projectConfigURL
        )
        let configResolveURL = try WorkspaceRunnerConfigurationTestSupport.writeConfigResolveDocument(
            in: rootURL,
            name: "config-resolve.json",
            workspaceURL: workspaceURL,
            model: "custom-path-model",
            pathOverrides: pathOverrides
        )
        let queueReadURL = try WorkspaceRunnerConfigurationTestSupport.writeQueueReadDocument(
            in: rootURL,
            name: "queue-read.json",
            workspaceURL: workspaceURL,
            activeTasks: [initialTask],
            nextRunnableTaskID: initialTask.id,
            pathOverrides: pathOverrides
        )
        let queueReadUpdatedURL = try WorkspaceRunnerConfigurationTestSupport.writeQueueReadDocument(
            in: rootURL,
            name: "queue-read-updated.json",
            workspaceURL: workspaceURL,
            activeTasks: [updatedTask],
            nextRunnableTaskID: updatedTask.id,
            pathOverrides: pathOverrides
        )
        let graphReadURL = try WorkspaceRunnerConfigurationTestSupport.writeGraphDocument(
            in: rootURL,
            name: "graph-read.json",
            tasks: [RalphMockCLITestSupport.graphNode(id: updatedTask.id, title: updatedTask.title)]
        )
        let dashboardReadURL = rootURL.appendingPathComponent("dashboard-read.json", isDirectory: false)
        try """
        {
          "version": 1,
          "dashboard": {
            "window_days": 7,
            "generated_at": "2026-04-25T01:00:00Z",
            "sections": {
              "productivity_summary": { "status": "unavailable", "data": null, "error_message": "not needed" },
              "productivity_velocity": { "status": "unavailable", "data": null, "error_message": "not needed" },
              "burndown": { "status": "unavailable", "data": null, "error_message": "not needed" },
              "queue_stats": {
                "status": "ok",
                "data": {
                  "summary": {
                    "total": 1,
                    "done": 0,
                    "rejected": 0,
                    "terminal": 0,
                    "active": 1,
                    "terminal_rate": 0
                  },
                  "tag_breakdown": []
                },
                "error_message": null
              },
              "history": { "status": "unavailable", "data": null, "error_message": "not needed" }
            }
          }
        }
        """.write(to: dashboardReadURL, atomically: true, encoding: .utf8)

        let queueReadCurrentURL = rootURL.appendingPathComponent("queue-read-current.json", isDirectory: false)
        try FileManager.default.copyItem(at: queueReadURL, to: queueReadCurrentURL)
        let cliSpecURL = try RalphMockCLITestSupport.writeJSONDocument(
            Self.workspaceOverviewCapabilitySpecDocument(supportsWorkspaceOverview: false),
            in: rootURL,
            name: "cli-spec-no-workspace-overview.json"
        )

        let script = """
            #!/bin/sh
            set -eu
            if [ "$1" = "--no-color" ]; then
              shift
            fi
            if [ "$1" = "machine" ] && [ "$2" = "workspace" ] && [ "$3" = "overview" ]; then
              echo "unrecognized subcommand 'overview'" >&2
              exit 64
            fi
            if [ "$1" = "machine" ] && [ "$2" = "cli-spec" ]; then
              cat "\(cliSpecURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "config" ] && [ "$3" = "resolve" ]; then
              cat "\(configResolveURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "queue" ] && [ "$3" = "read" ]; then
              cat "\(queueReadCurrentURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "queue" ] && [ "$3" = "graph" ]; then
              cat "\(graphReadURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "queue" ] && [ "$3" = "dashboard" ]; then
              cat "\(dashboardReadURL.path)"
              exit 0
            fi
            echo "unexpected args: $*" >&2
            exit 64
            """
        let scriptURL = try RalphMockCLITestSupport.makeExecutableScript(
            in: rootURL,
            name: "mock-ralph-custom-queue-load",
            body: script
        )
        workspace = Workspace(
            workingDirectoryURL: workspaceURL,
            client: try RalphCLIClient(executableURL: scriptURL),
            bootstrapRepositoryStateOnInit: false
        )

        await workspace.refreshWorkspaceOverviewState(retryConfiguration: .minimal)

        XCTAssertEqual(workspace.taskState.tasks.map(\.id), [initialTask.id])
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.model, "custom-path-model")
        XCTAssertNil(workspace.runState.runnerConfigErrorMessage)
        XCTAssertEqual(workspace.queueFileURL, customQueueURL)
        XCTAssertFalse(
            FileManager.default.fileExists(
                atPath: workspaceURL.appendingPathComponent(".ralph/queue.jsonc", isDirectory: false).path
            )
        )
        XCTAssertTrue(workspace.diagnosticsState.watcherHealth.isWatching)

        try WorkspaceTaskCreationTestSupport.removeItemIfExists(queueReadCurrentURL)
        try FileManager.default.copyItem(at: queueReadUpdatedURL, to: queueReadCurrentURL)
        try WorkspaceTaskCreationTestSupport.writeQueueDocument(to: customQueueURL, tasks: [updatedTask])

        let refreshed = await RalphCoreTestSupport.waitUntil(timeout: .seconds(10)) {
            await MainActor.run {
                workspace.taskState.tasks.map(\.id) == [updatedTask.id]
                    && workspace.taskState.lastQueueRefreshEvent?.source == .externalFileChange
            }
        }

        XCTAssertTrue(refreshed)
    }

    func test_refreshWorkspaceOverview_fallbackPreflightMissingConfiguredQueuePathSurfacesGuidance() async throws {
        var workspace: Workspace!
        let rootURL = try WorkspaceTaskCreationTestSupport.makeTempDir(prefix: "ralph-workspace-overview-missing-configured-queue-")
        let workspaceURL = rootURL.appendingPathComponent("workspace", isDirectory: true)
        let customStateURL = workspaceURL.appendingPathComponent("custom-state", isDirectory: true)
        defer { RalphCoreTestSupport.shutdownAndRemove(rootURL, workspace) }

        try FileManager.default.createDirectory(at: customStateURL, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(
            at: workspaceURL.appendingPathComponent(".ralph", isDirectory: true),
            withIntermediateDirectories: true
        )

        let missingQueueURL = customStateURL.appendingPathComponent("missing-queue.jsonc", isDirectory: false)
        let customDoneURL = customStateURL.appendingPathComponent("done.jsonc", isDirectory: false)
        let projectConfigURL = workspaceURL.appendingPathComponent(".ralph/config.jsonc", isDirectory: false)
        try "[]\n".write(to: customDoneURL, atomically: true, encoding: .utf8)
        try "{}\n".write(to: projectConfigURL, atomically: true, encoding: .utf8)

        let pathOverrides = RalphMockCLITestSupport.MockResolvedPathOverrides(
            queueURL: missingQueueURL,
            doneURL: customDoneURL,
            projectConfigURL: projectConfigURL
        )
        let configResolveURL = try WorkspaceRunnerConfigurationTestSupport.writeConfigResolveDocument(
            in: rootURL,
            name: "config-resolve.json",
            workspaceURL: workspaceURL,
            model: "missing-configured-queue-model",
            pathOverrides: pathOverrides
        )
        let queueReadURL = try WorkspaceRunnerConfigurationTestSupport.writeQueueReadDocument(
            in: rootURL,
            name: "queue-read.json",
            workspaceURL: workspaceURL,
            activeTasks: [
                RalphMockCLITestSupport.task(
                    id: "RQ-MISSING-CONFIG-QUEUE",
                    status: .todo,
                    title: "Should not load",
                    priority: .medium
                )
            ],
            nextRunnableTaskID: "RQ-MISSING-CONFIG-QUEUE",
            pathOverrides: pathOverrides
        )
        let cliSpecURL = try RalphMockCLITestSupport.writeJSONDocument(
            Self.workspaceOverviewCapabilitySpecDocument(supportsWorkspaceOverview: false),
            in: rootURL,
            name: "cli-spec-no-workspace-overview.json"
        )
        let commandLogURL = rootURL.appendingPathComponent("command-log.txt", isDirectory: false)

        let script = """
            #!/bin/sh
            set -eu
            if [ "$1" = "--no-color" ]; then
              shift
            fi
            printf '%s\n' "$*" >> "\(commandLogURL.path)"
            if [ "$1" = "machine" ] && [ "$2" = "workspace" ] && [ "$3" = "overview" ]; then
              echo "unrecognized subcommand 'overview'" >&2
              exit 64
            fi
            if [ "$1" = "machine" ] && [ "$2" = "cli-spec" ]; then
              cat "\(cliSpecURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "config" ] && [ "$3" = "resolve" ]; then
              cat "\(configResolveURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "queue" ] && [ "$3" = "read" ]; then
              cat "\(queueReadURL.path)"
              exit 0
            fi
            echo "unexpected args: $*" >&2
            exit 64
            """
        let scriptURL = try RalphMockCLITestSupport.makeExecutableScript(
            in: rootURL,
            name: "mock-ralph-overview-missing-configured-queue",
            body: script
        )
        workspace = Workspace(
            workingDirectoryURL: workspaceURL,
            client: try RalphCLIClient(executableURL: scriptURL),
            bootstrapRepositoryStateOnInit: false
        )

        await workspace.refreshWorkspaceOverviewState(retryConfiguration: .minimal)

        XCTAssertTrue(workspace.taskState.tasks.isEmpty)
        XCTAssertNil(workspace.taskState.nextRunnableTaskID)
        XCTAssertFalse(workspace.diagnosticsState.showErrorRecovery)
        XCTAssertEqual(workspace.taskState.tasksErrorMessage, Workspace.missingConfiguredQueueMessage(for: missingQueueURL))

        let commandLog = try String(contentsOf: commandLogURL, encoding: .utf8)
        XCTAssertTrue(commandLog.contains("machine workspace overview"))
        XCTAssertTrue(commandLog.contains("machine cli-spec"))
        XCTAssertTrue(commandLog.contains("machine config resolve"))
        XCTAssertFalse(commandLog.contains("machine queue read"))
    }
}
