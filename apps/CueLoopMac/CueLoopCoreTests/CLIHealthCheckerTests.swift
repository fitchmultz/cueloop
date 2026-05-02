/**
 CLIHealthCheckerTests

 Purpose:
 - Validate CLI health status classification and executable probing behavior.

 Responsibilities:
 - Validate CLI health status classification and executable probing behavior.
 - Cover timeout cleanup and fallback version probing behavior.

 Does not handle:
 - General recovery category formatting or workspace offline banners.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Mock executables behave like small shell scripts and must be marked executable.
 */

import Foundation
import XCTest
@testable import CueLoopCore

final class CLIHealthCheckerTests: CueLoopCoreTestCase {
    func testHealthStatusAvailable() {
        let workspaceURL = CueLoopCoreTestSupport.workspaceURL(label: "cli-health-available")
        let status = CLIHealthStatus(
            availability: .available,
            lastChecked: Date(),
            workspaceURL: workspaceURL
        )
        XCTAssertTrue(status.isAvailable)
    }

    func testHealthStatusUnavailableCLI() {
        let workspaceURL = CueLoopCoreTestSupport.workspaceURL(label: "cli-health-unavailable")
        let status = CLIHealthStatus(
            availability: .unavailable(reason: .cliNotFound),
            lastChecked: Date(),
            workspaceURL: workspaceURL
        )
        XCTAssertFalse(status.isAvailable)
    }

    func testHealthStatusUnknown() {
        let workspaceURL = CueLoopCoreTestSupport.workspaceURL(label: "cli-health-unknown")
        let status = CLIHealthStatus(
            availability: .unknown,
            lastChecked: Date(),
            workspaceURL: workspaceURL
        )
        XCTAssertFalse(status.isAvailable)
    }

    func testUnavailabilityReasonErrorCategory() {
        XCTAssertEqual(CLIHealthStatus.UnavailabilityReason.cliNotFound.errorCategory, .cliUnavailable)
        XCTAssertEqual(CLIHealthStatus.UnavailabilityReason.permissionDenied.errorCategory, .permissionDenied)
        XCTAssertEqual(CLIHealthStatus.UnavailabilityReason.timeout.errorCategory, .networkError)
    }

    func testIsCLIUnavailableError() {
        let notFoundError = CueLoopCLIClientError.executableNotFound(URL(fileURLWithPath: "/nonexistent"))
        XCTAssertTrue(CLIHealthChecker.isCLIUnavailableError(notFoundError))

        let notExecError = CueLoopCLIClientError.executableNotExecutable(
            CueLoopCoreTestSupport.workspaceURL(label: "cli-health-not-executable")
        )
        XCTAssertTrue(CLIHealthChecker.isCLIUnavailableError(notExecError))

        let genericError = NSError(domain: "Test", code: 1)
        XCTAssertFalse(CLIHealthChecker.isCLIUnavailableError(genericError))
    }

    func testDefaultTimeoutValue() {
        XCTAssertEqual(CLIHealthChecker.defaultTimeout, 30)
    }

    func testCheckHealth_usesProvidedExecutableOverride() async throws {
        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-health-override")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let script = """
        #!/bin/sh
        if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
          echo '{"version":1,"cli_version":"9.9.9"}'
          exit 0
        fi
        exit 1
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)

        let checker = CLIHealthChecker()
        let status = await checker.checkHealth(
            workspaceID: UUID(),
            workspaceURL: tempDir,
            timeout: 2,
            executableURL: scriptURL
        )

        XCTAssertEqual(status.availability, CLIHealthStatus.Availability.available)
    }

    func testCheckHealth_fallsBackToVersionSubcommandWhenDashVersionUnsupported() async throws {
        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-health-fallback")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let script = """
        #!/bin/sh
        if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
          echo '{"version":1,"cli_version":"9.9.9"}'
          exit 0
        fi
        exit 1
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)

        let checker = CLIHealthChecker()
        let status = await checker.checkHealth(
            workspaceID: UUID(),
            workspaceURL: tempDir,
            timeout: 2,
            executableURL: scriptURL
        )

        XCTAssertEqual(status.availability, CLIHealthStatus.Availability.available)
    }

    func testCheckHealth_invalidProvidedExecutableReportsCliNotFound() async throws {
        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-health-missing")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let checker = CLIHealthChecker()
        let status = await checker.checkHealth(
            workspaceID: UUID(),
            workspaceURL: tempDir,
            timeout: 2,
            executableURL: URL(fileURLWithPath: "/definitely/not/a/real/cueloop-binary")
        )

        XCTAssertEqual(
            status.availability,
            CLIHealthStatus.Availability.unavailable(reason: .cliNotFound)
        )
    }

    func testCheckHealth_missingExecutableDoesNotAttachRetryDiagnostics() async throws {
        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-health-missing-no-retry")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let checker = CLIHealthChecker()
        let status = await checker.checkHealth(
            workspaceID: UUID(),
            workspaceURL: tempDir,
            timeout: 0.2,
            executableURL: URL(fileURLWithPath: "/definitely/not/a/real/cueloop-binary"),
            retryConfiguration: RetryConfiguration(
                maxAttempts: 3,
                baseDelay: 0.01,
                maxDelay: 0.01,
                jitterRange: 0...0
            )
        )

        XCTAssertEqual(status.availability, .unavailable(reason: .cliNotFound))
        XCTAssertNil(status.diagnostics)
    }

    func testCheckHealth_retriesTimeoutThenSucceedsWithoutDiagnostics() async throws {
        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-health-timeout-retry-success")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let attemptsURL = tempDir.appendingPathComponent("attempts", isDirectory: false)
        let script = """
        #!/bin/sh
        ATTEMPTS="\(attemptsURL.path)"
        if [ -f "$ATTEMPTS" ]; then
          ATTEMPT=$(cat "$ATTEMPTS")
        else
          ATTEMPT=0
        fi
        ATTEMPT=$((ATTEMPT + 1))
        echo $ATTEMPT > "$ATTEMPTS"

        if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
          if [ "$ATTEMPT" -eq 1 ]; then
            sleep 3
          fi
          echo '{"version":1,"cli_version":"9.9.9"}'
          exit 0
        fi
        exit 64
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)

        let checker = CLIHealthChecker()
        let status = await checker.checkHealth(
            workspaceID: UUID(),
            workspaceURL: tempDir,
            timeout: 1,
            executableURL: scriptURL,
            retryConfiguration: RetryConfiguration(
                maxAttempts: 2,
                baseDelay: 0.01,
                maxDelay: 0.01,
                jitterRange: 0...0
            )
        )

        XCTAssertEqual(status.availability, .available)
        XCTAssertNil(status.diagnostics)
        XCTAssertEqual(try recordedAttempts(at: attemptsURL), 2)
    }

    func testCheckHealth_timeoutExhaustedReportsAttemptDiagnostics() async throws {
        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-health-timeout-exhausted")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let attemptsURL = tempDir.appendingPathComponent("attempts", isDirectory: false)
        let script = """
        #!/bin/sh
        ATTEMPTS="\(attemptsURL.path)"
        if [ -f "$ATTEMPTS" ]; then ATTEMPT=$(cat "$ATTEMPTS"); else ATTEMPT=0; fi
        ATTEMPT=$((ATTEMPT + 1))
        echo $ATTEMPT > "$ATTEMPTS"
        if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
          sleep 3
          exit 0
        fi
        exit 64
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)

        let checker = CLIHealthChecker()
        let status = await checker.checkHealth(
            workspaceID: UUID(),
            workspaceURL: tempDir,
            timeout: 1,
            executableURL: scriptURL,
            retryConfiguration: RetryConfiguration(
                maxAttempts: 2,
                baseDelay: 0.01,
                maxDelay: 0.01,
                jitterRange: 0...0
            )
        )

        XCTAssertEqual(status.availability, .unavailable(reason: .timeout))
        XCTAssertEqual(status.diagnostics?.attempts, 2)
        XCTAssertEqual(status.diagnostics?.maxAttempts, 2)
        XCTAssertTrue(status.diagnostics?.finalMessage?.localizedCaseInsensitiveContains("timed out") == true)
        XCTAssertEqual(try recordedAttempts(at: attemptsURL), 2)
    }

    func testCheckHealth_retriesRetryableProcessFailureThenSucceedsWithoutDiagnostics() async throws {
        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-health-process-retry-success")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let attemptsURL = tempDir.appendingPathComponent("attempts", isDirectory: false)
        let script = """
        #!/bin/sh
        ATTEMPTS="\(attemptsURL.path)"
        if [ -f "$ATTEMPTS" ]; then ATTEMPT=$(cat "$ATTEMPTS"); else ATTEMPT=0; fi
        ATTEMPT=$((ATTEMPT + 1))
        echo $ATTEMPT > "$ATTEMPTS"
        if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
          if [ "$ATTEMPT" -eq 1 ]; then
            echo "resource temporarily unavailable" >&2
            exit 75
          fi
          echo '{"version":1,"cli_version":"9.9.9"}'
          exit 0
        fi
        exit 64
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)

        let checker = CLIHealthChecker()
        let status = await checker.checkHealth(
            workspaceID: UUID(),
            workspaceURL: tempDir,
            timeout: 2,
            executableURL: scriptURL,
            retryConfiguration: RetryConfiguration(
                maxAttempts: 2,
                baseDelay: 0.01,
                maxDelay: 0.01,
                jitterRange: 0...0
            )
        )

        XCTAssertEqual(status.availability, .available)
        XCTAssertNil(status.diagnostics)
        XCTAssertEqual(try recordedAttempts(at: attemptsURL), 2)
    }

    func testCheckHealth_timeoutTerminatesUnderlyingProcess() async throws {
        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-health-timeout")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let pidFileURL = tempDir.appendingPathComponent("health.pid", isDirectory: false)
        let script = """
        #!/bin/sh
        echo $$ > "\(pidFileURL.path)"
        if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
          trap '' TERM INT
          sleep 30
          exit 0
        fi
        exit 1
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)

        let checker = CLIHealthChecker()
        let healthTask = Task {
            await checker.checkHealth(
                workspaceID: UUID(),
                workspaceURL: tempDir,
                timeout: 3,
                executableURL: scriptURL,
                retryConfiguration: RetryConfiguration(
                    maxAttempts: 1,
                    baseDelay: 0.01,
                    maxDelay: 0.01,
                    jitterRange: 0...0
                )
            )
        }

        let recordedPID = await CueLoopCoreTestSupport.waitForFile(pidFileURL, timeout: .seconds(2))
        XCTAssertTrue(
            recordedPID,
            "Health-check timeout fixture should record its process identifier before the deadline expires"
        )

        let status = await healthTask.value

        XCTAssertEqual(
            status.availability,
            CLIHealthStatus.Availability.unavailable(reason: .timeout)
        )
        let pidText = try XCTUnwrap(String(contentsOf: pidFileURL, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines))
        let pid = pid_t(try XCTUnwrap(Int32(pidText)))
        let terminated = await CueLoopCoreTestSupport.waitForProcessExit(pid, timeout: .seconds(3))
        XCTAssertTrue(terminated, "Health-check timeout should terminate the launched process")
    }

    func testCheckHealth_taskCancellationTerminatesUnderlyingProcess() async throws {
        let tempDir = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: "cueloop-health-cancel")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let logURL = tempDir.appendingPathComponent("health-cancel.log", isDirectory: false)
        let pidFileURL = tempDir.appendingPathComponent("health-cancel.pid", isDirectory: false)
        let script = """
        #!/bin/sh
        if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
          echo $$ > "\(pidFileURL.path)"
          trap 'printf "canceled\\n" >> "\(logURL.path)"; exit 130' INT TERM
          printf 'started\n' >> "\(logURL.path)"
          sleep 30
          printf 'finished\n' >> "\(logURL.path)"
          echo '{"version":1,"cli_version":"9.9.9"}'
          exit 0
        fi
        exit 1
        """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)

        let checker = CLIHealthChecker()
        let task = Task {
            await checker.checkHealth(
                workspaceID: UUID(),
                workspaceURL: tempDir,
                timeout: 10,
                executableURL: scriptURL
            )
        }

        let started = await CueLoopCoreTestSupport.waitUntil(timeout: .seconds(2)) {
            (try? String(contentsOf: logURL, encoding: .utf8).contains("started")) == true
        }
        XCTAssertTrue(started)
        let recordedPID = await CueLoopCoreTestSupport.waitForFile(pidFileURL, timeout: .seconds(2))
        XCTAssertTrue(recordedPID)

        task.cancel()
        let status = await task.value

        XCTAssertEqual(status.availability, .unknown)

        let pidText = try XCTUnwrap(
            String(contentsOf: pidFileURL, encoding: .utf8)
                .trimmingCharacters(in: .whitespacesAndNewlines)
        )
        let pid = pid_t(try XCTUnwrap(Int32(pidText)))
        let terminated = await CueLoopCoreTestSupport.waitForProcessExit(pid, timeout: .seconds(5))
        XCTAssertTrue(terminated)

        let log = try String(contentsOf: logURL, encoding: .utf8)
        XCTAssertFalse(log.contains("finished"))
    }

    func testOperationalIssueFromTimeoutStatusMentionsExhaustedAttempts() {
        let status = CLIHealthStatus(
            availability: .unavailable(reason: .timeout),
            lastChecked: Date(),
            workspaceURL: CueLoopCoreTestSupport.workspaceURL(label: "cli-health-timeout-issue"),
            diagnostics: CLIHealthStatus.Diagnostics(
                attempts: 3,
                maxAttempts: 3,
                finalMessage: "CLI health check timed out"
            )
        )

        let issue = WorkspaceOperationalIssue.fromCLIStatus(status)
        XCTAssertEqual(issue?.severity, .warning)
        XCTAssertTrue(issue?.message.contains("3 attempts") == true)
        XCTAssertTrue(issue?.recoverySuggestion?.contains("retried automatically") == true)
    }

    private func recordedAttempts(at attemptsURL: URL) throws -> Int {
        let text = try String(contentsOf: attemptsURL, encoding: .utf8)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return try XCTUnwrap(Int(text))
    }
}
