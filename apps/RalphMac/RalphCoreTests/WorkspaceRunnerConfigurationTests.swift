/**
 WorkspaceRunnerConfigurationTests

 Responsibilities:
 - Validate runner-configuration loading, refresh, and workspace-manager CLI override behavior.

 Does not handle:
 - Run-control streaming or task-mutation payload generation.

 Invariants/assumptions callers must respect:
 - Mock CLIs emulate only the specific argument surfaces asserted by each test.
 */

import XCTest
@testable import RalphCore

@MainActor
final class WorkspaceRunnerConfigurationTests: WorkspacePerformanceTestCase {
    func test_loadRunnerConfiguration_setsCurrentRunnerConfig() async throws {
        let tempDir = try WorkspacePerformanceTestSupport.makeTempDir(prefix: "ralph-workspace-config-")
        defer { RalphCoreTestSupport.assertRemoved(tempDir) }

        let script = """
            #!/bin/sh
            if [ "$1" = "--no-color" ] && [ "$2" = "config" ] && [ "$3" = "show" ] && [ "$4" = "--format" ] && [ "$5" = "json" ]; then
              cat <<'JSON'
            {"agent":{"model":"kimi-code/kimi-for-coding","phases":2,"iterations":3}}
            JSON
              exit 0
            fi
            echo "unexpected args: $*" 1>&2
            exit 64
            """
        let scriptURL = try WorkspacePerformanceTestSupport.makeExecutableScript(in: tempDir, name: "mock-ralph", body: script)
        let client = try RalphCLIClient(executableURL: scriptURL)
        let workspace = Workspace(workingDirectoryURL: tempDir, client: client)

        await workspace.loadRunnerConfiguration(retryConfiguration: .minimal)

        XCTAssertEqual(workspace.runState.currentRunnerConfig?.model, "kimi-code/kimi-for-coding")
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.phases, 2)
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.maxIterations, 3)
    }

    func test_loadRunnerConfiguration_onFailure_clearsCurrentRunnerConfig() async throws {
        let tempDir = try WorkspacePerformanceTestSupport.makeTempDir(prefix: "ralph-workspace-config-failure-")
        defer { RalphCoreTestSupport.assertRemoved(tempDir) }

        let successScript = """
            #!/bin/sh
            if [ "$2" = "config" ] && [ "$3" = "show" ]; then
              echo '{"agent":{"model":"kimi-initial","phases":3,"iterations":2}}'
              exit 0
            fi
            exit 64
            """
        let successScriptURL = try WorkspacePerformanceTestSupport.makeExecutableScript(
            in: tempDir,
            name: "mock-ralph-success",
            body: successScript
        )
        let successClient = try RalphCLIClient(executableURL: successScriptURL)
        let workspace = Workspace(workingDirectoryURL: tempDir, client: successClient)
        await workspace.loadRunnerConfiguration(retryConfiguration: .minimal)
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.model, "kimi-initial")
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.phases, 3)
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.maxIterations, 2)

        let failScript = """
            #!/bin/sh
            echo "config failed" 1>&2
            exit 1
            """
        let failScriptURL = try WorkspacePerformanceTestSupport.makeExecutableScript(
            in: tempDir,
            name: "mock-ralph-fail",
            body: failScript
        )
        let failClient = try RalphCLIClient(executableURL: failScriptURL)
        workspace.injectClient(failClient)

        let clearedRunnerConfig = await WorkspacePerformanceTestSupport.waitFor(timeout: 2.0) {
            workspace.runState.currentRunnerConfig == nil
        }
        XCTAssertTrue(clearedRunnerConfig)

        XCTAssertNil(workspace.runState.currentRunnerConfig)
    }

    func test_setWorkingDirectory_refreshesRunnerConfiguration() async throws {
        let rootDir = try WorkspacePerformanceTestSupport.makeTempDir(prefix: "ralph-workspace-config-switch-")
        defer { RalphCoreTestSupport.assertRemoved(rootDir) }
        let workspaceADir = rootDir.appendingPathComponent("workspace-a", isDirectory: true)
        let workspaceBDir = rootDir.appendingPathComponent("workspace-b", isDirectory: true)
        try FileManager.default.createDirectory(at: workspaceADir, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: workspaceBDir, withIntermediateDirectories: true)

        let switchScript = """
            #!/bin/sh
            if [ "$2" = "config" ] && [ "$3" = "show" ]; then
              case "$PWD" in
              */workspace-a)
                echo '{"agent":{"model":"model-a","phases":1,"iterations":1}}'
                ;;
              */workspace-b)
                echo '{"agent":{"model":"model-b","phases":2,"iterations":4}}'
                ;;
              *)
                echo '{"agent":{"model":"model-unknown","phases":3,"iterations":9}}'
                ;;
              esac
              exit 0
            fi
            exit 64
            """
        let scriptURL = try WorkspacePerformanceTestSupport.makeExecutableScript(
            in: rootDir,
            name: "mock-ralph-switch",
            body: switchScript
        )
        let client = try RalphCLIClient(executableURL: scriptURL)
        let workspace = Workspace(workingDirectoryURL: workspaceADir, client: client)

        await workspace.loadRunnerConfiguration(retryConfiguration: .minimal)
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.model, "model-a")
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.phases, 1)
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.maxIterations, 1)

        workspace.setWorkingDirectory(workspaceBDir)

        let switchedRunnerConfig = await WorkspacePerformanceTestSupport.waitFor(timeout: 2.0) {
            workspace.runState.currentRunnerConfig?.model == "model-b"
                && workspace.runState.currentRunnerConfig?.phases == 2
                && workspace.runState.currentRunnerConfig?.maxIterations == 4
        }
        XCTAssertTrue(switchedRunnerConfig)

        XCTAssertEqual(workspace.runState.currentRunnerConfig?.model, "model-b")
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.phases, 2)
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.maxIterations, 4)
    }

    func test_setWorkingDirectory_clearsRepositoryDerivedStateImmediately_andReloadsNewRepository() async throws {
        let rootDir = try WorkspacePerformanceTestSupport.makeTempDir(prefix: "ralph-workspace-retarget-")
        defer { RalphCoreTestSupport.assertRemoved(rootDir) }
        let workspaceADir = rootDir.appendingPathComponent("workspace-a", isDirectory: true)
        let workspaceBDir = rootDir.appendingPathComponent("workspace-b", isDirectory: true)
        try FileManager.default.createDirectory(at: workspaceADir, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: workspaceBDir, withIntermediateDirectories: true)
        try WorkspacePerformanceTestSupport.writeQueueFile(
            in: workspaceADir,
            tasksJSON: #"[{"id":"RQ-A","status":"todo","title":"Workspace A Task","priority":"high","tags":[],"created_at":"2026-03-05T00:00:00Z","updated_at":"2026-03-05T00:00:00Z"}]"#
        )
        try WorkspacePerformanceTestSupport.writeQueueFile(
            in: workspaceBDir,
            tasksJSON: #"[{"id":"RQ-B","status":"todo","title":"Workspace B Task","priority":"medium","tags":[],"created_at":"2026-03-06T00:00:00Z","updated_at":"2026-03-06T00:00:00Z"}]"#
        )

        let script = """
            #!/bin/sh
            case "$PWD" in
            */workspace-a) workspace="a" ;;
            */workspace-b) workspace="b" ;;
            *) workspace="unknown" ;;
            esac

            if [ "$workspace" = "b" ]; then
              sleep 0.3
            fi

            if [ "$2" = "queue" ] && [ "$3" = "list" ] && [ "$4" = "--format" ] && [ "$5" = "json" ]; then
              if [ "$workspace" = "a" ]; then
                echo '[{"id":"RQ-A","status":"todo","title":"Workspace A Task","priority":"high","tags":[],"created_at":"2026-03-05T00:00:00Z","updated_at":"2026-03-05T00:00:00Z"}]'
              else
                echo '[{"id":"RQ-B","status":"todo","title":"Workspace B Task","priority":"medium","tags":[],"created_at":"2026-03-06T00:00:00Z","updated_at":"2026-03-06T00:00:00Z"}]'
              fi
              exit 0
            fi

            if [ "$2" = "queue" ] && [ "$3" = "graph" ] && [ "$4" = "--format" ] && [ "$5" = "json" ]; then
              if [ "$workspace" = "a" ]; then
                echo '{"summary":{"total_tasks":1,"runnable_tasks":1,"blocked_tasks":0},"critical_paths":[],"tasks":[{"id":"RQ-A","title":"Graph A","status":"todo","dependencies":[],"dependents":[],"critical":false}]}'
              else
                echo '{"summary":{"total_tasks":1,"runnable_tasks":1,"blocked_tasks":0},"critical_paths":[],"tasks":[{"id":"RQ-B","title":"Graph B","status":"todo","dependencies":[],"dependents":[],"critical":false}]}'
              fi
              exit 0
            fi

            if [ "$2" = "__cli-spec" ] && [ "$3" = "--format" ] && [ "$4" = "json" ]; then
              if [ "$workspace" = "a" ]; then
                echo '{"version":1,"root":{"name":"ralph","path":["ralph"],"about":null,"long_about":null,"after_long_help":null,"hidden":false,"args":[],"subcommands":[{"name":"task-a","path":["ralph","task-a"],"about":"A","long_about":null,"after_long_help":null,"hidden":false,"args":[],"subcommands":[]}]}}'
              else
                echo '{"version":1,"root":{"name":"ralph","path":["ralph"],"about":null,"long_about":null,"after_long_help":null,"hidden":false,"args":[],"subcommands":[{"name":"task-b","path":["ralph","task-b"],"about":"B","long_about":null,"after_long_help":null,"hidden":false,"args":[],"subcommands":[]}]}}'
              fi
              exit 0
            fi

            if [ "$2" = "config" ] && [ "$3" = "show" ] && [ "$4" = "--format" ] && [ "$5" = "json" ]; then
              if [ "$workspace" = "a" ]; then
                echo '{"agent":{"model":"model-a","phases":1,"iterations":1}}'
              else
                echo '{"agent":{"model":"model-b","phases":2,"iterations":4}}'
              fi
              exit 0
            fi

            echo "unexpected args: $*" 1>&2
            exit 64
            """
        let scriptURL = try WorkspacePerformanceTestSupport.makeExecutableScript(
            in: rootDir,
            name: "mock-ralph-retarget",
            body: script
        )
        let client = try RalphCLIClient(executableURL: scriptURL)
        let workspace = Workspace(workingDirectoryURL: workspaceADir, client: client)

        await workspace.loadTasks(retryConfiguration: .minimal)
        await workspace.loadGraphData(retryConfiguration: .minimal)
        await workspace.loadCLISpec(retryConfiguration: .minimal)
        await workspace.loadRunnerConfiguration(retryConfiguration: .minimal)

        XCTAssertEqual(workspace.taskState.tasks.map(\.id), ["RQ-A"])
        XCTAssertEqual(workspace.insightsState.graphData?.tasks.map(\.id), ["RQ-A"])
        XCTAssertEqual(workspace.commandState.cliSpec?.root.subcommands.first?.name, "task-a")
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.model, "model-a")

        workspace.setWorkingDirectory(workspaceBDir)

        XCTAssertEqual(workspace.identityState.workingDirectoryURL, workspaceBDir)
        XCTAssertTrue(workspace.taskState.tasks.isEmpty)
        XCTAssertNil(workspace.insightsState.graphData)
        XCTAssertNil(workspace.commandState.cliSpec)
        XCTAssertNil(workspace.runState.currentRunnerConfig)
        XCTAssertTrue(workspace.runState.output.isEmpty)
        XCTAssertTrue(workspace.runState.executionHistory.isEmpty)

        let reloaded = await WorkspacePerformanceTestSupport.waitFor(timeout: 3.0) {
            workspace.taskState.tasks.map(\.id) == ["RQ-B"]
                && workspace.insightsState.graphData?.tasks.map(\.id) == ["RQ-B"]
                && workspace.commandState.cliSpec?.root.subcommands.first?.name == "task-b"
                && workspace.runState.currentRunnerConfig?.model == "model-b"
        }
        XCTAssertTrue(reloaded)
    }

    func test_repositoryGeneration_discardsLateResultsFromPreviousWorkspace() async throws {
        let rootDir = try WorkspacePerformanceTestSupport.makeTempDir(prefix: "ralph-workspace-retarget-stale-")
        defer { RalphCoreTestSupport.assertRemoved(rootDir) }
        let workspaceADir = rootDir.appendingPathComponent("workspace-a", isDirectory: true)
        let workspaceBDir = rootDir.appendingPathComponent("workspace-b", isDirectory: true)
        try FileManager.default.createDirectory(at: workspaceADir, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: workspaceBDir, withIntermediateDirectories: true)
        try WorkspacePerformanceTestSupport.writeQueueFile(
            in: workspaceADir,
            tasksJSON: #"[{"id":"RQ-A","status":"todo","title":"Stale A Task","priority":"high","tags":[],"created_at":"2026-03-05T00:00:00Z","updated_at":"2026-03-05T00:00:00Z"}]"#
        )
        try WorkspacePerformanceTestSupport.writeQueueFile(
            in: workspaceBDir,
            tasksJSON: #"[{"id":"RQ-B","status":"todo","title":"Fresh B Task","priority":"medium","tags":[],"created_at":"2026-03-06T00:00:00Z","updated_at":"2026-03-06T00:00:00Z"}]"#
        )

        let script = """
            #!/bin/sh
            case "$PWD" in
            */workspace-a) workspace="a" ;;
            */workspace-b) workspace="b" ;;
            *) workspace="unknown" ;;
            esac

            if [ "$workspace" = "a" ]; then
              sleep 0.5
            fi

            if [ "$2" = "queue" ] && [ "$3" = "list" ] && [ "$4" = "--format" ] && [ "$5" = "json" ]; then
              if [ "$workspace" = "a" ]; then
                echo '[{"id":"RQ-A","status":"todo","title":"Stale A Task","priority":"high","tags":[],"created_at":"2026-03-05T00:00:00Z","updated_at":"2026-03-05T00:00:00Z"}]'
              else
                echo '[{"id":"RQ-B","status":"todo","title":"Fresh B Task","priority":"medium","tags":[],"created_at":"2026-03-06T00:00:00Z","updated_at":"2026-03-06T00:00:00Z"}]'
              fi
              exit 0
            fi

            if [ "$2" = "queue" ] && [ "$3" = "graph" ] && [ "$4" = "--format" ] && [ "$5" = "json" ]; then
              if [ "$workspace" = "a" ]; then
                echo '{"summary":{"total_tasks":1,"runnable_tasks":1,"blocked_tasks":0},"critical_paths":[],"tasks":[{"id":"RQ-A","title":"Stale Graph A","status":"todo","dependencies":[],"dependents":[],"critical":false}]}'
              else
                echo '{"summary":{"total_tasks":1,"runnable_tasks":1,"blocked_tasks":0},"critical_paths":[],"tasks":[{"id":"RQ-B","title":"Fresh Graph B","status":"todo","dependencies":[],"dependents":[],"critical":false}]}'
              fi
              exit 0
            fi

            if [ "$2" = "__cli-spec" ] && [ "$3" = "--format" ] && [ "$4" = "json" ]; then
              if [ "$workspace" = "a" ]; then
                echo '{"version":1,"root":{"name":"ralph","path":["ralph"],"about":null,"long_about":null,"after_long_help":null,"hidden":false,"args":[],"subcommands":[{"name":"stale-a","path":["ralph","stale-a"],"about":"A","long_about":null,"after_long_help":null,"hidden":false,"args":[],"subcommands":[]}]}}'
              else
                echo '{"version":1,"root":{"name":"ralph","path":["ralph"],"about":null,"long_about":null,"after_long_help":null,"hidden":false,"args":[],"subcommands":[{"name":"fresh-b","path":["ralph","fresh-b"],"about":"B","long_about":null,"after_long_help":null,"hidden":false,"args":[],"subcommands":[]}]}}'
              fi
              exit 0
            fi

            if [ "$2" = "config" ] && [ "$3" = "show" ] && [ "$4" = "--format" ] && [ "$5" = "json" ]; then
              if [ "$workspace" = "a" ]; then
                echo '{"agent":{"model":"model-a-stale","phases":1,"iterations":1}}'
              else
                echo '{"agent":{"model":"model-b-fresh","phases":2,"iterations":2}}'
              fi
              exit 0
            fi

            echo "unexpected args: $*" 1>&2
            exit 64
            """
        let scriptURL = try WorkspacePerformanceTestSupport.makeExecutableScript(
            in: rootDir,
            name: "mock-ralph-retarget-stale",
            body: script
        )
        let client = try RalphCLIClient(executableURL: scriptURL)
        let workspace = Workspace(workingDirectoryURL: workspaceADir, client: client)

        let staleTaskLoad = Task { @MainActor in
            await workspace.loadTasks(retryConfiguration: .minimal)
        }
        let staleGraphLoad = Task { @MainActor in
            await workspace.loadGraphData(retryConfiguration: .minimal)
        }
        let staleSpecLoad = Task { @MainActor in
            await workspace.loadCLISpec(retryConfiguration: .minimal)
        }
        let staleConfigLoad = Task { @MainActor in
            await workspace.loadRunnerConfiguration(retryConfiguration: .minimal)
        }

        workspace.setWorkingDirectory(workspaceBDir)

        let loadedFreshWorkspace = await WorkspacePerformanceTestSupport.waitFor(timeout: 3.0) {
            workspace.taskState.tasks.map(\.id) == ["RQ-B"]
                && workspace.insightsState.graphData?.tasks.map(\.id) == ["RQ-B"]
                && workspace.commandState.cliSpec?.root.subcommands.first?.name == "fresh-b"
                && workspace.runState.currentRunnerConfig?.model == "model-b-fresh"
        }
        XCTAssertTrue(loadedFreshWorkspace)

        _ = await staleTaskLoad.result
        _ = await staleGraphLoad.result
        _ = await staleSpecLoad.result
        _ = await staleConfigLoad.result

        XCTAssertEqual(workspace.taskState.tasks.map(\.id), ["RQ-B"])
        XCTAssertEqual(workspace.insightsState.graphData?.tasks.map(\.id), ["RQ-B"])
        XCTAssertEqual(workspace.commandState.cliSpec?.root.subcommands.first?.name, "fresh-b")
        XCTAssertEqual(workspace.runState.currentRunnerConfig?.model, "model-b-fresh")
    }

    func test_setWorkingDirectory_invalidatesActiveRunState() async throws {
        let rootDir = try WorkspacePerformanceTestSupport.makeTempDir(prefix: "ralph-workspace-retarget-run-")
        defer { RalphCoreTestSupport.assertRemoved(rootDir) }
        let workspaceADir = rootDir.appendingPathComponent("workspace-a", isDirectory: true)
        let workspaceBDir = rootDir.appendingPathComponent("workspace-b", isDirectory: true)
        try FileManager.default.createDirectory(at: workspaceADir, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: workspaceBDir, withIntermediateDirectories: true)
        try WorkspacePerformanceTestSupport.writeEmptyQueueFile(in: workspaceADir)
        try WorkspacePerformanceTestSupport.writeEmptyQueueFile(in: workspaceBDir)

        let script = """
            #!/bin/sh
            trap 'exit 130' INT TERM

            if [ "$2" = "run" ] && [ "$3" = "one" ]; then
              echo "running-$PWD"
              sleep 5
              exit 0
            fi

            if [ "$2" = "config" ] && [ "$3" = "show" ] && [ "$4" = "--format" ] && [ "$5" = "json" ]; then
              echo '{"agent":{"model":"runner-model","phases":1,"iterations":1}}'
              exit 0
            fi

            if [ "$2" = "__cli-spec" ] && [ "$3" = "--format" ] && [ "$4" = "json" ]; then
              echo '{"version":1,"root":{"name":"ralph","path":["ralph"],"about":null,"long_about":null,"after_long_help":null,"hidden":false,"args":[],"subcommands":[]}}'
              exit 0
            fi

            if [ "$2" = "queue" ] && [ "$3" = "list" ] && [ "$4" = "--format" ] && [ "$5" = "json" ]; then
              echo '[]'
              exit 0
            fi

            if [ "$2" = "queue" ] && [ "$3" = "graph" ] && [ "$4" = "--format" ] && [ "$5" = "json" ]; then
              echo '{"summary":{"total_tasks":0,"runnable_tasks":0,"blocked_tasks":0},"critical_paths":[],"tasks":[]}'
              exit 0
            fi

            echo "unexpected args: $*" 1>&2
            exit 64
            """
        let scriptURL = try WorkspacePerformanceTestSupport.makeExecutableScript(
            in: rootDir,
            name: "mock-ralph-retarget-run",
            body: script
        )
        let client = try RalphCLIClient(executableURL: scriptURL)
        let workspace = Workspace(workingDirectoryURL: workspaceADir, client: client)

        workspace.run(arguments: ["--no-color", "run", "one"])

        let started = await WorkspacePerformanceTestSupport.waitFor(timeout: 2.0) {
            workspace.runState.isRunning && workspace.runState.output.contains("running-")
        }
        XCTAssertTrue(started)

        workspace.setWorkingDirectory(workspaceBDir)

        let cancelled = await WorkspacePerformanceTestSupport.waitFor(timeout: 3.0) {
            !workspace.runState.isRunning
                && workspace.runState.output.isEmpty
                && workspace.runState.currentTaskID == nil
        }
        XCTAssertTrue(cancelled)
        XCTAssertEqual(workspace.identityState.workingDirectoryURL, workspaceBDir)
        XCTAssertTrue(workspace.runState.executionHistory.isEmpty)
    }

    func test_workspaceManager_adoptCLIExecutable_rejectsValidPathOverride() async throws {
        let manager = WorkspaceManager.shared
        let baselinePath = manager.client?.executableURL.standardizedFileURL.resolvingSymlinksInPath().path
        let tempDir = try WorkspacePerformanceTestSupport.makeTempDir(prefix: "ralph-workspace-manager-cli-")
        defer { RalphCoreTestSupport.assertRemoved(tempDir) }
        let overrideURL = try WorkspacePerformanceTestSupport.makeVersionAwareMockCLI(in: tempDir, name: "mock-ralph-version-ok")

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

        manager.adoptCLIExecutable(path: "/definitely/not/a/real/ralph-binary")

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
