/**
 RunControlDetailSections+Configuration

 Responsibilities:
 - Render runner-configuration and execution-control cards for Run Control.
 - Keep execution actions and status presentation out of progress/history/safety sections.

 Does not handle:
 - Task-summary cards.
 - Queue-preview selection or phase-progress rendering.
 */

import RalphCore
import SwiftUI

@MainActor
struct RunControlRunnerConfigurationSection: View {
    @ObservedObject var workspace: Workspace

    var body: some View {
        RunControlGlassSection("Runner Configuration") {
            VStack(alignment: .leading, spacing: 8) {
                if workspace.runState.runnerConfigLoading {
                    HStack(spacing: 8) {
                        ProgressView()
                            .controlSize(.small)
                        Text("Loading resolved config...")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }

                RunControlConfigRow(icon: "cpu", label: "Model", value: workspace.runState.currentRunnerConfig?.model ?? "Default")
                RunControlConfigRow(icon: "square.split.2x1", label: "Phases", value: workspace.runState.currentRunnerConfig?.phases.map(String.init) ?? "Auto")
                RunControlConfigRow(icon: "number", label: "Max Iterations", value: workspace.runState.currentRunnerConfig?.maxIterations.map(String.init) ?? "Auto")

                if let configError = workspace.runState.runnerConfigErrorMessage {
                    Text(configError)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }
}

@MainActor
struct RunControlExecutionControlsSection: View {
    @ObservedObject var workspace: Workspace

    var body: some View {
        RunControlGlassSection("Controls") {
            VStack(spacing: 12) {
                let previewTask = workspace.runControlPreviewTask
                let hasSelectedTask = workspace.selectedRunControlTask != nil

                HStack(spacing: 12) {
                    if workspace.runState.isRunning {
                        Button(action: { workspace.cancel() }) {
                            Label("Stop", systemImage: "stop.circle.fill")
                                .foregroundStyle(.red)
                        }
                        .buttonStyle(GlassButtonStyle())
                        .accessibilityLabel("Stop execution")
                        .accessibilityHint("Cancel the current task execution")

                        if workspace.runState.isLoopMode {
                            Button(action: { workspace.stopLoop() }) {
                                Label("Stop After Current", systemImage: "pause.circle")
                                    .foregroundStyle(.orange)
                            }
                            .buttonStyle(GlassButtonStyle())
                        }
                    } else {
                        Button(action: {
                            workspace.runNextTask(
                                taskIDOverride: workspace.runState.runControlSelectedTaskID,
                                forceDirtyRepo: workspace.runState.runControlForceDirtyRepo
                            )
                        }) {
                            Label(hasSelectedTask ? "Run Selected Task" : "Run Next Task", systemImage: "play.circle.fill")
                        }
                        .buttonStyle(GlassButtonStyle())
                        .disabled(previewTask == nil)
                        .accessibilityLabel("Run next task")
                        .accessibilityHint("Starts execution of the selected task or next task in the queue")

                        Button(action: { workspace.startLoop(forceDirtyRepo: workspace.runState.runControlForceDirtyRepo) }) {
                            Label("Start Loop", systemImage: "repeat.circle")
                        }
                        .buttonStyle(GlassButtonStyle())
                        .disabled(workspace.nextTask() == nil)
                        .accessibilityLabel("Start task loop")
                        .accessibilityHint("Continuously run tasks until stopped")
                    }

                    Spacer()
                }

                if workspace.runState.isLoopMode {
                    HStack {
                        Image(systemName: "repeat.circle.fill")
                            .foregroundStyle(.blue)
                        Text("Loop Mode Active")
                            .font(.caption)
                            .foregroundStyle(.secondary)

                        if workspace.runState.stopAfterCurrent {
                            Text("(Stopping after current)")
                                .font(.caption)
                                .foregroundStyle(.orange)
                        }

                        Spacer()
                    }
                }

                if let status = workspace.runState.lastExitStatus, !workspace.runState.isRunning {
                    HStack {
                        Image(systemName: status.code == 0 ? "checkmark.circle.fill" : "xmark.circle.fill")
                            .foregroundStyle(status.code == 0 ? .green : .red)
                        Text("Exit: \(status.code)")
                            .font(.system(.caption, design: .monospaced))
                            .foregroundStyle(status.code == 0 ? .green : .red)
                        Spacer()
                    }
                }
            }
        }
    }
}
