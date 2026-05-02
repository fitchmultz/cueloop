/**
 CueLoopCLIExecutableLocator

 Purpose:
 - Provide a single place to resolve the on-disk `cueloop` executable used by the macOS GUI.

 Responsibilities:
 - Provide a single place to resolve the on-disk `cueloop` executable used by the macOS GUI.
 - Prefer the app-bundled `cueloop` placed next to the app executable (Contents/MacOS/cueloop).
 - Fall back to the legacy bundled `cueloop` executable during the migration window.

 Does not handle:
 - Building or copying the `cueloop` binary into the bundle (handled by the Xcode build phase).
 - Falling back to `PATH` lookup. If the binary isn't bundled, the GUI treats this as a configuration error.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - The GUI build step bundles an executable file named `cueloop` into the app bundle.
 */

public import Foundation

public enum CueLoopCLIExecutableLocator {
    public enum LocatorError: Error, Equatable {
        case bundledExecutableNotFound
    }

    public static func bundledCueLoopExecutableURL(bundle: Bundle = .main) throws -> URL {
        if let url = bundledExecutableURL(named: "cueloop", bundle: bundle) {
            return url
        }
        if let url = bundledExecutableURL(named: "cueloop", bundle: bundle) {
            return url
        }

        throw LocatorError.bundledExecutableNotFound
    }

    private static func bundledExecutableURL(named name: String, bundle: Bundle) -> URL? {
        if let url = bundle.url(forAuxiliaryExecutable: name) {
            return url
        }

        // Fallback for situations where Bundle APIs are picky about location.
        let candidate = bundle.bundleURL
            .appendingPathComponent("Contents", isDirectory: true)
            .appendingPathComponent("MacOS", isDirectory: true)
            .appendingPathComponent(name, isDirectory: false)

        guard FileManager.default.isExecutableFile(atPath: candidate.path) else {
            return nil
        }
        return candidate
    }
}
