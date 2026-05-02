/**
 WorkspaceTaskCreationTestSupport

 Purpose:
 - Centralize CLI/bootstrap and queue-document helpers for workspace task-creation and watcher integration tests.

 Responsibilities:
 - Centralize CLI/bootstrap and queue-document helpers for workspace task-creation and watcher integration tests.

 Does not handle:
 - Defining task-creation or watcher assertions.
 - UI automation flows.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - A deterministic `cueloop` binary is available via `CUELOOP_BIN_PATH` or the bundled app binary.
 */

import Foundation
import XCTest

@testable import CueLoopCore

enum WorkspaceTaskCreationTestSupport {
    static func runChecked(
        client: CueLoopCLIClient,
        arguments: [String],
        currentDirectoryURL: URL
    ) async throws {
        let result = try await client.runAndCollect(
            arguments: arguments,
            currentDirectoryURL: currentDirectoryURL
        )
        XCTAssertEqual(result.status.code, 0, "Command failed: \(arguments.joined(separator: " "))\nstderr:\n\(result.stderr)")
    }

    static func prepareWatcherFixture(at workspaceURL: URL) throws -> URL {
        let cueloopURL = workspaceURL.appendingPathComponent(".cueloop", isDirectory: true)
        try FileManager.default.createDirectory(at: cueloopURL, withIntermediateDirectories: true)
        try "[]\n".write(
            to: cueloopURL.appendingPathComponent("done.jsonc", isDirectory: false),
            atomically: true,
            encoding: .utf8
        )
        try "{}\n".write(
            to: cueloopURL.appendingPathComponent("config.jsonc", isDirectory: false),
            atomically: true,
            encoding: .utf8
        )
        return cueloopURL
    }

    static func writeQueueDocument(to url: URL, tasks: [CueLoopTask]) throws {
        let document = CueLoopTaskQueueDocument(tasks: tasks)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        encoder.dateEncodingStrategy = .iso8601
        let data = try encoder.encode(document)
        try data.write(to: url, options: .atomic)
    }

    static func removeItemIfExists(_ url: URL) throws {
        guard FileManager.default.fileExists(atPath: url.path) else { return }
        try FileManager.default.removeItem(at: url)
    }

    private static let binaryPathEnvKey = "CUELOOP_BIN_PATH"

    static func resolveCueLoopBinaryURL() throws -> URL {
        if let override = binaryPathOverride(from: ProcessInfo.processInfo.environment), !override.isEmpty {
            let overrideURL = URL(fileURLWithPath: override)
            guard FileManager.default.isExecutableFile(atPath: overrideURL.path) else {
                throw NSError(
                    domain: "WorkspaceTaskCreationTests",
                    code: 2,
                    userInfo: [NSLocalizedDescriptionKey: "CUELOOP_BIN_PATH points to a non-executable path: \(overrideURL.path)"]
                )
            }
            return overrideURL
        }

        let executableDirectory = Bundle(for: CueLoopCoreTestCase.self).bundleURL
            .deletingLastPathComponent()
            .appendingPathComponent("CueLoopMac.app", isDirectory: true)
            .appendingPathComponent("Contents", isDirectory: true)
            .appendingPathComponent("MacOS", isDirectory: true)
        let bundledURL = executableDirectory.appendingPathComponent("cueloop", isDirectory: false)
        if FileManager.default.isExecutableFile(atPath: bundledURL.path) {
            return bundledURL
        }

        throw NSError(
            domain: "WorkspaceTaskCreationTests",
            code: 2,
            userInfo: [NSLocalizedDescriptionKey: "Failed to locate a usable cueloop binary for WorkspaceTaskCreationTests"]
        )
    }

    private static func binaryPathOverride(from environment: [String: String]) -> String? {
        return environment[binaryPathEnvKey]?.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    static func makeTempDir(prefix: String) throws -> URL {
        try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: prefix)
    }
}

extension WorkspaceQueueRefreshTests {
    static func workspaceOverviewCapabilitySpecDocument(
        supportsWorkspaceOverview: Bool
    ) -> MachineCLISpecDocument {
        let queueCommand = commandSpec(
            name: "queue",
            path: ["cueloop", "machine", "queue"]
        )
        let workspaceOverviewCommand = commandSpec(
            name: "overview",
            path: ["cueloop", "machine", "workspace", "overview"]
        )
        let workspaceCommand = commandSpec(
            name: "workspace",
            path: ["cueloop", "machine", "workspace"],
            subcommands: supportsWorkspaceOverview ? [workspaceOverviewCommand] : []
        )
        let machineSubcommands = supportsWorkspaceOverview
            ? [queueCommand, workspaceCommand]
            : [queueCommand]

        return MachineCLISpecDocument(
            version: CueLoopMachineContract.cliSpecVersion,
            spec: CueLoopCLISpecDocument(
                version: CueLoopCLISpecDocument.expectedVersion,
                root: commandSpec(
                    name: "cueloop",
                    path: ["cueloop"],
                    subcommands: [
                        commandSpec(
                            name: "machine",
                            path: ["cueloop", "machine"],
                            subcommands: machineSubcommands
                        )
                    ]
                )
            )
        )
    }

    static func commandSpec(
        name: String,
        path: [String],
        subcommands: [CueLoopCLICommandSpec] = []
    ) -> CueLoopCLICommandSpec {
        CueLoopCLICommandSpec(
            name: name,
            path: path,
            about: nil,
            longAbout: nil,
            afterLongHelp: nil,
            hidden: false,
            args: [],
            subcommands: subcommands
        )
    }
}
