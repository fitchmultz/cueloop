/**
 KanbanBoardView

 Responsibilities:
 - Display a horizontal Kanban board with all status columns.
 - Handle drag-and-drop between columns to change task status.
 - Provide "Start Work" button for quick status transitions.
 - Coordinate with Workspace for status updates and task reloads.

 Does not handle:
 - Task editing (delegates to TaskDetailView via navigation).
 - Task creation (handled by parent view).
 - Direct CLI execution (delegates to Workspace).

 Invariants/assumptions callers must respect:
 - Workspace is injected and provides task data.
 - selectedTaskID binding is used for navigation.
 - Status changes are persisted via CLI calls.
 */

import SwiftUI
import RalphCore

struct KanbanBoardView: View {
    @ObservedObject var workspace: Workspace
    @Binding var selectedTaskID: String?

    @State private var isUpdating = false
    @State private var updateError: String?
    @State private var recentlyChangedTaskIDs: Set<String> = []

    var body: some View {
        ScrollView(.horizontal, showsIndicators: true) {
            HStack(spacing: 16) {
                ForEach(RalphTaskStatus.allCases, id: \.self) { status in
                    let statusTasks = tasks(for: status)

                    KanbanColumnView(
                        status: status,
                        tasks: statusTasks,
                        isTaskBlocked: { task in workspace.isTaskBlocked(task) },
                        isTaskOverdue: { task in workspace.isTaskOverdue(task) },
                        onTaskDrop: { taskID in
                            handleTaskDrop(taskID: taskID, to: status)
                        },
                        onTaskSelect: { taskID in
                            selectedTaskID = taskID
                        }
                    )
                }
            }
            .padding(20)
        }
        .background(.clear)
        .overlay {
            if isUpdating {
                ProgressView()
                    .scaleEffect(1.2)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(.ultraThinMaterial)
            }
        }
        .alert("Update Error", isPresented: .constant(updateError != nil)) {
            Button("OK") { updateError = nil }
        } message: {
            Text(updateError ?? "")
        }
        .task {
            await workspace.loadTasks()
        }
        .onReceive(NotificationCenter.default.publisher(for: .queueFilesExternallyChanged)) { notification in
            if let userInfo = notification.userInfo,
               let previousTasks = userInfo["previousTasks"] as? [RalphTask],
               let currentTasks = userInfo["currentTasks"] as? [RalphTask] {
                let changes = workspace.detectTaskChanges(previous: previousTasks, current: currentTasks)
                
                var changedIDs = Set(changes.changed.map { $0.id })
                changedIDs.formUnion(changes.added.map { $0.id })
                
                withAnimation(.easeInOut(duration: 0.3)) {
                    recentlyChangedTaskIDs = changedIDs
                }
                
                DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) {
                    withAnimation(.easeInOut(duration: 0.5)) {
                        recentlyChangedTaskIDs.removeAll()
                    }
                }
            }
        }
    }

    private func tasks(for status: RalphTaskStatus) -> [RalphTask] {
        // Apply same filters as TaskListView
        workspace.filteredAndSortedTasks()
            .filter { $0.status == status }
    }

    private func handleTaskDrop(taskID: String, to status: RalphTaskStatus) {
        // Find the task
        guard let task = workspace.tasks.first(where: { $0.id == taskID }) else { return }

        // Skip if status hasn't changed
        guard task.status != status else { return }

        isUpdating = true

        Task {
            do {
                try await workspace.updateTaskStatus(taskID: taskID, to: status)
                await MainActor.run {
                    isUpdating = false
                }
            } catch {
                await MainActor.run {
                    isUpdating = false
                    updateError = error.localizedDescription
                }
            }
        }
    }
}

#Preview {
    struct PreviewWrapper: View {
        @State private var selectedTaskID: String?

        var body: some View {
            KanbanBoardView(
                workspace: previewWorkspace(),
                selectedTaskID: $selectedTaskID
            )
        }

        func previewWorkspace() -> Workspace {
            let workspace = Workspace(workingDirectoryURL: URL(fileURLWithPath: "/tmp"))
            // Note: In real usage, tasks would be loaded from the CLI
            return workspace
        }
    }

    return PreviewWrapper()
}
