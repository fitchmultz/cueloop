/**
 MenuBarManager

 Responsibilities:
 - Manage the menu bar extra state and preferences.
 - Provide shared state for menu bar visibility toggle.
 - Coordinate between menu bar UI and workspace state.

 Does not handle:
 - Direct menu rendering (see MenuBarContentView).
 - Window management (delegates to App via notifications).

 Invariants/assumptions callers must respect:
 - Must be accessed from MainActor only.
 - Uses UserDefaults for persistence of user preferences.
 */

public import Foundation
public import Combine
import SwiftUI
import OSLog

/// Manages the menu bar extra state and user preferences.
@MainActor
public final class MenuBarManager: ObservableObject {
    public static let shared = MenuBarManager()
    
    private var _isMenuBarExtraVisible: Bool

    /// Whether the menu bar extra is visible (persisted to UserDefaults)
    public var isMenuBarExtraVisible: Bool {
        get { _isMenuBarExtraVisible }
        set { setMenuBarExtraVisibility(newValue) }
    }
    
    /// Whether to show task status notifications (persisted to UserDefaults)
    @Published public var showStatusNotifications: Bool {
        didSet {
            UserDefaults.standard.set(showStatusNotifications, forKey: Self.notificationsKey)
        }
    }
    
    /// Whether to show recent tasks in the menu bar (persisted to UserDefaults)
    @Published public var showRecentTasks: Bool {
        didSet {
            UserDefaults.standard.set(showRecentTasks, forKey: Self.recentTasksKey)
        }
    }
    
    /// Maximum number of recent tasks to show (persisted to UserDefaults)
    @Published public var maxRecentTasks: Int {
        didSet {
            UserDefaults.standard.set(maxRecentTasks, forKey: Self.maxRecentTasksKey)
        }
    }
    
    // MARK: - UserDefaults Keys
    
    private static let visibilityKey = "com.mitchfultz.ralph.menuBarExtraVisible"
    private static let notificationsKey = "com.mitchfultz.ralph.menuBarNotifications"
    private static let recentTasksKey = "com.mitchfultz.ralph.menuBarShowRecentTasks"
    private static let maxRecentTasksKey = "com.mitchfultz.ralph.menuBarMaxRecentTasks"
    
    // MARK: - Initialization
    
    private init() {
        // Initialize from UserDefaults with sensible defaults
        self._isMenuBarExtraVisible = UserDefaults.standard.object(forKey: Self.visibilityKey) as? Bool ?? true
        self.showStatusNotifications = UserDefaults.standard.object(forKey: Self.notificationsKey) as? Bool ?? false
        self.showRecentTasks = UserDefaults.standard.object(forKey: Self.recentTasksKey) as? Bool ?? true
        self.maxRecentTasks = UserDefaults.standard.object(forKey: Self.maxRecentTasksKey) as? Int ?? 5
        
        RalphLogger.shared.debug("MenuBarManager initialized", category: .lifecycle)
    }
    
    // MARK: - Public Methods
    
    /// Reset all menu bar preferences to defaults
    public func resetToDefaults() {
        setMenuBarExtraVisibility(true)
        showStatusNotifications = false
        showRecentTasks = true
        maxRecentTasks = 5
        
        RalphLogger.shared.info("Menu bar preferences reset to defaults", category: .lifecycle)
    }
    
    /// Toggle menu bar extra visibility
    public func toggleVisibility() {
        setMenuBarExtraVisibility(!_isMenuBarExtraVisible)
    }

    /// Update menu bar visibility while suppressing no-op publishes.
    public func setMenuBarExtraVisibility(_ newValue: Bool) {
        guard _isMenuBarExtraVisible != newValue else { return }
        objectWillChange.send()
        _isMenuBarExtraVisible = newValue
        UserDefaults.standard.set(newValue, forKey: Self.visibilityKey)
        RalphLogger.shared.debug("Menu bar extra visibility changed to: \(newValue)", category: .lifecycle)
    }
}
