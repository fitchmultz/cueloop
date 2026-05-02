//! Workspace+ConflictDetection
//!
//! Purpose:
//! - Diff task snapshots to identify added, removed, and changed tasks.
//!
//! Responsibilities:
//! - Diff task snapshots to identify added, removed, and changed tasks.
//! - Detect optimistic-locking conflicts from updated-at timestamps.
//! - Compute field-level conflict details for merge and review flows.
//!
//! Does not handle:
//! - Applying task mutations.
//! - Queue file watching or notifications.
//! - Task presentation or sorting.
//!
//!
//! Usage:
//! - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.
//! Invariants/assumptions callers must respect:
//! - Task IDs are unique within a snapshot.
//! - Timestamp conflict checks are best-effort and require the original timestamp.
//! - Field diffing compares app-visible task fields only.
//!
public import Foundation

public extension Workspace {
    struct TaskConflict: Sendable {
        public let localTask: CueLoopTask
        public let externalTask: CueLoopTask
        public let conflictedFields: [String]

        public init(localTask: CueLoopTask, externalTask: CueLoopTask, conflictedFields: [String]) {
            self.localTask = localTask
            self.externalTask = externalTask
            self.conflictedFields = conflictedFields
        }
    }

    struct TaskChanges: Sendable {
        public let added: [CueLoopTask]
        public let removed: [CueLoopTask]
        public let changed: [CueLoopTask]

        public init(added: [CueLoopTask], removed: [CueLoopTask], changed: [CueLoopTask]) {
            self.added = added
            self.removed = removed
            self.changed = changed
        }

        public var hasChanges: Bool {
            !added.isEmpty || !removed.isEmpty || !changed.isEmpty
        }

        public static func diff(previous: [CueLoopTask], current: [CueLoopTask]) -> Self {
            let previousIDs = Set(previous.map(\.id))
            let currentIDs = Set(current.map(\.id))

            let added = current.filter { !previousIDs.contains($0.id) }
            let removed = previous.filter { !currentIDs.contains($0.id) }
            let previousByID = Dictionary(uniqueKeysWithValues: previous.map { ($0.id, $0) })

            var changed: [CueLoopTask] = []
            changed.reserveCapacity(current.count)

            for task in current {
                guard let previousTask = previousByID[task.id] else { continue }
                if task.status != previousTask.status ||
                    task.title != previousTask.title ||
                    task.priority != previousTask.priority ||
                    task.tags != previousTask.tags ||
                    task.agent != previousTask.agent {
                    changed.append(task)
                }
            }

            return TaskChanges(added: added, removed: removed, changed: changed)
        }
    }

    func detectTaskChanges(previous: [CueLoopTask], current: [CueLoopTask]) -> TaskChanges {
        TaskChanges.diff(previous: previous, current: current)
    }

    func checkForConflict(taskID: String, originalUpdatedAt: Date?) -> CueLoopTask? {
        guard let currentTask = taskState.tasks.first(where: { $0.id == taskID }) else {
            return nil
        }

        guard let originalUpdatedAt else {
            return nil
        }

        if let currentUpdatedAt = currentTask.updatedAt, currentUpdatedAt != originalUpdatedAt {
            return currentTask
        }

        return nil
    }

    func detectConflictedFields(local: CueLoopTask, external: CueLoopTask) -> [String] {
        TaskConflictField.allCases
            .filter { $0.differs(local: local, external: external) }
            .map(\.rawValue)
    }
}

public typealias TaskConflict = Workspace.TaskConflict
public typealias TaskChanges = Workspace.TaskChanges
