/**
 WorkspaceManager+Routing

 Responsibilities:
 - Bridge scene-scoped route registration to the shared WorkspaceSceneRouter.
 - Reveal workspaces and persist registered window states for unfocused surfaces.

 Does not handle:
 - Window restoration storage.
 - Workspace creation and migration.

 Invariants/assumptions callers must respect:
 - Route actions must be registered before unfocused surfaces attempt to target them.
 - Revealing a workspace prefers the focused scene router when available.
 */

public import Foundation

public extension WorkspaceManager {
    var effectiveWorkspace: Workspace? {
        if let focusedWorkspaceID = focusedWorkspace?.id,
           let focusedWorkspace = workspaces.first(where: { $0.id == focusedWorkspaceID }) {
            return focusedWorkspace
        }

        if let lastActiveWorkspaceID,
           let activeWorkspace = workspaces.first(where: { $0.id == lastActiveWorkspaceID }) {
            return activeWorkspace
        }

        if let lastWorkspace = workspaces.last {
            return lastWorkspace
        }
        return workspaces.first
    }

    func registerWindowRouteActions(for windowID: UUID, actions: WindowRouteActions) {
        sceneRouter.registerWindowRouteActions(for: windowID, actions: actions)
    }

    func unregisterWindowRouteActions(for windowID: UUID) {
        sceneRouter.unregisterWindowRouteActions(for: windowID)
    }

    func registerWorkspaceRouteActions(
        for workspaceID: UUID,
        perform: @escaping (WorkspaceSceneRoute) -> Void
    ) {
        sceneRouter.registerWorkspaceRouteActions(for: workspaceID, perform: perform)
    }

    func unregisterWorkspaceRouteActions(for workspaceID: UUID) {
        sceneRouter.unregisterWorkspaceRouteActions(for: workspaceID)
    }

    func route(_ route: WorkspaceSceneRoute, to workspaceID: UUID) {
        revealWorkspace(workspaceID)
        sceneRouter.route(route, to: workspaceID)
    }

    func markWorkspaceActive(_ workspace: Workspace?) {
        let newWorkspaceID = workspace?.id

        if focusedWorkspace?.id == newWorkspaceID, lastActiveWorkspaceID == newWorkspaceID {
            return
        }

        guard let workspace,
              workspaces.contains(where: { $0.id == workspace.id }) else {
            if focusedWorkspace == nil, lastActiveWorkspaceID == nil {
                return
            }
            focusedWorkspace = nil
            if let lastActiveWorkspaceID,
               !workspaces.contains(where: { $0.id == lastActiveWorkspaceID }) {
                self.lastActiveWorkspaceID = effectiveWorkspace?.id
            }
            return
        }

        focusedWorkspace = workspace
        lastActiveWorkspaceID = workspace.id
    }

    func revealWorkspace(_ workspaceID: UUID) {
        if sceneRouter.focusWorkspace(
            workspaceID,
            focusedWorkspaceID: focusedWorkspace?.id
        ) {
            markWorkspaceActive(workspaces.first(where: { $0.id == workspaceID }))
            return
        }

        if let workspace = workspaces.first(where: { $0.id == workspaceID }) {
            lastActiveWorkspaceID = workspace.id
        }
    }

    func persistRegisteredWindowStates() {
        sceneRouter.persistRegisteredWindowStates()
    }

    func resetSceneRoutingForTests() {
        sceneRouter.resetForTests()
    }
}
