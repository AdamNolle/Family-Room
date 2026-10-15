import Foundation
import ImageIO
import UniformTypeIdentifiers
import Vision

struct DetectedFace: Identifiable, Sendable {
    let id: String
    let assetId: String
    let confidence: Float
    let crop: Data
}

actor FamilyIntelligenceService {
    static let shared = FamilyIntelligenceService()

    private init() {}

    /// Produces face crops only for a member to review and identify. No embeddings,
    /// names, or face crops are persisted by this service.
    func reviewFaces(in url: URL, assetId: String) async throws -> [DetectedFace] {
        try Task.checkCancellation()
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else {
            throw FaceReviewError.unreadableImage
        }
        let thumbnailOptions: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceThumbnailMaxPixelSize: 1_600,
            kCGImageSourceShouldCacheImmediately: true,
        ]
        guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, thumbnailOptions as CFDictionary) else {
            throw FaceReviewError.unreadableImage
        }

        let request = VNDetectFaceRectanglesRequest()
        try VNImageRequestHandler(cgImage: image, orientation: .up).perform([request])
        let faces = (request.results ?? []).filter {
            $0.confidence >= 0.55 && $0.boundingBox.width >= 0.035 && $0.boundingBox.height >= 0.035
        }

        return try faces.enumerated().compactMap { index, face in
            try Task.checkCancellation()
            guard let cropRect = Self.pixelCrop(for: face.boundingBox, imageSize: CGSize(width: image.width, height: image.height)),
                  let cropped = image.cropping(to: cropRect),
                  let data = Self.jpegData(cropped) else { return nil }
            return DetectedFace(
                id: "\(assetId)-\(index)",
                assetId: assetId,
                confidence: face.confidence,
                crop: data
            )
        }
    }

    /// Vision face boxes use a normalized lower-left origin; CGImage crops use pixels
    /// from the top-left. Expand slightly so the review crop keeps hair and jawline.
    nonisolated static func pixelCrop(for bounds: CGRect, imageSize: CGSize, margin: CGFloat = 0.16) -> CGRect? {
        guard imageSize.width > 0, imageSize.height > 0, bounds.width > 0, bounds.height > 0 else { return nil }
        let expanded = bounds.insetBy(dx: -bounds.width * margin, dy: -bounds.height * margin)
            .intersection(CGRect(x: 0, y: 0, width: 1, height: 1))
        guard !expanded.isNull, expanded.width > 0, expanded.height > 0 else { return nil }
        let crop = CGRect(
            x: expanded.minX * imageSize.width,
            y: (1 - expanded.maxY) * imageSize.height,
            width: expanded.width * imageSize.width,
            height: expanded.height * imageSize.height
        ).integral.intersection(CGRect(origin: .zero, size: imageSize))
        return crop.width >= 24 && crop.height >= 24 ? crop : nil
    }

    private static func jpegData(_ image: CGImage) -> Data? {
        let data = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(data, UTType.jpeg.identifier as CFString, 1, nil) else {
            return nil
        }
        CGImageDestinationAddImage(destination, image, [kCGImageDestinationLossyCompressionQuality: 0.82] as CFDictionary)
        guard CGImageDestinationFinalize(destination) else { return nil }
        return data as Data
    }
}

enum FaceReviewError: LocalizedError {
    case unreadableImage

    var errorDescription: String? {
        switch self {
        case .unreadableImage: "This photo could not be analyzed on this device."
        }
    }
}
