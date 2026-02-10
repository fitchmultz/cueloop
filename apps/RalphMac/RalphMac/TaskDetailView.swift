/**
 TaskDetailView

 Responsibilities:
 - Display a comprehensive form for viewing and editing all task fields.
 - Support inline editing with proper form controls (pickers, text editors, tag editors).
 - Integrate with Workspace to persist changes via CLI.
 - Display as inline detail view within NavigationSplitView (not as sheet).

 Does not handle:
 - Task creation (see task builder workflow).
 - Batch operations on multiple tasks.
 - Navigation or dismissal (handled by parent NavigationSplitView).

 Invariants/assumptions callers must respect:
 - Task is passed in and copied to @State for editing.
 - Changes are only persisted when user explicitly saves.
 - onTaskUpdated callback is called after successful save.
 - View is displayed as detail column in NavigationSplitView.
 */

import SwiftUI
import RalphCore

@MainActor
struct TaskDetailView: View {
    @ObservedObject var workspace: Workspace
    let task: RalphTask
    var onTaskUpdated: ((RalphTask) -> Void)? = nil

    // State for mutable copy of task being edited
    @State private var draftTask: RalphTask
    @State private var isSaving = false
    @State private var saveError: String?
    @State private var showingUnsavedChangesAlert = false
    @State private var saveSuccess = false
    
    // State for conflict detection (optimistic locking)
    @State private var originalUpdatedAt: Date?
    @State private var hasConflict = false
    @State private var conflictedExternalTask: RalphTask?
    @State private var showingConflictAlert = false
    @State private var showingConflictResolver = false

    init(workspace: Workspace, task: RalphTask, onTaskUpdated: ((RalphTask) -> Void)? = nil) {
        self.workspace = workspace
        self.task = task
        self.onTaskUpdated = onTaskUpdated
        self._draftTask = State(initialValue: task)
        self._originalUpdatedAt = State(initialValue: task.updatedAt)
    }

    var body: some View {
        contentView
            .withTaskDetailToolbar(
                hasConflict: hasConflict,
                isSaving: isSaving,
                saveSuccess: saveSuccess,
                hasChanges: hasChanges(),
                onSave: { saveChanges() }
            )
            .withTaskDetailAlerts(
                showingUnsavedChangesAlert: $showingUnsavedChangesAlert,
                showingConflictAlert: $showingConflictAlert,
                showingConflictResolver: $showingConflictResolver,
                saveError: $saveError,
                task: task,
                draftTask: draftTask,
                conflictedExternalTask: conflictedExternalTask,
                onDiscard: { draftTask = task },
                onForceSave: { saveChanges(force: true) },
                onDiscardExternal: { discardLocalChanges() },
                onMerge: { mergedTask in
                    self.draftTask = mergedTask
                    self.hasConflict = false
                    self.showingConflictResolver = false
                }
            )
            .onChange(of: task.id) { _, _ in
                // Task changed, reset draft and conflict state
                draftTask = task
                originalUpdatedAt = task.updatedAt
                hasConflict = false
                conflictedExternalTask = nil
                saveSuccess = false
            }
            .onReceive(NotificationCenter.default.publisher(for: .queueFilesExternallyChanged)) { _ in
                checkForExternalChanges()
            }
    }
    
    private var contentView: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                basicInfoSection()
                statusSection()
                executionOverridesSection()
                tagsSection()
                contentSections()
                relationshipsSection()
                metadataSection()
            }
            .padding(20)
        }
        .background(.clear)
        .navigationTitle(draftTask.title)
        .navigationSubtitle(task.id)
    }

    // MARK: - Sections

    @ViewBuilder
    private func basicInfoSection() -> some View {
        glassGroupBox("Basic Information") {
            VStack(alignment: .leading, spacing: 16) {
                // Title
                VStack(alignment: .leading, spacing: 4) {
                    Text("Title")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    TextField("Task title", text: $draftTask.title)
                        .textFieldStyle(.roundedBorder)
                        .accessibilityLabel("Task title")
                        .accessibilityHint("Enter the task title")
                }

                // Description
                VStack(alignment: .leading, spacing: 4) {
                    Text("Description")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    TextEditor(text: Binding(
                        get: { draftTask.description ?? "" },
                        set: { draftTask.description = $0.isEmpty ? nil : $0 }
                    ))
                    .font(.body)
                    .frame(minHeight: 80, maxHeight: 120)
                    .padding(4)
                    .background(Color(NSColor.textBackgroundColor))
                    .cornerRadius(6)
                    .accessibilityLabel("Task description")
                    .accessibilityHint("Enter a detailed description of the task")
                }
            }
        }
    }

    @ViewBuilder
    private func statusSection() -> some View {
        glassGroupBox("Status & Priority") {
            HStack(spacing: 20) {
                // Status Picker
                VStack(alignment: .leading, spacing: 4) {
                    Text("Status")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Picker("Status", selection: $draftTask.status) {
                        ForEach(RalphTaskStatus.allCases, id: \.self) { status in
                            HStack(spacing: 6) {
                                Circle()
                                    .fill(statusColor(status))
                                    .frame(width: 8, height: 8)
                                    .accessibilityLabel("Status: \(status.displayName)")
                                Text(status.displayName)
                            }
                            .tag(status)
                        }
                    }
                    .pickerStyle(.menu)
                    .frame(width: 140)
                    .accessibilityLabel("Task status")
                }

                // Priority Picker
                VStack(alignment: .leading, spacing: 4) {
                    Text("Priority")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Picker("Priority", selection: $draftTask.priority) {
                        ForEach(RalphTaskPriority.allCases, id: \.self) { priority in
                            HStack(spacing: 6) {
                                Circle()
                                    .fill(priorityColor(priority))
                                    .frame(width: 8, height: 8)
                                    .accessibilityLabel("Priority: \(priority.displayName)")
                                Text(priority.displayName)
                            }
                            .tag(priority)
                        }
                    }
                    .pickerStyle(.menu)
                    .frame(width: 140)
                    .accessibilityLabel("Task priority")
                }

                Spacer()
            }
        }
    }

    @ViewBuilder
    private func tagsSection() -> some View {
        glassGroupBox("Tags") {
            TagEditorView(tags: $draftTask.tags)
        }
    }

    private static let runnerOptions = ["codex", "opencode", "gemini", "claude", "cursor", "kimi", "pi"]
    private static let effortOptions = ["low", "medium", "high", "xhigh"]

    @ViewBuilder
    private func executionOverridesSection() -> some View {
        glassGroupBox("Execution Overrides") {
            VStack(alignment: .leading, spacing: 14) {
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

                Text(taskEffortDisabled
                    ? "Reasoning effort is ignored unless runner is codex. Set runner to codex or inherit."
                    : "Reasoning effort is only used when the resolved runner is codex."
                )
                    .font(.caption2)
                    .foregroundStyle(.secondary)

                Divider()

                Text("Per-Phase Overrides")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                phaseOverrideEditor(title: "Phase 1 (Planning)", phase: 1)
                phaseOverrideEditor(title: "Phase 2 (Implementation)", phase: 2)
                phaseOverrideEditor(title: "Phase 3 (Review)", phase: 3)

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

    @ViewBuilder
    private func contentSections() -> some View {
        // Scope
        if draftTask.scope != nil || isEditingNewArrayField("scope") {
            glassGroupBox("Scope") {
                StringArrayEditor(
                    items: Binding(
                        get: { draftTask.scope ?? [] },
                        set: { draftTask.scope = $0.isEmpty ? nil : $0 }
                    ),
                    placeholder: "Add file path..."
                )
            }
        }

        // Evidence
        if draftTask.evidence != nil || isEditingNewArrayField("evidence") {
            glassGroupBox("Evidence") {
                StringArrayEditor(
                    items: Binding(
                        get: { draftTask.evidence ?? [] },
                        set: { draftTask.evidence = $0.isEmpty ? nil : $0 }
                    ),
                    placeholder: "Add evidence item..."
                )
            }
        }

        // Plan
        if draftTask.plan != nil || isEditingNewArrayField("plan") {
            glassGroupBox("Plan") {
                StringArrayEditor(
                    items: Binding(
                        get: { draftTask.plan ?? [] },
                        set: { draftTask.plan = $0.isEmpty ? nil : $0 }
                    ),
                    placeholder: "Add plan step..."
                )
            }
        }

        // Notes
        if draftTask.notes != nil || isEditingNewArrayField("notes") {
            glassGroupBox("Notes") {
                StringArrayEditor(
                    items: Binding(
                        get: { draftTask.notes ?? [] },
                        set: { draftTask.notes = $0.isEmpty ? nil : $0 }
                    ),
                    placeholder: "Add note..."
                )
            }
        }

        // Add Field Buttons
        glassGroupBox("Add Fields") {
            FlowLayout(spacing: 8) {
                if draftTask.scope == nil {
                    addFieldButton("+ Scope", action: { draftTask.scope = [] })
                }
                if draftTask.evidence == nil {
                    addFieldButton("+ Evidence", action: { draftTask.evidence = [] })
                }
                if draftTask.plan == nil {
                    addFieldButton("+ Plan", action: { draftTask.plan = [] })
                }
                if draftTask.notes == nil {
                    addFieldButton("+ Notes", action: { draftTask.notes = [] })
                }
            }
        }
    }

    @ViewBuilder
    private func relationshipsSection() -> some View {
        let allTaskIDs = workspace.tasks.map { $0.id }.filter { $0 != task.id }
        let existingEdges = buildExistingEdges()

        glassGroupBox("Relationships") {
            VStack(alignment: .leading, spacing: 16) {
                // Depends On
                if draftTask.dependsOn != nil || isEditingNewArrayField("dependsOn") {
                    TaskRelationshipPicker(
                        label: "Depends On",
                        relatedTaskIDs: Binding(
                            get: { draftTask.dependsOn ?? [] },
                            set: { draftTask.dependsOn = $0.isEmpty ? nil : $0 }
                        ),
                        allTaskIDs: allTaskIDs,
                        currentTaskID: task.id,
                        edgeType: .dependency,
                        existingEdges: existingEdges
                    )
                }

                // Blocks
                if draftTask.blocks != nil || isEditingNewArrayField("blocks") {
                    TaskRelationshipPicker(
                        label: "Blocks",
                        relatedTaskIDs: Binding(
                            get: { draftTask.blocks ?? [] },
                            set: { draftTask.blocks = $0.isEmpty ? nil : $0 }
                        ),
                        allTaskIDs: allTaskIDs,
                        currentTaskID: task.id,
                        edgeType: .blocks,
                        existingEdges: existingEdges
                    )
                }

                // Relates To
                if draftTask.relatesTo != nil || isEditingNewArrayField("relatesTo") {
                    TaskRelationshipPicker(
                        label: "Relates To",
                        relatedTaskIDs: Binding(
                            get: { draftTask.relatesTo ?? [] },
                            set: { draftTask.relatesTo = $0.isEmpty ? nil : $0 }
                        ),
                        allTaskIDs: allTaskIDs,
                        currentTaskID: task.id,
                        edgeType: .relatesTo,
                        existingEdges: existingEdges
                    )
                }

                // Add Relationship Buttons
                if draftTask.dependsOn == nil || draftTask.blocks == nil || draftTask.relatesTo == nil {
                    FlowLayout(spacing: 8) {
                        if draftTask.dependsOn == nil {
                            addFieldButton("+ Depends On", action: { draftTask.dependsOn = [] })
                        }
                        if draftTask.blocks == nil {
                            addFieldButton("+ Blocks", action: { draftTask.blocks = [] })
                        }
                        if draftTask.relatesTo == nil {
                            addFieldButton("+ Relates To", action: { draftTask.relatesTo = [] })
                        }
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func metadataSection() -> some View {
        glassGroupBox("Metadata") {
            VStack(alignment: .leading, spacing: 8) {
                metadataRow(label: "Created", date: draftTask.createdAt)
                metadataRow(label: "Updated", date: draftTask.updatedAt)
                metadataRow(label: "Started", date: draftTask.startedAt)
                metadataRow(label: "Completed", date: draftTask.completedAt)
            }
        }
    }

    @ViewBuilder
    private func metadataRow(label: String, date: Date?) -> some View {
        HStack {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(width: 70, alignment: .leading)

            if let date = date {
                Text(formatDate(date))
                    .font(.caption)
                    .foregroundStyle(.primary)
            } else {
                Text("—")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer()
        }
        .accessibilityLabel("\(label): \(date.map(formatDateForAccessibility) ?? "Not set")")
    }

    @ViewBuilder
    private func addFieldButton(_ title: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(.caption)
                .padding(.horizontal, 10)
                .padding(.vertical, 4)
        }
        .buttonStyle(GlassButtonStyle())
        .accessibilityLabel("Add \(title) field")
    }

    @ViewBuilder
    private func phaseOverrideEditor(title: String, phase: Int) -> some View {
        let effortDisabled = phaseEffortDisabled(phase: phase)

        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(title)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Clear") {
                    setPhaseOverride(nil, phase: phase)
                }
                .buttonStyle(.borderless)
                .controlSize(.small)
                .disabled(phaseOverride(for: phase) == nil)
            }

            HStack(spacing: 12) {
                Picker("Runner", selection: phaseRunnerBinding(phase: phase)) {
                    Text("Inherit").tag("inherit")
                    ForEach(Self.runnerOptions, id: \.self) { runner in
                        Text(runner).tag(runner)
                    }
                }
                .pickerStyle(.menu)
                .frame(width: 160)

                TextField("Model (inherit if empty)", text: phaseModelBinding(phase: phase))
                    .textFieldStyle(.roundedBorder)

                Picker("Effort", selection: phaseEffortBinding(phase: phase)) {
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

    private var taskEffortDisabled: Bool {
        guard let runner = normalizedRunnerName(draftTask.agent?.runner) else { return false }
        return runner != "codex"
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

    private func phaseOverride(for phase: Int) -> RalphTaskPhaseOverride? {
        switch phase {
        case 1: return draftTask.agent?.phaseOverrides?.phase1
        case 2: return draftTask.agent?.phaseOverrides?.phase2
        case 3: return draftTask.agent?.phaseOverrides?.phase3
        default: return nil
        }
    }

    private func setPhaseOverride(_ value: RalphTaskPhaseOverride?, phase: Int) {
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

    private func phaseRunnerBinding(phase: Int) -> Binding<String> {
        Binding(
            get: { phaseOverride(for: phase)?.runner ?? "inherit" },
            set: { value in
                var updated = phaseOverride(for: phase) ?? RalphTaskPhaseOverride()
                updated.runner = value == "inherit" ? nil : value
                setPhaseOverride(updated.isEmpty ? nil : updated, phase: phase)
            }
        )
    }

    private func phaseModelBinding(phase: Int) -> Binding<String> {
        Binding(
            get: { phaseOverride(for: phase)?.model ?? "" },
            set: { value in
                var updated = phaseOverride(for: phase) ?? RalphTaskPhaseOverride()
                let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
                updated.model = trimmed.isEmpty ? nil : trimmed
                setPhaseOverride(updated.isEmpty ? nil : updated, phase: phase)
            }
        )
    }

    private func phaseEffortBinding(phase: Int) -> Binding<String> {
        Binding(
            get: { phaseOverride(for: phase)?.reasoningEffort ?? "inherit" },
            set: { value in
                var updated = phaseOverride(for: phase) ?? RalphTaskPhaseOverride()
                updated.reasoningEffort = value == "inherit" ? nil : value
                setPhaseOverride(updated.isEmpty ? nil : updated, phase: phase)
            }
        )
    }

    private func phaseEffortDisabled(phase: Int) -> Bool {
        let phaseRunner = normalizedRunnerName(phaseOverride(for: phase)?.runner)
        let taskRunner = normalizedRunnerName(draftTask.agent?.runner)
        guard let effectiveRunner = phaseRunner ?? taskRunner else { return false }
        return effectiveRunner != "codex"
    }

    private func normalizedRunnerName(_ value: String?) -> String? {
        guard let value else { return nil }
        let normalized = value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return normalized.isEmpty ? nil : normalized
    }

    private func mutateTaskAgent(_ mutate: (inout RalphTaskAgent) -> Void) {
        var agent = draftTask.agent ?? RalphTaskAgent()
        mutate(&agent)
        if let effort = agent.modelEffort?.trimmingCharacters(in: .whitespacesAndNewlines),
           effort.lowercased() == "default" {
            agent.modelEffort = nil
        }
        if let phases = agent.phases, !(1...3).contains(phases) {
            agent.phases = nil
        }
        if let iterations = agent.iterations, iterations < 1 {
            agent.iterations = nil
        }
        if let overrides = agent.phaseOverrides, overrides.isEmpty {
            agent.phaseOverrides = nil
        }
        draftTask.agent = agent.isEmpty ? nil : agent
    }

    // MARK: - Helper Methods

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

    private func hasChanges() -> Bool {
        draftTask != task
    }

    private func saveChanges(force: Bool = false) {
        // Check for conflict before saving (unless force)
        if !force && hasConflict {
            showingConflictAlert = true
            return
        }
        
        isSaving = true
        saveError = nil
        saveSuccess = false

        Task { @MainActor in
            do {
                // Pass originalUpdatedAt for optimistic locking check
                try await workspace.updateTask(
                    from: task,
                    to: draftTask,
                    originalUpdatedAt: force ? nil : originalUpdatedAt
                )
                isSaving = false
                saveSuccess = true
                hasConflict = false
                onTaskUpdated?(draftTask)
                
                // Update original timestamp after successful save
                originalUpdatedAt = draftTask.updatedAt
                
                // Clear success indicator after 2 seconds
                DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
                    saveSuccess = false
                }
            } catch let error as Workspace.WorkspaceError {
                isSaving = false
                if case .taskConflict(let currentTask) = error {
                    hasConflict = true
                    conflictedExternalTask = currentTask
                    showingConflictAlert = true
                } else {
                    saveError = error.localizedDescription
                }
            } catch {
                isSaving = false
                saveError = error.localizedDescription
            }
        }
    }
    
    // MARK: - Conflict Detection
    
    private func checkForExternalChanges() {
        // If no local changes, silently update the draft to match external changes
        guard hasChanges() else {
            if let currentTask = workspace.tasks.first(where: { $0.id == task.id }) {
                draftTask = currentTask
                originalUpdatedAt = currentTask.updatedAt
                hasConflict = false
            }
            return
        }
        
        // Check for conflict using optimistic locking
        if let externalTask = workspace.checkForConflict(
            taskID: task.id,
            originalUpdatedAt: originalUpdatedAt
        ) {
            hasConflict = true
            conflictedExternalTask = externalTask
            showingConflictAlert = true
        }
    }
    
    private func discardLocalChanges() {
        if let externalTask = conflictedExternalTask {
            draftTask = externalTask
            originalUpdatedAt = externalTask.updatedAt
            hasConflict = false
            conflictedExternalTask = nil
        }
    }

    private func statusColor(_ status: RalphTaskStatus) -> Color {
        switch status {
        case .draft:
            return .gray
        case .todo:
            return .blue
        case .doing:
            return .orange
        case .done:
            return .green
        case .rejected:
            return .red
        }
    }

    private func priorityColor(_ priority: RalphTaskPriority) -> Color {
        switch priority {
        case .critical:
            return .red
        case .high:
            return .orange
        case .medium:
            return .yellow
        case .low:
            return .gray
        }
    }

    private func formatDate(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }

    private func formatDateForAccessibility(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .long
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }

    private func isEditingNewArrayField(_ field: String) -> Bool {
        // Used to check if we're currently editing a field that was just added
        // This helps with conditional display of optional array fields
        false
    }
    
    /// Builds the complete set of edges from all tasks in the workspace
    /// Used for cycle detection in TaskRelationshipPicker
    private func buildExistingEdges() -> [GraphEdge] {
        var edges: [GraphEdge] = []
        
        for task in workspace.tasks {
            // Depends on relationships (current task depends on others)
            for depId in task.dependsOn ?? [] {
                edges.append(GraphEdge(from: task.id, to: depId, type: .dependency))
            }
            
            // Blocks relationships (current task blocks others)
            for blockedId in task.blocks ?? [] {
                edges.append(GraphEdge(from: task.id, to: blockedId, type: .blocks))
            }
            
            // Relates to relationships (bidirectional)
            for relatedId in task.relatesTo ?? [] where task.id < relatedId {
                edges.append(GraphEdge(from: task.id, to: relatedId, type: .relatesTo))
            }
        }
        
        return edges
    }
}

// MARK: - View Modifiers

private struct TaskDetailToolbarModifier: ViewModifier {
    let hasConflict: Bool
    let isSaving: Bool
    let saveSuccess: Bool
    let hasChanges: Bool
    let onSave: () -> Void
    
    func body(content: Content) -> some View {
        content
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    HStack(spacing: 8) {
                        if hasConflict {
                            Image(systemName: "exclamationmark.triangle.fill")
                                .foregroundStyle(.orange)
                                .help("Task modified externally - save may overwrite changes")
                                .accessibilityLabel("External modification warning")
                        }
                        
                        if isSaving {
                            ProgressView()
                                .scaleEffect(0.8)
                                .controlSize(.small)
                        } else if saveSuccess {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundStyle(.green)
                                .transition(.opacity)
                        }

                        Button("Save", action: onSave)
                            .disabled(!hasChanges || isSaving)
                            .keyboardShortcut("s", modifiers: .command)
                            .accessibilityLabel("Save changes")
                            .accessibilityHint("Save all changes to this task")
                    }
                }

                ToolbarItem(placement: .cancellationAction) {
                    Button("Reset") {
                        // Will be handled by alert
                    }
                    .disabled(!hasChanges)
                    .accessibilityLabel("Reset changes")
                    .accessibilityHint("Discard all changes and revert to saved version")
                }
            }
    }
}

private struct TaskDetailAlertsModifier: ViewModifier {
    @Binding var showingUnsavedChangesAlert: Bool
    @Binding var showingConflictAlert: Bool
    @Binding var showingConflictResolver: Bool
    @Binding var saveError: String?
    let task: RalphTask
    let draftTask: RalphTask
    let conflictedExternalTask: RalphTask?
    let onDiscard: () -> Void
    let onForceSave: () -> Void
    let onDiscardExternal: () -> Void
    let onMerge: (RalphTask) -> Void
    
    func body(content: Content) -> some View {
        content
            .alert("Discard Changes?", isPresented: $showingUnsavedChangesAlert) {
                Button("Discard", role: .destructive, action: onDiscard)
                Button("Keep Editing", role: .cancel) {}
            } message: {
                Text("You have unsaved changes. Are you sure you want to discard them and reset to the saved version?")
            }
            .alert("Save Error", isPresented: .constant(saveError != nil)) {
                Button("OK") { saveError = nil }
            } message: {
                Text(saveError ?? "")
            }
            .alert("External Changes Detected", isPresented: $showingConflictAlert) {
                Button("Overwrite External Changes", role: .destructive, action: onForceSave)
                Button("Discard My Changes", action: onDiscardExternal)
                Button("Resolve Conflicts...") { showingConflictResolver = true }
                Button("Cancel", role: .cancel) {}
            } message: {
                Text("This task has been modified externally (via CLI or another window). Your changes conflict with the external changes.\n\nWhat would you like to do?")
            }
            .sheet(isPresented: $showingConflictResolver) {
                if let externalTask = conflictedExternalTask {
                    TaskConflictResolverView(
                        localTask: draftTask,
                        externalTask: externalTask,
                        onMerge: onMerge,
                        onCancel: { showingConflictResolver = false }
                    )
                }
            }
    }
}

extension View {
    func withTaskDetailToolbar(
        hasConflict: Bool,
        isSaving: Bool,
        saveSuccess: Bool,
        hasChanges: Bool,
        onSave: @escaping () -> Void
    ) -> some View {
        modifier(TaskDetailToolbarModifier(
            hasConflict: hasConflict,
            isSaving: isSaving,
            saveSuccess: saveSuccess,
            hasChanges: hasChanges,
            onSave: onSave
        ))
    }
    
    func withTaskDetailAlerts(
        showingUnsavedChangesAlert: Binding<Bool>,
        showingConflictAlert: Binding<Bool>,
        showingConflictResolver: Binding<Bool>,
        saveError: Binding<String?>,
        task: RalphTask,
        draftTask: RalphTask,
        conflictedExternalTask: RalphTask?,
        onDiscard: @escaping () -> Void,
        onForceSave: @escaping () -> Void,
        onDiscardExternal: @escaping () -> Void,
        onMerge: @escaping (RalphTask) -> Void
    ) -> some View {
        modifier(TaskDetailAlertsModifier(
            showingUnsavedChangesAlert: showingUnsavedChangesAlert,
            showingConflictAlert: showingConflictAlert,
            showingConflictResolver: showingConflictResolver,
            saveError: saveError,
            task: task,
            draftTask: draftTask,
            conflictedExternalTask: conflictedExternalTask,
            onDiscard: onDiscard,
            onForceSave: onForceSave,
            onDiscardExternal: onDiscardExternal,
            onMerge: onMerge
        ))
    }
}

// Preview
#Preview {
    TaskDetailView(
        workspace: Workspace(workingDirectoryURL: URL(fileURLWithPath: "/tmp")),
        task: RalphTask(
            id: "RQ-0001",
            status: .todo,
            title: "Sample Task",
            description: "This is a sample task description.",
            priority: .high,
            tags: ["swift", "ui"],
            scope: ["apps/RalphMac/TaskDetailView.swift"],
            createdAt: Date(),
            updatedAt: Date()
        )
    )
}
