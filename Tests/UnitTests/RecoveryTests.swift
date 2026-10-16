import CoreData
import XCTest
@testable import FamilyRoom

/// A library that cannot be opened must not crash the app at launch.
@MainActor
final class RecoveryTests: XCTestCase {
    private var dir: URL!

    override func setUp() {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("fr-recovery-\(UUID().uuidString)", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    }

    override func tearDown() { try? FileManager.default.removeItem(at: dir) }

    func testDamagedLibraryIsSetAsideAndAFreshOneOpens() throws {
        try Data("this is not a database".utf8).write(to: dir.appendingPathComponent("private.sqlite"))
        let persistence = Persistence(directory: dir, cloud: false)
        let folder = try XCTUnwrap(persistence.setAsideFolder)
        XCTAssertTrue(FileManager.default.fileExists(atPath: folder.appendingPathComponent("private.sqlite").path))
        let store = RoomStore(persistence: persistence)
        _ = store.createRoom(named: "Fresh")
        XCTAssertEqual(store.rooms.count, 1)
    }

    /// SQLite stores keep a copy of the model that wrote them, so Core Data can migrate a library written
    /// by an older schema in place. Such a library must open normally, not be set aside.
    func testLibraryFromAnOlderSchemaMigratesInPlace() throws {
        let room = NSEntityDescription(); room.name = "Room"; room.managedObjectClassName = NSStringFromClass(NSManagedObject.self)
        let id = NSAttributeDescription(); id.name = "id"; id.attributeType = .UUIDAttributeType; id.isOptional = true
        let name = NSAttributeDescription(); name.name = "name"; name.attributeType = .stringAttributeType; name.isOptional = true
        room.properties = [id, name]
        let older = NSManagedObjectModel(); older.entities = [room]
        let coordinator = NSPersistentStoreCoordinator(managedObjectModel: older)
        let url = dir.appendingPathComponent("private.sqlite")
        let store = try coordinator.addPersistentStore(type: .sqlite, at: url)
        let context = NSManagedObjectContext(.mainQueue); context.persistentStoreCoordinator = coordinator
        let saved = NSManagedObject(entity: room, insertInto: context)
        saved.setValue(UUID(), forKey: "id"); saved.setValue("From before", forKey: "name")
        try context.save()
        try coordinator.remove(store)

        let persistence = Persistence(directory: dir, cloud: false)
        XCTAssertNil(persistence.setAsideFolder, "A migratable library should open in place")
        XCTAssertEqual(RoomStore(persistence: persistence).rooms.map(\.displayName), ["From before"])
    }

    func testHealthyLibraryIsLeftAlone() throws {
        let first = Persistence(directory: dir, cloud: false)
        let store = RoomStore(persistence: first)
        _ = store.createRoom(named: "Kept")
        for s in first.container.persistentStoreCoordinator.persistentStores { try first.container.persistentStoreCoordinator.remove(s) }

        let reopened = Persistence(directory: dir, cloud: false)
        XCTAssertNil(reopened.setAsideFolder)
        XCTAssertEqual(RoomStore(persistence: reopened).rooms.map(\.displayName), ["Kept"])
    }
}
