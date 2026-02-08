/**
 KanbanCardView

 Responsibilities:
 - Display a task as a card in the Kanban board.
 - Show priority, status, tags, and visual indicators.
 - Support drag initiation for moving between columns.
 - Indicate blocked status and dependency relationships.

 Does not handle:
 - Drop operations (handled by parent column).
 - Status changes directly (delegates to parent).

 Invariants/assumptions callers must respect:
 - Task is non-nil and valid.
 - isBlocked is pre-computed by parent.
 */

import SwiftUI
import RalphCore

struct KanbanCardView: View {
    let task: RalphTask
    let isBlocked: Bool
    let isOverdue: Bool
    let hasDependencies: Bool
    let blockedCount: Int

    @State private var isDragging = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Header: Priority dot + Task ID
            HStack {
                Circle()
                    .fill(priorityColor(task.priority))
                    .frame(width: 8, height: 8)

                Spacer()

                // Blocked indicator
                if isBlocked {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .font(.caption2)
                        .foregroundStyle(.red)
                        .help("Blocked by dependencies")
                }

                // Overdue indicator
                if isOverdue {
                    Image(systemName: "clock.badge.exclamationmark.fill")
                        .font(.caption2)
                        .foregroundStyle(.orange)
                        .help("Overdue task")
                }

                // Dependencies indicator
                if hasDependencies {
                    HStack(spacing: 2) {
                        Image(systemName: "link")
                            .font(.caption2)
                        if blockedCount > 0 {
                            Text("\(blockedCount)")
                                .font(.caption2)
                        }
                    }
                    .foregroundStyle(.secondary)
                }

                Text(task.id)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .monospaced()
            }

            // Title
            Text(task.title)
                .font(.system(.body, design: .default))
                .lineLimit(3)
                .foregroundStyle(isBlocked ? .secondary : .primary)

            // Tags (if any)
            if !task.tags.isEmpty {
                FlowLayout(spacing: 4) {
                    ForEach(task.tags.prefix(3), id: \.self) { tag in
                        Text(tag)
                            .font(.caption2)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(.secondary.opacity(0.12))
                            .foregroundStyle(.secondary)
                            .cornerRadius(4)
                    }
                }
            }
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .fill(Color(NSColor.controlBackgroundColor))
                .shadow(color: .black.opacity(0.05), radius: 2, x: 0, y: 1)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .stroke(isBlocked ? Color.red.opacity(0.3) : Color.clear, lineWidth: 1)
        )
        .opacity(isDragging ? 0.5 : 1.0)
        .scaleEffect(isDragging ? 0.95 : 1.0)
        .animation(.easeInOut(duration: 0.15), value: isDragging)
    }

    private func priorityColor(_ priority: RalphTaskPriority) -> Color {
        switch priority {
        case .critical: return .red
        case .high: return .orange
        case .medium: return .yellow
        case .low: return .gray
        }
    }
}

#Preview {
    KanbanCardView(
        task: RalphTask(
            id: "RQ-0001",
            status: .todo,
            title: "Build Kanban board view for task management",
            description: "Create a visual Kanban board",
            priority: .high,
            tags: ["ui", "macos", "swiftui"],
            scope: ["RalphMac"],
            createdAt: Date(),
            updatedAt: Date()
        ),
        isBlocked: false,
        isOverdue: true,
        hasDependencies: true,
        blockedCount: 2
    )
    .frame(width: 260)
    .padding()
}
