/**
 CoordinatorDiagnosticsPersistenceTests

 Purpose:
 - Verify app-level diagnostics coordinators surface persistence failure/success outcomes deterministically.

 Responsibilities:
 - Assert workspace-routing diagnostics captures publish write-failure outcomes and recover after transient storage failures.
 - Assert settings diagnostics captures persistence write failures and keeps loading/model fields stable when config loading fails.

 Scope:
 - WorkspaceContractPresentationCoordinator and SettingsPresentationCoordinator persistence behavior only.

 Usage:
 - Runs as part of the CueLoopMac unit-test bundle.

 Invariants/Assumptions:
 - Tests run on the main actor because both coordinators are main-actor isolated.
 - Storage failures are injected through ContractDiagnosticsPersistenceStorage closures.
 */

import AppKit
import Foundation
import CueLoopCore
import XCTest

@testable import CueLoopMac

@MainActor
final class CoordinatorDiagnosticsPersistenceTests: XCTestCase {
    private enum ExpectedFailure: Error {
        case writeFailed
    }

    private final class FailFirstWriteRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private var shouldFail = true

        func write(_ data: Data, to url: URL) throws {
            lock.lock()
            defer { lock.unlock() }

            if shouldFail {
                shouldFail = false
                throw ExpectedFailure.writeFailed
            }

            try data.write(to: url, options: .atomic)
        }
    }

    private func ensureAppIsInitialized() {
        _ = NSApplication.shared
    }

    private func makeTemporaryDirectory(prefix: String) throws -> URL {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("\(prefix)-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory
    }

    private func removeItemIfExists(_ url: URL) {
        if FileManager.default.fileExists(atPath: url.path) {
            XCTAssertNoThrow(try FileManager.default.removeItem(at: url))
        }
    }

    private func makeWorkspace(in directory: URL) throws -> Workspace {
        let workspaceURL = directory.appendingPathComponent("workspace", isDirectory: true)
        try FileManager.default.createDirectory(at: workspaceURL, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(
            at: workspaceURL.appendingPathComponent(".cueloop", isDirectory: true),
            withIntermediateDirectories: true
        )
        return Workspace(
            workingDirectoryURL: workspaceURL,
            bootstrapRepositoryStateOnInit: false
        )
    }

    func testWorkspaceCoordinator_capturePublishesWriteFailure() throws {
        ensureAppIsInitialized()

        let rootURL = try makeTemporaryDirectory(prefix: "workspace-diagnostics-coordinator-write-failure")
        defer { removeItemIfExists(rootURL) }

        let diagnosticsURL = rootURL.appendingPathComponent("workspace-diagnostics.json", isDirectory: false)
        let workspace = try makeWorkspace(in: rootURL)
        let navigation = NavigationViewModel(workspaceID: workspace.id)
        let coordinator = WorkspaceContractPresentationCoordinator(
            diagnosticsFileURL: diagnosticsURL,
            persistenceStorage: ContractDiagnosticsPersistenceStorage(
                createDirectory: { _ in },
                writeData: { _, _ in throw ExpectedFailure.writeFailed }
            )
        )

        coordinator.capture(
            workspace: workspace,
            navigation: navigation,
            showingTaskCreation: false,
            showingTaskDecompose: false,
            taskDecomposeContext: .init(selectedTaskID: nil)
        )

        XCTAssertEqual(coordinator.diagnostics.persistence.outcome, .failure)
        XCTAssertEqual(coordinator.diagnostics.persistence.path, diagnosticsURL.path)
        XCTAssertTrue(coordinator.diagnostics.persistence.errorMessage?.contains("writeFailed") == true)
    }

    func testWorkspaceCoordinator_refreshRecoversToSuccessAfterTransientWriteFailure() throws {
        ensureAppIsInitialized()

        let rootURL = try makeTemporaryDirectory(prefix: "workspace-diagnostics-coordinator-recover-success")
        defer { removeItemIfExists(rootURL) }

        let diagnosticsURL = rootURL.appendingPathComponent("workspace-diagnostics.json", isDirectory: false)
        let workspace = try makeWorkspace(in: rootURL)
        let navigation = NavigationViewModel(workspaceID: workspace.id)
        let recorder = FailFirstWriteRecorder()
        let coordinator = WorkspaceContractPresentationCoordinator(
            diagnosticsFileURL: diagnosticsURL,
            persistenceStorage: ContractDiagnosticsPersistenceStorage(
                createDirectory: { url in
                    try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
                },
                writeData: { data, url in
                    try recorder.write(data, to: url)
                }
            )
        )

        coordinator.capture(
            workspace: workspace,
            navigation: navigation,
            showingTaskCreation: false,
            showingTaskDecompose: false,
            taskDecomposeContext: .init(selectedTaskID: nil)
        )
        let firstStatus = coordinator.diagnostics.persistence

        coordinator.refresh()

        XCTAssertEqual(firstStatus.outcome, .failure)
        XCTAssertEqual(coordinator.diagnostics.persistence.outcome, .success)
        XCTAssertEqual(coordinator.diagnostics.persistence.path, diagnosticsURL.path)
        XCTAssertNil(coordinator.diagnostics.persistence.errorMessage)
    }

    func testSettingsCoordinator_preparePublishesWriteFailure() throws {
        ensureAppIsInitialized()

        let rootURL = try makeTemporaryDirectory(prefix: "settings-diagnostics-coordinator-write-failure")
        defer { removeItemIfExists(rootURL) }

        let diagnosticsURL = rootURL.appendingPathComponent("settings-diagnostics.json", isDirectory: false)
        let workspace = try makeWorkspace(in: rootURL)
        let coordinator = SettingsPresentationCoordinator(
            diagnosticsFileURL: diagnosticsURL,
            persistenceStorage: ContractDiagnosticsPersistenceStorage(
                createDirectory: { _ in },
                writeData: { _, _ in throw ExpectedFailure.writeFailed }
            )
        )

        coordinator.prepare(workspace: workspace, source: .commandSurface)

        XCTAssertEqual(coordinator.diagnostics.requestSequence, 1)
        XCTAssertEqual(coordinator.diagnostics.persistence.outcome, .failure)
        XCTAssertEqual(coordinator.diagnostics.persistence.path, diagnosticsURL.path)
        XCTAssertTrue(coordinator.diagnostics.persistence.errorMessage?.contains("writeFailed") == true)
    }

    func testSettingsCoordinator_prepareMalformedConfigClearsRunnerAndModelWithSuccessfulPersistence() throws {
        ensureAppIsInitialized()

        let rootURL = try makeTemporaryDirectory(prefix: "settings-diagnostics-coordinator-malformed-config")
        defer { removeItemIfExists(rootURL) }

        let diagnosticsURL = rootURL.appendingPathComponent("settings-diagnostics.json", isDirectory: false)
        let workspace = try makeWorkspace(in: rootURL)
        let malformedConfigURL = workspace.identityState.workingDirectoryURL
            .appendingPathComponent(".cueloop/config.jsonc", isDirectory: false)
        try "{\n  \"agent\": {\n".write(to: malformedConfigURL, atomically: true, encoding: .utf8)

        let coordinator = SettingsPresentationCoordinator(
            diagnosticsFileURL: diagnosticsURL,
            persistenceStorage: ContractDiagnosticsPersistenceStorage(
                createDirectory: { url in
                    try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
                },
                writeData: { data, url in
                    try data.write(to: url, options: .atomic)
                }
            )
        )

        coordinator.prepare(workspace: workspace, source: .commandSurface)

        XCTAssertEqual(coordinator.diagnostics.requestSequence, 1)
        XCTAssertFalse(coordinator.diagnostics.settingsIsLoading)
        XCTAssertNil(coordinator.diagnostics.settingsRunnerValue)
        XCTAssertNil(coordinator.diagnostics.settingsModelValue)
        XCTAssertEqual(coordinator.diagnostics.persistence.outcome, .success)
        XCTAssertEqual(coordinator.diagnostics.persistence.path, diagnosticsURL.path)
        XCTAssertNil(coordinator.diagnostics.persistence.errorMessage)
    }
}
