import CoreData
import XCTest
@testable import FamilyRoom

/// A Room someone joined lives in the shared store. Everything they add must land in that store too,
/// or Core Data refuses to save the cross-store relationship.
@MainActor
final class SharedRoomTests: XCTestCase {
    func testParticipantCanAddToASharedRoom() throws {
        let persistence = Persistence(testingWithSharedStore: true)
        let store = RoomStore(persistence: persistence)
        let shared = try XCTUnwrap(persistence.sharedStore)

        // Stand-in for a Room that arrived through an accepted invitation.
        let room = Room(context: store.context)
        room.id = UUID(); room.name = "Grandma's"; room.createdAt = Date()
        store.context.assign(room, to: shared)
        try store.context.save()
        XCTAssertFalse(persistence.isOwned(room))

        let date = Calendar.current.date(from: DateComponents(year: 2025, month: 5, day: 4, hour: 9))!
        let item = store.add(ImportedMedia(data: Data([1]), thumbnail: nil, isVideo: false, capturedAt: date,
                                           pixelSize: CGSize(width: 1, height: 1), duration: 0, fileExtension: "jpg", photoIdentifier: nil), to: room)
        let person = store.createPerson(named: "Iris", in: room)
        let moment = try XCTUnwrap(item.moment)
        store.toggle(person, in: moment)
        store.addNote("lovely", to: moment)
        store.toggleLike(moment)
        let face = store.insert(FaceObservation.self, near: item)
        face.id = UUID(); face.media = item; face.person = person
        store.saveStory(title: "May", subtitle: "", movie: Data([2]), thumbnail: nil, duration: 3, moment: moment, room: room)
        try store.context.save()

        let created: [NSManagedObject] = [item, moment, person, face] + Array(moment.notes ?? []) + Array(moment.reactions ?? []) + Array(room.stories ?? [])
        for object in created {
            XCTAssertEqual(object.objectID.persistentStore, shared, "\(object.entity.name ?? "?") was saved outside the shared store")
        }
    }

    func testOwnedRoomsStayPrivate() throws {
        let persistence = Persistence(testingWithSharedStore: true)
        let store = RoomStore(persistence: persistence)
        let room = store.createRoom(named: "Ours")
        XCTAssertEqual(room.objectID.persistentStore, persistence.privateStore)
        let person = store.createPerson(named: "Theo", in: room)
        XCTAssertEqual(person.objectID.persistentStore, persistence.privateStore)
        XCTAssertTrue(persistence.isOwned(room))
    }
}
