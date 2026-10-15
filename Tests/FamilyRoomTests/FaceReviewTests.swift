import XCTest
import ImageIO
import UniformTypeIdentifiers
@testable import FamilyRoom

final class FaceReviewTests: XCTestCase {
    func testPlainPhotoYieldsNoFaceCrop() async throws {
        let context = CGContext(
            data: nil,
            width: 320,
            height: 240,
            bitsPerComponent: 8,
            bytesPerRow: 0,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue
        )!
        context.setFillColor(CGColor(gray: 0.94, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 320, height: 240))
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("family-room-blank-face-test.png")
        defer { try? FileManager.default.removeItem(at: url) }
        let destination = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil)!
        CGImageDestinationAddImage(destination, context.makeImage()!, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))

        let faces = try await FamilyIntelligenceService.shared.reviewFaces(in: url, assetId: "blank")
        XCTAssertTrue(faces.isEmpty)
    }

    func testVisionFaceBoundsExpandAndConvertLowerLeftToPixelCoordinates() {
        let crop = FamilyIntelligenceService.pixelCrop(
            for: CGRect(x: 0.2, y: 0.25, width: 0.3, height: 0.4),
            imageSize: CGSize(width: 1_000, height: 800),
            margin: 0.1
        )

        XCTAssertEqual(crop, CGRect(x: 170, y: 247, width: 360, height: 385))
    }

    func testFaceCropClampsToImageAndRejectsTinyRegions() {
        let edgeCrop = FamilyIntelligenceService.pixelCrop(
            for: CGRect(x: 0, y: 0.8, width: 0.12, height: 0.2),
            imageSize: CGSize(width: 640, height: 480)
        )
        XCTAssertNotNil(edgeCrop)
        XCTAssertEqual(edgeCrop?.minX, 0)
        XCTAssertGreaterThanOrEqual(edgeCrop?.minY ?? -1, 0)
        XCTAssertLessThanOrEqual(edgeCrop?.maxX ?? 641, 640)
        XCTAssertLessThanOrEqual(edgeCrop?.maxY ?? 481, 480)

        let tiny = FamilyIntelligenceService.pixelCrop(
            for: CGRect(x: 0.5, y: 0.5, width: 0.01, height: 0.01),
            imageSize: CGSize(width: 640, height: 480)
        )
        XCTAssertNil(tiny)
    }
}
