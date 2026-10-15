import XCTest
@testable import FamilyRoom

/// A family library of a few years: 4,000 items across 400 days, 8 people.
/// Checks that what the Hearth and Family Circle compute per render stays fast.
@MainActor
final class ScaleTests: XCTestCase {
    private var store: RoomStore!
    private var room: Room!

    override func setUp() async throws {
        store = RoomStore(persistence: Persistence(inMemory: true))
        room = store.createRoom(named: "Big")
        let people = (0..<8).map { store.createPerson(named: "Person \($0)", in: room) }
        let start = Calendar.current.date(from: DateComponents(year: 2024, month: 1, day: 1))!
        for i in 0..<4_000 {
            let date = start.addingTimeInterval(Double(i / 10) * 86_400 * 2 + Double(i % 10) * 600)
            let item = store.add(ImportedMedia(data: Data([UInt8(i % 255)]), thumbnail: nil, isVideo: i % 9 == 0, capturedAt: date,
                                               pixelSize: CGSize(width: 10, height: 10), duration: 3, fileExtension: "jpg", photoIdentifier: nil), to: room)
            item.addedAt = Date().addingTimeInterval(-Double(i) * 60)
            if i % 10 == 0, let moment = item.moment { moment.people = Set(people.shuffled().prefix(3)) }
        }
        store.save()
    }

    func testHearthSummariesStayFast() {
        XCTAssertEqual(room.moments?.count, 400)
        let started = Date()
        let summaries = room.momentList.map(\.summary)
        let selected: Set<UUID> = [summaries[3].people.first!.id]
        let active = summaries.filter { Catalog.matches($0, selected: selected, query: "person") }
        _ = Catalog.stageMoment(summaries, preferred: nil) { Catalog.matches($0, selected: selected, query: "") }
        let elapsed = Date().timeIntervalSince(started)
        print("Hearth build for 400 moments: \(String(format: "%.3f", elapsed))s")
        XCTAssertFalse(active.isEmpty)
        XCTAssertLessThan(elapsed, 0.25, "Building the Hearth for 400 moments took \(elapsed)s")
    }

    func testSharedThisWeekAndPeopleCountsStayFast() {
        var started = Date()
        let shared = store.contributions(in: room)
        XCTAssertFalse(shared.isEmpty)
        XCTAssertLessThan(Date().timeIntervalSince(started), 0.3, "Shared this week took too long")

        started = Date()
        let counts = room.peopleList.map(\.photoCount)
        XCTAssertGreaterThan(counts.reduce(0, +), 0)
        XCTAssertLessThan(Date().timeIntervalSince(started), 0.3, "People counts took too long")
    }
}
