/**
 WorkspaceWindowCleanupTests

 Purpose:
 - Verify workspace-window duplicate cleanup keeps one canonical candidate while selecting only exact stacked duplicates.

 Responsibilities:
 - Assert same-frame workspace candidates are selected for cleanup after the first canonical survivor.
 - Assert intentionally offset candidates are not selected as duplicates.

 Scope:
 - MainWindowService duplicate-selection logic only; no headed UI automation or XCUITest driving.

 Usage:
 - Runs as part of the CueLoopMac unit-test bundle.

 Invariants/Assumptions:
 - Tests use pure duplicate candidates, not real NSWindow instances, so the regression is deterministic and desktop-safe.
 */

import AppKit
import XCTest

@testable import CueLoopMac

@MainActor
final class WorkspaceWindowCleanupTests: XCTestCase {
    func testDuplicateWorkspaceWindowIndexesToClose_selectsOnlyExactStackedDuplicates() {
        let sharedFrame = NSRect(x: 120, y: 180, width: 1400, height: 900)
        let survivor = candidate(index: 0, frame: sharedFrame, priority: 0, order: 10)
        let firstDuplicate = candidate(index: 1, frame: sharedFrame, priority: 2, order: 11)
        let secondDuplicate = candidate(index: 2, frame: sharedFrame, priority: 2, order: 12)
        let offsetWindow = candidate(index: 3, frame: sharedFrame.offsetBy(dx: 24, dy: -24), priority: 2, order: 13)

        let indexesToClose = MainWindowService.shared.duplicateWorkspaceWindowIndexesToCloseForTesting([
            firstDuplicate,
            offsetWindow,
            secondDuplicate,
            survivor,
        ])

        XCTAssertFalse(indexesToClose.contains(survivor.index))
        XCTAssertTrue(indexesToClose.contains(firstDuplicate.index))
        XCTAssertTrue(indexesToClose.contains(secondDuplicate.index))
        XCTAssertFalse(indexesToClose.contains(offsetWindow.index))
    }

    func testDuplicateWorkspaceWindowIndexesToClose_doesNotSelectSingleWindowGroups() {
        let first = candidate(index: 0, frame: NSRect(x: 0, y: 0, width: 1200, height: 800), priority: 2, order: 10)
        let second = candidate(index: 1, frame: NSRect(x: 32, y: 32, width: 1200, height: 800), priority: 2, order: 11)

        let indexesToClose = MainWindowService.shared.duplicateWorkspaceWindowIndexesToCloseForTesting([first, second])

        XCTAssertTrue(indexesToClose.isEmpty)
    }

    private func candidate(
        index: Int,
        frame: NSRect,
        priority: Int,
        order: Int,
        title: String = "CueLoopMac"
    ) -> WorkspaceWindowDuplicateCandidate {
        WorkspaceWindowDuplicateCandidate(
            index: index,
            title: title,
            frame: frame,
            priority: priority,
            order: order
        )
    }
}
