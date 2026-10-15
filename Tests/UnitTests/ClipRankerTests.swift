import XCTest
@testable import FamilyRoom

final class ClipRankerTests: XCTestCase {
    private func signals(_ offset: Double, video: Bool = false, favorite: Bool = false, faces: Int = 0, largest: Double = 0) -> ClipSignals {
        ClipSignals(id: UUID(), isVideo: video, isFavorite: favorite, faceCount: faces, largestFace: largest, capturedAt: Date(timeIntervalSince1970: offset), duration: video ? 12 : 0)
    }

    func testRankingPrefersFavoritesFacesAndMotionAndKeepsChronology() {
        let plain = signals(1), fav = signals(2, favorite: true), group = signals(3, faces: 3), video = signals(4, video: true), closeup = signals(5, faces: 1, largest: 0.3)
        let ranked = ClipRanker.rank([plain, fav, group, video, closeup], peopleInRoom: 3, limit: 4)
        XCTAssertEqual(ranked.count, 4)
        XCTAssertFalse(ranked.contains { $0.id == plain.id })
        XCTAssertEqual(ranked.map(\.signals.capturedAt), ranked.map(\.signals.capturedAt).sorted())
        XCTAssertEqual(ranked.first { $0.id == fav.id }?.reason, "Favorite")
        XCTAssertEqual(ranked.first { $0.id == group.id }?.reason, "Everyone in frame")
        XCTAssertEqual(ranked.first { $0.id == video.id }?.reason, "Motion")
        XCTAssertEqual(ranked.first { $0.id == closeup.id }?.reason, "Close-up")
    }

    func testUnremarkableClipsAreStillOffered() {
        let ranked = ClipRanker.rank([signals(1), signals(2)], peopleInRoom: 0)
        XCTAssertEqual(ranked.count, 2)
        XCTAssertEqual(ranked.first?.reason, "Fresh")
        XCTAssertEqual(ClipRanker.totalDuration(selected: 6), 18)
    }
}
