/**
 RalphCLIExecutableLocatorTests

 Purpose:
 - Validate macOS bundled CLI executable lookup during the CueLoop transition.

 Responsibilities:
 - Prove the locator prefers the bundled `cueloop` executable.
 - Prove the locator still falls back to the legacy `ralph` executable during the migration window.

 Does not handle:
 - Building the real CLI or validating Xcode bundle phases.
 - PATH-based lookup, which the app intentionally does not support.

 Usage:
 - Run through the RalphCore test target.

 Invariants/assumptions callers must respect:
 - Test-created bundle directories mirror the `Contents/MacOS` structure used by macOS apps.
 */

import Foundation
import XCTest

@testable import RalphCore

final class RalphCLIExecutableLocatorTests: RalphCoreTestCase {
    func testBundledExecutableLookupPrefersCueLoop() throws {
        let bundleURL = try makeBundle(named: "BothCLIs")
        let cueloopURL = try makeExecutable(named: "cueloop", in: bundleURL)
        _ = try makeExecutable(named: "ralph", in: bundleURL)
        let bundle = try XCTUnwrap(Bundle(url: bundleURL))

        let resolved = try RalphCLIExecutableLocator.bundledRalphExecutableURL(bundle: bundle)

        XCTAssertEqual(resolved.path, cueloopURL.path)
    }

    func testBundledExecutableLookupFallsBackToLegacyRalph() throws {
        let bundleURL = try makeBundle(named: "LegacyCLI")
        let ralphURL = try makeExecutable(named: "ralph", in: bundleURL)
        let bundle = try XCTUnwrap(Bundle(url: bundleURL))

        let resolved = try RalphCLIExecutableLocator.bundledRalphExecutableURL(bundle: bundle)

        XCTAssertEqual(resolved.path, ralphURL.path)
    }

    private func makeBundle(named name: String) throws -> URL {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("cueloop-cli-locator-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        addTeardownBlock {
            try? FileManager.default.removeItem(at: root)
        }
        let bundleURL = root.appendingPathComponent("\(name).app", isDirectory: true)
        let executableDirectory = bundleURL
            .appendingPathComponent("Contents", isDirectory: true)
            .appendingPathComponent("MacOS", isDirectory: true)
        try FileManager.default.createDirectory(at: executableDirectory, withIntermediateDirectories: true)
        return bundleURL
    }

    private func makeExecutable(named name: String, in bundleURL: URL) throws -> URL {
        let executableURL = bundleURL
            .appendingPathComponent("Contents", isDirectory: true)
            .appendingPathComponent("MacOS", isDirectory: true)
            .appendingPathComponent(name, isDirectory: false)
        try "#!/bin/sh\nexit 0\n".write(to: executableURL, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: executableURL.path)
        return executableURL
    }
}
