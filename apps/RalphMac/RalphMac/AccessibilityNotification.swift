/**
 AccessibilityNotification

 Responsibilities:
 - Provide centralized accessibility announcements for VoiceOver.
 - Announce state changes, actions, and status updates.
 - Ensure users with visual impairments receive timely feedback.

 Does not handle:
 - View-specific accessibility modifiers (handled in individual views).
 - Focus management (use @AccessibilityFocusState in views).

 Invariants/assumptions callers must respect:
 - Call on main thread for UI-related announcements.
 - Provide concise, actionable messages (avoid verbosity).
 */

import SwiftUI

enum AccessibilityNotification {
    /// Posts an accessibility announcement for VoiceOver users.
    /// - Parameter message: The message to announce.
    static func announce(_ message: String) {
        // Use NSAccessibility post notification for macOS
        // Post to the main application element since mainWindow may be nil
        let appElement = NSApp as Any
        NSAccessibility.post(
            element: appElement,
            notification: .announcementRequested,
            userInfo: [
                .announcement: message,
                .priority: NSAccessibilityPriorityLevel.high.rawValue
            ]
        )
    }
}

// MARK: - View Modifiers for Common Accessibility Patterns

extension View {
    /// Combines child views into a single accessibility element with custom labels.
    /// - Parameters:
    ///   - label: The primary accessibility label.
    ///   - value: Optional accessibility value for additional context.
    ///   - hint: Optional accessibility hint describing the action.
    func accessibleElement(label: String, value: String? = nil, hint: String? = nil) -> some View {
        self
            .accessibilityElement(children: .combine)
            .accessibilityLabel(label)
            .applyIfLet(value) { view, val in
                view.accessibilityValue(val)
            }
            .applyIfLet(hint) { view, h in
                view.accessibilityHint(h)
            }
    }

    /// Adds button traits and accessibility attributes.
    /// - Parameters:
    ///   - label: The accessibility label.
    ///   - hint: Optional hint describing what the button does.
    func accessibleButton(label: String, hint: String? = nil) -> some View {
        self
            .accessibilityElement(children: .combine)
            .accessibilityLabel(label)
            .accessibilityAddTraits(.isButton)
            .applyIfLet(hint) { view, h in
                view.accessibilityHint(h)
            }
    }

    /// Applies a transformation only if the value is non-nil.
    private func applyIfLet<T>(_ value: T?, _ transform: (Self, T) -> some View) -> some View {
        if let value = value {
            return transform(self, value)
        } else {
            return self
        }
    }
}
