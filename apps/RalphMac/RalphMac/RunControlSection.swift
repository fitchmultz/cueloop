/**
 RunControlSection

 Responsibilities:
 - Provide Run Control content column with working directory and live console.
 - Provide Run Control detail column with task cards, progress, controls, and history.
 - Display execution state: current task, phase progress, up-next preview, controls.

 Does not handle:
 - Console output rendering (delegated to RunControlConsoleView).
 - Direct task execution logic (delegated to Workspace).
 - Task selection from queue (handled by QueueContent).

 Invariants/assumptions callers must respect:
 - Workspace is injected via @ObservedObject.
 - View updates when workspace.executionHistory or isRunning changes.
 - Requires main actor for UI updates.
 */

import SwiftUI
import RalphCore

@MainActor
struct RunControlContentColumn: View {
    @ObservedObject var workspace: Workspace
    let navTitle: (String) -> String

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            WorkingDirectoryHeader(workspace: workspace)
                .padding(16)

            Divider()

            RunControlConsoleView(workspace: workspace)
                .padding(16)
        }
        .contentBackground(cornerRadius: 12)
        .navigationTitle(navTitle("Run Control"))
    }
}

@MainActor
struct RunControlDetailColumn: View {
    @ObservedObject var workspace: Workspace
    let navTitle: (String) -> String

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                currentTaskSection()

                if workspace.isRunning {
                    phaseProgressSection()
                }

                runTargetSection()

                runnerConfigSection()

                executionControlsSection()

                executionHistorySection()
            }
            .padding(20)
        }
        .background(.clear)
        .navigationTitle(navTitle("Run Control"))
        .task(id: workspace.workingDirectoryURL.path) {
            await workspace.refreshRunControlData()
        }
    }

    @ViewBuilder
    private func currentTaskSection() -> some View {
        if workspace.isRunning, let taskID = workspace.currentTaskID,
           let task = workspace.tasks.first(where: { $0.id == taskID }) {
            currentTaskCard(task: task)
        } else if !workspace.isRunning && !workspace.executionHistory.isEmpty {
            lastRunSummary()
        } else {
            noExecutionView()
        }
    }

    @ViewBuilder
    private func currentTaskCard(task: RalphTask) -> some View {
        glassGroupBox("Current Task") {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    Text(task.id)
                        .font(.system(.caption, design: .monospaced))
                        .foregroundStyle(.secondary)
                        .accessibilityLabel("Task ID: \(task.id)")

                    Spacer()

                    PriorityBadge(priority: task.priority)
                }

                Text(task.title)
                    .font(.headline)
                    .lineLimit(2)

                if let description = task.description, !description.isEmpty {
                    Text(description)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .lineLimit(3)
                }

                HStack {
                    StatusBadge(status: task.status)

                    if !task.tags.isEmpty {
                        TagChips(tags: Array(task.tags.prefix(3)))
                    }

                    Spacer()

                    if let startTime = workspace.executionStartTime {
                        ElapsedTimeView(startTime: startTime)
                            .font(.system(.caption, design: .monospaced))
                            .foregroundStyle(.secondary)
                            .accessibilityLabel("Elapsed time")
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func phaseProgressSection() -> some View {
        glassGroupBox("Phase Progress") {
            VStack(alignment: .leading, spacing: 16) {
                GeometryReader { geo in
                    ZStack(alignment: .leading) {
                        RoundedRectangle(cornerRadius: 6)
                            .fill(.quaternary.opacity(0.3))
                            .frame(height: 12)

                        if let phase = workspace.currentPhase {
                            RoundedRectangle(cornerRadius: 6)
                                .fill(phase.color)
                                .frame(width: geo.size.width * phase.progressFraction, height: 12)
                                .animation(.easeInOut(duration: 0.3), value: phase)
                        }

                        HStack(spacing: 0) {
                            ForEach(Workspace.ExecutionPhase.allCases, id: \.self) { phase in
                                Rectangle()
                                    .fill(.separator.opacity(0.5))
                                    .frame(width: 1, height: 12)
                                    .frame(maxWidth: .infinity, alignment: .trailing)
                            }
                        }
                    }
                }
                .frame(height: 12)
                .accessibilityElement(children: .combine)
                .accessibilityLabel("Phase progress: \(workspace.currentPhase?.displayName ?? "Not started")")

                HStack(spacing: 0) {
                    ForEach(Workspace.ExecutionPhase.allCases, id: \.self) { phase in
                        HStack(spacing: 4) {
                            Image(systemName: phase.icon)
                                .font(.caption)
                            Text(phase.displayName)
                                .font(.caption)
                        }
                        .foregroundStyle(phase == workspace.currentPhase ? phase.color : .secondary)
                        .frame(maxWidth: .infinity)
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func runTargetSection() -> some View {
        glassGroupBox("Up Next") {
            VStack(alignment: .leading, spacing: 12) {
                if let previewTask = workspace.runControlPreviewTask {
                    HStack(alignment: .top, spacing: 10) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(previewTask.id)
                                .font(.system(.caption, design: .monospaced))
                                .foregroundStyle(.secondary)
                            Text(previewTask.title)
                                .font(.subheadline.weight(.semibold))
                                .lineLimit(2)
                        }

                        Spacer()

                        PriorityBadge(priority: previewTask.priority)
                    }
                } else {
                    Text("No todo tasks in this workspace queue.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                HStack(alignment: .firstTextBaseline, spacing: 12) {
                    Picker("Task", selection: $workspace.runControlSelectedTaskID) {
                        Text("Auto (next runnable)")
                            .tag(Optional<String>.none)
                        ForEach(workspace.runControlTodoTasks, id: \.id) { task in
                            Text("\(task.id) · \(task.title)")
                                .lineLimit(1)
                                .tag(Optional(task.id))
                        }
                    }
                    .pickerStyle(.menu)
                    .frame(maxWidth: 420, alignment: .leading)

                    Toggle("Force", isOn: $workspace.runControlForceDirtyRepo)
                        .toggleStyle(.switch)
                        .controlSize(.small)
                        .help("Pass --force to run commands when repo is dirty.")

                    Spacer()

                    Button {
                        Task { @MainActor in
                            await workspace.refreshRunControlData()
                        }
                    } label: {
                        Image(systemName: "arrow.clockwise")
                    }
                    .buttonStyle(.plain)
                    .help("Refresh queue + config")
                }

                if workspace.runControlSelectedTaskID != nil {
                    Text("Loop mode still follows queue order; selected task applies to one-off run.")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    @ViewBuilder
    private func runnerConfigSection() -> some View {
        glassGroupBox("Runner Configuration") {
            VStack(alignment: .leading, spacing: 8) {
                if workspace.runnerConfigLoading {
                    HStack(spacing: 8) {
                        ProgressView()
                            .controlSize(.small)
                        Text("Loading resolved config...")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }

                ConfigRow(icon: "cpu", label: "Model", value: workspace.currentRunnerConfig?.model ?? "Default")
                ConfigRow(icon: "square.split.2x1", label: "Phases", value: workspace.currentRunnerConfig?.phases.map(String.init) ?? "Auto")
                ConfigRow(icon: "number", label: "Max Iterations", value: workspace.currentRunnerConfig?.maxIterations.map(String.init) ?? "Auto")

                if let configError = workspace.runnerConfigErrorMessage {
                    Text(configError)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    @ViewBuilder
    private func executionControlsSection() -> some View {
        glassGroupBox("Controls") {
            VStack(spacing: 12) {
                let previewTask = workspace.runControlPreviewTask
                let hasSelectedTask = workspace.selectedRunControlTask != nil

                HStack(spacing: 12) {
                    if workspace.isRunning {
                        Button(action: { workspace.cancel() }) {
                            Label("Stop", systemImage: "stop.circle.fill")
                                .foregroundStyle(.red)
                        }
                        .buttonStyle(GlassButtonStyle())
                        .accessibilityLabel("Stop execution")
                        .accessibilityHint("Cancel the current task execution")

                        if workspace.isLoopMode {
                            Button(action: { workspace.stopLoop() }) {
                                Label("Stop After Current", systemImage: "pause.circle")
                                    .foregroundStyle(.orange)
                            }
                            .buttonStyle(GlassButtonStyle())
                        }
                    } else {
                        Button(action: {
                            workspace.runNextTask(
                                taskIDOverride: workspace.runControlSelectedTaskID,
                                forceDirtyRepo: workspace.runControlForceDirtyRepo
                            )
                        }) {
                            Label(hasSelectedTask ? "Run Selected Task" : "Run Next Task", systemImage: "play.circle.fill")
                        }
                        .buttonStyle(GlassButtonStyle())
                        .disabled(previewTask == nil)
                        .accessibilityLabel("Run next task")
                        .accessibilityHint("Starts execution of the selected task or next task in the queue")

                        Button(action: { workspace.startLoop(forceDirtyRepo: workspace.runControlForceDirtyRepo) }) {
                            Label("Start Loop", systemImage: "repeat.circle")
                        }
                        .buttonStyle(GlassButtonStyle())
                        .disabled(workspace.nextTask() == nil)
                        .accessibilityLabel("Start task loop")
                        .accessibilityHint("Continuously run tasks until stopped")
                    }

                    Spacer()
                }

                if workspace.isLoopMode {
                    HStack {
                        Image(systemName: "repeat.circle.fill")
                            .foregroundStyle(.blue)
                        Text("Loop Mode Active")
                            .font(.caption)
                            .foregroundStyle(.secondary)

                        if workspace.stopAfterCurrent {
                            Text("(Stopping after current)")
                                .font(.caption)
                                .foregroundStyle(.orange)
                        }

                        Spacer()
                    }
                }

                if let status = workspace.lastExitStatus, !workspace.isRunning {
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

    @ViewBuilder
    private func executionHistorySection() -> some View {
        if !workspace.executionHistory.isEmpty {
            glassGroupBox("Recent History") {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(workspace.executionHistory.prefix(5)) { record in
                        ExecutionHistoryRow(record: record)
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func noExecutionView() -> some View {
        VStack(spacing: 16) {
            Image(systemName: "play.circle")
                .font(.system(size: 48))
                .foregroundStyle(.secondary)

            Text("No Active Execution")
                .font(.headline)

            Text("Run a task to see execution progress and live output.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 300)
        }
        .frame(maxWidth: .infinity, minHeight: 200)
    }

    @ViewBuilder
    private func lastRunSummary() -> some View {
        if let lastRun = workspace.executionHistory.first {
            glassGroupBox("Last Run") {
                HStack {
                    ExecutionStatusIcon(record: lastRun)

                    if let taskID = lastRun.taskID {
                        Text(taskID)
                            .font(.system(.body, design: .monospaced))
                    }

                    Spacer()

                    if let duration = lastRun.duration {
                        Text(formatDuration(duration))
                            .font(.system(.body, design: .monospaced))
                            .foregroundStyle(.secondary)
                    }
                }
            }
        }
    }

    private func glassGroupBox<Content: View>(_ title: String, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.system(.caption, weight: .semibold))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 12)

            content()
                .padding(12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .underPageBackground(cornerRadius: 10, isEmphasized: false)
        }
    }

    private func formatDuration(_ duration: TimeInterval) -> String {
        if duration < 60 {
            return String(format: "%.0fs", duration)
        } else {
            let minutes = Int(duration) / 60
            let seconds = Int(duration) % 60
            return String(format: "%d:%02d", minutes, seconds)
        }
    }
}

// MARK: - Supporting Views

@MainActor
struct TagChips: View {
    let tags: [String]

    var body: some View {
        HStack(spacing: 4) {
            ForEach(tags, id: \.self) { tag in
                Text(tag)
                    .font(.caption2)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(.quaternary.opacity(0.3))
                    .clipShape(RoundedRectangle(cornerRadius: 4))
            }
        }
    }
}

@MainActor
struct ConfigRow: View {
    let icon: String
    let label: String
    let value: String

    var body: some View {
        HStack {
            Image(systemName: icon)
                .foregroundStyle(.secondary)
                .frame(width: 20)
            Text(label)
                .foregroundStyle(.secondary)
            Spacer()
            Text(value)
                .font(.system(.body, design: .monospaced))
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(label): \(value)")
    }
}

@MainActor
struct ExecutionHistoryRow: View {
    let record: Workspace.ExecutionRecord

    var body: some View {
        HStack {
            ExecutionStatusIcon(record: record)

            if let taskID = record.taskID {
                Text(taskID)
                    .font(.system(.caption, design: .monospaced))
            } else {
                Text("Unknown task")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            if let duration = record.duration {
                Text(formatDuration(duration))
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func formatDuration(_ duration: TimeInterval) -> String {
        if duration < 60 {
            return String(format: "%.0fs", duration)
        } else {
            let minutes = Int(duration) / 60
            let seconds = Int(duration) % 60
            return String(format: "%d:%02d", minutes, seconds)
        }
    }
}

@MainActor
struct ExecutionStatusIcon: View {
    let record: Workspace.ExecutionRecord

    var body: some View {
        Image(systemName: iconName)
            .foregroundStyle(iconColor)
    }

    private var iconName: String {
        if record.wasCancelled {
            return "xmark.octagon.fill"
        }
        return record.success ? "checkmark.circle.fill" : "xmark.circle.fill"
    }

    private var iconColor: Color {
        if record.wasCancelled {
            return .orange
        }
        return record.success ? .green : .red
    }
}
