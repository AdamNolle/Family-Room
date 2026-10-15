import XCTest
@testable import FamilyRoom

final class FilmEditingTests: XCTestCase {
    func testShorteningKeepsInterpolatedAutomationUntilTheCut() throws {
        let clip = FilmClip(assetId: "video", duration: 8, opacityKeyframes: [FilmKeyframe(time: 0, value: 0), FilmKeyframe(time: 8, value: 1)], volumeKeyframes: [FilmKeyframe(time: 2, value: 0.2), FilmKeyframe(time: 6, value: 0.8)])
        let short = clip.resized(to: 4)
        XCTAssertEqual(short.opacityKeyframes.last?.value, 0.5)
        XCTAssertEqual(try XCTUnwrap(short.volumeKeyframes.last).value, 0.5, accuracy: 0.000001)
        for step in 0...40 {
            let time = Double(step) / 10
            XCTAssertEqual(short.value(at: time, keyframes: short.opacityKeyframes, fallback: 1), clip.value(at: time, keyframes: clip.opacityKeyframes, fallback: 1), accuracy: 0.000001)
            XCTAssertEqual(short.value(at: time, keyframes: short.volumeKeyframes, fallback: 1), clip.value(at: time, keyframes: clip.volumeKeyframes, fallback: 1), accuracy: 0.000001)
        }
        XCTAssertEqual(short.resized(to: 6).opacityKeyframes, short.opacityKeyframes)
        XCTAssertEqual(clip.resized(to: .nan), clip)
    }

    func testFormatGuardsAcceptLegacyAndPreserveFutureProjects() throws {
        let project = FilmProject(roomId: "room")
        var value = try XCTUnwrap(JSONSerialization.jsonObject(with: LibraryCoding.encoder.encode(project)) as? [String: Any])
        XCTAssertEqual(value["format_version"] as? Int, 1)
        value.removeValue(forKey: "format_version")
        let legacy = try LibraryCoding.decoder.decode(FilmProject.self, from: JSONSerialization.data(withJSONObject: value))
        XCTAssertTrue(legacy.supportsEditing)
        value["format_version"] = 2
        let future = try LibraryCoding.decoder.decode(FilmProject.self, from: JSONSerialization.data(withJSONObject: value))
        XCTAssertFalse(future.supportsEditing)
        XCTAssertEqual(future.formatVersion, 2)
    }
    func testRepeatedSplitsPreserveTimingAutomationAndOverlappingFades() throws {
        let clip = FilmClip(assetId: "video", track: 2, start: 7, trimIn: 5, duration: 8, speed: 2,
            volume: 0.8, opacity: 0.7, fadeIn: 3, fadeOut: 3,
            opacityKeyframes: [FilmKeyframe(time: 1, value: 0.2), FilmKeyframe(time: 3, value: 0.9), FilmKeyframe(time: 7, value: 0.4)],
            volumeKeyframes: [FilmKeyframe(time: 0, value: 0.1), FilmKeyframe(time: 5, value: 0.8), FilmKeyframe(time: 8, value: 0.3)])
        let halves = try XCTUnwrap(clip.split(at: 4))
        let quarters = try XCTUnwrap(halves.right.split(at: 2))
        let pieces = [halves.left, quarters.left, quarters.right]
        XCTAssertEqual(halves.left.id, clip.id)
        XCTAssertEqual(Set(pieces.map(\.id)).count, 3)
        XCTAssertEqual(pieces.last?.end, clip.end)
        XCTAssertEqual(pieces.map(\.trimIn), [5, 9, 11])
        XCTAssertEqual(pieces.map(\.start), [7, 9, 10])
        for piece in pieces {
            XCTAssertTrue((piece.opacityKeyframes + piece.volumeKeyframes).allSatisfy { $0.time >= 0 && $0.time <= piece.duration })
            for step in 0...100 {
                let local = piece.duration / piece.speed * Double(step) / 100
                let original = piece.start - clip.start + local
                XCTAssertEqual(piece.opacity(at: local), clip.opacity(at: original), accuracy: 0.0000001)
                XCTAssertEqual(piece.audioGain(at: local), clip.audioGain(at: original), accuracy: 0.0000001)
            }
        }
        let project = FilmProject(roomId: "room", clips: pieces)
        XCTAssertEqual(try LibraryCoding.decoder.decode(FilmProject.self, from: LibraryCoding.encoder.encode(project)), project)
        XCTAssertNil(clip.split(at: 0))
        XCTAssertNil(clip.split(at: clip.duration))
        XCTAssertNil(clip.split(at: .nan))
        let constant = FilmClip(assetId: "photo", duration: 4, volume: 0.4, opacity: 0.6)
        let split = try XCTUnwrap(constant.split(at: 2))
        XCTAssertTrue(split.right.opacityKeyframes.isEmpty)
        XCTAssertTrue(split.left.volumeKeyframes.isEmpty)
        XCTAssertEqual(split.right.audioGain(at: 1), 0.4)
    }

    func testAudioRampsComposeKeyframesAndFadesInsteadOfReplacingThem() {
        let clip = FilmClip(assetId: "audio", duration: 4, speed: 2, fadeIn: 1.5, fadeOut: 1.5,
            volumeKeyframes: [FilmKeyframe(time: 0, value: 0.2), FilmKeyframe(time: 1.4, value: 0.8), FilmKeyframe(time: 4, value: 0.1)])
        let points = FilmRenderer.audioAutomation(clip, outputDuration: 2)
        XCTAssertEqual(points.first?.time, 0)
        XCTAssertEqual(points.last?.time, 2)
        XCTAssertTrue(zip(points, points.dropFirst()).allSatisfy { $0.time < $1.time })
        for step in 0...1000 {
            let time = Double(step) / 500
            let approximated = clip.value(at: time, keyframes: points, fallback: 0)
            XCTAssertEqual(approximated, clip.audioGain(at: time), accuracy: 0.0015)
        }
        XCTAssertLessThan(points.count, 200, "A smooth four-second envelope should not require a ramp for every audio sample.")
    }
}
