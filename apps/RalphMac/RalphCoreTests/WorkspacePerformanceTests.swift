/**
 WorkspacePerformanceTests

 Responsibilities:
 - Validate performance characteristics of Workspace methods with large datasets.
 - Ensure detectTaskChanges and isTaskBlocked maintain O(N) time complexity.

 Does not handle:
 - Functional correctness (covered by other tests).
 - Memory pressure testing.

 Invariants/assumptions callers must respect:
 - Tests run on the main actor.
 - Tests use synthetic data; actual task file structure not required.
 */

import Foundation
import XCTest
@testable import RalphCore

@MainActor
final class WorkspacePerformanceTests: XCTestCase {
    
    var workspace: Workspace!
    
    override func setUp() async throws {
        try await super.setUp()
        workspace = Workspace(workingDirectoryURL: URL(fileURLWithPath: "/tmp"))
    }
    
    override func tearDown() async throws {
        workspace = nil
        try await super.tearDown()
    }
    
    // MARK: - Performance Tests
    
    func test_detectTaskChanges_performance_1000Tasks() {
        let previous = generateTasks(count: 1000)
        let current = generateTasks(count: 1000, mutateFrom: previous)
        
        measure {
            _ = workspace.detectTaskChanges(previous: previous, current: current)
        }
    }
    
    func test_isTaskBlocked_performance_500Tasks() {
        // Set up workspace with 500 tasks
        workspace.tasks = generateTasksWithDependencies(count: 500)
        
        let testTask = RalphTask(
            id: "RQ-TEST",
            status: .todo,
            title: "Test Task",
            priority: .high,
            dependsOn: (1...10).map { "RQ-\($0)" }  // Depends on 10 tasks
        )
        
        measure {
            for _ in 0..<100 {
                _ = workspace.isTaskBlocked(testTask)
            }
        }
    }
    
    // MARK: - Helpers
    
    private func generateTasks(count: Int) -> [RalphTask] {
        return (1...count).map { index in
            RalphTask(
                id: String(format: "RQ-%04d", index),
                status: index % 5 == 0 ? .done : .todo,
                title: "Task \(index)",
                description: "Description for task \(index)",
                priority: [.low, .medium, .high, .critical][index % 4],
                tags: ["tag\(index % 5)", "tag\(index % 3)"],
                createdAt: Date().addingTimeInterval(-Double(index * 3600)),
                updatedAt: Date()
            )
        }
    }
    
    private func generateTasks(count: Int, mutateFrom base: [RalphTask]) -> [RalphTask] {
        return base.map { task in
            // Modify ~10% of tasks
            if Int.random(in: 1...10) == 1 {
                return RalphTask(
                    id: task.id,
                    status: task.status == .todo ? .doing : .todo,
                    title: task.title + " (modified)",
                    description: task.description,
                    priority: task.priority,
                    tags: task.tags,
                    scope: task.scope,
                    evidence: task.evidence,
                    plan: task.plan,
                    notes: task.notes,
                    request: task.request,
                    createdAt: task.createdAt,
                    updatedAt: Date(),
                    startedAt: task.startedAt,
                    completedAt: task.completedAt,
                    dependsOn: task.dependsOn,
                    blocks: task.blocks,
                    relatesTo: task.relatesTo,
                    customFields: task.customFields
                )
            }
            return task
        }
    }
    
    private func generateTasksWithDependencies(count: Int) -> [RalphTask] {
        return (1...count).map { index in
            let dependsOn: [String]?
            if index > 10 {
                // Each task depends on up to 3 previous tasks
                dependsOn = (1...min(3, index - 1)).map { "RQ-\(index - $0)" }
            } else {
                dependsOn = nil
            }
            
            return RalphTask(
                id: String(format: "RQ-%04d", index),
                status: index % 3 == 0 ? .done : .todo,
                title: "Task \(index)",
                priority: .medium,
                dependsOn: dependsOn
            )
        }
    }
}
