import CoreData
import XCTest
@testable import FamilyRoom

@MainActor
final class LibraryTests: XCTestCase {
    private var persistence: Persistence!
    private var store: RoomStore!

    override func setUp() async throws {
        persistence = Persistence(inMemory: true)
        store = RoomStore(persistence: persistence)
    }

    private func photo(day: Int) -> ImportedMedia {
        let date = Calendar.current.date(from: DateComponents(year: 2024, month: 8, day: day, hour: 18))!
        return ImportedMedia(data: Data([0xFF, 0xD8, 0xFF]), thumbnail: nil, isVideo: false, capturedAt: date, pixelSize: CGSize(width: 10, height: 10), duration: 0, fileExtension: "jpg", photoIdentifier: nil)
    }

    func testModelLoadsWithoutCloudAndGroupsImportsByDay() {
        XCTAssertFalse(persistence.cloudEnabled)
        let room = store.createRoom(named: "The Alderhouse")
        _ = store.add(photo(day: 9), to: room)
        _ = store.add(photo(day: 9), to: room)
        _ = store.add(photo(day: 10), to: room)
        store.save()
        XCTAssertEqual(room.moments?.count, 2)
        let aug9 = room.momentList.first!
        XCTAssertEqual(aug9.photoCount, 2)
        XCTAssertEqual(aug9.displayTitle, Catalog.defaultMomentTitle(aug9.date!))
        XCTAssertEqual(store.currentRoom, room)
    }

    func testPeopleTagsNotesLikesAndStories() {
        let room = store.createRoom(named: "Room")
        let item = store.add(photo(day: 9), to: room)
        store.save()
        let moment = item.moment!
        let iris = store.createPerson(named: "Iris", role: "Daughter", in: room)
        store.toggle(iris, in: moment)
        XCTAssertEqual(iris.photoCount, 1)
        XCTAssertEqual(moment.summary.people.map(\.name), ["Iris"])
        store.toggle(iris, in: moment)
        XCTAssertEqual(iris.photoCount, 0)

        store.addNote("best day", to: moment)
        store.addNote("   ", to: moment)
        XCTAssertEqual(moment.noteList.count, 1)
        store.toggleLike(moment)
        XCTAssertTrue(moment.liked(by: store.identity.authorID))
        store.toggleLike(moment)
        XCTAssertEqual(moment.likeCount, 0)

        let story = store.saveStory(title: "Lake", subtitle: "Aug 09", movie: Data([1, 2, 3]), thumbnail: nil, duration: 18, moment: moment, room: room)
        XCTAssertEqual(room.storyList.first, story)
        XCTAssertEqual(story.sourceMoment, moment)

        let groups = store.contributions(in: room, since: .distantPast)
        XCTAssertEqual(groups.count, 1)
        XCTAssertEqual(groups.first?.photoCount, 1)
        XCTAssertEqual(groups.first?.notes, 1)
    }

    func testFaceGroupPromotionTagsMoments() {
        let room = store.createRoom(named: "Room")
        let item = store.add(photo(day: 9), to: room)
        let group = store.createPerson(named: "Face 1", faceGroup: true, in: room)
        let face = FaceObservation(context: store.context)
        face.id = UUID(); face.media = item; face.person = group
        store.save()
        XCTAssertEqual(group.photoCount, 1)
        store.promote(faceGroup: group, named: "Lucy")
        XCTAssertFalse(group.isFaceGroup)
        XCTAssertEqual(item.moment?.peopleList.first?.displayName, "Lucy")

        let other = store.createPerson(named: "Face 2", faceGroup: true, in: room)
        let face2 = FaceObservation(context: store.context)
        face2.id = UUID(); face2.media = item; face2.person = other
        store.merge(other, into: group)
        XCTAssertEqual(group.faces?.count, 2)
        XCTAssertFalse(room.peopleList.contains(other))
    }

    func testDeletingLastItemRemovesMoment() {
        let room = store.createRoom(named: "Room")
        let item = store.add(photo(day: 9), to: room)
        store.save()
        XCTAssertEqual(room.moments?.count, 1)
        store.delete(item)
        XCTAssertEqual(room.moments?.count, 0)
    }
}
