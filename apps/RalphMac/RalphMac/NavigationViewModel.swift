/**
 NavigationViewModel

 Responsibilities:
 - Manage the selected sidebar section (Queue, Quick Actions, Advanced Runner)
 - Track the selected task ID for the Queue section
 - Track the selected command ID for the Advanced section
 - Control sidebar visibility state (collapsed/expanded)
 - Handle navigation notifications from keyboard shortcuts

 Does not handle:
 - Window-level tab state (see WindowState)
 - Workspace data/content (see Workspace)
 - Direct UI rendering

 Invariants/assumptions callers must respect:
 - Must be created as @StateObject at the view level that needs navigation state
 - Notifications are sent via NotificationCenter for cross-view communication
 */

import SwiftUI
import Combine
import RalphCore

/// Represents the main sidebar navigation sections
enum SidebarSection: String, CaseIterable, Identifiable {
    case queue = "Queue"
    case quickActions = "Quick Actions"
    case advancedRunner = "Advanced Runner"

    var id: String { rawValue }

    var icon: String {
        switch self {
        case .queue: return "list.bullet.rectangle"
        case .quickActions: return "bolt.fill"
        case .advancedRunner: return "terminal.fill"
        }
    }

    var keyboardShortcut: KeyEquivalent {
        switch self {
        case .queue: return "1"
        case .quickActions: return "2"
        case .advancedRunner: return "3"
        }
    }
}

@MainActor
final class NavigationViewModel: ObservableObject {
    // MARK: - Published Properties

    @Published var selectedSection: SidebarSection = .queue
    @Published var selectedTaskID: String?
    @Published var sidebarVisibility: NavigationSplitViewVisibility = .automatic

    // MARK: - Private Properties

    private var cancellables = Set<AnyCancellable>()

    // MARK: - Initialization

    init() {
        setupNotificationHandlers()
    }

    // MARK: - Public Methods

    /// Navigate to a specific sidebar section
    func navigate(to section: SidebarSection) {
        selectedSection = section
    }

    /// Toggle sidebar visibility between automatic and detail-only
    func toggleSidebar() {
        sidebarVisibility = sidebarVisibility == .detailOnly ? .automatic : .detailOnly
    }

    /// Select a task by ID (clears if already selected)
    func selectTask(_ taskID: String?) {
        selectedTaskID = taskID
    }

    /// Clear the current task selection
    func clearTaskSelection() {
        selectedTaskID = nil
    }

    // MARK: - Private Methods

    private func setupNotificationHandlers() {
        // Handle show sidebar section notifications
        NotificationCenter.default.publisher(for: .showSidebarSection)
            .compactMap { $0.object as? SidebarSection }
            .receive(on: DispatchQueue.main)
            .sink { [weak self] section in
                self?.navigate(to: section)
            }
            .store(in: &cancellables)

        // Handle toggle sidebar notifications
        NotificationCenter.default.publisher(for: .toggleSidebar)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                self?.toggleSidebar()
            }
            .store(in: &cancellables)

        // Handle clear task selection when workspace changes
        NotificationCenter.default.publisher(for: .workspaceTasksUpdated)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                // Validate selected task still exists
                // This is handled by the view, but we could add validation here
            }
            .store(in: &cancellables)
    }
}

// MARK: - Notification Names

extension Notification.Name {
    static let showSidebarSection = Notification.Name("showSidebarSection")
    static let toggleSidebar = Notification.Name("toggleSidebar")
    static let workspaceTasksUpdated = Notification.Name("workspaceTasksUpdated")
}
