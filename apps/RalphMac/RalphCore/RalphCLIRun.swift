/**
 RalphCLIRun

 Purpose:
 - Own a single running Ralph CLI subprocess.

 Responsibilities:
 - Own a single running Ralph CLI subprocess.
 - Bridge pipe readability callbacks into async event streams.
 - Coordinate cooperative termination and final exit-status delivery.

 Does not handle:
 - Building commands to execute.
 - Retry policies or health checks.
 - Parsing streamed output into domain models.

 Usage:
 - Used by the RalphMac app or RalphCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Instances are created only by `RalphCLIClient.start(...)`.
 - Cancellation is best-effort: interrupt first, then escalate to terminate and hard-kill after the grace period on Darwin.
 - Event streams finish only after process termination and both pipes reach EOF.
 */

import Foundation

#if canImport(Darwin)
import Darwin
#endif

final class RalphCLIProcessSignalState: @unchecked Sendable {
    private let lock = NSLock()
    private var processGroupID: pid_t?

    func prepareProcessGroup(for process: Process) {
        #if canImport(Darwin)
        let pid = process.processIdentifier
        guard pid > 0 else { return }

        let setGroupSucceeded = setpgid(pid, pid) == 0
        let isGroupLeader = getpgid(pid) == pid
        guard setGroupSucceeded || isGroupLeader else { return }

        lock.lock()
        processGroupID = pid
        lock.unlock()
        #endif
    }

    func interrupt(_ process: Process) {
        #if canImport(Darwin)
        if signalProcessGroup(process, signal: SIGINT) {
            return
        }
        #endif
        process.interrupt()
    }

    func terminate(_ process: Process) {
        #if canImport(Darwin)
        if signalProcessGroup(process, signal: SIGTERM) {
            return
        }
        #endif
        process.terminate()
    }

    #if canImport(Darwin)
    func kill(_ process: Process, fallbackPID: pid_t) {
        if signalProcessGroup(process, signal: SIGKILL) {
            return
        }
        _ = Darwin.kill(fallbackPID, SIGKILL)
    }

    private func signalProcessGroup(_ process: Process, signal: Int32) -> Bool {
        guard process.isRunning else { return true }

        lock.lock()
        let processGroupID = processGroupID
        lock.unlock()

        guard let processGroupID else { return false }
        return Darwin.kill(-processGroupID, signal) == 0
    }
    #endif
}

public actor RalphCLIRun {
    public let events: AsyncStream<RalphCLIEvent>
    private static let maxBufferedEvents = 512
    private static let maxEventReadBytes = 64 * 1024

    nonisolated func requestCancel(gracePeriod: TimeInterval = 2) {
        Task { [weak self] in
            await self?.cancel(gracePeriod: gracePeriod)
        }
    }

    private let ioQueue: DispatchQueue
    private let process: Process
    private let processSignalState: RalphCLIProcessSignalState
    private let stdoutHandle: FileHandle
    private let stderrHandle: FileHandle

    private var eventsContinuation: AsyncStream<RalphCLIEvent>.Continuation?
    private var didRequestCancel = false
    private var didFinishEvents = false
    private var didTerminateProcess = false
    private var didEscalateTermination = false
    private var stdoutClosed = false
    private var stderrClosed = false
    private var exitStatus: RalphCLIExitStatus?
    private var exitWaiters: [CheckedContinuation<RalphCLIExitStatus, Never>] = []
    private var droppedOutputByteCount = 0

    internal init(
        ioQueue: DispatchQueue,
        process: Process,
        processSignalState: RalphCLIProcessSignalState,
        stdoutHandle: FileHandle,
        stderrHandle: FileHandle
    ) {
        self.ioQueue = ioQueue
        self.process = process
        self.processSignalState = processSignalState
        self.stdoutHandle = stdoutHandle
        self.stderrHandle = stderrHandle

        var continuation: AsyncStream<RalphCLIEvent>.Continuation?
        let stream = AsyncStream<RalphCLIEvent>(
            bufferingPolicy: .bufferingNewest(Self.maxBufferedEvents)
        ) { cont in
            continuation = cont
        }
        events = stream
        eventsContinuation = continuation
        eventsContinuation?.onTermination = { @Sendable [weak self] _ in
            self?.requestCancel()
        }

        configureNonBlockingRead(stdoutHandle)
        configureNonBlockingRead(stderrHandle)
        setupIOHandlers()
    }

    deinit {
        requestCancel()
    }

    public func cancel() {
        cancel(gracePeriod: 2)
    }

    func cancel(gracePeriod: TimeInterval) {
        guard !didRequestCancel else { return }
        didRequestCancel = true

        guard process.isRunning else { return }
        processSignalState.interrupt(process)

        #if canImport(Darwin)
        let pid = process.processIdentifier
        ioQueue.asyncAfter(deadline: .now() + gracePeriod) { [weak self] in
            guard let self else { return }
            Task { [weak self] in
                await self?.terminateIfStillRunning()
            }
        }
        ioQueue.asyncAfter(deadline: .now() + (gracePeriod * 2)) { [weak self] in
            guard let self else { return }
            Task { [weak self] in
                await self?.killIfStillRunning(pid: pid)
            }
        }
        #endif
    }

    private func terminateIfStillRunning() {
        guard process.isRunning else { return }
        guard !didEscalateTermination else { return }
        didEscalateTermination = true
        processSignalState.terminate(process)
    }

    #if canImport(Darwin)
    private func killIfStillRunning(pid: pid_t) {
        guard process.isRunning else { return }
        processSignalState.kill(process, fallbackPID: pid)
    }
    #endif

    public func waitUntilExit() async -> RalphCLIExitStatus {
        if let existing = exitStatus {
            return existing
        }

        return await withCheckedContinuation { cont in
            if let existing = exitStatus {
                cont.resume(returning: existing)
                return
            }
            exitWaiters.append(cont)
        }
    }

    public func droppedOutputBytes() -> Int {
        droppedOutputByteCount
    }

    private nonisolated func setupIOHandlers() {
        stdoutHandle.readabilityHandler = { [weak self] handle in
            guard let self else { return }
            Task {
                await self.handleReadable(stream: .stdout, handle: handle)
            }
        }

        stderrHandle.readabilityHandler = { [weak self] handle in
            guard let self else { return }
            Task {
                await self.handleReadable(stream: .stderr, handle: handle)
            }
        }

        process.terminationHandler = { [weak self] process in
            guard let self else { return }
            Task {
                await self.handleTermination(process: process)
            }
        }
    }

    private func handleReadable(stream: RalphCLIEvent.Stream, handle: FileHandle) {
        switch readBoundedChunk(from: handle) {
        case .data(let data):
            yieldBoundedEvent(stream: stream, data: data)
        case .wouldBlock:
            return
        case .eof:
            handle.readabilityHandler = nil

            switch stream {
            case .stdout:
                stdoutClosed = true
            case .stderr:
                stderrClosed = true
            }

            finishIfComplete()
            return
        }
    }

    private enum PipeReadResult {
        case data(Data)
        case eof
        case wouldBlock
    }

    private nonisolated func configureNonBlockingRead(_ handle: FileHandle) {
        #if canImport(Darwin)
        let descriptor = handle.fileDescriptor
        let currentFlags = fcntl(descriptor, F_GETFL)
        if currentFlags >= 0 {
            _ = fcntl(descriptor, F_SETFL, currentFlags | O_NONBLOCK)
        }
        #endif
    }

    private func readBoundedChunk(from handle: FileHandle) -> PipeReadResult {
        #if canImport(Darwin)
        var buffer = [UInt8](repeating: 0, count: Self.maxEventReadBytes)
        let bytesRead = buffer.withUnsafeMutableBytes { rawBuffer in
            Darwin.read(handle.fileDescriptor, rawBuffer.baseAddress, rawBuffer.count)
        }

        if bytesRead > 0 {
            return .data(Data(buffer.prefix(bytesRead)))
        }
        if bytesRead == 0 {
            return .eof
        }
        if errno == EAGAIN || errno == EWOULDBLOCK {
            return .wouldBlock
        }
        return .eof
        #else
        let data = (try? handle.read(upToCount: Self.maxEventReadBytes)) ?? Data()
        return data.isEmpty ? .eof : .data(data)
        #endif
    }

    private func yieldBoundedEvent(stream: RalphCLIEvent.Stream, data: Data) {
        guard data.count > Self.maxEventReadBytes else {
            yieldEvent(stream: stream, data: data)
            return
        }

        var offset = 0
        while offset < data.count {
            let nextOffset = min(offset + Self.maxEventReadBytes, data.count)
            yieldEvent(stream: stream, data: data.subdata(in: offset..<nextOffset))
            offset = nextOffset
        }
    }

    private func yieldEvent(stream: RalphCLIEvent.Stream, data: Data) {
        let result = eventsContinuation?.yield(RalphCLIEvent(stream: stream, data: data))
        if case .dropped(let droppedEvent) = result {
            droppedOutputByteCount += droppedEvent.data.count
        }
    }

    private func handleTermination(process: Process) {
        didTerminateProcess = true
        RalphLogger.shared.debug("CLI process terminated with status: \(process.terminationStatus)", category: .cli)

        let reason: RalphCLIExitStatus.TerminationReason
        switch process.terminationReason {
        case .exit:
            reason = .exit
        case .uncaughtSignal:
            reason = .uncaughtSignal
        @unknown default:
            reason = .exit
        }

        let status = RalphCLIExitStatus(code: process.terminationStatus, reason: reason)
        stdoutHandle.readabilityHandler = nil
        stderrHandle.readabilityHandler = nil

        drainRemainingOutput(stream: .stdout, handle: stdoutHandle)
        drainRemainingOutput(stream: .stderr, handle: stderrHandle)

        stdoutClosed = true
        stderrClosed = true

        if exitStatus == nil {
            exitStatus = status
            let waiters = exitWaiters
            exitWaiters.removeAll(keepingCapacity: false)
            for waiter in waiters {
                waiter.resume(returning: status)
            }
        }

        finishIfComplete()
    }

    private func drainRemainingOutput(stream: RalphCLIEvent.Stream, handle: FileHandle) {
        while true {
            switch readBoundedChunk(from: handle) {
            case .data(let data):
                yieldBoundedEvent(stream: stream, data: data)
            case .eof, .wouldBlock:
                return
            }
        }
    }

    private func finishIfComplete() {
        guard didTerminateProcess, stdoutClosed, stderrClosed else { return }
        guard !didFinishEvents else { return }

        didFinishEvents = true
        eventsContinuation?.finish()
        eventsContinuation = nil
    }
}
