import ImageIO
import UniformTypeIdentifiers
import XCTest
@testable import FamilyRoom

/// The path every photo and video takes into a Room.
final class ImportTests: XCTestCase {
    /// A JPEG carrying a camera capture date in its EXIF data.
    private func jpeg(capturedAt stamp: String) throws -> Data {
        let image = try XCTUnwrap(MediaImporter.decode(DemoSeed.gradientJPEGSync([0xA9CBD4, 0x3E6A7D], seed: 3), maxSide: 900))
        let data = NSMutableData()
        let dest = try XCTUnwrap(CGImageDestinationCreateWithData(data, UTType.jpeg.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(dest, image, [kCGImagePropertyExifDictionary: [kCGImagePropertyExifDateTimeOriginal: stamp]] as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(dest))
        return data as Data
    }

    func testPhotoKeepsItsCameraDateAndGetsAThumbnail() async throws {
        let data = try jpeg(capturedAt: "2023:07:04 18:30:00")
        let imported = try await MediaImporter.prepare(data: data, type: .jpeg, fileName: "IMG_0001.JPG")
        XCTAssertFalse(imported.isVideo)
        XCTAssertEqual(imported.fileExtension, "jpg")
        let parts = Calendar.current.dateComponents([.year, .month, .day, .hour, .minute], from: imported.capturedAt)
        XCTAssertEqual([parts.year, parts.month, parts.day, parts.hour, parts.minute], [2023, 7, 4, 18, 30])
        XCTAssertEqual(imported.pixelSize, CGSize(width: 900, height: 600))
        let thumb = try XCTUnwrap(imported.thumbnail.flatMap { MediaImporter.decode($0, maxSide: 2000) })
        XCTAssertLessThanOrEqual(max(thumb.width, thumb.height), Int(MediaImporter.thumbnailSide))
    }

    func testPhotoWithoutADateUsesTheFallback() async throws {
        let fallback = Date(timeIntervalSince1970: 1_700_000_000)
        let imported = try await MediaImporter.prepare(data: DemoSeed.gradientJPEGSync([0xF3CFA6, 0xC86F52], seed: 1), type: .jpeg, fileName: nil, fallbackDate: fallback)
        XCTAssertEqual(imported.capturedAt, fallback)
    }

    func testVideoIsMeasuredAndGetsAPoster() async throws {
        let (movie, _) = try await StoryRenderer.render([.init(data: DemoSeed.gradientJPEGSync([0xF2A65A, 0xB4493A], seed: 2), isVideo: false, fileExtension: "jpg")]) { _ in }
        defer { try? FileManager.default.removeItem(at: movie.deletingLastPathComponent()) }
        let imported = try await MediaImporter.prepare(data: try Data(contentsOf: movie), type: .mpeg4Movie, fileName: "clip.mp4")
        XCTAssertTrue(imported.isVideo)
        XCTAssertEqual(imported.fileExtension, "mp4")
        XCTAssertEqual(imported.duration, 3, accuracy: 0.2)
        XCTAssertEqual(imported.pixelSize, StoryRenderer.renderSize)
        XCTAssertNotNil(imported.thumbnail)
    }

    func testUnreadableFilesAreRejectedWithAPlainMessage() async {
        do {
            _ = try await MediaImporter.prepare(data: Data("not a photo".utf8), type: .jpeg, fileName: "note.jpg")
            XCTFail("A text file should not import as a photo")
        } catch {
            XCTAssertEqual(error.localizedDescription, "This file is not a photo or video Family Room can read.")
        }
    }
}
