import XCTest
@testable import FamilyRoom

@MainActor
final class DemoSeedTests: XCTestCase {
    func testSeedBuildsAFullRoom() {
        let store = RoomStore(persistence: Persistence(inMemory: true))
        DemoSeed.seed(store)
        let room = store.currentRoom!
        XCTAssertEqual(room.momentList.count, 12)
        XCTAssertEqual(room.peopleList.filter { !$0.isFaceGroup }.count, 5)
        XCTAssertEqual(room.momentList.first(where: \.isFeatured)?.displayTitle, "Dad's 72nd")
        XCTAssertGreaterThan(room.media?.count ?? 0, 30)
        XCTAssertFalse(room.mediaList.first!.data!.isEmpty)
    }
}
