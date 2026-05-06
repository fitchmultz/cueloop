/**
 CueLoopMacApp+URLRouting

 Purpose:
 - Handle incoming `cueloop://open` URLs and route or create workspaces.

 Responsibilities:
 - Handle incoming `cueloop://open` URLs and route or create workspaces.
 - Reuse bootstrap workspaces when the app launches into a placeholder workspace.

 Does not handle:
 - Command menu wiring.
 - Window bootstrap mechanics.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - `cueloop://open?workspace=...` is the URL form.
 - URL-provided CLI overrides are always rejected.
 */

import AppKit
import Foundation
import CueLoopCore

@MainActor
enum CueLoopURLRouter {
    private static var duplicateWindowCleanupTasks: [Task<Void, Never>] = []

    static func handle(_ url: URL) {
        guard isSupportedScheme(url.scheme) else {
            CueLoopLogger.shared.info("Received URL with unexpected scheme: \(url.scheme ?? "nil")", category: .lifecycle)
            return
        }

        let scheme = url.scheme ?? "cueloop"
        if url.host == "settings" {
            SettingsService.showSettingsWindow(source: .urlScheme)
            CueLoopLogger.shared.info("Opened settings via \(scheme)://settings", category: .lifecycle)
            return
        }

        guard url.host == "open" else {
            CueLoopLogger.shared.info("Received \(scheme):// URL with unexpected host: \(url.host ?? "nil")", category: .lifecycle)
            return
        }

        guard let components = URLComponents(url: url, resolvingAgainstBaseURL: true),
              let queryItems = components.queryItems,
              let workspaceItem = queryItems.first(where: { $0.name == "workspace" }),
              let encodedPath = workspaceItem.value,
              let path = encodedPath.removingPercentEncoding else {
            CueLoopLogger.shared.info("Received \(scheme)://open URL without valid workspace parameter", category: .lifecycle)
            return
        }

        if queryItems.contains(where: { $0.name == "cli" }) {
            CueLoopLogger.shared.error(
                "Ignoring deprecated insecure cli= URL parameter",
                category: .cli
            )
        }

        openWorkspace(at: URL(fileURLWithPath: path, isDirectory: true))
    }

    private static func isSupportedScheme(_ scheme: String?) -> Bool {
        scheme == "cueloop"
    }

    static func openWorkspace(at rawWorkspaceURL: URL) {
        let workspaceURL = Workspace.normalizedWorkingDirectoryURL(rawWorkspaceURL)
        let path = workspaceURL.path

        var isDir: ObjCBool = false
        let exists = FileManager.default.fileExists(atPath: path, isDirectory: &isDir)
        guard exists && isDir.boolValue else {
            CueLoopLogger.shared.error("Workspace path does not exist or is not a directory: \(path)", category: .workspace)
            return
        }

        if let existingWorkspace = WorkspaceManager.shared.workspaces.first(where: { $0.matchesWorkingDirectory(workspaceURL) }) {
            revealWorkspaceAfterEnsuringWindow(existingWorkspace.id)
            CueLoopLogger.shared.info("Activated existing workspace: \(path)", category: .workspace)
            return
        }

        if let bootstrapWorkspace = bootstrapWorkspaceForURLOpen() {
            closeOtherBootstrapPlaceholders(except: bootstrapWorkspace.id)
            bootstrapWorkspace.setWorkingDirectory(workspaceURL)
            revealWorkspaceAfterEnsuringWindow(bootstrapWorkspace.id)
            CueLoopLogger.shared.info("Repurposed bootstrap workspace for URL: \(path)", category: .workspace)
            return
        }

        let workspace = WorkspaceManager.shared.createWorkspace(workingDirectory: workspaceURL)
        revealWorkspaceAfterEnsuringWindow(workspace.id)
        CueLoopLogger.shared.info("Created new workspace from URL: \(path)", category: .workspace)
    }

    static func bootstrapWorkspaceForURLOpen() -> Workspace? {
        let manager = WorkspaceManager.shared
        let placeholderWorkspaces = manager.workspaces.filter(\.isURLRoutingPlaceholderWorkspace)
        guard !placeholderWorkspaces.isEmpty else { return nil }

        if let registeredWorkspaceID = WorkspaceWindowRegistry.shared.preferredActiveWorkspaceID(),
           let registeredWorkspace = placeholderWorkspaces.first(where: { $0.id == registeredWorkspaceID }) {
            return registeredWorkspace
        }

        if let focusedWorkspace = manager.focusedWorkspace,
           placeholderWorkspaces.contains(where: { $0.id == focusedWorkspace.id }) {
            return focusedWorkspace
        }

        if let effectiveWorkspace = manager.effectiveWorkspace,
           placeholderWorkspaces.contains(where: { $0.id == effectiveWorkspace.id }) {
            return effectiveWorkspace
        }

        if let onlyVisiblePlaceholder = placeholderWorkspaces.first(where: { workspace in
            workspace.id == manager.lastActiveWorkspaceID
        }) {
            return onlyVisiblePlaceholder
        }

        guard placeholderWorkspaces.count == 1 else { return nil }
        return placeholderWorkspaces[0]
    }

    private static func closeOtherBootstrapPlaceholders(except workspaceID: UUID) {
        let manager = WorkspaceManager.shared
        let duplicatePlaceholders = manager.workspaces.filter {
            $0.id != workspaceID && $0.isURLRoutingPlaceholderWorkspace
        }
        for workspace in duplicatePlaceholders {
            manager.closeWorkspace(workspace)
        }
    }

    private static func revealWorkspaceAfterEnsuringWindow(_ workspaceID: UUID) {
        MainWindowService.shared.revealOrOpenPrimaryWindow()
        WorkspaceManager.shared.scheduleWorkspaceReveal(workspaceID)
        CueLoopMacPresentationRuntime.activateApplicationIfAllowed()
        scheduleStackedDuplicateWindowCleanup()
    }

    private static func scheduleStackedDuplicateWindowCleanup() {
        duplicateWindowCleanupTasks.forEach { $0.cancel() }
        duplicateWindowCleanupTasks = [250_000_000, 900_000_000].map { delay in
            Task { @MainActor in
                try? await Task.sleep(nanoseconds: UInt64(delay))
                guard !Task.isCancelled else { return }
                MainWindowService.shared.closeStackedDuplicateWorkspaceWindows()
            }
        }
    }
}
