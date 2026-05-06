/**
 SettingsViewModelTests

 Purpose:
 - Verify Settings runner/model affordances stay aligned with machine-advertised execution controls.

 Responsibilities:
 - Assert Settings uses machine runner display names and model defaults when available.
 - Assert configured unknown runners remain selectable instead of being dropped.

 Scope:
 - SettingsViewModel presentation helpers only.

 Usage:
 - Runs as part of the CueLoopMac unit-test bundle.

 Invariants/Assumptions:
 - Tests run on the main actor because SettingsViewModel and Workspace state are main-actor isolated.
 */

import CueLoopCore
import XCTest

@testable import CueLoopMac

@MainActor
final class SettingsViewModelTests: XCTestCase {
    private func makeWorkspace() -> Workspace {
        Workspace(
            workingDirectoryURL: FileManager.default.temporaryDirectory
                .appendingPathComponent("settings-view-model-\(UUID().uuidString)", isDirectory: true),
            bootstrapRepositoryStateOnInit: false
        )
    }

    private func makeExecutionControls() -> MachineExecutionControls {
        MachineExecutionControls(
            runners: [
                MachineRunnerOption(
                    id: "pi",
                    displayName: "Pi Coding Agent",
                    source: "built_in",
                    reasoningEffortSupported: true,
                    supportsArbitraryModel: true,
                    allowedModels: [],
                    defaultModel: "zai/glm-5.1"
                ),
                MachineRunnerOption(
                    id: "acme.runner",
                    displayName: "Acme Runner",
                    source: "project_plugin",
                    reasoningEffortSupported: false,
                    supportsArbitraryModel: true,
                    allowedModels: ["acme-fast", "acme-deep"],
                    defaultModel: "acme-fast"
                ),
            ],
            reasoningEfforts: ["low", "medium", "high", "xhigh"],
            parallelWorkers: MachineParallelWorkersControl(min: 2, max: 255, defaultMissingValue: 2)
        )
    }

    func testSettingsUsesMachineAdvertisedRunnerOptionsAndPiModelDefault() {
        let workspace = makeWorkspace()
        workspace.runState.currentRunnerConfig = Workspace.RunnerConfig(
            runner: "pi",
            model: "zai/glm-5.1",
            executionControls: makeExecutionControls()
        )

        let viewModel = SettingsViewModel(workspace: workspace)
        viewModel.runner = "pi"

        XCTAssertEqual(
            viewModel.runnerChoices.map(\.displayName),
            ["Pi Coding Agent", "Acme Runner"]
        )
        XCTAssertEqual(viewModel.suggestedModels, ["zai/glm-5.1"])
    }

    func testSettingsKeepsConfiguredUnknownRunnerSelectable() {
        let workspace = makeWorkspace()
        workspace.runState.currentRunnerConfig = Workspace.RunnerConfig(
            runner: "future.runner",
            model: "future-model",
            executionControls: makeExecutionControls()
        )

        let viewModel = SettingsViewModel(workspace: workspace)
        viewModel.runner = "future.runner"

        XCTAssertEqual(viewModel.runnerChoices.last?.value, "future.runner")
        XCTAssertEqual(viewModel.runnerChoices.last?.displayName, "Configured: future.runner")
        XCTAssertEqual(viewModel.suggestedModels, [])
    }

    func testSettingsUsesMachineAllowedModelsForPluginRunner() {
        let workspace = makeWorkspace()
        workspace.runState.currentRunnerConfig = Workspace.RunnerConfig(
            runner: "acme.runner",
            model: "acme-deep",
            executionControls: makeExecutionControls()
        )

        let viewModel = SettingsViewModel(workspace: workspace)
        viewModel.runner = "acme.runner"

        XCTAssertEqual(viewModel.suggestedModels, ["acme-fast", "acme-deep"])
    }

    func testRunnerChangeUsesNewRunnerSuggestionsBeforeBindingSettles() {
        let workspace = makeWorkspace()
        workspace.runState.currentRunnerConfig = Workspace.RunnerConfig(
            runner: "pi",
            model: "zai/glm-5.1",
            executionControls: makeExecutionControls()
        )

        let viewModel = SettingsViewModel(workspace: workspace)
        viewModel.runner = "pi"
        viewModel.model = ""

        viewModel.handleRunnerChanged(to: "acme.runner")

        XCTAssertEqual(viewModel.model, "acme-fast")
    }
}
