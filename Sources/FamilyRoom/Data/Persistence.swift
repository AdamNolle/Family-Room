import CoreData
import CloudKit
import OSLog

/// Owns the Core Data stack. Prefers iCloud (private + shared CloudKit databases) and falls back to a
/// local-only store when the build is not entitled for iCloud, so the app still works everywhere.
final class Persistence {
    static let containerIdentifier = "iCloud.com.familyroom.app"
    static let shared = Persistence()
    private static let log = Logger(subsystem: "com.familyroom.app", category: "Persistence")

    let container: NSPersistentCloudKitContainer
    let cloudEnabled: Bool
    private(set) var privateStore: NSPersistentStore?
    private(set) var sharedStore: NSPersistentStore?
    var viewContext: NSManagedObjectContext { container.viewContext }

    /// Tests: a private and a shared store, both local, wired like a participant's device.
    init(testingWithSharedStore: Bool) {
        let container = NSPersistentCloudKitContainer(name: "FamilyRoomTest", managedObjectModel: FamilyRoomModel.shared)
        let base = FileManager.default.temporaryDirectory.appendingPathComponent("fr-test-\(UUID().uuidString)", isDirectory: true)
        try? FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        container.persistentStoreDescriptions = ["private", "shared"].map { NSPersistentStoreDescription(url: base.appendingPathComponent("\($0).sqlite")) }
        var stores: [String: NSPersistentStore] = [:]
        container.loadPersistentStores { description, error in
            precondition(error == nil, "Test stores failed to load: \(String(describing: error))")
            stores[description.url!.deletingPathExtension().lastPathComponent] = container.persistentStoreCoordinator.persistentStore(for: description.url!)
        }
        self.container = container
        privateStore = stores["private"]; sharedStore = stores["shared"]; cloudEnabled = false
        container.viewContext.mergePolicy = NSMergeByPropertyObjectTrumpMergePolicy
    }

    init(inMemory: Bool = false) {
        let wantsCloud = !inMemory && ProcessInfo.processInfo.environment["FAMILY_ROOM_LOCAL_ONLY"] == nil && Persistence.isEntitledForCloud
        if wantsCloud, let loaded = Persistence.load(cloud: true, inMemory: false) {
            container = loaded.container; privateStore = loaded.private; sharedStore = loaded.shared; cloudEnabled = true
        } else if let loaded = Persistence.load(cloud: false, inMemory: inMemory) {
            container = loaded.container; privateStore = loaded.private; sharedStore = loaded.shared; cloudEnabled = false
        } else {
            fatalError("Family Room could not open its library.")
        }
        container.viewContext.automaticallyMergesChangesFromParent = true
        container.viewContext.mergePolicy = NSMergeByPropertyObjectTrumpMergePolicy
        container.viewContext.transactionAuthor = "app"
        Persistence.log.info("Library opened. iCloud: \(self.cloudEnabled)")
    }

    /// iCloud needs a signed build with a Developer team. The team is written into Info.plist by the build,
    /// so an unsigned or package build knows to stay local instead of trapping inside CloudKit.
    static var isEntitledForCloud: Bool {
        let team = (Bundle.main.object(forInfoDictionaryKey: "FamilyRoomTeam") as? String) ?? ""
        return !team.trimmingCharacters(in: .whitespaces).isEmpty
    }

    private static var directory: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        let dir = base.appendingPathComponent("FamilyRoom", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    private static func load(cloud: Bool, inMemory: Bool) -> (container: NSPersistentCloudKitContainer, private: NSPersistentStore?, shared: NSPersistentStore?)? {
        let container = NSPersistentCloudKitContainer(name: "FamilyRoom", managedObjectModel: FamilyRoomModel.shared)
        let privateURL = inMemory ? URL(fileURLWithPath: "/dev/null") : directory.appendingPathComponent("private.sqlite")
        let privateDescription = NSPersistentStoreDescription(url: privateURL)
        privateDescription.setOption(true as NSNumber, forKey: NSPersistentHistoryTrackingKey)
        privateDescription.setOption(true as NSNumber, forKey: NSPersistentStoreRemoteChangeNotificationPostOptionKey)
        var descriptions = [privateDescription]
        if cloud {
            let privateOptions = NSPersistentCloudKitContainerOptions(containerIdentifier: containerIdentifier)
            privateOptions.databaseScope = .private
            privateDescription.cloudKitContainerOptions = privateOptions
            let sharedDescription = NSPersistentStoreDescription(url: directory.appendingPathComponent("shared.sqlite"))
            sharedDescription.setOption(true as NSNumber, forKey: NSPersistentHistoryTrackingKey)
            sharedDescription.setOption(true as NSNumber, forKey: NSPersistentStoreRemoteChangeNotificationPostOptionKey)
            let sharedOptions = NSPersistentCloudKitContainerOptions(containerIdentifier: containerIdentifier)
            sharedOptions.databaseScope = .shared
            sharedDescription.cloudKitContainerOptions = sharedOptions
            descriptions.append(sharedDescription)
        }
        container.persistentStoreDescriptions = descriptions
        var failure: Error?
        var privateStore: NSPersistentStore?, sharedStore: NSPersistentStore?
        container.loadPersistentStores { description, error in
            if let error { failure = error; return }
            let store = container.persistentStoreCoordinator.persistentStore(for: description.url!)
            if description.cloudKitContainerOptions?.databaseScope == .shared { sharedStore = store } else { privateStore = store }
        }
        if let failure {
            log.error("Store load failed (iCloud: \(cloud)): \(failure.localizedDescription)")
            for store in container.persistentStoreCoordinator.persistentStores { try? container.persistentStoreCoordinator.remove(store) }
            return nil
        }
        return (container, privateStore, sharedStore)
    }

    func save() {
        let context = container.viewContext
        guard context.hasChanges else { return }
        do { try context.save() } catch {
            Persistence.log.error("Save failed: \(error.localizedDescription)")
            context.rollback()
        }
    }

    /// The store an object will be saved to, including objects not yet saved.
    func store(for object: NSManagedObject) -> NSPersistentStore? {
        object.objectID.persistentStore ?? (object.objectID.isTemporaryID ? nil : privateStore)
    }

    func isOwned(_ object: NSManagedObject) -> Bool {
        guard let store = object.objectID.persistentStore else { return true }
        return store == privateStore || sharedStore == nil
    }

    func newBackgroundContext() -> NSManagedObjectContext {
        let context = container.newBackgroundContext()
        context.mergePolicy = NSMergeByPropertyObjectTrumpMergePolicy
        context.transactionAuthor = "worker"
        return context
    }
}
