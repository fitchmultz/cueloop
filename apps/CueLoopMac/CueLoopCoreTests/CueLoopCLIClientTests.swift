/**
 CueLoopCLIClientTests

 Purpose:
 - Validate `CueLoopCLIClient` subprocess behavior in isolation.

 Responsibilities:
 - Validate `CueLoopCLIClient` subprocess behavior in isolation.
 - Cover success, non-zero exit, stdout/stderr streaming, working directory configuration, and cancellation.

 Does not handle:
 - CueLoop-specific command semantics (covered by E2E smoke tests).
 - UI rendering or SwiftUI integration.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Tests run on macOS with `/bin/sh` and `/bin/echo` available.
 */

import Foundation
import XCTest

@testable import CueLoopCore

final class CueLoopCLIClientTests: CueLoopCoreTestCase {
    func test_success_exitCodeZero_and_streamsStdout() async throws {
        let client = try CueLoopCLIClient(executableURL: URL(fileURLWithPath: "/bin/echo"))
        let run = try client.start(arguments: ["hello"])

        var stdout = Data()
        var stderr = Data()
        for await event in await run.events {
            switch event.stream {
            case .stdout:
                stdout.append(event.data)
            case .stderr:
                stderr.append(event.data)
            }
        }

        let status = await run.waitUntilExit()
        XCTAssertEqual(status.code, 0)
        XCTAssertEqual(status.reason, .exit)
        XCTAssertEqual(String(decoding: stdout, as: UTF8.self), "hello\n")
        XCTAssertTrue(stderr.isEmpty)
    }

    func test_failure_exitCodeNonZero() async throws {
        let client = try CueLoopCLIClient(executableURL: URL(fileURLWithPath: "/bin/sh"))
        let run = try client.start(arguments: ["-c", "exit 42"])

        for await _ in await run.events {
            // Drain.
        }

        let status = await run.waitUntilExit()
        XCTAssertEqual(status.code, 42)
        XCTAssertEqual(status.reason, .exit)
    }

    func test_runAndCollect_collectsStdoutStderr_and_exitStatus() async throws {
        let client = try CueLoopCLIClient(executableURL: URL(fileURLWithPath: "/bin/sh"))
        let collected = try await client.runAndCollect(arguments: ["-c", "echo out1; echo err1 1>&2; exit 7"])

        XCTAssertEqual(collected.status.reason, .exit)
        XCTAssertEqual(collected.status.code, 7)
        XCTAssertTrue(collected.stdout.contains("out1"))
        XCTAssertTrue(collected.stderr.contains("err1"))
    }

    func test_streaming_stdout_and_stderr() async throws {
        let client = try CueLoopCLIClient(executableURL: URL(fileURLWithPath: "/bin/sh"))
        let script = "echo out1; echo err1 1>&2; echo out2; echo err2 1>&2"
        let run = try client.start(arguments: ["-c", script])

        var stdout = ""
        var stderr = ""
        for await event in await run.events {
            switch event.stream {
            case .stdout:
                stdout.append(event.text)
            case .stderr:
                stderr.append(event.text)
            }
        }

        let status = await run.waitUntilExit()
        XCTAssertEqual(status.code, 0)
        XCTAssertTrue(stdout.contains("out1"))
        XCTAssertTrue(stdout.contains("out2"))
        XCTAssertTrue(stderr.contains("err1"))
        XCTAssertTrue(stderr.contains("err2"))
    }

    func test_streamingOutputBufferIsBoundedWhenConsumerIsDelayed() async throws {
        let tempDir = try Self.makeTempDir(prefix: "cueloop-client-bounded-stream-")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let script = """
            #!/bin/sh
            /usr/bin/perl -e 'print "x" x (40 * 1024 * 1024)'
            """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)
        let client = try CueLoopCLIClient(executableURL: scriptURL)
        let run = try client.start(arguments: [])

        let status = await run.waitUntilExit()
        XCTAssertEqual(status.code, 0)

        var collectedBytes = 0
        for await event in await run.events where event.stream == .stdout {
            collectedBytes += event.data.count
        }

        XCTAssertGreaterThan(collectedBytes, 0)
        XCTAssertLessThanOrEqual(collectedBytes, 33 * 1024 * 1024)
    }

    func test_runAndCollect_appliesDefaultOutputLimit() async throws {
        let tempDir = try Self.makeTempDir(prefix: "cueloop-client-collected-limit-")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let script = """
            #!/bin/sh
            /usr/bin/perl -e 'print "x" x (20 * 1024 * 1024)'
            """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)
        let client = try CueLoopCLIClient(executableURL: scriptURL)

        let collected = try await client.runAndCollect(arguments: [])

        XCTAssertEqual(collected.status.code, 0)
        XCTAssertLessThanOrEqual(collected.stdout.count, CueLoopCLIClient.defaultCollectedOutputLimit + (64 * 1024))
        XCTAssertTrue(collected.stderr.contains("output exceeded maximum size"))
    }

    func test_currentDirectoryURL_used() async throws {
        let tempDir = try Self.makeTempDir(prefix: "cueloop-client-cwd-")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let client = try CueLoopCLIClient(executableURL: URL(fileURLWithPath: "/bin/sh"))
        let run = try client.start(
            arguments: ["-c", "pwd"],
            currentDirectoryURL: tempDir
        )

        var stdout = ""
        for await event in await run.events {
            if event.stream == .stdout {
                stdout.append(event.text)
            }
        }

        _ = await run.waitUntilExit()

        let printed = stdout.trimmingCharacters(in: .whitespacesAndNewlines)
        let printedURL = URL(fileURLWithPath: printed).resolvingSymlinksInPath()
        XCTAssertEqual(printedURL.path, tempDir.resolvingSymlinksInPath().path)
    }

    func test_start_setsForegroundUIEnvironmentByDefault() async throws {
        let client = try CueLoopCLIClient(executableURL: URL(fileURLWithPath: "/bin/sh"))
        let run = try client.start(arguments: ["-c", "printf %s \"${CUELOOP_UI_ACTIVE:-missing}\""])

        var stdout = ""
        for await event in await run.events where event.stream == .stdout {
            stdout.append(event.text)
        }

        let status = await run.waitUntilExit()
        XCTAssertEqual(status.code, 0)
        XCTAssertEqual(stdout, "1")
    }

    func test_launchEnvironment_forcesForegroundUIEnvironmentAfterOverrides() {
        let environment = CueLoopCLIClient.launchEnvironment(
            base: ["BASE": "1"],
            overrides: [
                "OVERRIDE": "2",
                CueLoopCLIClient.uiActiveEnvironmentKey: "0",
            ]
        )

        XCTAssertEqual(environment["BASE"], "1")
        XCTAssertEqual(environment["OVERRIDE"], "2")
        XCTAssertEqual(environment[CueLoopCLIClient.uiActiveEnvironmentKey], "1")
    }

    func test_cancellation_terminatesProcess() async throws {
        let client = try CueLoopCLIClient(executableURL: URL(fileURLWithPath: "/bin/sleep"))
        let run = try client.start(arguments: ["60"])

        await run.cancel()
        for await _ in await run.events {
            // Drain until process exits.
        }

        let status = await run.waitUntilExit()
        XCTAssertTrue(status.reason == .uncaughtSignal || status.reason == .exit)
        XCTAssertNotEqual(status.code, 0)
    }

    func test_cancellation_interruptsBeforeTerminate() async throws {
        let tempDir = try Self.makeTempDir(prefix: "cueloop-client-interrupt-")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let readyURL = tempDir.appendingPathComponent("cancel-ready.log", isDirectory: false)
        let signalURL = tempDir.appendingPathComponent("cancel-signal.log", isDirectory: false)
        let script = """
            #!/bin/sh
            trap 'printf "INT\n" >> "\(signalURL.path)"; exit 130' INT
            trap 'printf "TERM\n" >> "\(signalURL.path)"; exit 143' TERM
            printf "ready\n" >> "\(readyURL.path)"
            sleep 30
            """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)
        let client = try CueLoopCLIClient(executableURL: scriptURL)
        let run = try client.start(arguments: [])

        let ready = await CueLoopCoreTestSupport.waitForFile(readyURL, timeout: .seconds(2))
        XCTAssertTrue(ready)
        await run.cancel()
        for await _ in await run.events {
            // Drain until process exits.
        }

        let status = await run.waitUntilExit()
        XCTAssertTrue(status.reason == .uncaughtSignal || status.reason == .exit)
        let signal = try String(contentsOf: signalURL, encoding: .utf8)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        XCTAssertEqual(signal, "INT")
    }

    func test_runAndCollect_taskCancellation_terminatesProcessAndThrowsCancellation() async throws {
        let tempDir = try Self.makeTempDir(prefix: "cueloop-client-cancel-")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let logURL = tempDir.appendingPathComponent("run-and-collect-cancel.log", isDirectory: false)
        let pidFileURL = tempDir.appendingPathComponent("run-and-collect-cancel.pid", isDirectory: false)
        let script = """
            #!/bin/sh
            echo $$ > "\(pidFileURL.path)"
            trap 'printf "canceled\\n" >> "\(logURL.path)"; exit 130' INT TERM
            printf 'started\n' >> "\(logURL.path)"
            sleep 30
            printf 'finished\n' >> "\(logURL.path)"
            """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: script)
        let client = try CueLoopCLIClient(executableURL: scriptURL)

        let task = Task {
            try await client.runAndCollect(arguments: [])
        }

        let started = await CueLoopCoreTestSupport.waitUntil(timeout: .seconds(2)) {
            (try? String(contentsOf: logURL, encoding: .utf8).contains("started")) == true
        }
        XCTAssertTrue(started)
        let recordedPID = await CueLoopCoreTestSupport.waitForFile(pidFileURL, timeout: .seconds(2))
        XCTAssertTrue(recordedPID)

        task.cancel()

        do {
            _ = try await task.value
            XCTFail("Expected task cancellation to throw CancellationError")
        } catch is CancellationError {
            // Expected.
        }

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

    // MARK: - Version Parsing Integration

    func test_runAndCollect_versionOutput_parsableByVersionValidator() async throws {
        // Simulate a CLI that outputs a version string
        let tempDir = try Self.makeTempDir(prefix: "cueloop-version-")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }
        let compatibleVersion = VersionCompatibility.minimumCLIVersion

        let scriptContent = """
            #!/bin/sh
            echo "cueloop \(compatibleVersion)"
            """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: scriptContent)

        let client = try CueLoopCLIClient(executableURL: scriptURL)
        let collected = try await client.runAndCollect(arguments: ["--version"])

        XCTAssertEqual(collected.status.code, 0)

        // Verify the output can be parsed by VersionValidator
        let versionString = collected.stdout.trimmingCharacters(in: .whitespacesAndNewlines)
        let validator = VersionValidator()
        let result = validator.validate(versionString)

        XCTAssertTrue(result.isCompatible, "Version '\(versionString)' should be compatible")
        XCTAssertEqual(result.rawVersion, "cueloop \(compatibleVersion)")
    }

    func test_runAndCollect_versionOutput_withVPrefix_parsable() async throws {
        // Simulate a CLI that outputs version with v prefix
        let tempDir = try Self.makeTempDir(prefix: "cueloop-version-")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }
        let compatibleVersion = VersionCompatibility.maximumCLIVersion

        let scriptContent = """
            #!/bin/sh
            echo "v\(compatibleVersion)"
            """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: scriptContent)

        let client = try CueLoopCLIClient(executableURL: scriptURL)
        let collected = try await client.runAndCollect(arguments: ["--version"])

        let versionString = collected.stdout.trimmingCharacters(in: .whitespacesAndNewlines)
        let validator = VersionValidator()
        let result = validator.validate(versionString)

        XCTAssertTrue(result.isCompatible, "Version '\(versionString)' should be compatible")
    }

    func test_runAndCollect_incompatibleVersion_detected() async throws {
        // Simulate a CLI with an incompatible (too new) version
        let tempDir = try Self.makeTempDir(prefix: "cueloop-version-")
        defer { CueLoopCoreTestSupport.assertRemoved(tempDir) }

        let scriptContent = """
            #!/bin/sh
            echo "1.5.0"
            """
        let scriptURL = try CueLoopMockCLITestSupport.makeExecutableScript(in: tempDir, body: scriptContent)

        let client = try CueLoopCLIClient(executableURL: scriptURL)
        let collected = try await client.runAndCollect(arguments: ["--version"])

        let versionString = collected.stdout.trimmingCharacters(in: .whitespacesAndNewlines)
        let validator = VersionValidator()
        let result = validator.validate(versionString)

        XCTAssertFalse(result.isCompatible, "Version '\(versionString)' should be incompatible (too new)")
        XCTAssertNotNil(result.errorMessage)
    }

    private static func makeTempDir(prefix: String) throws -> URL {
        try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: prefix)
    }
}
