/**
 CueLoopMockCLITestSupport

 Purpose:
 - Centralize mock CueLoop CLI script creation for CueLoopCore tests.

 Responsibilities:
 - Centralize mock CueLoop CLI script creation for CueLoopCore tests.
 - Generate machine-readable config, queue, graph, and CLI-spec fixtures with workspace-resolved paths.
 - Build test workspaces that opt into mock clients without triggering unrelated repository refreshes.
 - Keep test payloads aligned with CueLoopCore Codable contracts so path and JSON shape drift is caught in one place.

 Does not handle:
 - Production CLI behavior.
 - UI-test harness setup.
 - Highly stateful shell routing beyond writing the mock executable and fixture documents.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Encoded task dates use ISO8601 to match production decode paths.
 - Machine payloads always derive queue/config/done paths under `.cueloop` from the provided workspace URL.
 - Tests may compose custom shell routing, but fixture documents should come from this helper instead of ad hoc inline JSON.
 */

import Foundation
@testable import CueLoopCore

enum CueLoopMockCLITestSupport {
    struct Fixture {
        let rootURL: URL
        let workspaceURL: URL
        let cueloopDirectoryURL: URL
        let queueURL: URL
        let doneURL: URL
        let configURL: URL
        let scriptURL: URL
        let logURL: URL?
    }

    static let defaultSafetySummary = MachineConfigSafetySummary(
        repoTrusted: true,
        dirtyRepo: false,
        gitPublishMode: "off",
        approvalMode: "default",
        ciGateEnabled: true,
        gitRevertMode: "ask",
        parallelConfigured: false,
        executionInteractivity: "noninteractive_streaming",
        interactiveApprovalSupported: false
    )

    static let defaultExecutionControls = MachineExecutionControls(
        runners: [
            MachineRunnerOption(
                id: "claude",
                displayName: "Anthropic Claude Code",
                source: "built_in",
                reasoningEffortSupported: false,
                supportsArbitraryModel: true,
                allowedModels: [],
                defaultModel: "sonnet"
            ),
            MachineRunnerOption(
                id: "codex",
                displayName: "OpenAI Codex CLI",
                source: "built_in",
                reasoningEffortSupported: true,
                supportsArbitraryModel: false,
                allowedModels: ["gpt-5.4", "gpt-5.3-codex", "gpt-5.3-codex-spark", "gpt-5.3"],
                defaultModel: "gpt-5.4"
            ),
            MachineRunnerOption(
                id: "opencode",
                displayName: "Opencode",
                source: "built_in",
                reasoningEffortSupported: false,
                supportsArbitraryModel: true,
                allowedModels: [],
                defaultModel: "gpt-5.3"
            ),
            MachineRunnerOption(
                id: "gemini",
                displayName: "Google Gemini CLI",
                source: "built_in",
                reasoningEffortSupported: false,
                supportsArbitraryModel: true,
                allowedModels: [],
                defaultModel: "gemini-3-pro-preview"
            ),
            MachineRunnerOption(
                id: "cursor",
                displayName: "Cursor Agent",
                source: "built_in",
                reasoningEffortSupported: false,
                supportsArbitraryModel: true,
                allowedModels: [],
                defaultModel: "gpt-5.3"
            ),
            MachineRunnerOption(
                id: "kimi",
                displayName: "Kimi CLI",
                source: "built_in",
                reasoningEffortSupported: false,
                supportsArbitraryModel: true,
                allowedModels: [],
                defaultModel: "kimi-for-coding"
            ),
            MachineRunnerOption(
                id: "pi",
                displayName: "Pi Coding Agent",
                source: "built_in",
                reasoningEffortSupported: true,
                supportsArbitraryModel: true,
                allowedModels: [],
                defaultModel: "gpt-5.3"
            ),
        ],
        reasoningEfforts: ["low", "medium", "high", "xhigh"],
        parallelWorkers: MachineParallelWorkersControl(min: 2, max: 255, defaultMissingValue: 2)
    )

    static let emptyRunnability: CueLoopJSONValue = .object([:])

    static func makeFixture(
        prefix: String,
        workspaceName: String? = nil,
        scriptName: String = "mock-cueloop",
        logFileName: String? = nil,
        seedQueueTasks: [CueLoopTask]? = nil,
        seedDoneTasks: [CueLoopTask]? = nil,
        createConfigFile: Bool = false
    ) throws -> Fixture {
        let rootURL = try CueLoopCoreTestSupport.makeTemporaryDirectory(prefix: prefix)
        let workspaceURL: URL
        if let workspaceName {
            workspaceURL = rootURL.appendingPathComponent(workspaceName, isDirectory: true)
            try FileManager.default.createDirectory(at: workspaceURL, withIntermediateDirectories: true)
        } else {
            workspaceURL = rootURL
        }

        let cueloopDirectoryURL = workspaceURL.appendingPathComponent(".cueloop", isDirectory: true)
        try FileManager.default.createDirectory(at: cueloopDirectoryURL, withIntermediateDirectories: true)

        let queueURL = cueloopDirectoryURL.appendingPathComponent("queue.jsonc", isDirectory: false)
        let doneURL = cueloopDirectoryURL.appendingPathComponent("done.jsonc", isDirectory: false)
        let configURL = cueloopDirectoryURL.appendingPathComponent("config.jsonc", isDirectory: false)
        if let seedQueueTasks {
            try writeQueueFile(in: workspaceURL, tasks: seedQueueTasks)
        }
        if let seedDoneTasks {
            try writeDoneFile(in: workspaceURL, tasks: seedDoneTasks)
        }
        if createConfigFile {
            try "{}\n".write(to: configURL, atomically: true, encoding: .utf8)
        }

        let scriptURL = rootURL.appendingPathComponent(scriptName, isDirectory: false)
        let logURL = logFileName.map { rootURL.appendingPathComponent($0, isDirectory: false) }

        return Fixture(
            rootURL: rootURL,
            workspaceURL: workspaceURL,
            cueloopDirectoryURL: cueloopDirectoryURL,
            queueURL: queueURL,
            doneURL: doneURL,
            configURL: configURL,
            scriptURL: scriptURL,
            logURL: logURL
        )
    }

    static func makeExecutableScript(in directory: URL, name: String = "mock-cueloop", body: String) throws -> URL {
        let scriptURL = directory.appendingPathComponent(name, isDirectory: false)
        try body.write(to: scriptURL, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes(
            [.posixPermissions: NSNumber(value: Int16(0o755))],
            ofItemAtPath: scriptURL.path
        )
        return scriptURL
    }

    static func makeVersionAwareMockCLI(in directory: URL, name: String = "mock-cueloop") throws -> URL {
        let script = """
            #!/bin/sh
            if [ "$1" = "--version" ] || [ "$1" = "version" ]; then
              echo "cueloop \(VersionCompatibility.minimumCLIVersion)"
              exit 0
            fi
            if [ "$1" = "--no-color" ] && [ "$2" = "machine" ] && [ "$3" = "system" ] && [ "$4" = "info" ]; then
              echo '{"version":1,"cli_version":"\(VersionCompatibility.minimumCLIVersion)"}'
              exit 0
            fi
            echo "unexpected args: $*" 1>&2
            exit 64
            """
        return try makeExecutableScript(in: directory, name: name, body: script)
    }

    @MainActor
    static func makeWorkspaceWithoutInitialRefresh(
        workingDirectoryURL: URL,
        client: CueLoopCLIClient
    ) -> Workspace {
        let workspace = Workspace(workingDirectoryURL: workingDirectoryURL)
        workspace.client = client
        return workspace
    }

    struct MockResolvedPathOverrides {
        let queueURL: URL?
        let doneURL: URL?
        let projectConfigURL: URL?

        init(
            queueURL: URL? = nil,
            doneURL: URL? = nil,
            projectConfigURL: URL? = nil
        ) {
            self.queueURL = queueURL
            self.doneURL = doneURL
            self.projectConfigURL = projectConfigURL
        }
    }

    static func resolvedPaths(
        for workspaceURL: URL,
        overrides: MockResolvedPathOverrides? = nil
    ) -> MachineQueuePaths {
        let workspacePath = workspaceURL.path
        let queueURL = overrides?.queueURL
            ?? workspaceURL.appendingPathComponent(".cueloop/queue.jsonc", isDirectory: false)
        let doneURL = overrides?.doneURL
            ?? workspaceURL.appendingPathComponent(".cueloop/done.jsonc", isDirectory: false)
        let projectConfigURL = overrides?.projectConfigURL
            ?? workspaceURL.appendingPathComponent(".cueloop/config.jsonc", isDirectory: false)
        return MachineQueuePaths(
            repoRoot: workspacePath,
            queuePath: queueURL.path,
            donePath: doneURL.path,
            projectConfigPath: projectConfigURL.path,
            globalConfigPath: nil
        )
    }

    static func configResolveDocument(
        workspaceURL: URL,
        pathOverrides: MockResolvedPathOverrides? = nil,
        safety: MachineConfigSafetySummary = defaultSafetySummary,
        agent: AgentConfig = AgentConfig(),
        executionControls: MachineExecutionControls = defaultExecutionControls,
        resumePreview: MachineResumeDecision? = nil
    ) -> MachineConfigResolveDocument {
        MachineConfigResolveDocument(
            version: CueLoopMachineContract.configResolveVersion,
            paths: resolvedPaths(for: workspaceURL, overrides: pathOverrides),
            safety: safety,
            config: CueLoopConfig(agent: agent),
            executionControls: executionControls,
            resumePreview: resumePreview
        )
    }

    static func workspaceOverviewDocument(
        workspaceURL: URL,
        activeTasks: [CueLoopTask],
        doneTasks: [CueLoopTask] = [],
        nextRunnableTaskID: String? = nil,
        runnability: CueLoopJSONValue = emptyRunnability,
        pathOverrides: MockResolvedPathOverrides? = nil,
        safety: MachineConfigSafetySummary = defaultSafetySummary,
        agent: AgentConfig = AgentConfig(),
        executionControls: MachineExecutionControls = defaultExecutionControls,
        resumePreview: MachineResumeDecision? = nil
    ) -> MachineWorkspaceOverviewDocument {
        MachineWorkspaceOverviewDocument(
            version: 1,
            queue: queueReadDocument(
                workspaceURL: workspaceURL,
                activeTasks: activeTasks,
                doneTasks: doneTasks,
                nextRunnableTaskID: nextRunnableTaskID,
                runnability: runnability,
                pathOverrides: pathOverrides
            ),
            config: configResolveDocument(
                workspaceURL: workspaceURL,
                pathOverrides: pathOverrides,
                safety: safety,
                agent: agent,
                executionControls: executionControls,
                resumePreview: resumePreview
            )
        )
    }

    static func queueReadDocument(
        workspaceURL: URL,
        activeTasks: [CueLoopTask],
        doneTasks: [CueLoopTask] = [],
        nextRunnableTaskID: String? = nil,
        runnability: CueLoopJSONValue = emptyRunnability,
        pathOverrides: MockResolvedPathOverrides? = nil
    ) -> MachineQueueReadDocument {
        MachineQueueReadDocument(
            version: 1,
            paths: resolvedPaths(for: workspaceURL, overrides: pathOverrides),
            active: CueLoopTaskQueueDocument(tasks: activeTasks),
            done: CueLoopTaskQueueDocument(tasks: doneTasks),
            nextRunnableTaskID: nextRunnableTaskID,
            runnability: runnability
        )
    }

    static func graphReadDocument(
        tasks: [CueLoopGraphNode],
        runnableTasks: Int? = nil,
        blockedTasks: Int = 0,
        criticalPaths: [CueLoopCriticalPath] = []
    ) -> MachineGraphReadDocument {
        MachineGraphReadDocument(
            version: 1,
            graph: CueLoopGraphDocument(
                summary: CueLoopGraphSummary(
                    totalTasks: tasks.count,
                    runnableTasks: runnableTasks ?? tasks.count,
                    blockedTasks: blockedTasks
                ),
                criticalPaths: criticalPaths,
                tasks: tasks
            )
        )
    }

    static func cliSpecDocument(machineLeafName: String? = nil, about: String? = nil) -> MachineCLISpecDocument {
        let leafCommands: [CueLoopCLICommandSpec]
        if let machineLeafName {
            leafCommands = [
                CueLoopCLICommandSpec(
                    name: machineLeafName,
                    path: ["cueloop", "machine", machineLeafName],
                    about: about,
                    longAbout: nil,
                    afterLongHelp: nil,
                    hidden: false,
                    args: [],
                    subcommands: []
                )
            ]
        } else {
            leafCommands = []
        }

        return MachineCLISpecDocument(
            version: 2,
            spec: CueLoopCLISpecDocument(
                version: 2,
                root: CueLoopCLICommandSpec(
                    name: "cueloop",
                    path: ["cueloop"],
                    about: nil,
                    longAbout: nil,
                    afterLongHelp: nil,
                    hidden: false,
                    args: [],
                    subcommands: [
                        CueLoopCLICommandSpec(
                            name: "machine",
                            path: ["cueloop", "machine"],
                            about: "Machine",
                            longAbout: nil,
                            afterLongHelp: nil,
                            hidden: false,
                            args: [],
                            subcommands: leafCommands
                        )
                    ]
                )
            )
        )
    }

    static func graphNode(
        id: String,
        title: String,
        status: CueLoopTaskStatus = .todo,
        dependencies: [String] = [],
        dependents: [String] = [],
        critical: Bool = false
    ) -> CueLoopGraphNode {
        CueLoopGraphNode(
            id: id,
            title: title,
            status: status.rawValue,
            kind: .workItem,
            dependencies: dependencies,
            dependents: dependents,
            isCritical: critical
        )
    }

    static func task(
        id: String,
        status: CueLoopTaskStatus,
        title: String,
        priority: CueLoopTaskPriority,
        tags: [String] = [],
        createdAt: String? = nil,
        updatedAt: String? = nil,
        agent: CueLoopTaskAgent? = nil
    ) -> CueLoopTask {
        CueLoopTask(
            id: id,
            status: status,
            title: title,
            priority: priority,
            tags: tags,
            agent: agent,
            createdAt: createdAt.flatMap(date(from:)),
            updatedAt: updatedAt.flatMap(date(from:))
        )
    }

    @discardableResult
    static func writeJSONDocument<T: Encodable>(_ value: T, to url: URL) throws -> URL {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(value)
        try data.write(to: url)
        return url
    }

    @discardableResult
    static func writeJSONDocument<T: Encodable>(
        _ value: T,
        in directory: URL,
        name: String
    ) throws -> URL {
        let url = directory.appendingPathComponent(name, isDirectory: false)
        return try writeJSONDocument(value, to: url)
    }

    static func writeQueueFile(in workspaceURL: URL, tasks: [CueLoopTask]) throws {
        let cueloopDirectoryURL = workspaceURL.appendingPathComponent(".cueloop", isDirectory: true)
        try FileManager.default.createDirectory(at: cueloopDirectoryURL, withIntermediateDirectories: true)
        try writeJSONDocument(
            CueLoopTaskQueueDocument(tasks: tasks),
            to: cueloopDirectoryURL.appendingPathComponent("queue.jsonc", isDirectory: false)
        )
    }

    static func writeDoneFile(in workspaceURL: URL, tasks: [CueLoopTask]) throws {
        let cueloopDirectoryURL = workspaceURL.appendingPathComponent(".cueloop", isDirectory: true)
        try FileManager.default.createDirectory(at: cueloopDirectoryURL, withIntermediateDirectories: true)
        try writeJSONDocument(
            CueLoopTaskQueueDocument(tasks: tasks),
            to: cueloopDirectoryURL.appendingPathComponent("done.jsonc", isDirectory: false)
        )
    }

    private static func date(from iso8601: String) -> Date? {
        let formatter = ISO8601DateFormatter()
        return formatter.date(from: iso8601)
    }
}
