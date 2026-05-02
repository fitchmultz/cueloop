/**
 WorkspaceView

 Purpose:
 - Display the CueLoop UI using a modern three-column NavigationSplitView layout.

 Responsibilities:
 - Display the CueLoop UI using a modern three-column NavigationSplitView layout.
 - Left sidebar: Navigation sections (Queue, Quick Actions, Run Control, Advanced Runner, Analytics).
 - Middle column: Content list (delegated to section-specific content views).
 - Right column: Detail/inspector view (delegated to section-specific detail views).
 - Bind to a specific Workspace instance for isolated state management.

 Does not handle:
 - Window-level tab management (see WindowView).
 - Cross-workspace operations.
 - Direct navigation state persistence (see NavigationViewModel).
 - Section-specific UI details (delegated to `WorkspaceView+...` companion files and section views).

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Workspace is injected via @ObservedObject.
 - NavigationViewModel manages sidebar state.
 - View updates when workspace state changes.
 - Scene-scoped route actions are registered while the workspace view is visible.
 */

import CueLoopCore
import SwiftUI

@MainActor
struct WorkspaceView: View {
    private static let isUITestingLaunch = ProcessInfo.processInfo.arguments.contains("--uitesting")

    @ObservedObject var workspace: Workspace
    @StateObject var navigation: NavigationViewModel
    @State var showingCommandPalette: Bool = false
    @State var showingTaskCreation: Bool = false
    @State var showingTaskDecompose: Bool = false
    @State var showingOperationalHealth = false
    @State var taskDecomposeContext = TaskDecomposeView.PresentationContext()
    @State var commandActions = WorkspaceUIActions()
    @State private var navigationIssueSinkTask: Task<Void, Never>?
    @FocusedValue(\.workspaceWindowActions) var workspaceWindowActions
    let manager = WorkspaceManager.shared

    private var showErrorRecoveryBinding: Binding<Bool> {
        Binding(
            get: { workspace.diagnosticsState.showErrorRecovery },
            set: { workspace.diagnosticsState.showErrorRecovery = $0 }
        )
    }

    init(workspace: Workspace) {
        self._workspace = ObservedObject(wrappedValue: workspace)
        self._navigation = StateObject(
            wrappedValue: NavigationViewModel(
                workspaceID: workspace.id
            )
        )
    }

    func navTitle(_ context: String) -> String {
        "\(workspace.projectDisplayName) · \(context)"
    }

    var body: some View {
        splitViewShell
            .frame(minWidth: 1200, minHeight: 640)
            .background(.clear)
            .overlay(alignment: .topLeading) {
                workspaceStateProbeOverlay
            }
            .focusedSceneValue(\.workspaceUIActions, commandActions)
            .sheet(isPresented: showErrorRecoveryBinding) { errorRecoverySheet() }
            .sheet(isPresented: $showingCommandPalette) { commandPaletteSheet() }
            .sheet(isPresented: $showingTaskCreation) {
                TaskCreationView(workspace: workspace)
            }
            .sheet(isPresented: $showingTaskDecompose) {
                TaskDecomposeView(workspace: workspace, context: taskDecomposeContext)
            }
            .sheet(isPresented: $showingOperationalHealth) { operationalHealthSheet() }
            .onAppear {
                bindNavigationPersistenceIssueSink()
                workspace.scheduleInitialRepositoryBootstrapIfNeeded()
                configureCommandActions()
                registerWorkspaceRouteActions()
                refreshContractDiagnostics()
            }
            .onChange(of: workspace.identityState.retargetRevision) { _, _ in
                handleRepositoryRetarget()
                refreshContractDiagnostics()
            }
            .onChange(of: navigation.selectedSection) { _, _ in
                refreshContractDiagnostics()
            }
            .onChange(of: navigation.selectedTaskID) { _, _ in
                refreshContractDiagnostics()
            }
            .onChange(of: navigation.selectedTaskIDs) { _, _ in
                refreshContractDiagnostics()
            }
            .onChange(of: showingTaskCreation) { _, _ in
                refreshContractDiagnostics()
            }
            .onChange(of: showingTaskDecompose) { _, _ in
                refreshContractDiagnostics()
            }
            .onChange(of: taskDecomposeContext.selectedTaskID) { _, _ in
                refreshContractDiagnostics()
            }
            .onChange(of: workspace.taskState.tasks.count) { _, _ in
                refreshContractDiagnostics()
            }
            .onChange(of: workspace.taskState.tasksLoading) { _, _ in
                refreshContractDiagnostics()
            }
            .onChange(of: workspace.taskState.tasksErrorMessage) { _, _ in
                refreshContractDiagnostics()
            }
            .onDisappear {
                navigationIssueSinkTask?.cancel()
                navigationIssueSinkTask = nil
                manager.unregisterWorkspaceRouteActions(for: workspace.id)
                if CueLoopAppDefaults.isWorkspaceRoutingContract {
                    WorkspaceContractPresentationCoordinator.shared.unregister(workspaceID: workspace.id)
                }
            }
    }

    private var splitViewShell: some View {
        NavigationSplitView(columnVisibility: $navigation.sidebarVisibility) {
            sidebarColumn()
                .navigationSplitViewColumnWidth(min: 180, ideal: 200, max: 250)
        } content: {
            contentColumn()
                .navigationSplitViewColumnWidth(min: 280, ideal: 420, max: .infinity)
        } detail: {
            detailColumn()
                .navigationSplitViewColumnWidth(min: 450, ideal: 550, max: .infinity)
        }
    }

    @ViewBuilder
    private var workspaceStateProbeOverlay: some View {
        if Self.isUITestingLaunch {
            WorkspaceStateAccessibilityProbe(workspace: workspace)
        }
    }

    private func bindNavigationPersistenceIssueSink() {
        navigationIssueSinkTask?.cancel()
        navigationIssueSinkTask = Task { @MainActor [navigation, weak workspace] in
            await Task.yield()
            guard !Task.isCancelled else { return }
            navigation.setPersistenceIssueSink { [weak workspace] issue in
                workspace?.updateNavigationPersistenceIssue(issue)
            }
        }
    }
}
