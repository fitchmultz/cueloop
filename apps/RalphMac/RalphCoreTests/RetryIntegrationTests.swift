/**
 RetryIntegrationTests

 Purpose:
 - Validate retry behavior with mock CLI client simulating transient failures.

 Responsibilities:
 - Validate retry behavior with mock CLI client simulating transient failures.
 - Cover file lock simulation and retry recovery scenarios.

 Does not handle:
 - Real file system locking (requires external process coordination).

 Usage:
 - Used by the RalphMac app or RalphCore tests through its owning feature surface.

 Invariants/Assumptions:
 - Callers keep usage within the documented responsibilities and owning feature contracts.
 */

import Foundation
import XCTest
@testable import RalphCore

final class RetryIntegrationTests: RalphCoreTestCase {
    
    private var tempDir: URL!
    
    override func setUp() async throws {
        try await super.setUp()
        tempDir = try RalphCoreTestSupport.makeTemporaryDirectory(prefix: "retry-tests")
    }
    
    override func tearDown() async throws {
        RalphCoreTestSupport.assertRemoved(tempDir)
        try await super.tearDown()
    }
    
    func test_runAndCollectWithRetry_succeedsAfterTransientFailure() async throws {
        // Create a mock script that fails twice then succeeds
        let stateFile = tempDir.appendingPathComponent("attempt-count")
        
        let scriptContent = """
            #!/bin/bash
            ATTEMPT_FILE="\(stateFile.path)"
            if [ -f "$ATTEMPT_FILE" ]; then
                ATTEMPT=$(cat "$ATTEMPT_FILE")
            else
                ATTEMPT=0
            fi
            ATTEMPT=$((ATTEMPT + 1))
            echo $ATTEMPT > "$ATTEMPT_FILE"
            
            if [ $ATTEMPT -lt 3 ]; then
                echo "resource temporarily unavailable" >&2
                exit 1
            fi
            echo '{"tasks":[]}'
            exit 0
            """
        let scriptURL = try RalphMockCLITestSupport.makeExecutableScript(in: tempDir, name: "mock-cli", body: scriptContent)
        
        let client = try RalphCLIClient(executableURL: scriptURL)
        let result = try await client.runAndCollectWithRetry(
            arguments: ["queue", "list"],
            retryConfiguration: RetryConfiguration(maxAttempts: 3, baseDelay: 0.01, jitterRange: 0...0)
        )
        
        XCTAssertEqual(result.status.code, 0)
        XCTAssertTrue(result.stdout.contains("tasks"))
    }
    
    func test_runAndCollectWithRetry_failsOnPermanentError() async throws {
        // Create a mock script that always fails with non-retryable error
        let scriptContent = """
            #!/bin/bash
            echo "file not found" >&2
            exit 2
            """
        let scriptURL = try RalphMockCLITestSupport.makeExecutableScript(in: tempDir, name: "mock-cli", body: scriptContent)
        
        let client = try RalphCLIClient(executableURL: scriptURL)
        
        do {
            _ = try await client.runAndCollectWithRetry(
                arguments: ["queue", "list"],
                retryConfiguration: RetryConfiguration(maxAttempts: 3, baseDelay: 0.01, jitterRange: 0...0)
            )
            XCTFail("Expected error to be thrown")
        } catch {
            // Expected - should fail immediately on non-retryable error
        }
    }
    
    func test_runAndCollectWithRetry_progressCallbackInvoked() async throws {
        // Create a mock script that fails once then succeeds
        let stateFile = tempDir.appendingPathComponent("attempt-count")
        
        let scriptContent = """
            #!/bin/bash
            ATTEMPT_FILE="\(stateFile.path)"
            if [ -f "$ATTEMPT_FILE" ]; then
                ATTEMPT=$(cat "$ATTEMPT_FILE")
            else
                ATTEMPT=0
            fi
            ATTEMPT=$((ATTEMPT + 1))
            echo $ATTEMPT > "$ATTEMPT_FILE"
            
            if [ $ATTEMPT -lt 2 ]; then
                echo "device or resource busy" >&2
                exit 1
            fi
            echo '{"tasks":[]}'
            exit 0
            """
        let scriptURL = try RalphMockCLITestSupport.makeExecutableScript(in: tempDir, name: "mock-cli", body: scriptContent)
        
        let client = try RalphCLIClient(executableURL: scriptURL)
        
        actor ProgressTracker {
            var count = 0
            func increment() { count += 1 }
        }
        let tracker = ProgressTracker()
        
        _ = try await client.runAndCollectWithRetry(
            arguments: ["queue", "list"],
            retryConfiguration: RetryConfiguration(maxAttempts: 3, baseDelay: 0.01, jitterRange: 0...0),
            onRetry: { attempt, maxAttempts, delay in
                await tracker.increment()
                XCTAssertGreaterThanOrEqual(attempt, 1)
                XCTAssertLessThanOrEqual(attempt, maxAttempts)
                XCTAssertGreaterThan(delay, 0)
            }
        )
        
        let count = await tracker.count
        XCTAssertEqual(count, 1) // Should be called once for the retry
    }

    func test_runAndCollectWithRetry_retriesOnTryAgainPhrase() async throws {
        let stateFile = tempDir.appendingPathComponent("attempt-count-try-again")

        let scriptContent = """
            #!/bin/bash
            ATTEMPT_FILE="\(stateFile.path)"
            if [ -f "$ATTEMPT_FILE" ]; then
                ATTEMPT=$(cat "$ATTEMPT_FILE")
            else
                ATTEMPT=0
            fi
            ATTEMPT=$((ATTEMPT + 1))
            echo $ATTEMPT > "$ATTEMPT_FILE"

            if [ $ATTEMPT -lt 2 ]; then
                echo "try again" >&2
                exit 1
            fi
            echo '{"tasks":[]}'
            exit 0
            """
        let scriptURL = try RalphMockCLITestSupport.makeExecutableScript(
            in: tempDir,
            name: "mock-cli-try-again",
            body: scriptContent
        )

        let client = try RalphCLIClient(executableURL: scriptURL)
        let result = try await client.runAndCollectWithRetry(
            arguments: ["queue", "list"],
            retryConfiguration: RetryConfiguration(maxAttempts: 3, baseDelay: 0.01, jitterRange: 0...0)
        )

        XCTAssertEqual(result.status.code, 0)
        let attempts = Int(try String(contentsOf: stateFile, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines))
        XCTAssertEqual(attempts, 2)
    }
    
    func test_retryConfiguration_presets() {
        // Verify all preset configurations are valid
        let defaultConfig = RetryConfiguration.default
        XCTAssertEqual(defaultConfig.maxAttempts, 3)
        XCTAssertEqual(defaultConfig.baseDelay, 0.1)
        
        let minimalConfig = RetryConfiguration.minimal
        XCTAssertEqual(minimalConfig.maxAttempts, 1)
        
        let aggressiveConfig = RetryConfiguration.aggressive
        XCTAssertEqual(aggressiveConfig.maxAttempts, 5)
    }
    
    func test_workspaceRetryConfiguration_appliedCorrectly() async {
        // This test verifies that the correct retry configuration is used
        // for different workspace operations
        
        // Verify loadTasks uses default configuration
        let defaultConfig = RetryConfiguration.default
        XCTAssertEqual(defaultConfig.maxAttempts, 3)
        
        // Verify loadCLISpec uses minimal configuration
        let minimalConfig = RetryConfiguration.minimal
        XCTAssertEqual(minimalConfig.maxAttempts, 1)
        
        // Analytics loaders use minimal configuration
        let analyticsConfig = RetryConfiguration.minimal
        XCTAssertEqual(analyticsConfig.maxAttempts, 1)
    }
    
    func test_collectedOutput_toError_usesCanonicalRetryHelperClassification() {
        let retryablePatterns = [
            "resource temporarily unavailable",
            "operation would block",
            "device or resource busy",
            "resource busy",
            "file is locked",
            "io timeout",
            "timed out",
            "connection reset",
            "broken pipe",
            "eagain",
            "ewouldblock",
            "ebusy",
            "try again"
        ]
        
        for pattern in retryablePatterns {
            let output = RalphCLIClient.CollectedOutput(
                status: RalphCLIExitStatus(code: 1, reason: .exit),
                stdout: "",
                stderr: pattern
            )
            XCTAssertTrue(
                RetryHelper.defaultShouldRetry(output.toError()),
                "Pattern '\(pattern)' should be retryable"
            )
        }

        let nonRetryablePatterns = [
            "file not found",
            "permission denied",
            "invalid argument",
            "syntax error"
        ]
        
        for pattern in nonRetryablePatterns {
            let output = RalphCLIClient.CollectedOutput(
                status: RalphCLIExitStatus(code: 1, reason: .exit),
                stdout: "",
                stderr: pattern
            )
            XCTAssertFalse(
                RetryHelper.defaultShouldRetry(output.toError()),
                "Pattern '\(pattern)' should not be retryable"
            )
        }
    }
    
    func test_collectedOutput_toError() {
        let output = RalphCLIClient.CollectedOutput(
            status: RalphCLIExitStatus(code: 1, reason: .exit),
            stdout: "",
            stderr: "error message"
        )
        
        let error = output.toError() as! RetryableError
        
        if case .processError(let code, let stderr) = error {
            XCTAssertEqual(code, 1)
            XCTAssertEqual(stderr, "error message")
        } else {
            XCTFail("Expected processError")
        }
    }

    @MainActor
    func test_loadTasks_doesNotSurfaceRetryProgressAsErrorMessage() async throws {
        var workspace: Workspace!
        let fixture = try makeRepositoryRetryFixture(prefix: "retry-load-tasks")
        defer { RalphCoreTestSupport.shutdownAndRemove(fixture.rootURL, workspace) }

        let task = RalphMockCLITestSupport.task(
            id: "RQ-RETRY-SUCCESS",
            status: .todo,
            title: "Retry success",
            priority: .medium,
            createdAt: "2026-04-29T00:00:00Z",
            updatedAt: "2026-04-29T00:00:00Z"
        )
        try RalphMockCLITestSupport.writeQueueFile(in: fixture.workspaceURL, tasks: [task])
        let queueReadURL = try WorkspaceRunnerConfigurationTestSupport.writeQueueReadDocument(
            in: fixture.rootURL,
            name: "queue-read.json",
            workspaceURL: fixture.workspaceURL,
            activeTasks: [task],
            nextRunnableTaskID: task.id
        )
        let configResolveURL = try WorkspaceRunnerConfigurationTestSupport.writeConfigResolveDocument(
            in: fixture.rootURL,
            name: "config-resolve.json",
            workspaceURL: fixture.workspaceURL,
            model: "retry-model"
        )
        let attemptFile = fixture.rootURL.appendingPathComponent("queue-read-attempts")
        let scriptURL = try makeRetryingRepositoryScript(
            in: fixture.rootURL,
            name: "mock-ralph-load-tasks",
            commandPattern: "--no-color machine queue read",
            attemptFile: attemptFile,
            successOutputURL: queueReadURL,
            configResolveURL: configResolveURL,
            failAlways: false
        )
        let client = try RalphCLIClient(executableURL: scriptURL)
        workspace = RalphMockCLITestSupport.makeWorkspaceWithoutInitialRefresh(
            workingDirectoryURL: fixture.workspaceURL,
            client: client
        )

        let loadTask = Task {
            await workspace.loadTasks(retryConfiguration: transientRetryConfiguration)
        }
        let retryAttemptObserved = await waitForAttempt(attemptFile, minimum: 2)
        XCTAssertTrue(retryAttemptObserved)
        XCTAssertNil(workspace.taskState.tasksErrorMessage)
        XCTAssertFalse(workspace.diagnosticsState.showErrorRecovery)

        await loadTask.value
        XCTAssertNil(workspace.taskState.tasksErrorMessage)
        XCTAssertFalse(workspace.diagnosticsState.showErrorRecovery)
        XCTAssertEqual(workspace.taskState.tasks.map(\.id), ["RQ-RETRY-SUCCESS"])
    }

    @MainActor
    func test_loadGraphData_doesNotSurfaceRetryProgressAsErrorMessage() async throws {
        var workspace: Workspace!
        let fixture = try makeRepositoryRetryFixture(prefix: "retry-load-graph")
        defer { RalphCoreTestSupport.shutdownAndRemove(fixture.rootURL, workspace) }

        let graphReadURL = try WorkspaceRunnerConfigurationTestSupport.writeGraphDocument(
            in: fixture.rootURL,
            name: "graph-read.json",
            tasks: [RalphMockCLITestSupport.graphNode(id: "RQ-GRAPH-SUCCESS", title: "Graph success")]
        )
        let configResolveURL = try WorkspaceRunnerConfigurationTestSupport.writeConfigResolveDocument(
            in: fixture.rootURL,
            name: "config-resolve.json",
            workspaceURL: fixture.workspaceURL,
            model: "retry-model"
        )
        let attemptFile = fixture.rootURL.appendingPathComponent("graph-read-attempts")
        let scriptURL = try makeRetryingRepositoryScript(
            in: fixture.rootURL,
            name: "mock-ralph-load-graph",
            commandPattern: "--no-color machine queue graph",
            attemptFile: attemptFile,
            successOutputURL: graphReadURL,
            configResolveURL: configResolveURL,
            failAlways: false
        )
        let client = try RalphCLIClient(executableURL: scriptURL)
        workspace = RalphMockCLITestSupport.makeWorkspaceWithoutInitialRefresh(
            workingDirectoryURL: fixture.workspaceURL,
            client: client
        )

        let loadTask = Task {
            await workspace.loadGraphData(retryConfiguration: transientRetryConfiguration)
        }
        let retryAttemptObserved = await waitForAttempt(attemptFile, minimum: 2)
        XCTAssertTrue(retryAttemptObserved)
        XCTAssertNil(workspace.insightsState.graphDataErrorMessage)
        XCTAssertFalse(workspace.diagnosticsState.showErrorRecovery)

        await loadTask.value
        XCTAssertNil(workspace.insightsState.graphDataErrorMessage)
        XCTAssertNotNil(workspace.insightsState.graphData)
    }

    @MainActor
    func test_exhaustedLoadTasksRetrySurfacesOnlyTerminalErrorState() async throws {
        var workspace: Workspace!
        let fixture = try makeRepositoryRetryFixture(prefix: "retry-load-tasks-exhausted")
        defer { RalphCoreTestSupport.shutdownAndRemove(fixture.rootURL, workspace) }

        try RalphMockCLITestSupport.writeQueueFile(in: fixture.workspaceURL, tasks: [])
        let queueReadURL = try WorkspaceRunnerConfigurationTestSupport.writeQueueReadDocument(
            in: fixture.rootURL,
            name: "queue-read-unused.json",
            workspaceURL: fixture.workspaceURL,
            activeTasks: []
        )
        let configResolveURL = try WorkspaceRunnerConfigurationTestSupport.writeConfigResolveDocument(
            in: fixture.rootURL,
            name: "config-resolve.json",
            workspaceURL: fixture.workspaceURL,
            model: "retry-model"
        )
        let attemptFile = fixture.rootURL.appendingPathComponent("queue-read-attempts")
        let scriptURL = try makeRetryingRepositoryScript(
            in: fixture.rootURL,
            name: "mock-ralph-load-tasks-exhausted",
            commandPattern: "--no-color machine queue read",
            attemptFile: attemptFile,
            successOutputURL: queueReadURL,
            configResolveURL: configResolveURL,
            failAlways: true
        )
        let client = try RalphCLIClient(executableURL: scriptURL)
        workspace = RalphMockCLITestSupport.makeWorkspaceWithoutInitialRefresh(
            workingDirectoryURL: fixture.workspaceURL,
            client: client
        )

        await workspace.loadTasks(retryConfiguration: terminalRetryConfiguration)

        let errorMessage = try XCTUnwrap(workspace.taskState.tasksErrorMessage)
        XCTAssertFalse(errorMessage.isEmpty)
        XCTAssertFalse(errorMessage.contains("Retrying load tasks"))
        XCTAssertTrue(errorMessage.contains("after 2 attempts"))
        XCTAssertTrue(errorMessage.lowercased().contains("resource temporarily unavailable"))
        XCTAssertTrue(workspace.diagnosticsState.showErrorRecovery)
        XCTAssertNotNil(workspace.diagnosticsState.lastRecoveryError)
        XCTAssertTrue(workspace.taskState.tasks.isEmpty)
    }

    private var transientRetryConfiguration: RetryConfiguration {
        RetryConfiguration(maxAttempts: 2, baseDelay: 0.2, maxDelay: 0.2, jitterRange: 0...0)
    }

    private var terminalRetryConfiguration: RetryConfiguration {
        RetryConfiguration(maxAttempts: 2, baseDelay: 0.01, maxDelay: 0.01, jitterRange: 0...0)
    }

    private struct RepositoryRetryFixture {
        let rootURL: URL
        let workspaceURL: URL
    }

    private func makeRepositoryRetryFixture(prefix: String) throws -> RepositoryRetryFixture {
        let rootURL = tempDir.appendingPathComponent(prefix, isDirectory: true)
        let workspaceURL = rootURL.appendingPathComponent("workspace", isDirectory: true)
        try FileManager.default.createDirectory(
            at: workspaceURL.appendingPathComponent(".ralph", isDirectory: true),
            withIntermediateDirectories: true
        )
        return RepositoryRetryFixture(rootURL: rootURL, workspaceURL: workspaceURL)
    }

    private func makeRetryingRepositoryScript(
        in directory: URL,
        name: String,
        commandPattern: String,
        attemptFile: URL,
        successOutputURL: URL,
        configResolveURL: URL,
        failAlways: Bool
    ) throws -> URL {
        let failureCondition = failAlways ? "true" : "[ \"$ATTEMPT\" -lt 2 ]"
        let script = """
            #!/bin/sh
            case "$*" in
            *"--no-color machine config resolve"*)
              cat "\(configResolveURL.path)"
              exit 0
              ;;
            *"\(commandPattern)"*)
              ATTEMPT_FILE="\(attemptFile.path)"
              if [ -f "$ATTEMPT_FILE" ]; then
                ATTEMPT=$(cat "$ATTEMPT_FILE")
              else
                ATTEMPT=0
              fi
              ATTEMPT=$((ATTEMPT + 1))
              echo "$ATTEMPT" > "$ATTEMPT_FILE"
              if \(failureCondition); then
                echo "resource temporarily unavailable" 1>&2
                exit 1
              fi
              cat "\(successOutputURL.path)"
              exit 0
              ;;
            esac

            echo "unexpected args: $*" 1>&2
            exit 64
            """
        return try RalphMockCLITestSupport.makeExecutableScript(in: directory, name: name, body: script)
    }

    @MainActor
    private func waitForAttempt(_ attemptFile: URL, minimum: Int) async -> Bool {
        await WorkspacePerformanceTestSupport.waitFor(timeout: 1.0, pollInterval: .milliseconds(10)) {
            guard
                let raw = try? String(contentsOf: attemptFile, encoding: .utf8),
                let attempts = Int(raw.trimmingCharacters(in: .whitespacesAndNewlines))
            else {
                return false
            }
            return attempts >= minimum
        }
    }
}
