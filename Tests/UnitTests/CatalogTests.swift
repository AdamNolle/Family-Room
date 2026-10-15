import XCTest
@testable import FamilyRoom

final class CatalogTests: XCTestCase {
    private func person(_ name: String) -> PersonSummary {
        PersonSummary(id: UUID(), name: name, role: "", toneHex: "C97A5D", accentHex: "F0BFA8", isFaceGroup: false, photoCount: 0)
    }
    private func moment(_ title: String, _ month: Int, _ day: Int, people: [PersonSummary] = [], place: String? = nil, photos: Int = 3, videos: Int = 0, featured: Bool = false) -> MomentSummary {
        let date = Calendar.current.date(from: DateComponents(year: 2024, month: month, day: day))!
        return MomentSummary(id: UUID(), title: title, date: date, place: place, quote: nil, isFeatured: featured, people: people, photoCount: photos, videoCount: videos)
    }

    func testMatchesByPersonTitlePlaceAndMonth() {
        let iris = person("Iris"), henry = person("Henry")
        let m = moment("Iris's first bike ride", 3, 14, people: [iris, henry], place: "Our backyard")
        XCTAssertTrue(Catalog.matches(m, selected: [], query: ""))
        XCTAssertTrue(Catalog.matches(m, selected: [iris.id], query: ""))
        XCTAssertFalse(Catalog.matches(m, selected: [UUID()], query: ""))
        XCTAssertTrue(Catalog.matches(m, selected: [], query: "bike"))
        XCTAssertTrue(Catalog.matches(m, selected: [], query: "backyard"))
        XCTAssertTrue(Catalog.matches(m, selected: [], query: "henry"))
        XCTAssertTrue(Catalog.matches(m, selected: [], query: "mar"))
        XCTAssertFalse(Catalog.matches(m, selected: [], query: "lake"))
    }

    func testStagePrefersPinnedThenFeaturedThenNewest() {
        let a = moment("A", 1, 2), b = moment("B", 5, 5, featured: true), c = moment("C", 9, 9)
        let all = [a, b, c]
        XCTAssertEqual(Catalog.stageMoment(all, preferred: a.id, matching: { _ in true })?.id, a.id)
        XCTAssertEqual(Catalog.stageMoment(all, preferred: nil, matching: { _ in true })?.id, b.id)
        XCTAssertEqual(Catalog.stageMoment(all, preferred: nil, matching: { $0.id != b.id })?.id, c.id)
        XCTAssertNil(Catalog.stageMoment(all, preferred: nil, matching: { _ in false }))
    }

    func testRiverPositionsAreMonotonic() {
        let dates = [moment("A", 1, 1), moment("B", 6, 15), moment("C", 12, 31)].map(\.date)
        let fractions = dates.map { Catalog.yearFraction($0) }
        XCTAssertEqual(fractions[0], 0, accuracy: 0.0001)
        XCTAssertEqual(fractions[2], 1, accuracy: 0.0001)
        XCTAssertTrue(fractions[0] < fractions[1] && fractions[1] < fractions[2])
        XCTAssertEqual(Catalog.monthMarks(year: 2024).count, 12)
        XCTAssertEqual(Catalog.monthMarks(year: 2024, skip: 2).count, 10)
    }

    func testNearestRespectsFilter() {
        let a = moment("A", 2, 1), b = moment("B", 8, 1)
        XCTAssertEqual(Catalog.nearest([a, b], to: 0.55, matching: { _ in true })?.id, b.id)
        XCTAssertEqual(Catalog.nearest([a, b], to: 0.55, matching: { $0.id == a.id })?.id, a.id)
    }

    func testLabelsAndFormatting() {
        XCTAssertEqual(Catalog.pad(7), "07")
        XCTAssertEqual(Catalog.pad(12), "12")
        XCTAssertEqual(Catalog.duration(185), "3:05")
        XCTAssertEqual(Catalog.duration(48), "0:48")
        XCTAssertEqual(Catalog.kicker(selectedNames: [], query: "lake", year: 2024), "Matching “lake”")
        XCTAssertEqual(Catalog.kicker(selectedNames: ["Cora", "Iris"], query: "", year: 2024), "With Cora and Iris")
        XCTAssertEqual(Catalog.kicker(selectedNames: [], query: "", year: Calendar.current.component(.year, from: Date())), "This year · everyone")
        XCTAssertEqual(moment("x", 1, 1, photos: 0).weight, 1)
        XCTAssertEqual(moment("x", 1, 1, photos: 61, videos: 3).weight, 5)
    }

    func testContributionMessage() {
        var c = ContributionSummary(id: "k", momentID: UUID(), authorName: "Iris", authorInitial: "I", photoCount: 4, videoCount: 0, latest: Date(), likes: 0, notes: 0)
        XCTAssertEqual(c.message, "Iris added 4 photos")
        c.photoCount = 0; c.videoCount = 1
        XCTAssertEqual(c.message, "Iris added a video")
        c.photoCount = 1
        XCTAssertEqual(c.message, "Iris added 1 photo and a video")
    }
}
