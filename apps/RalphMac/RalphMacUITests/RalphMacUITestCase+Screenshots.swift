/**
 Purpose:
 - Isolate screenshot and attachment helpers for Ralph macOS UI tests.

 Responsibilities:
 - Capture checkpoints and optional timeline screenshots.
 - Sanitize attachment names and manage timeline capture lifecycle.

 Scope:
 - Screenshot support only.

 Usage:
 - Base harness setup/teardown and scenario tests call `captureScreenshot` indirectly through this extension.

 Invariants/Assumptions:
 - Timeline capture runs on the main actor while the app is alive.
 */

import XCTest

@MainActor
extension RalphMacUITestCase {
    func captureScreenshot(named step: String) {
        guard screenshotMode != .off else { return }
        guard app != nil else { return }

        screenshotSequence += 1
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = "\(sanitizedTestName())-\(String(format: "%03d", screenshotSequence))-\(sanitizedAttachmentToken(step))"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func startTimelineCaptureIfNeeded() {
        guard screenshotMode == .timeline else { return }
        stopTimelineCapture()

        timelineCaptureTask = Task { @MainActor [weak self] in
            guard let self else { return }
            var frameIndex = 0
            while !Task.isCancelled && frameIndex < self.timelineMaxFrames {
                await MainActor.run {
                    let frameDeadline = Date().addingTimeInterval(self.timelineInterval)
                    while !Task.isCancelled && Date() < frameDeadline {
                        RunLoop.current.run(
                            mode: .default,
                            before: min(frameDeadline, Date().addingTimeInterval(0.1))
                        )
                    }
                }
                guard !Task.isCancelled else { break }
                self.captureScreenshot(named: "timeline-\(frameIndex)")
                frameIndex += 1
            }
        }
    }

    func stopTimelineCapture() {
        timelineCaptureTask?.cancel()
        timelineCaptureTask = nil
    }

    func sanitizedTestName() -> String {
        let cleaned = name
            .replacingOccurrences(of: "^[-\\[]+", with: "", options: .regularExpression)
            .replacingOccurrences(of: "[\\] ]+$", with: "", options: .regularExpression)
            .replacingOccurrences(of: "[^A-Za-z0-9._-]+", with: "-", options: .regularExpression)
            .trimmingCharacters(in: CharacterSet(charactersIn: "-"))
        return cleaned.isEmpty ? "ui-test" : cleaned
    }

    func sanitizedAttachmentToken(_ raw: String) -> String {
        let cleaned = raw
            .replacingOccurrences(of: "[^A-Za-z0-9._-]+", with: "-", options: .regularExpression)
            .trimmingCharacters(in: CharacterSet(charactersIn: "-"))
        return cleaned.isEmpty ? "checkpoint" : cleaned
    }
}
