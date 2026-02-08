/**
 QueueFileWatcher

 Responsibilities:
 - Monitor .ralph/queue.json and .ralph/done.json for external changes using FSEvents.
 - Emit notifications when files change with debouncing to batch rapid changes.
 - Handle file system events efficiently with minimal resource usage.

 Does not handle:
 - Direct UI updates (delegates via NotificationCenter).
 - Parsing or interpreting file contents.
 - Retry logic for transient errors (logs and continues watching).

 Invariants/assumptions callers must respect:
 - start() must be called to begin monitoring; stop() to clean up.
 - Debounce interval batches multiple rapid changes into single notification.
 - Callbacks occur on a private serial queue (not main thread).
 */

public import Foundation
import CoreServices

public final class QueueFileWatcher: @unchecked Sendable {
    // MARK: - Types

    public struct ChangeEvent: Sendable {
        public let fileURL: URL
        public let changeType: ChangeType

        public enum ChangeType: Sendable {
            case modified
            case renamed
            case deleted
        }
    }

    // MARK: - Properties

    private var stream: FSEventStreamRef?
    private let workingDirectoryURL: URL
    private let debounceInterval: TimeInterval = 0.5  // 500ms debounce
    private var pendingChanges: Set<String> = []
    private let lock = NSLock()
    private var debounceWorkItem: DispatchWorkItem?
    private let callbackQueue = DispatchQueue(label: "com.mitchfultz.ralph.filewatcher")

    /// Callback invoked on MainActor when file changes are detected (after debounce)
    public var onFileChanged: (@Sendable () -> Void)?

    /// Whether the watcher is currently active
    public private(set) var isWatching = false

    // MARK: - Initialization

    public init(workingDirectoryURL: URL) {
        self.workingDirectoryURL = workingDirectoryURL
    }

    deinit {
        stop()
    }

    // MARK: - Public Methods

    /// Start watching the queue files for changes
    public func start() {
        callbackQueue.sync {
            guard !self.isWatching else { return }

            let queueDir = self.workingDirectoryURL.appendingPathComponent(".ralph")
            let pathsToWatch = [queueDir.path as NSString]

            // Create context with self reference
            var context = FSEventStreamContext(
                version: 0,
                info: Unmanaged.passUnretained(self).toOpaque(),
                retain: nil,
                release: nil,
                copyDescription: nil
            )

            // Create the event stream
            self.stream = FSEventStreamCreate(
                kCFAllocatorDefault,
                { (_, clientCallBackInfo, numEvents, eventPaths, eventFlags, _) in
                    guard let info = clientCallBackInfo else { return }
                    let watcher = Unmanaged<QueueFileWatcher>.fromOpaque(info).takeUnretainedValue()
                    watcher.handleFSEvents(
                        numEvents: numEvents,
                        eventPaths: eventPaths,
                        eventFlags: eventFlags
                    )
                },
                &context,
                pathsToWatch as CFArray,
                FSEventStreamEventId(kFSEventStreamEventIdSinceNow),
                0.1,  // Latency in seconds
                FSEventStreamCreateFlags(
                    kFSEventStreamCreateFlagFileEvents | kFSEventStreamCreateFlagUseCFTypes)
            )

            guard let stream = self.stream else {
                print("[QueueFileWatcher] Failed to create FSEvent stream")
                return
            }

            // Use dispatch queue instead of run loop (modern approach)
            FSEventStreamSetDispatchQueue(stream, self.callbackQueue)

            guard FSEventStreamStart(stream) else {
                print("[QueueFileWatcher] Failed to start FSEvent stream")
                FSEventStreamInvalidate(stream)
                self.stream = nil
                return
            }

            self.isWatching = true
            print("[QueueFileWatcher] Started watching \(queueDir.path)")
        }
    }

    /// Stop watching and clean up resources
    public func stop() {
        callbackQueue.sync {
            guard let stream = self.stream else { return }

            FSEventStreamStop(stream)
            FSEventStreamInvalidate(stream)
            // Don't release since we're using dispatch queue

            self.stream = nil
            self.isWatching = false

            // Clean up debounce work item
            self.debounceWorkItem?.cancel()
            self.debounceWorkItem = nil

            self.lock.lock()
            self.pendingChanges.removeAll()
            self.lock.unlock()
        }
    }

    /// Update the working directory and restart watching if active
    public func updateWorkingDirectory(_ url: URL) {
        callbackQueue.sync {
            _ = self.isWatching
            self.stop()
        }
    }

    // MARK: - Private Methods

    private func handleFSEvents(
        numEvents: Int, eventPaths: UnsafeMutableRawPointer,
        eventFlags: UnsafePointer<FSEventStreamEventFlags>
    ) {
        guard let paths = Unmanaged<CFArray>.fromOpaque(eventPaths).takeUnretainedValue() as? [String]
        else {
            return
        }

        let relevantFiles = ["queue.json", "done.json"]
        var hasRelevantChange = false

        for i in 0..<numEvents {
            let path = paths[i]
            let flags = eventFlags[i]

            // Check if this is one of our target files
            guard relevantFiles.contains(where: { path.hasSuffix($0) }) else {
                continue
            }

            // Check for modification or creation events
            let isModified = (flags & UInt32(kFSEventStreamEventFlagItemModified)) != 0
            let isCreated = (flags & UInt32(kFSEventStreamEventFlagItemCreated)) != 0
            let isRenamed = (flags & UInt32(kFSEventStreamEventFlagItemRenamed)) != 0
            let isRemoved = (flags & UInt32(kFSEventStreamEventFlagItemRemoved)) != 0

            if isModified || isCreated || isRenamed || isRemoved {
                hasRelevantChange = true
                self.lock.lock()
                pendingChanges.insert(path)
                self.lock.unlock()
            }
        }

        if hasRelevantChange {
            scheduleDebouncedNotification()
        }
    }

    private func scheduleDebouncedNotification() {
        // Cancel existing work item
        debounceWorkItem?.cancel()

        // Create new work item
        let workItem = DispatchWorkItem { [weak self] in
            guard let self = self else { return }

            self.lock.lock()
            self.pendingChanges.removeAll()
            self.lock.unlock()

            // Notify on main actor
            DispatchQueue.main.async {
                self.onFileChanged?()
            }
        }

        debounceWorkItem = workItem

        // Schedule after debounce interval
        callbackQueue.asyncAfter(deadline: .now() + debounceInterval, execute: workItem)
    }
}
