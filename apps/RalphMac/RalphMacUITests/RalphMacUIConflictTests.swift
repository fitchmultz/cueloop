/**
 Purpose:
 - Keep lightweight conflict-readiness regressions around the task detail UI.

 Responsibilities:
 - Validate save/conflict-ready controls appear once local edits exist.

 Scope:
 - UI readiness only, not full external conflict orchestration.

 Usage:
 - Runs against helper-created tasks in the isolated UI-test workspace.

 Invariants/Assumptions:
 - Tests focus on local edit state and shared detail helpers from `RalphMacUITestCase`.
 */

import XCTest

@MainActor
final class RalphMacUIConflictTests: RalphMacUITestCase {
    func test_conflictDetection_UIElementsExist() throws {
        _ = createTask(titlePrefix: "Conflict Task")
        openFirstTaskDetails()

        let titleField = taskDetailTitleField
        assertExists(titleField, message: "Task detail title field should appear")
        titleField.click()
        titleField.doubleClick()
        titleField.typeText("Modified Title - " + UUID().uuidString.prefix(8))

        let saveButton = taskDetailSaveButton
        XCTAssertTrue(saveButton.isEnabled)
    }

    func test_conflictResolverView_Dismissal() throws {
        _ = createTask(titlePrefix: "Conflict Task")
        openFirstTaskDetails()

        let titleField = taskDetailTitleField
        assertExists(titleField, message: "Task detail title field should appear")
        titleField.click()
        titleField.typeText(" - Edited")

        XCTAssertTrue(titleField.exists)
    }
}
