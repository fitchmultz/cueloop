/**
 TaskExecutionOverridesSection

 Responsibilities:
 - Display and edit task-level execution overrides (runner, model, effort, phases, iterations).
 - Support quick presets for common execution configurations.
 - Manage per-phase overrides for multi-phase execution.
 - Provide visual feedback on inherited vs overridden values.

 Does not handle:
 - Task persistence (handled by parent TaskDetailView).
 - Workspace-level runner config (read-only from workspace).

 Invariants/assumptions callers must respect:
 - Draft task is passed via binding for two-way editing.
 - Workspace provides inherited config for display purposes.
 - All mutations go through the provided mutateTaskAgent closure.
 */

import SwiftUI
import RalphCore

@MainActor
struct TaskExecutionOverridesSection: View {
    @Binding var draftTask: RalphTask
    let workspace: Workspace
    let mutateTaskAgent: ((inout RalphTaskAgent) -> Void) -> Void

    private static let runnerOptions = ["codex", "opencode", "gemini", "claude", "cursor", "kimi", "pi"]
    private static let effortOptions = ["low", "medium", "high", "xhigh"]

    var body: some View {
        glassGroupBox("Execution Overrides") {
            VStack(alignment: .leading, spacing: 14) {
                presetsSection()
                summaryCaption()
                mainOverridesSection()
                effortExplanation()
                inheritedConfigCaption()
                phaseOverridesSection()

                HStack {
                    Spacer()
                    Button("Clear Execution Overrides") {
                        draftTask.agent = nil
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    .disabled(draftTask.agent == nil)
                }
            }
        }
    }

    // MARK: - Presets

    @ViewBuilder
    private func presetsSection() -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Quick Presets")
                .font(.caption)
                .foregroundStyle(.secondary)

            ViewThatFits(in: .horizontal) {
                FlowLayout(spacing: 8) {
                    presetButtons
                }
                ScrollView(.horizontal) {
                    HStack(spacing: 8) {
                        presetButtons
                    }
                }
                .scrollIndicators(.hidden)
            }

            if activeExecutionPreset == nil, draftTask.agent != nil {
                Label("Custom override active", systemImage: "slider.horizontal.3")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
    }

    @ViewBuilder
    private var presetButtons: some View {
        ForEach(RalphTaskExecutionPreset.allCases) { preset in
            PresetButton(
                preset: preset,
                isActive: activeExecutionPreset == preset,
                action: { applyExecutionPreset(preset) }
            )
        }
    }

    private var activeExecutionPreset: RalphTaskExecutionPreset? {
        RalphTaskExecutionPreset.matchingPreset(for: draftTask.agent)
    }

    private func applyExecutionPreset(_ preset: RalphTaskExecutionPreset) {
        draftTask.agent = RalphTaskAgent.normalizedOverride(preset.agentOverride)
    }

    // MARK: - Summary Caption

    @ViewBuilder
    private func summaryCaption() -> some View {
        Label(overrideSummaryCaption, systemImage: draftTask.agent == nil ? "arrow.down.circle" : "slider.horizontal.3")
            .font(.caption)
            .foregroundStyle(.secondary)
    }

    private var overrideSummaryCaption: String {
        guard let agent = RalphTaskAgent.normalizedOverride(draftTask.agent) else {
            return "No task override. Runner/model/phases/iterations inherit from config."
        }

        var parts: [String] = []
        if let runner = agent.runner { parts.append("runner \(runner)") }
        if let model = agent.model { parts.append("model \(model)") }
        if let effort = agent.modelEffort { parts.append("effort \(effort)") }
        if let phases = agent.phases { parts.append("phases \(phases)") }
        if let iterations = agent.iterations { parts.append("iterations \(iterations)") }
        if let overrides = agent.phaseOverrides, !overrides.isEmpty {
            let count = [overrides.phase1, overrides.phase2, overrides.phase3].compactMap { $0 }.count
            parts.append("\(count) phase override\(count == 1 ? "" : "s")")
        }
        return parts.isEmpty ? "Task override active" : "Task override: \(parts.joined(separator: ", "))"
    }

    // MARK: - Main Overrides

    @ViewBuilder
    private func mainOverridesSection() -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 16) {
                Picker("Runner", selection: taskRunnerBinding) {
                    Text("Inherit").tag("inherit")
                    ForEach(Self.runnerOptions, id: \.self) { runner in
                        Text(runner).tag(runner)
                    }
                }
                .pickerStyle(.menu)
                .frame(width: 170)

                VStack(alignment: .leading, spacing: 4) {
                    Text("Model")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    TextField("Inherit from config", text: taskModelBinding)
                        .textFieldStyle(.roundedBorder)
                        .frame(minWidth: 220)
                }

                Spacer()
            }

            HStack(spacing: 16) {
                Picker("Reasoning Effort", selection: taskEffortBinding) {
                    Text("Inherit").tag("inherit")
                    ForEach(Self.effortOptions, id: \.self) { effort in
                        Text(effort).tag(effort)
                    }
                }
                .pickerStyle(.menu)
                .frame(width: 170)
                .disabled(taskEffortDisabled)

                Picker("Phases", selection: taskPhasesBinding) {
                    Text("Inherit").tag(0)
                    Text("1").tag(1)
                    Text("2").tag(2)
                    Text("3").tag(3)
                }
                .pickerStyle(.menu)
                .frame(width: 130)

                Picker("Iterations", selection: taskIterationsBinding) {
                    Text("Inherit").tag(0)
                    ForEach(1...10, id: \.self) { iteration in
                        Text(String(iteration)).tag(iteration)
                    }
                }
                .pickerStyle(.menu)
                .frame(width: 130)

                Spacer()
            }
        }
    }

    @ViewBuilder
    private func effortExplanation() -> some View {
        Text(taskEffortDisabled
            ? "Reasoning effort is ignored unless runner is codex. Set runner to codex or inherit."
            : "Reasoning effort is only used when the resolved runner is codex."
        )
            .font(.caption2)
            .foregroundStyle(.secondary)
    }

    private var taskEffortDisabled: Bool {
        guard let runner = normalizedRunnerName(draftTask.agent?.runner) else { return false }
        return runner != "codex"
    }

    @ViewBuilder
    private func inheritedConfigCaption() -> some View {
        if let caption = inheritedConfigCaptionText {
            Text(caption)
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
    }

    private var inheritedConfigCaptionText: String? {
        guard let runnerConfig = workspace.currentRunnerConfig else { return nil }
        let inheritedModel = runnerConfig.model ?? "default"
        let inheritedIterations = runnerConfig.maxIterations.map(String.init) ?? "default"
        let inheritedPhases = runnerConfig.phases.map(String.init) ?? "default"
        return "Current inherited config: model \(inheritedModel), phases \(inheritedPhases), iterations \(inheritedIterations)."
    }

    // MARK: - Phase Overrides

    @ViewBuilder
    private func phaseOverridesSection() -> some View {
        Divider()

        HStack {
            Text("Per-Phase Overrides")
                .font(.caption)
                .foregroundStyle(.secondary)
            Spacer()
            Text("Using \(resolvedPhaseCount) phase\(resolvedPhaseCount == 1 ? "" : "s")")
                .font(.caption2)
                .foregroundStyle(.secondary)
        }

        ForEach(1...resolvedPhaseCount, id: \.self) { phase in
            PhaseOverrideEditor(
                title: phaseTitle(phase),
                phase: phase,
                draftTask: $draftTask,
                mutateTaskAgent: mutateTaskAgent,
                resolvedPhaseCount: resolvedPhaseCount
            )
        }

        if hasIgnoredPhaseOverrides {
            IgnoredOverridesWarning(draftTask: $draftTask, resolvedPhaseCount: resolvedPhaseCount)
        }
    }

    private var resolvedPhaseCount: Int {
        let taskPhases = draftTask.agent?.phases
        let inheritedPhases = workspace.currentRunnerConfig?.phases
        return min(max(taskPhases ?? inheritedPhases ?? 3, 1), 3)
    }

    private func phaseTitle(_ phase: Int) -> String {
        switch phase {
        case 1: return "Phase 1 (Planning)"
        case 2: return "Phase 2 (Implementation)"
        case 3: return "Phase 3 (Review)"
        default: return "Phase \(phase)"
        }
    }

    private var hasIgnoredPhaseOverrides: Bool {
        let overrides = draftTask.agent?.phaseOverrides
        if resolvedPhaseCount < 3, overrides?.phase3 != nil { return true }
        if resolvedPhaseCount < 2, overrides?.phase2 != nil { return true }
        return false
    }

    // MARK: - Bindings

    private var taskRunnerBinding: Binding<String> {
        Binding(
            get: { draftTask.agent?.runner ?? "inherit" },
            set: { value in
                mutateTaskAgent { agent in
                    agent.runner = value == "inherit" ? nil : value
                }
            }
        )
    }

    private var taskModelBinding: Binding<String> {
        Binding(
            get: { draftTask.agent?.model ?? "" },
            set: { value in
                mutateTaskAgent { agent in
                    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
                    agent.model = trimmed.isEmpty ? nil : trimmed
                }
            }
        )
    }

    private var taskEffortBinding: Binding<String> {
        Binding(
            get: { draftTask.agent?.modelEffort ?? "inherit" },
            set: { value in
                mutateTaskAgent { agent in
                    agent.modelEffort = value == "inherit" ? nil : value
                }
            }
        )
    }

    private var taskPhasesBinding: Binding<Int> {
        Binding(
            get: { draftTask.agent?.phases ?? 0 },
            set: { value in
                mutateTaskAgent { agent in
                    agent.phases = value == 0 ? nil : value
                }
            }
        )
    }

    private var taskIterationsBinding: Binding<Int> {
        Binding(
            get: { draftTask.agent?.iterations ?? 0 },
            set: { value in
                mutateTaskAgent { agent in
                    agent.iterations = value == 0 ? nil : value
                }
            }
        )
    }

    private func normalizedRunnerName(_ value: String?) -> String? {
        guard let value else { return nil }
        let normalized = value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return normalized.isEmpty ? nil : normalized
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
        .accessibilityLabel("\(title) section")
    }
}

// MARK: - Preset Button

@MainActor
struct PresetButton: View {
    let preset: RalphTaskExecutionPreset
    let isActive: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 2) {
                Text(preset.displayName)
                    .font(.caption.weight(.semibold))
                Text(preset.description)
                    .font(.caption2)
                    .lineLimit(2)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .foregroundStyle(isActive ? Color.white : Color.primary)
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .frame(minWidth: 160, idealWidth: 180, maxWidth: 220, alignment: .leading)
            .background(
                RoundedRectangle(cornerRadius: 8)
                    .fill(isActive ? Color.accentColor : Color(NSColor.windowBackgroundColor).opacity(0.35))
            )
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(isActive ? Color.accentColor : Color.secondary.opacity(0.25), lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Phase Override Editor

@MainActor
struct PhaseOverrideEditor: View {
    let title: String
    let phase: Int
    @Binding var draftTask: RalphTask
    let mutateTaskAgent: ((inout RalphTaskAgent) -> Void) -> Void
    let resolvedPhaseCount: Int

    private static let runnerOptions = ["codex", "opencode", "gemini", "claude", "cursor", "kimi", "pi"]
    private static let effortOptions = ["low", "medium", "high", "xhigh"]

    var body: some View {
        let effortDisabled = phaseEffortDisabled
        let hasOverride = phaseOverride != nil

        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(title)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Clear") {
                    setPhaseOverride(nil)
                }
                .buttonStyle(.borderless)
                .controlSize(.small)
                .disabled(!hasOverride)
            }

            HStack(spacing: 12) {
                Picker("Runner", selection: phaseRunnerBinding) {
                    Text("Inherit").tag("inherit")
                    ForEach(Self.runnerOptions, id: \.self) { runner in
                        Text(runner).tag(runner)
                    }
                }
                .pickerStyle(.menu)
                .frame(width: 160)

                TextField("Model (inherit if empty)", text: phaseModelBinding)
                    .textFieldStyle(.roundedBorder)

                Picker("Effort", selection: phaseEffortBinding) {
                    Text("Inherit").tag("inherit")
                    ForEach(Self.effortOptions, id: \.self) { effort in
                        Text(effort).tag(effort)
                    }
                }
                .pickerStyle(.menu)
                .frame(width: 140)
                .disabled(effortDisabled)
            }

            if effortDisabled {
                Text("Reasoning effort applies only when the effective runner is codex.")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(8)
        .background(Color(NSColor.windowBackgroundColor).opacity(0.35))
        .cornerRadius(8)
    }

    private var phaseOverride: RalphTaskPhaseOverride? {
        switch phase {
        case 1: return draftTask.agent?.phaseOverrides?.phase1
        case 2: return draftTask.agent?.phaseOverrides?.phase2
        case 3: return draftTask.agent?.phaseOverrides?.phase3
        default: return nil
        }
    }

    private func setPhaseOverride(_ value: RalphTaskPhaseOverride?) {
        mutateTaskAgent { agent in
            var overrides = agent.phaseOverrides ?? RalphTaskPhaseOverrides()
            switch phase {
            case 1: overrides.phase1 = value
            case 2: overrides.phase2 = value
            case 3: overrides.phase3 = value
            default: break
            }
            agent.phaseOverrides = overrides.isEmpty ? nil : overrides
        }
    }

    private var phaseRunnerBinding: Binding<String> {
        Binding(
            get: { phaseOverride?.runner ?? "inherit" },
            set: { value in
                var updated = phaseOverride ?? RalphTaskPhaseOverride()
                updated.runner = value == "inherit" ? nil : value
                setPhaseOverride(updated.isEmpty ? nil : updated)
            }
        )
    }

    private var phaseModelBinding: Binding<String> {
        Binding(
            get: { phaseOverride?.model ?? "" },
            set: { value in
                var updated = phaseOverride ?? RalphTaskPhaseOverride()
                let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
                updated.model = trimmed.isEmpty ? nil : trimmed
                setPhaseOverride(updated.isEmpty ? nil : updated)
            }
        )
    }

    private var phaseEffortBinding: Binding<String> {
        Binding(
            get: { phaseOverride?.reasoningEffort ?? "inherit" },
            set: { value in
                var updated = phaseOverride ?? RalphTaskPhaseOverride()
                updated.reasoningEffort = value == "inherit" ? nil : value
                setPhaseOverride(updated.isEmpty ? nil : updated)
            }
        )
    }

    private var phaseEffortDisabled: Bool {
        let phaseRunner = normalizedRunnerName(phaseOverride?.runner)
        let taskRunner = normalizedRunnerName(draftTask.agent?.runner)
        guard let effectiveRunner = phaseRunner ?? taskRunner else { return false }
        return effectiveRunner != "codex"
    }

    private func normalizedRunnerName(_ value: String?) -> String? {
        guard let value else { return nil }
        let normalized = value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return normalized.isEmpty ? nil : normalized
    }
}

// MARK: - Ignored Overrides Warning

@MainActor
struct IgnoredOverridesWarning: View {
    @Binding var draftTask: RalphTask
    let resolvedPhaseCount: Int

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(.yellow)
            VStack(alignment: .leading, spacing: 4) {
                Text("Some phase overrides are currently ignored.")
                    .font(.caption)
                    .foregroundStyle(.primary)
                Text("Overrides for phases above your selected phase count are not used until you increase phases again.")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            Button("Trim Ignored") {
                trimIgnoredPhaseOverrides()
            }
            .buttonStyle(.bordered)
            .controlSize(.small)
        }
        .padding(8)
        .background(Color(NSColor.windowBackgroundColor).opacity(0.35))
        .clipShape(.rect(cornerRadius: 8))
    }

    private func trimIgnoredPhaseOverrides() {
        guard var agent = draftTask.agent else { return }
        guard var overrides = agent.phaseOverrides else { return }

        if resolvedPhaseCount < 2 {
            overrides.phase2 = nil
        }
        if resolvedPhaseCount < 3 {
            overrides.phase3 = nil
        }
        agent.phaseOverrides = overrides.isEmpty ? nil : overrides
        draftTask.agent = RalphTaskAgent.normalizedOverride(agent)
    }
}
