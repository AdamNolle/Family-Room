#if os(iOS)
import BackgroundTasks
import Foundation
import OSLog

@MainActor
enum AppleBackgroundSyncScheduler {
    static let taskIdentifier = "com.familyroom.app.background-sync"

    private static let logger = Logger(
        subsystem: "com.familyroom.app",
        category: "BackgroundSync"
    )

    static func register(store: LibraryStore) {
        let registered = BGTaskScheduler.shared.register(
            forTaskWithIdentifier: taskIdentifier,
            using: nil
        ) { task in
            guard let task = task as? BGProcessingTask else {
                task.setTaskCompleted(success: false)
                return
            }

            let completion = BackgroundTaskCompletion()
            let operation = Task { @MainActor in
                let succeeded = await store.performBackgroundSync()
                completion.finish(task, success: succeeded)
            }
            task.expirationHandler = {
                operation.cancel()
                completion.finish(task, success: false)
            }
        }

        if !registered {
            logger.error("Could not register the background sync task.")
        }
    }

    static func schedule(hasRemoteStorage: Bool) {
        guard hasRemoteStorage else {
            BGTaskScheduler.shared.cancel(taskRequestWithIdentifier: taskIdentifier)
            return
        }

        let request = BGProcessingTaskRequest(identifier: taskIdentifier)
        request.requiresNetworkConnectivity = true
        request.requiresExternalPower = false
        request.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60)

        do {
            try BGTaskScheduler.shared.submit(request)
        } catch {
            logger.error("Could not schedule background sync: \(error.localizedDescription, privacy: .public)")
        }
    }
}

private final class BackgroundTaskCompletion: @unchecked Sendable {
    private let lock = NSLock()
    private var finished = false

    func finish(_ task: BGTask, success: Bool) {
        lock.lock()
        guard !finished else {
            lock.unlock()
            return
        }
        finished = true
        lock.unlock()

        task.setTaskCompleted(success: success)
    }
}
#endif
