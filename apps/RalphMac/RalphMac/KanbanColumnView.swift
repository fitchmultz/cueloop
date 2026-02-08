/**
 KanbanColumnView

 Responsibilities:
 - Display a single Kanban column for a specific status.
 - Show column header with task count badge.
 - Accept dropped tasks and trigger status updates.
 - Display tasks in a scrollable list.

 Does not handle:
 - Actual status change execution (delegates to workspace).
 - Cross-column drag visualization (handled by SwiftUI).

 Invariants/assumptions callers must respect:
 - Status is one of RalphTaskStatus cases.
 - Tasks are pre-filtered by the parent board.
 - onTaskDrop is called with the dragged task ID.
 */

import SwiftUI
import RalphCore

struct KanbanColumnView: View {
    let status: RalphTaskStatus
    let tasks: [RalphTask]
    let isTaskBlocked: (RalphTask) -> Bool
    let isTaskOverdue: (RalphTask) -> Bool
    let onTaskDrop: (String) -> Void
    let onTaskSelect: (String) -> Void
    var highlightedTaskIDs: Set<String> = []

    @State private var isTargeted = false

    var body: some View {
        let column = VStack(spacing: 0) {
            columnHeader
            taskList
        }
        .frame(width: 280)
        .background(Color(NSColor.controlBackgroundColor))
        .cornerRadius(12)
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(Color.gray.opacity(0.3), lineWidth: 1)
        )
        
        return column
            .dropDestination(for: String.self) { items, _ in
                guard let taskID = items.first else { return false }
                onTaskDrop(taskID)
                return true
            } isTargeted: { targeted in
                withAnimation(.easeInOut(duration: 0.15)) {
                    isTargeted = targeted
                }
            }
    }
    
    private var columnHeader: some View {
        HStack {
            Circle()
                .fill(statusColor(status))
                .frame(width: 8, height: 8)

            Text(status.displayName)
                .font(.headline)

            Spacer()

            // Task count badge
            Text("\(tasks.count)")
                .font(.caption.weight(.medium))
                .padding(.horizontal, 8)
                .padding(.vertical, 2)
                .background(Color.gray.opacity(0.15))
                .foregroundStyle(.secondary)
                .cornerRadius(10)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(Color(NSColor.controlBackgroundColor))
        .overlay(
            Rectangle()
                .frame(height: 1)
                .foregroundColor(.gray.opacity(0.3)),
            alignment: .bottom
        )
    }
    
    private var taskList: some View {
        ScrollView {
            LazyVStack(spacing: 8) {
                ForEach(tasks) { task in
                    taskCard(for: task)
                }
            }
            .padding(12)
        }
        .background(
            Color(NSColor.controlBackgroundColor)
                .opacity(0.5)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 0)
                .stroke(isTargeted ? Color.accentColor : Color.clear, lineWidth: 2)
        )
    }
    
    private func taskCard(for task: RalphTask) -> some View {
        KanbanCardView(
            task: task,
            isBlocked: isTaskBlocked(task),
            isOverdue: isTaskOverdue(task),
            hasDependencies: task.dependsOn?.isEmpty == false,
            blockedCount: task.dependsOn?.count ?? 0,
            isHighlighted: highlightedTaskIDs.contains(task.id)
        )
        .contentShape(Rectangle())
        .onTapGesture {
            onTaskSelect(task.id)
        }
        .draggable(task.id) {
            // Drag preview
            Text(task.title)
                .padding(8)
                .background(Color.accentColor)
                .foregroundColor(.white)
                .cornerRadius(8)
        }
    }

    private func statusColor(_ status: RalphTaskStatus) -> Color {
        switch status {
        case .draft: return .gray
        case .todo: return .blue
        case .doing: return .orange
        case .done: return .green
        case .rejected: return .red
        }
    }
}

#Preview {
    KanbanColumnView(
        status: .todo,
        tasks: [
            RalphTask(
                id: "RQ-0001",
                status: .todo,
                title: "Build Kanban board view",
                priority: .high,
                tags: ["ui", "macos"],
                createdAt: Date(),
                updatedAt: Date()
            ),
            RalphTask(
                id: "RQ-0002",
                status: .todo,
                title: "Add drag and drop support",
                priority: .medium,
                tags: ["ux"],
                createdAt: Date(),
                updatedAt: Date()
            )
        ],
        isTaskBlocked: { _ in false },
        isTaskOverdue: { _ in false },
        onTaskDrop: { _ in },
        onTaskSelect: { _ in }
    )
    .frame(height: 400)
    .padding()
}
