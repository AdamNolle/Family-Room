import AVFoundation
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers
#if canImport(UIKit)
import UIKit
#else
import AppKit
#endif

/// A photo or video prepared for the library: original bytes plus a small JPEG thumbnail.
struct ImportedMedia {
    var data: Data
    var thumbnail: Data?
    var isVideo: Bool
    var capturedAt: Date
    var pixelSize: CGSize
    var duration: Double
    var fileExtension: String
    var photoIdentifier: String?
}

enum MediaImporter {
    static let thumbnailSide: CGFloat = 640

    static func prepare(data: Data, type: UTType?, fileName: String?, photoIdentifier: String? = nil, fallbackDate: Date = Date()) async throws -> ImportedMedia {
        let ext = (fileName.map { URL(fileURLWithPath: $0).pathExtension } ?? type?.preferredFilenameExtension ?? "").lowercased()
        let isVideo = type?.conforms(to: .movie) == true || type?.conforms(to: .video) == true || ["mov", "mp4", "m4v"].contains(ext)
        if isVideo {
            let tmp = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString).appendingPathExtension(ext.isEmpty ? "mov" : ext)
            try data.write(to: tmp, options: .atomic)
            defer { try? FileManager.default.removeItem(at: tmp) }
            let asset = AVURLAsset(url: tmp)
            let duration = (try? await asset.load(.duration).seconds) ?? 0
            var size = CGSize.zero
            if let track = try? await asset.loadTracks(withMediaType: .video).first {
                let natural = (try? await track.load(.naturalSize)) ?? .zero
                let transform = (try? await track.load(.preferredTransform)) ?? .identity
                size = natural.applying(transform); size = CGSize(width: abs(size.width), height: abs(size.height))
            }
            let created = (try? await asset.load(.creationDate)?.load(.dateValue)) ?? nil
            let thumb = await videoThumbnail(asset: asset)
            return ImportedMedia(data: data, thumbnail: thumb, isVideo: true, capturedAt: created ?? fallbackDate,
                                 pixelSize: size, duration: duration, fileExtension: ext.isEmpty ? "mov" : ext, photoIdentifier: photoIdentifier)
        }
        guard let source = CGImageSourceCreateWithData(data as CFData, nil) else { throw ImportError.unreadable }
        let props = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any] ?? [:]
        let width = (props[kCGImagePropertyPixelWidth] as? CGFloat) ?? 0
        let height = (props[kCGImagePropertyPixelHeight] as? CGFloat) ?? 0
        let captured = exifDate(props) ?? fallbackDate
        let thumb = jpegThumbnail(source: source)
        return ImportedMedia(data: data, thumbnail: thumb, isVideo: false, capturedAt: captured,
                             pixelSize: CGSize(width: width, height: height), duration: 0,
                             fileExtension: ext.isEmpty ? "jpg" : ext, photoIdentifier: photoIdentifier)
    }

    static func prepare(fileURL: URL) async throws -> ImportedMedia {
        let data = try Data(contentsOf: fileURL)
        let type = UTType(filenameExtension: fileURL.pathExtension)
        let attrs = try? FileManager.default.attributesOfItem(atPath: fileURL.path)
        return try await prepare(data: data, type: type, fileName: fileURL.lastPathComponent, fallbackDate: (attrs?[.creationDate] as? Date) ?? Date())
    }

    enum ImportError: LocalizedError {
        case unreadable
        var errorDescription: String? { "This file is not a photo or video Family Room can read." }
    }

    private static func exifDate(_ props: [CFString: Any]) -> Date? {
        let exif = props[kCGImagePropertyExifDictionary] as? [CFString: Any]
        let tiff = props[kCGImagePropertyTIFFDictionary] as? [CFString: Any]
        let raw = (exif?[kCGImagePropertyExifDateTimeOriginal] as? String) ?? (exif?[kCGImagePropertyExifDateTimeDigitized] as? String) ?? (tiff?[kCGImagePropertyTIFFDateTime] as? String)
        guard let raw else { return nil }
        let f = DateFormatter(); f.locale = Locale(identifier: "en_US_POSIX"); f.dateFormat = "yyyy:MM:dd HH:mm:ss"
        return f.date(from: raw)
    }

    static func jpegThumbnail(source: CGImageSource, side: CGFloat = thumbnailSide) -> Data? {
        let options: [CFString: Any] = [kCGImageSourceCreateThumbnailFromImageAlways: true,
                                        kCGImageSourceThumbnailMaxPixelSize: side,
                                        kCGImageSourceCreateThumbnailWithTransform: true]
        guard let cg = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else { return nil }
        return jpeg(cg)
    }

    static func jpeg(_ image: CGImage, quality: CGFloat = 0.82) -> Data? {
        let data = NSMutableData()
        guard let dest = CGImageDestinationCreateWithData(data, UTType.jpeg.identifier as CFString, 1, nil) else { return nil }
        CGImageDestinationAddImage(dest, image, [kCGImageDestinationLossyCompressionQuality: quality] as CFDictionary)
        return CGImageDestinationFinalize(dest) ? data as Data : nil
    }

    static func videoThumbnail(asset: AVAsset, at seconds: Double = 0.5) async -> Data? {
        let generator = AVAssetImageGenerator(asset: asset)
        generator.appliesPreferredTrackTransform = true
        generator.maximumSize = CGSize(width: thumbnailSide, height: thumbnailSide)
        let time = CMTime(seconds: seconds, preferredTimescale: 600)
        guard let (image, _) = try? await generator.image(at: time) else { return nil }
        return jpeg(image)
    }

    /// Decodes a CGImage from original or thumbnail bytes, downsampled to `maxSide`.
    static func decode(_ data: Data, maxSide: CGFloat) -> CGImage? {
        guard let source = CGImageSourceCreateWithData(data as CFData, nil) else { return nil }
        let options: [CFString: Any] = [kCGImageSourceCreateThumbnailFromImageAlways: true,
                                        kCGImageSourceThumbnailMaxPixelSize: maxSide,
                                        kCGImageSourceCreateThumbnailWithTransform: true,
                                        kCGImageSourceShouldCacheImmediately: true]
        return CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary)
    }
}
