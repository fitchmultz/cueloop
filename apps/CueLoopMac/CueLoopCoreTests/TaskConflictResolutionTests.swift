/**
 TaskConflictResolutionTests

 Purpose:
 - Verify conflict field grouping and initial selections for merge UI.

 Responsibilities:
 - Verify conflict field grouping and initial selections for merge UI.
 - Protect merge application logic after moving it out of SwiftUI.
 - Ensure agent override conflicts stay part of the shared model.

 Scope:
 - Limited to the responsibilities listed above.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/Assumptions:
 - Callers keep usage within the documented responsibilities and owning feature contracts.
 */

import Foundation
import XCTest

@testable import CueLoopCore

final class TaskConflictResolutionTests: CueLoopCoreTestCase {
    func testModelBuildsSectionsAndInitialSelectionsFromConflicts() {
        let local = CueLoopTask(
            id: "RQ-1",
            status: .doing,
            title: "Local",
            description: "Shared description",
            priority: .high,
            tags: ["swift"],
            updatedAt: Date(),
            dependsOn: ["RQ-0"]
        )
        let external = CueLoopTask(
            id: "RQ-1",
            status: .todo,
            title: "External",
            description: "Shared description",
            priority: .medium,
            tags: ["rust"],
            updatedAt: Date(),
            dependsOn: ["RQ-2"]
        )

        let model = TaskConflictResolutionModel(localTask: local, externalTask: external)

        XCTAssertEqual(model.continuationHeadline, "Task continuation is blocked on a conflict.")
        XCTAssertEqual(
            model.continuationDetail,
            "Review the changed fields, choose local or external values, then continue by saving the merged task."
        )
        XCTAssertEqual(
            Set(model.sections.map { $0.section }),
            Set<TaskConflictFieldSection>([.basicInformation, .tags, .relationships])
        )
        XCTAssertEqual(model.initialSelections[TaskConflictField.title], TaskConflictMergeChoice.external)
        XCTAssertEqual(model.initialSelections[TaskConflictField.status], TaskConflictMergeChoice.external)
        XCTAssertEqual(model.initialSelections[TaskConflictField.tags], TaskConflictMergeChoice.external)
        XCTAssertEqual(model.initialSelections[TaskConflictField.dependsOn], TaskConflictMergeChoice.external)
        XCTAssertNil(model.initialSelections[TaskConflictField.description])
    }

    func testApplySelectionsUsesExternalAsBaseAndOptsIntoLocalFields() {
        let local = CueLoopTask(
            id: "RQ-2",
            status: .doing,
            title: "Local title",
            priority: .high,
            tags: ["swift"],
            agent: CueLoopTaskAgent(runner: "codex", model: "gpt-5.4"),
            updatedAt: Date()
        )
        let external = CueLoopTask(
            id: "RQ-2",
            status: .todo,
            title: "External title",
            priority: .medium,
            tags: ["rust"],
            agent: CueLoopTaskAgent(runner: "claude", model: "sonnet"),
            updatedAt: Date()
        )

        let merged = TaskConflictResolutionModel.applySelections(
            localTask: local,
            externalTask: external,
            selections: [
                TaskConflictField.title: TaskConflictMergeChoice.local,
                TaskConflictField.agent: TaskConflictMergeChoice.local
            ]
        )

        XCTAssertEqual(merged.title, "Local title")
        XCTAssertEqual(merged.agent?.runner, "codex")
        XCTAssertEqual(merged.status, .todo)
        XCTAssertEqual(merged.priority, .medium)
        XCTAssertEqual(merged.tags, ["rust"])
    }
}
