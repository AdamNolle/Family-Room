import AVFoundation
import XCTest
@testable import FamilyRoom

/// End-to-end checks of face grouping and film rendering on real photographs.
/// The portraits are demo contact photos that ship inside macOS; they are read in place, never copied
/// into the repository, and the face tests skip on systems that do not have them.
@MainActor
final class MediaPipelineTests: XCTestCase {
    private static let portraits = "/System/Library/PrivateFrameworks/HealthDaemon.framework/Versions/A/Resources"

    private func portrait(_ name: String) throws -> Data {
        let url = URL(fileURLWithPath: "\(Self.portraits)/\(name).png")
        guard FileManager.default.fileExists(atPath: url.path) else { throw XCTSkip("Demo portrait \(name) is not on this system.") }
        return try Data(contentsOf: url)
    }

    private func imported(_ data: Data, day: Int) -> ImportedMedia {
        let date = Calendar.current.date(from: DateComponents(year: 2025, month: 6, day: day, hour: 12))!
        return ImportedMedia(data: data, thumbnail: nil, isVideo: false, capturedAt: date, pixelSize: CGSize(width: 1000, height: 1000), duration: 0, fileExtension: "png", photoIdentifier: nil)
    }

    /// A different encoding of the same photo: smaller, JPEG, slightly cropped.
    private func variant(of data: Data) throws -> Data {
        let image = try XCTUnwrap(MediaImporter.decode(data, maxSide: 760))
        let crop = CGRect(x: image.width / 20, y: image.height / 20, width: image.width * 9 / 10, height: image.height * 9 / 10)
        return try XCTUnwrap(MediaImporter.jpeg(try XCTUnwrap(image.cropping(to: crop)), quality: 0.7))
    }

    func testFindsTheFaceInARealPortrait() async throws {
        let faces = try await FaceScanner.detect(in: try portrait("LaniMartinez"))
        XCTAssertEqual(faces.count, 1)
        XCTAssertNotNil(faces.first?.crop)
        XCTAssertNotNil(faces.first?.featurePrint)
        XCTAssertGreaterThan(faces.first?.confidence ?? 0, 0.8)
    }

    func testScanGroupsOnePersonAndSeparatesAnother() async throws {
        let lani = try portrait("LaniMartinez"), dave = try portrait("DaveKnox")
        let store = RoomStore(persistence: Persistence(inMemory: true))
        let room = store.createRoom(named: "Room")
        let first = store.add(imported(lani, day: 1), to: room)
        let second = store.add(imported(try variant(of: lani), day: 2), to: room)
        let other = store.add(imported(dave, day: 3), to: room)
        store.save()

        await store.scanFaces(in: room)

        XCTAssertTrue(room.mediaList.allSatisfy(\.faceScanned))
        let laniGroup = try XCTUnwrap(first.faces?.first?.person)
        XCTAssertEqual(second.faces?.first?.person, laniGroup, "Two photos of the same person should share a face group.")
        if let daveGroup = other.faces?.first?.person {
            XCTAssertNotEqual(daveGroup, laniGroup, "A different person should get a separate face group.")
        }
        XCTAssertEqual(laniGroup.photoCount, 2)

        // Naming the group tags both moments.
        store.promote(faceGroup: laniGroup, named: "Lani")
        XCTAssertEqual(first.moment?.peopleList.map(\.displayName), ["Lani"])
        XCTAssertEqual(second.moment?.peopleList.map(\.displayName), ["Lani"])

        // A second scan does not reprocess photos it already checked.
        let faceCount = room.mediaList.reduce(0) { $0 + ($1.faces?.count ?? 0) }
        await store.scanFaces(in: room)
        XCTAssertEqual(room.mediaList.reduce(0) { $0 + ($1.faces?.count ?? 0) }, faceCount)
    }

    func testRendersAFilmFromPhotosAndFromVideo() async throws {
        let photoA = DemoSeed.gradientJPEG([0xF3CFA6, 0xC86F52], seed: 1)
        let photoB = DemoSeed.gradientJPEG([0xA9CBD4, 0x3E6A7D], seed: 2)
        var last = 0.0
        let (movie, poster) = try await StoryRenderer.render([
            .init(data: photoA, isVideo: false, fileExtension: "jpg"),
            .init(data: photoB, isVideo: false, fileExtension: "jpg")
        ]) { last = $0 }
        defer { try? FileManager.default.removeItem(at: movie.deletingLastPathComponent()) }
        XCTAssertNotNil(poster)
        XCTAssertEqual(last, 1, accuracy: 0.001)
        let asset = AVURLAsset(url: movie)
        let duration = try await asset.load(.duration).seconds
        XCTAssertEqual(duration, 6, accuracy: 0.25)
        let tracks = try await asset.loadTracks(withMediaType: .video)
        let track = try XCTUnwrap(tracks.first)
        let size = try await track.load(.naturalSize)
        XCTAssertEqual(size.width, StoryRenderer.renderSize.width)
        XCTAssertEqual(size.height, StoryRenderer.renderSize.height)

        // The middle of the first clip should show the warm photo, not the dark letterbox.
        let generator = AVAssetImageGenerator(asset: asset)
        generator.requestedTimeToleranceBefore = .zero; generator.requestedTimeToleranceAfter = .zero
        let (frame, _) = try await generator.image(at: CMTime(seconds: 1.5, preferredTimescale: 600))
        let (r, g, b) = averageColor(frame)
        XCTAssertGreaterThan(r, 0.55, "Frame should be the peach photo")
        XCTAssertGreaterThan(r, b, "Frame should lean warm")
        XCTAssertGreaterThan(g, 0.3)

        // Feed the finished film back in as a video clip: the excerpt path must work too.
        let videoData = try Data(contentsOf: movie)
        let (second, _) = try await StoryRenderer.render([
            .init(data: videoData, isVideo: true, fileExtension: "mp4"),
            .init(data: photoA, isVideo: false, fileExtension: "jpg")
        ]) { _ in }
        defer { try? FileManager.default.removeItem(at: second.deletingLastPathComponent()) }
        let secondDuration = try await AVURLAsset(url: second).load(.duration).seconds
        XCTAssertEqual(secondDuration, 6, accuracy: 0.25)
    }

    func testRenderingNothingFails() async {
        do {
            _ = try await StoryRenderer.render([]) { _ in }
            XCTFail("Rendering without clips should throw.")
        } catch {
            XCTAssertEqual(error.localizedDescription, "Pick at least one clip.")
        }
    }

    private func averageColor(_ image: CGImage) -> (Double, Double, Double) {
        var pixel = [UInt8](repeating: 0, count: 4)
        let ctx = CGContext(data: &pixel, width: 1, height: 1, bitsPerComponent: 8, bytesPerRow: 4, space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        ctx.interpolationQuality = .medium
        ctx.draw(image, in: CGRect(x: 0, y: 0, width: 1, height: 1))
        return (Double(pixel[0]) / 255, Double(pixel[1]) / 255, Double(pixel[2]) / 255)
    }
}
