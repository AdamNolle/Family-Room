import AVFoundation
import CoreGraphics
import Foundation
import ImageIO

/// Stitches photos and video excerpts into a short MP4, three seconds per clip. Everything runs on device.
enum StoryRenderer {
    static let renderSize = CGSize(width: 1920, height: 1080)
    static let fps: Int32 = 30

    struct Clip {
        var data: Data
        var isVideo: Bool
        var fileExtension: String
    }

    enum RenderError: LocalizedError {
        case noClips, exportFailed(String)
        var errorDescription: String? {
            switch self {
            case .noClips: return "Pick at least one clip."
            case .exportFailed(let why): return why
            }
        }
    }

    static func render(_ clips: [Clip], secondsPerClip: Double = ClipRanker.clipSeconds, progress: @escaping (Double) -> Void) async throws -> (movie: URL, poster: Data?) {
        guard !clips.isEmpty else { throw RenderError.noClips }
        let work = FileManager.default.temporaryDirectory.appendingPathComponent("story-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: work, withIntermediateDirectories: true)
        let composition = AVMutableComposition()
        guard let videoTrack = composition.addMutableTrack(withMediaType: .video, preferredTrackID: kCMPersistentTrackID_Invalid) else { throw RenderError.exportFailed("Cannot create the video track.") }
        let audioTrack = composition.addMutableTrack(withMediaType: .audio, preferredTrackID: kCMPersistentTrackID_Invalid)
        var cursor = CMTime.zero
        var instructions: [AVMutableVideoCompositionInstruction] = []
        let clipDuration = CMTime(seconds: secondsPerClip, preferredTimescale: 600)
        var poster: Data?
        for (index, clip) in clips.enumerated() {
            try Task.checkCancellation()
            let url = work.appendingPathComponent("clip-\(index).\(clip.isVideo ? clip.fileExtension : "mp4")")
            if clip.isVideo {
                try clip.data.write(to: url)
            } else {
                guard let image = MediaImporter.decode(clip.data, maxSide: 2200) else { continue }
                if poster == nil { poster = MediaImporter.jpeg(image) }
                try await writeStill(image, duration: secondsPerClip, destination: url)
            }
            let asset = AVURLAsset(url: url)
            guard let source = try await asset.loadTracks(withMediaType: .video).first else { continue }
            let total = try await asset.load(.duration)
            let start = clip.isVideo ? CMTime(seconds: max(0, (total.seconds - secondsPerClip) / 2), preferredTimescale: 600) : .zero
            let range = CMTimeRange(start: start, duration: min(clipDuration, total - start))
            try videoTrack.insertTimeRange(range, of: source, at: cursor)
            if clip.isVideo, poster == nil { poster = await MediaImporter.videoThumbnail(asset: asset, at: start.seconds + 0.3) }
            if clip.isVideo, let audioTrack, let sourceAudio = try await asset.loadTracks(withMediaType: .audio).first {
                try? audioTrack.insertTimeRange(range, of: sourceAudio, at: cursor)
            }
            let instruction = AVMutableVideoCompositionInstruction()
            instruction.timeRange = CMTimeRange(start: cursor, duration: range.duration)
            let layer = AVMutableVideoCompositionLayerInstruction(assetTrack: videoTrack)
            let natural = try await source.load(.naturalSize), preferred = try await source.load(.preferredTransform)
            let oriented = natural.applying(preferred)
            let size = CGSize(width: abs(oriented.width), height: abs(oriented.height))
            let scale = min(renderSize.width / size.width, renderSize.height / size.height)
            let fit = CGAffineTransform(scaleX: scale, y: scale)
            let offset = CGAffineTransform(translationX: (renderSize.width - size.width * scale) / 2 - min(0, oriented.width) * scale,
                                           y: (renderSize.height - size.height * scale) / 2 - min(0, oriented.height) * scale)
            layer.setTransform(preferred.concatenating(fit).concatenating(offset), at: cursor)
            // Short fades keep cuts soft.
            let fade = CMTime(seconds: 0.25, preferredTimescale: 600)
            layer.setOpacityRamp(fromStartOpacity: 0, toEndOpacity: 1, timeRange: CMTimeRange(start: cursor, duration: fade))
            layer.setOpacityRamp(fromStartOpacity: 1, toEndOpacity: 0, timeRange: CMTimeRange(start: cursor + range.duration - fade, duration: fade))
            instruction.layerInstructions = [layer]
            instructions.append(instruction)
            cursor = cursor + range.duration
            progress(Double(index + 1) / Double(clips.count) * 0.6)
        }
        guard cursor > .zero else { throw RenderError.noClips }
        let videoComposition = AVMutableVideoComposition()
        videoComposition.renderSize = renderSize
        videoComposition.frameDuration = CMTime(value: 1, timescale: fps)
        videoComposition.instructions = instructions
        let output = work.appendingPathComponent("story.mp4")
        guard let export = AVAssetExportSession(asset: composition, presetName: AVAssetExportPreset1920x1080) else { throw RenderError.exportFailed("Movie export is unavailable on this device.") }
        export.outputURL = output; export.outputFileType = .mp4; export.videoComposition = videoComposition; export.shouldOptimizeForNetworkUse = true
        let poll = Task { while !Task.isCancelled { progress(0.6 + Double(export.progress) * 0.4); try? await Task.sleep(nanoseconds: 120_000_000) } }
        await export.export()
        poll.cancel()
        guard export.status == .completed else { throw RenderError.exportFailed(export.error?.localizedDescription ?? "The movie did not finish exporting.") }
        progress(1)
        return (output, poster)
    }

    private static func writeStill(_ image: CGImage, duration: Double, destination: URL) async throws {
        let size = renderSize
        let writer = try AVAssetWriter(outputURL: destination, fileType: .mp4)
        let input = AVAssetWriterInput(mediaType: .video, outputSettings: [AVVideoCodecKey: AVVideoCodecType.h264, AVVideoWidthKey: Int(size.width), AVVideoHeightKey: Int(size.height)])
        let adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input, sourcePixelBufferAttributes: [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32ARGB,
            kCVPixelBufferWidthKey as String: Int(size.width), kCVPixelBufferHeightKey as String: Int(size.height)])
        writer.add(input)
        guard writer.startWriting() else { throw writer.error ?? RenderError.exportFailed("Cannot start encoding a photo.") }
        writer.startSession(atSourceTime: .zero)
        guard let pool = adaptor.pixelBufferPool else { throw RenderError.exportFailed("No pixel buffer pool.") }
        let frames = max(1, Int(duration * Double(fps)))
        let imageAspect = CGFloat(image.width) / CGFloat(image.height)
        for frame in 0..<frames {
            try Task.checkCancellation()
            var maybe: CVPixelBuffer?
            guard CVPixelBufferPoolCreatePixelBuffer(nil, pool, &maybe) == kCVReturnSuccess, let buffer = maybe else { throw RenderError.exportFailed("Cannot allocate a frame.") }
            CVPixelBufferLockBaseAddress(buffer, [])
            if let ctx = CGContext(data: CVPixelBufferGetBaseAddress(buffer), width: Int(size.width), height: Int(size.height), bitsPerComponent: 8,
                                   bytesPerRow: CVPixelBufferGetBytesPerRow(buffer), space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipFirst.rawValue) {
                ctx.setFillColor(CGColor(red: 0.11, green: 0.09, blue: 0.07, alpha: 1)); ctx.fill(CGRect(origin: .zero, size: size))
                // A slow push-in so stills breathe.
                let t = Double(frame) / Double(max(1, frames - 1))
                let zoom = 1.0 + 0.06 * t
                var drawSize = imageAspect >= size.width / size.height
                    ? CGSize(width: size.width, height: size.width / imageAspect)
                    : CGSize(width: size.height * imageAspect, height: size.height)
                drawSize = CGSize(width: drawSize.width * zoom, height: drawSize.height * zoom)
                let rect = CGRect(x: (size.width - drawSize.width) / 2, y: (size.height - drawSize.height) / 2, width: drawSize.width, height: drawSize.height)
                ctx.draw(image, in: rect)
            }
            CVPixelBufferUnlockBaseAddress(buffer, [])
            while !input.isReadyForMoreMediaData { try await Task.sleep(nanoseconds: 2_000_000) }
            guard adaptor.append(buffer, withPresentationTime: CMTime(value: CMTimeValue(frame), timescale: fps)) else { throw writer.error ?? RenderError.exportFailed("Photo encoding failed.") }
        }
        input.markAsFinished()
        await writer.finishWriting()
        guard writer.status == .completed else { throw writer.error ?? RenderError.exportFailed("Photo encoding failed.") }
    }
}
