/**
 ASettingsInfra

 Responsibilities:
 - Define shared settings-window identity constants used across the decomposed settings scene runtime.
 - Keep the root infra file as a thin facade while adjacent files own window service, diagnostics, appearance, and scene composition.

 Does not handle:
 - Settings tab content (defined in `AppSettings.swift` and companion files).
 - Settings open command wiring (defined in `SettingsService.swift`).
 - Window-service, diagnostics, or focus-anchor implementation details.
 */

import AppKit
import RalphCore
import SwiftUI

enum SettingsWindowIdentity {
    static let sceneID = "settings"
    static let windowIdentifier = "com.mitchfultz.ralph.settings-window"
    static let legacyWindowIdentifier = "com_apple_SwiftUI_Settings_window"
    static let title = "Settings"
}
