/**
 WorkspaceRunnerConfigurationTestSupport

 Purpose:
 - Centralize machine-document fixture writers for workspace runner-configuration regression tests.

 Responsibilities:
 - Centralize machine-document fixture writers for workspace runner-configuration regression tests.

 Does not handle:
 - Defining runner-configuration assertions.
 - Owning workspace lifecycle for tests.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Fixture documents mirror the current mock-CLI contracts used by the runner-configuration suites.
 */

import Foundation

@testable import CueLoopCore

enum WorkspaceRunnerConfigurationTestSupport {
    static func writeConfigResolveDocument(
        in directory: URL,
        name: String,
        workspaceURL: URL,
        model: String,
        pathOverrides: CueLoopMockCLITestSupport.MockResolvedPathOverrides? = nil,
        runner: String? = nil,
        reasoningEffort: String? = nil,
        phases: Int? = nil,
        iterations: Int? = nil,
        gitPublishMode: String? = nil,
        safety: MachineConfigSafetySummary = CueLoopMockCLITestSupport.defaultSafetySummary,
        executionControls: MachineExecutionControls = CueLoopMockCLITestSupport.defaultExecutionControls
    ) throws -> URL {
        try CueLoopMockCLITestSupport.writeJSONDocument(
            CueLoopMockCLITestSupport.configResolveDocument(
                workspaceURL: workspaceURL,
                pathOverrides: pathOverrides,
                safety: safety,
                agent: AgentConfig(
                    runner: runner,
                    model: model,
                    phases: phases,
                    iterations: iterations,
                    reasoningEffort: reasoningEffort,
                    gitPublishMode: gitPublishMode
                ),
                executionControls: executionControls
            ),
            in: directory,
            name: name
        )
    }

    static func writeQueueReadDocument(
        in directory: URL,
        name: String,
        workspaceURL: URL,
        activeTasks: [CueLoopTask],
        doneTasks: [CueLoopTask] = [],
        nextRunnableTaskID: String? = nil,
        pathOverrides: CueLoopMockCLITestSupport.MockResolvedPathOverrides? = nil
    ) throws -> URL {
        try CueLoopMockCLITestSupport.writeJSONDocument(
            CueLoopMockCLITestSupport.queueReadDocument(
                workspaceURL: workspaceURL,
                activeTasks: activeTasks,
                doneTasks: doneTasks,
                nextRunnableTaskID: nextRunnableTaskID,
                pathOverrides: pathOverrides
            ),
            in: directory,
            name: name
        )
    }

    static func writeWorkspaceOverviewDocument(
        in directory: URL,
        name: String,
        workspaceURL: URL,
        activeTasks: [CueLoopTask],
        doneTasks: [CueLoopTask] = [],
        nextRunnableTaskID: String? = nil,
        model: String,
        pathOverrides: CueLoopMockCLITestSupport.MockResolvedPathOverrides? = nil,
        phases: Int? = nil,
        iterations: Int? = nil,
        gitPublishMode: String? = nil,
        safety: MachineConfigSafetySummary = CueLoopMockCLITestSupport.defaultSafetySummary
    ) throws -> URL {
        try CueLoopMockCLITestSupport.writeJSONDocument(
            CueLoopMockCLITestSupport.workspaceOverviewDocument(
                workspaceURL: workspaceURL,
                activeTasks: activeTasks,
                doneTasks: doneTasks,
                nextRunnableTaskID: nextRunnableTaskID,
                pathOverrides: pathOverrides,
                safety: safety,
                agent: AgentConfig(
                    model: model,
                    phases: phases,
                    iterations: iterations,
                    gitPublishMode: gitPublishMode
                )
            ),
            in: directory,
            name: name
        )
    }

    static func writeGraphDocument(
        in directory: URL,
        name: String,
        tasks: [CueLoopGraphNode],
        runnableTasks: Int? = nil,
        blockedTasks: Int = 0
    ) throws -> URL {
        try CueLoopMockCLITestSupport.writeJSONDocument(
            CueLoopMockCLITestSupport.graphReadDocument(
                tasks: tasks,
                runnableTasks: runnableTasks,
                blockedTasks: blockedTasks
            ),
            in: directory,
            name: name
        )
    }

    static func writeCLISpecDocument(
        in directory: URL,
        name: String,
        machineLeafName: String?,
        about: String? = nil
    ) throws -> URL {
        try CueLoopMockCLITestSupport.writeJSONDocument(
            CueLoopMockCLITestSupport.cliSpecDocument(machineLeafName: machineLeafName, about: about),
            in: directory,
            name: name
        )
    }
}
