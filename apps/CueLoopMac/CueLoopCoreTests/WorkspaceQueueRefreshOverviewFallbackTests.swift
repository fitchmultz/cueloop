/**
 WorkspaceQueueRefreshOverviewFallbackTests

 Purpose:
 - Validate workspace-overview fallback admission and denial behavior.

 Responsibilities:
 - Cover unknown CLI spec, supported workspace-overview capability, unsupported machine-error version, and structured machine error responses.
 - Ensure fallback remains gated by explicit unsupported capability evidence.

 Does not handle:
 - Watcher-driven refresh and retargeting scenarios.
 - Custom queue path happy-path fallback loading scenarios.

 Usage:
 - Executed by the CueLoopCore test target as companion methods on WorkspaceQueueRefreshTests.

 Invariants/assumptions callers must respect:
 - Tests initialize isolated temp workspaces and rely on deterministic queue, watcher, and analytics convergence checks.
 */

import Foundation
import XCTest

@testable import CueLoopCore

@MainActor
extension WorkspaceQueueRefreshTests {
    func test_refreshWorkspaceOverview_doesNotFallbackWhenCapabilityProbeIsUnknown() async throws {
        var workspace: Workspace!
        let rootURL = try WorkspaceTaskCreationTestSupport.makeTempDir(prefix: "cueloop-workspace-overview-capability-unknown-")
        let workspaceURL = rootURL.appendingPathComponent("workspace", isDirectory: true)
        defer { CueLoopCoreTestSupport.shutdownAndRemove(rootURL, workspace) }

        try FileManager.default.createDirectory(
            at: workspaceURL.appendingPathComponent(".cueloop", isDirectory: true),
            withIntermediateDirectories: true
        )

        let fallbackTask = CueLoopMockCLITestSupport.task(
            id: "RQ-OVERVIEW-CAPABILITY-UNKNOWN",
            status: .todo,
            title: "Should not load",
            priority: .medium,
            createdAt: "2026-04-26T00:00:00Z",
            updatedAt: "2026-04-26T00:00:00Z"
        )
        let queueReadURL = try WorkspaceRunnerConfigurationTestSupport.writeQueueReadDocument(
            in: rootURL,
            name: "queue-read.json",
            workspaceURL: workspaceURL,
            activeTasks: [fallbackTask],
            nextRunnableTaskID: fallbackTask.id
        )
        let configResolveURL = try WorkspaceRunnerConfigurationTestSupport.writeConfigResolveDocument(
            in: rootURL,
            name: "config-resolve.json",
            workspaceURL: workspaceURL,
            model: "capability-unknown-model"
        )
        let invalidSpecURL = rootURL.appendingPathComponent("cli-spec-invalid.json", isDirectory: false)
        try "not-json\n".write(to: invalidSpecURL, atomically: true, encoding: .utf8)
        let commandLogURL = rootURL.appendingPathComponent("command-log.txt", isDirectory: false)

        let script = """
            #!/bin/sh
            set -eu
            if [ "$1" = "--no-color" ]; then
              shift
            fi
            printf '%s\n' "$*" >> "\(commandLogURL.path)"
            if [ "$1" = "machine" ] && [ "$2" = "workspace" ] && [ "$3" = "overview" ]; then
              echo "usage: cueloop machine workspace overview [OPTIONS]" >&2
              exit 64
            fi
            if [ "$1" = "machine" ] && [ "$2" = "cli-spec" ]; then
              cat "\(invalidSpecURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "queue" ] && [ "$3" = "read" ]; then
              cat "\(queueReadURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "config" ] && [ "$3" = "resolve" ]; then
              cat "\(configResolveURL.path)"
              exit 0
            fi
            echo "unexpected args: $*" >&2
            exit 64
            """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(
            in: rootURL,
            name: "mock-cueloop-overview-capability-unknown",
            body: script
        )
        workspace = Workspace(
            workingDirectoryURL: workspaceURL,
            client: try CueLoopCLIClient(executableURL: scriptURL),
            bootstrapRepositoryStateOnInit: false
        )

        await workspace.refreshWorkspaceOverviewState(retryConfiguration: .minimal)

        XCTAssertTrue(workspace.taskState.tasks.isEmpty)
        XCTAssertFalse(workspace.diagnosticsState.watcherHealth.isWatching)
        XCTAssertTrue(
            workspace.taskState.tasksErrorMessage?.contains("usage: cueloop machine workspace overview [OPTIONS]") ?? false
        )
        XCTAssertTrue(
            workspace.runState.runnerConfigErrorMessage?.contains("usage: cueloop machine workspace overview [OPTIONS]") ?? false
        )

        let commandLog = try String(contentsOf: commandLogURL, encoding: .utf8)
        XCTAssertTrue(commandLog.contains("machine workspace overview"))
        XCTAssertTrue(commandLog.contains("machine cli-spec"))
        XCTAssertFalse(commandLog.contains("machine queue read"))
        XCTAssertFalse(commandLog.contains("machine config resolve"))
    }

    func test_refreshWorkspaceOverview_doesNotFallbackWhenCliSpecSupportsWorkspaceOverview() async throws {
        var workspace: Workspace!
        let rootURL = try WorkspaceTaskCreationTestSupport.makeTempDir(prefix: "cueloop-workspace-overview-capability-supported-")
        let workspaceURL = rootURL.appendingPathComponent("workspace", isDirectory: true)
        defer { CueLoopCoreTestSupport.shutdownAndRemove(rootURL, workspace) }

        try FileManager.default.createDirectory(
            at: workspaceURL.appendingPathComponent(".cueloop", isDirectory: true),
            withIntermediateDirectories: true
        )

        let fallbackTask = CueLoopMockCLITestSupport.task(
            id: "RQ-OVERVIEW-FALLBACK",
            status: .todo,
            title: "Should not load",
            priority: .medium,
            createdAt: "2026-04-26T00:00:00Z",
            updatedAt: "2026-04-26T00:00:00Z"
        )
        let queueReadURL = try WorkspaceRunnerConfigurationTestSupport.writeQueueReadDocument(
            in: rootURL,
            name: "queue-read.json",
            workspaceURL: workspaceURL,
            activeTasks: [fallbackTask],
            nextRunnableTaskID: fallbackTask.id
        )
        let configResolveURL = try WorkspaceRunnerConfigurationTestSupport.writeConfigResolveDocument(
            in: rootURL,
            name: "config-resolve.json",
            workspaceURL: workspaceURL,
            model: "capability-supported-model"
        )
        let cliSpecURL = try CueLoopMockCLITestSupport.writeJSONDocument(
            Self.workspaceOverviewCapabilitySpecDocument(supportsWorkspaceOverview: true),
            in: rootURL,
            name: "cli-spec-with-workspace-overview.json"
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
              echo "usage: cueloop machine workspace overview [OPTIONS]" >&2
              exit 64
            fi
            if [ "$1" = "machine" ] && [ "$2" = "cli-spec" ]; then
              cat "\(cliSpecURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "queue" ] && [ "$3" = "read" ]; then
              cat "\(queueReadURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "config" ] && [ "$3" = "resolve" ]; then
              cat "\(configResolveURL.path)"
              exit 0
            fi
            echo "unexpected args: $*" >&2
            exit 64
            """

        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(
            in: rootURL,
            name: "mock-cueloop-overview-capability-supported",
            body: script
        )
        workspace = Workspace(
            workingDirectoryURL: workspaceURL,
            client: try CueLoopCLIClient(executableURL: scriptURL),
            bootstrapRepositoryStateOnInit: false
        )

        await workspace.refreshWorkspaceOverviewState(retryConfiguration: .minimal)

        XCTAssertTrue(workspace.taskState.tasks.isEmpty)
        XCTAssertFalse(workspace.diagnosticsState.watcherHealth.isWatching)
        XCTAssertTrue(
            workspace.taskState.tasksErrorMessage?.contains("usage: cueloop machine workspace overview [OPTIONS]") ?? false
        )
        XCTAssertTrue(
            workspace.runState.runnerConfigErrorMessage?.contains("usage: cueloop machine workspace overview [OPTIONS]") ?? false
        )

        let commandLog = try String(contentsOf: commandLogURL, encoding: .utf8)
        XCTAssertTrue(commandLog.contains("machine workspace overview"))
        XCTAssertTrue(commandLog.contains("machine cli-spec"))
        XCTAssertFalse(commandLog.contains("machine queue read"))
        XCTAssertFalse(commandLog.contains("machine config resolve"))
    }

    func test_refreshWorkspaceOverview_unsupportedMachineErrorVersionBlocksFallback() async throws {
        var workspace: Workspace!
        let rootURL = try WorkspaceTaskCreationTestSupport.makeTempDir(prefix: "cueloop-workspace-overview-machine-error-version-")
        let workspaceURL = rootURL.appendingPathComponent("workspace", isDirectory: true)
        defer { CueLoopCoreTestSupport.shutdownAndRemove(rootURL, workspace) }

        try FileManager.default.createDirectory(
            at: workspaceURL.appendingPathComponent(".cueloop", isDirectory: true),
            withIntermediateDirectories: true
        )

        let fallbackTask = CueLoopMockCLITestSupport.task(
            id: "RQ-OVERVIEW-MACHINE-ERROR-VERSION",
            status: .todo,
            title: "Should not load",
            priority: .medium,
            createdAt: "2026-04-26T00:00:00Z",
            updatedAt: "2026-04-26T00:00:00Z"
        )
        let queueReadURL = try WorkspaceRunnerConfigurationTestSupport.writeQueueReadDocument(
            in: rootURL,
            name: "queue-read.json",
            workspaceURL: workspaceURL,
            activeTasks: [fallbackTask],
            nextRunnableTaskID: fallbackTask.id
        )
        let configResolveURL = try WorkspaceRunnerConfigurationTestSupport.writeConfigResolveDocument(
            in: rootURL,
            name: "config-resolve.json",
            workspaceURL: workspaceURL,
            model: "machine-error-version-model"
        )
        let cliSpecURL = try CueLoopMockCLITestSupport.writeJSONDocument(
            Self.workspaceOverviewCapabilitySpecDocument(supportsWorkspaceOverview: false),
            in: rootURL,
            name: "cli-spec-no-workspace-overview.json"
        )
        let unsupportedMachineErrorURL = rootURL.appendingPathComponent("workspace-overview-machine-error-version.json", isDirectory: false)
        try """
        {
          "version": 999,
          "code": "resource_busy",
          "message": "Workspace overview failed.",
          "detail": "mocked version mismatch",
          "retryable": false
        }
        """.write(to: unsupportedMachineErrorURL, atomically: true, encoding: .utf8)
        let commandLogURL = rootURL.appendingPathComponent("command-log.txt", isDirectory: false)

        let script = """
            #!/bin/sh
            set -eu
            if [ "$1" = "--no-color" ]; then
              shift
            fi
            printf '%s\n' "$*" >> "\(commandLogURL.path)"
            if [ "$1" = "machine" ] && [ "$2" = "workspace" ] && [ "$3" = "overview" ]; then
              cat "\(unsupportedMachineErrorURL.path)" >&2
              exit 70
            fi
            if [ "$1" = "machine" ] && [ "$2" = "cli-spec" ]; then
              cat "\(cliSpecURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "queue" ] && [ "$3" = "read" ]; then
              cat "\(queueReadURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "config" ] && [ "$3" = "resolve" ]; then
              cat "\(configResolveURL.path)"
              exit 0
            fi
            echo "unexpected args: $*" >&2
            exit 64
            """

        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(
            in: rootURL,
            name: "mock-cueloop-overview-machine-error-version",
            body: script
        )
        workspace = Workspace(
            workingDirectoryURL: workspaceURL,
            client: try CueLoopCLIClient(executableURL: scriptURL),
            bootstrapRepositoryStateOnInit: false
        )

        await workspace.refreshWorkspaceOverviewState(retryConfiguration: .minimal)

        XCTAssertTrue(workspace.taskState.tasks.isEmpty)
        XCTAssertTrue(
            workspace.taskState.tasksErrorMessage?.contains("Unsupported machine error version 999") ?? false
        )
        XCTAssertTrue(
            workspace.runState.runnerConfigErrorMessage?.contains("Unsupported machine error version 999") ?? false
        )

        let commandLog = try String(contentsOf: commandLogURL, encoding: .utf8)
        XCTAssertTrue(commandLog.contains("machine workspace overview"))
        XCTAssertFalse(commandLog.contains("machine cli-spec"))
        XCTAssertFalse(commandLog.contains("machine queue read"))
        XCTAssertFalse(commandLog.contains("machine config resolve"))
    }

    func test_refreshWorkspaceOverview_structuredMachineErrorDoesNotTriggerFallback() async throws {
        var workspace: Workspace!
        let rootURL = try WorkspaceTaskCreationTestSupport.makeTempDir(prefix: "cueloop-workspace-overview-machine-error-")
        let workspaceURL = rootURL.appendingPathComponent("workspace", isDirectory: true)
        defer { CueLoopCoreTestSupport.shutdownAndRemove(rootURL, workspace) }

        try FileManager.default.createDirectory(
            at: workspaceURL.appendingPathComponent(".cueloop", isDirectory: true),
            withIntermediateDirectories: true
        )

        let fallbackTask = CueLoopMockCLITestSupport.task(
            id: "RQ-OVERVIEW-MACHINE-ERROR",
            status: .todo,
            title: "Should not load",
            priority: .medium,
            createdAt: "2026-04-26T00:00:00Z",
            updatedAt: "2026-04-26T00:00:00Z"
        )
        let queueReadURL = try WorkspaceRunnerConfigurationTestSupport.writeQueueReadDocument(
            in: rootURL,
            name: "queue-read.json",
            workspaceURL: workspaceURL,
            activeTasks: [fallbackTask],
            nextRunnableTaskID: fallbackTask.id
        )
        let configResolveURL = try WorkspaceRunnerConfigurationTestSupport.writeConfigResolveDocument(
            in: rootURL,
            name: "config-resolve.json",
            workspaceURL: workspaceURL,
            model: "machine-error-model"
        )
        let cliSpecURL = try CueLoopMockCLITestSupport.writeJSONDocument(
            Self.workspaceOverviewCapabilitySpecDocument(supportsWorkspaceOverview: false),
            in: rootURL,
            name: "cli-spec-no-workspace-overview.json"
        )
        let machineError = MachineErrorDocument(
            version: MachineErrorDocument.expectedVersion,
            code: .resourceBusy,
            message: "Workspace overview failed.",
            detail: "mocked machine contract failure",
            retryable: false
        )
        let machineErrorURL = try CueLoopMockCLITestSupport.writeJSONDocument(
            machineError,
            in: rootURL,
            name: "workspace-overview-machine-error.json"
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
              cat "\(machineErrorURL.path)" >&2
              exit 70
            fi
            if [ "$1" = "machine" ] && [ "$2" = "cli-spec" ]; then
              cat "\(cliSpecURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "queue" ] && [ "$3" = "read" ]; then
              cat "\(queueReadURL.path)"
              exit 0
            fi
            if [ "$1" = "machine" ] && [ "$2" = "config" ] && [ "$3" = "resolve" ]; then
              cat "\(configResolveURL.path)"
              exit 0
            fi
            echo "unexpected args: $*" >&2
            exit 64
            """

        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(
            in: rootURL,
            name: "mock-cueloop-overview-machine-error",
            body: script
        )
        workspace = Workspace(
            workingDirectoryURL: workspaceURL,
            client: try CueLoopCLIClient(executableURL: scriptURL),
            bootstrapRepositoryStateOnInit: false
        )

        await workspace.refreshWorkspaceOverviewState(retryConfiguration: .minimal)

        XCTAssertTrue(workspace.taskState.tasks.isEmpty)
        XCTAssertEqual(workspace.taskState.tasksErrorMessage, machineError.message)
        XCTAssertEqual(workspace.runState.runnerConfigErrorMessage, machineError.message)

        let commandLog = try String(contentsOf: commandLogURL, encoding: .utf8)
        XCTAssertTrue(commandLog.contains("machine workspace overview"))
        XCTAssertFalse(commandLog.contains("machine cli-spec"))
        XCTAssertFalse(commandLog.contains("machine queue read"))
        XCTAssertFalse(commandLog.contains("machine config resolve"))
    }
}
