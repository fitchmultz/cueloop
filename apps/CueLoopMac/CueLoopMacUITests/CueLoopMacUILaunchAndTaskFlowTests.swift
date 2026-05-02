/**
 Purpose:
 - Cover primary launch and task-flow regressions in a single workspace window.

 Responsibilities:
 - Validate launch, task creation, editing, view switching, and start-work persistence.

 Scope:
 - Single-window task flows only.

 Usage:
 - Runs against the shared CLI-seeded UI-test workspace.

 Invariants/Assumptions:
 - Tests inherit shared setup, synchronization, and task helpers from `CueLoopMacUITestCase`.
 */

import XCTest

@MainActor
final class CueLoopMacUILaunchAndTaskFlowTests: CueLoopMacUITestCase {
    func test_appLaunches_andShowsMainWindow() throws {
        let window = app.windows.firstMatch
        XCTAssertTrue(window.exists, "Main window should exist")

        let sidebar = app.outlines["Main navigation"]
        assertExists(sidebar, message: "Main navigation sidebar should exist")
        XCTAssertTrue(sidebar.staticTexts["Queue"].exists)
    }

    func test_createNewTask_viaQuickCreate() throws {
        _ = createTask(titlePrefix: "UI Test Task")
        let taskList = requireTaskList()
        XCTAssertTrue(taskList.exists)
    }

    func test_editTaskTitle_andVerifyPersistence() throws {
        _ = createTask(titlePrefix: "UI Test Task")
        openFirstTaskDetails()

        let titleField = taskDetailTitleField
        assertExists(titleField, message: "Task detail title field should appear")

        let newTitle = "Updated Task Title - " + UUID().uuidString.prefix(8)
        titleField.click()
        titleField.doubleClick()
        titleField.typeText(newTitle)

        let saveButton = taskDetailSaveButton
        assertExists(saveButton, message: "Save button should appear")
        assertEventually("Save button should be hittable in the active workspace window") {
            saveButton.isHittable
        }
        saveButton.click()

        assertEventually("Save button should disable again after persistence succeeds") {
            !taskDetailSaveButton.isEnabled
        }
    }

    func test_switchBetweenViewModes() throws {
        assertEventually("Task view mode picker should appear") { taskViewModePicker().exists }

        selectTaskViewMode("Kanban")
        assertExists(currentWorkspaceWindow().scrollViews["Kanban board"], message: "Kanban board should appear")

        selectTaskViewMode("Graph")
        assertExists(currentWorkspaceWindow().scrollViews.firstMatch, message: "Graph scroll view should appear")

        selectTaskViewMode("List")
        XCTAssertTrue(requireTaskList().exists)
    }

    func test_startWorkKeyboardShortcut() throws {
        _ = createTask(titlePrefix: "UI Test Task")
        openFirstTaskDetails()

        currentWorkspaceWindow().typeKey(XCUIKeyboardKey.return, modifierFlags: .command)

        assertEventually("Task status should change to 'Doing' after Cmd+Enter") {
            do {
                return try uiTestWorkspaceTasks().contains(where: { $0.status.lowercased() == "doing" })
            } catch {
                XCTFail("Failed to read UI test workspace tasks: \(error)")
                return false
            }
        }
    }
}
