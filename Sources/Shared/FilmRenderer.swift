import Foundation
import AVFoundation
import CoreImage
import CoreText
import ImageIO

enum FilmRenderError: LocalizedError {
    case invalid(String)
    var errorDescription: String? { if case .invalid(let text) = self { return text }; return nil }
}
private struct RenderLayer {
    let clip: FilmClip
    let trackId: CMPersistentTrackID
    let transform: CGAffineTransform
}
private final class FilmInstruction: NSObject, AVVideoCompositionInstructionProtocol {
    let timeRange: CMTimeRange
    let enablePostProcessing = false
    let containsTweening = true
    let passthroughTrackID = kCMPersistentTrackID_Invalid
    var requiredSourceTrackIDs: [NSValue]? { layers.map { NSNumber(value: $0.trackId) } }
    let layers: [RenderLayer]
    init(duration: CMTime, layers: [RenderLayer]) { timeRange = CMTimeRange(start: .zero, duration: duration); self.layers = layers }
}
final class FamilyFilmCompositor: NSObject, AVVideoCompositing {
    var sourcePixelBufferAttributes: [String: any Sendable]? { [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA] }
    var requiredPixelBufferAttributesForRenderContext: [String: any Sendable] { [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA] }
    private let queue = DispatchQueue(label: "com.familyroom.film-compositor")
    private let context = CIContext(options: [.cacheIntermediates: false])
    func renderContextChanged(_ newRenderContext: AVVideoCompositionRenderContext) {}
    func startRequest(_ request: AVAsynchronousVideoCompositionRequest) {
        queue.async {
            guard let instruction = request.videoCompositionInstruction as? FilmInstruction, let buffer = request.renderContext.newPixelBuffer() else {
                request.finish(with: FilmRenderError.invalid("Cannot allocate the movie frame.")); return
            }
            let time = request.compositionTime.seconds
            let rect = CGRect(origin: .zero, size: request.renderContext.size)
            var canvas = CIImage(color: .black).cropped(to: rect)
            for layer in instruction.layers.sorted(by: { $0.clip.track < $1.clip.track }) {
                let clip = layer.clip
                guard time >= clip.start, time < clip.end, let source = request.sourceFrame(byTrackID: layer.trackId) else { continue }
                var image = CIImage(cvPixelBuffer: source).transformed(by: layer.transform)
                let extent = image.extent
                guard extent.width > 0, extent.height > 0 else { continue }
                image = image.transformed(by: CGAffineTransform(translationX: -extent.minX, y: -extent.minY))
                let scale = min(rect.width / extent.width, rect.height / extent.height)
                image = image.transformed(by: CGAffineTransform(scaleX: scale, y: scale)).transformed(by: CGAffineTransform(translationX: (rect.width - extent.width * scale) / 2, y: (rect.height - extent.height * scale) / 2))
                image = image.applyingFilter("CIExposureAdjust", parameters: [kCIInputEVKey: clip.exposure]).applyingFilter("CIColorControls", parameters: [kCIInputSaturationKey: clip.saturation])
                let local = time - clip.start
                let alpha = clip.opacity(at: local)
                image = image.applyingFilter("CIColorMatrix", parameters: ["inputAVector": CIVector(x: 0, y: 0, z: 0, w: min(1, max(0, alpha)))])
                canvas = image.composited(over: canvas)
                if !clip.title.isEmpty, let overlay = Self.titleImage(clip.title, size: rect.size) { canvas = overlay.composited(over: canvas) }
            }
            self.context.render(canvas, to: buffer, bounds: rect, colorSpace: CGColorSpaceCreateDeviceRGB())
            request.finish(withComposedVideoFrame: buffer)
        }
    }
    func cancelAllPendingVideoCompositionRequests() { queue.sync {} }
    private static func titleImage(_ text: String, size: CGSize) -> CIImage? {
        guard let ctx = CGContext(data: nil, width: Int(size.width), height: Int(size.height), bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        let attributes: [NSAttributedString.Key: Any] = [NSAttributedString.Key(kCTFontAttributeName as String): CTFontCreateWithName("Helvetica-Bold" as CFString, max(16, size.height * 0.04), nil), NSAttributedString.Key(kCTForegroundColorAttributeName as String): CGColor(gray: 1, alpha: 1)]
        let line = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: attributes))
        ctx.setFillColor(CGColor(gray: 0, alpha: 0.55)); ctx.fill(CGRect(x: size.width * 0.05, y: size.height * 0.06, width: size.width * 0.9, height: size.height * 0.09))
        ctx.textPosition = CGPoint(x: size.width * 0.07, y: size.height * 0.085); CTLineDraw(line, ctx)
        guard let image = ctx.makeImage() else { return nil }; return CIImage(cgImage: image)
    }
}
enum FilmRenderer {
    private static func time(_ seconds: Double) -> CMTime { CMTime(seconds: seconds, preferredTimescale: 600) }
    static func render(_ project: FilmProject, assets: [LibraryAsset], urls: [String: URL], destination: URL) async throws {
        guard project.supportsEditing else { throw FilmRenderError.invalid("Update Family Room before rendering this film’s project format.") }
        guard !project.clips.isEmpty, project.duration > 0, project.duration.isFinite, project.width > 0, project.height > 0 else { throw FilmRenderError.invalid("Add a valid clip before rendering.") }
        let composition = AVMutableComposition()
        var layers: [RenderLayer] = [], audioParameters: [AVAudioMixInputParameters] = []
        var scratch: [URL] = []
        defer { for url in scratch { try? FileManager.default.removeItem(at: url) } }
        for clip in project.clips {
            guard clip.start >= 0, clip.trimIn >= 0, clip.duration > 0, clip.speed > 0, let originalURL = urls[clip.assetId], let model = assets.first(where: { $0.id == clip.assetId }) else { throw FilmRenderError.invalid("A timeline original is unavailable.") }
            var url = originalURL
            if model.kind == "photo" {
                url = destination.deletingLastPathComponent().appendingPathComponent("Still-\(UUID().uuidString).mp4")
                scratch.append(url)
                try await writeStill(originalURL, duration: clip.trimIn + clip.duration, size: CGSize(width: project.width, height: project.height), destination: url)
            }
            let source = AVURLAsset(url: url)
            let duration = try await source.load(.duration).seconds
            guard clip.trimIn < duration else { throw FilmRenderError.invalid("Trim starts beyond the end of a source clip.") }
            let available = min(clip.duration, duration - clip.trimIn)
            let range = CMTimeRange(start: time(clip.trimIn), duration: time(available))
            let start = time(clip.start)
            if let video = try await source.loadTracks(withMediaType: .video).first {
                guard let target = composition.addMutableTrack(withMediaType: .video, preferredTrackID: kCMPersistentTrackID_Invalid) else { throw FilmRenderError.invalid("Cannot create a video layer.") }
                try target.insertTimeRange(range, of: video, at: start)
                target.scaleTimeRange(CMTimeRange(start: start, duration: range.duration), toDuration: time(available / clip.speed))
                layers.append(RenderLayer(clip: clip, trackId: target.trackID, transform: try await video.load(.preferredTransform)))
            }
            if let audio = try await source.loadTracks(withMediaType: .audio).first {
                guard let target = composition.addMutableTrack(withMediaType: .audio, preferredTrackID: kCMPersistentTrackID_Invalid) else { throw FilmRenderError.invalid("Cannot create an audio layer.") }
                try target.insertTimeRange(range, of: audio, at: start)
                target.scaleTimeRange(CMTimeRange(start: start, duration: range.duration), toDuration: time(available / clip.speed))
                let parameters = AVMutableAudioMixInputParameters(track: target)
                let points = audioAutomation(clip, outputDuration: available / clip.speed)
                parameters.setVolume(Float(clip.audioGain(at: 0)), at: .zero)
                for (a, b) in zip(points, points.dropFirst()) {
                    parameters.setVolumeRamp(fromStartVolume: Float(a.value), toEndVolume: Float(b.value),
                        timeRange: CMTimeRange(start: time(clip.start + a.time), duration: time(b.time - a.time)))
                }
                audioParameters.append(parameters)
            }
        }
        guard !layers.isEmpty else { throw FilmRenderError.invalid("A film needs at least one photo or video layer.") }
        let videoComposition = AVMutableVideoComposition()
        videoComposition.customVideoCompositorClass = FamilyFilmCompositor.self
        videoComposition.renderSize = CGSize(width: project.width, height: project.height)
        videoComposition.frameDuration = time(1/30)
        videoComposition.instructions = [FilmInstruction(duration: time(project.duration), layers: layers)]
        let audioMix = AVMutableAudioMix(); audioMix.inputParameters = audioParameters
        guard let export = AVAssetExportSession(asset: composition, presetName: AVAssetExportPresetHighestQuality) else { throw FilmRenderError.invalid("Movie export is unavailable.") }
        try? FileManager.default.removeItem(at: destination)
        export.outputURL = destination; export.outputFileType = .mp4; export.videoComposition = videoComposition; export.audioMix = audioMix; export.timeRange = CMTimeRange(start: .zero, duration: time(project.duration)); export.shouldOptimizeForNetworkUse = true
        await export.export()
        guard export.status == .completed else { try? FileManager.default.removeItem(at: destination); throw export.error ?? FilmRenderError.invalid("Movie export did not complete.") }
        // Portable chapters also accompany standalone exports, without altering originals.
        if !project.chapters.isEmpty { let data = try LibraryCoding.encoder.encode(project.chapters); try data.write(to: destination.appendingPathExtension("chapters.json"), options: .atomic) }
    }
    /// Combine gain automation and fades before creating AVFoundation ramps.
    /// Adaptive subdivisions retain their product rather than overwriting one
    /// set of ramps with another. Quarter-point checks also catch cubic curves.
    static func audioAutomation(_ clip: FilmClip, outputDuration: Double) -> [FilmKeyframe] {
        var boundaries = [0.0, outputDuration]
        boundaries += clip.volumeKeyframes.map { $0.time / clip.speed }
        boundaries += [clip.fadeIn - (clip.fadeInOffset ?? 0), clip.duration / clip.speed + (clip.fadeOutOffset ?? 0) - clip.fadeOut]
        boundaries = Array(Set(boundaries.filter { $0.isFinite && $0 >= 0 && $0 <= outputDuration })).sorted()
        var result = [FilmKeyframe(time: 0, value: clip.audioGain(at: 0))]
        func appendSegment(_ a: Double, _ b: Double, depth: Int) {
            let start = clip.audioGain(at: a), end = clip.audioGain(at: b)
            var error = 0.0
            let fractions: [Double] = [0.25, 0.5, 0.75]
            for fraction in fractions {
                let actual = clip.audioGain(at: a + (b - a) * fraction)
                let interpolated = start + (end - start) * fraction
                error = max(error, abs(actual - interpolated))
            }
            if error > 1.0 / 1024 && depth < 12 && b - a > 1.0 / 600 {
                let midpoint = (a + b) / 2
                appendSegment(a, midpoint, depth: depth + 1)
                appendSegment(midpoint, b, depth: depth + 1)
            } else { result.append(FilmKeyframe(time: b, value: end)) }
        }
        for (a, b) in zip(boundaries, boundaries.dropFirst()) where b > a { appendSegment(a, b, depth: 0) }
        return result
    }
    private static func writeStill(_ url: URL, duration: Double, size: CGSize, destination: URL) async throws {
        guard duration > 0, duration.isFinite, let source = CGImageSourceCreateWithURL(url as CFURL, nil), let image = CGImageSourceCreateThumbnailAtIndex(source, 0, [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceThumbnailMaxPixelSize: max(size.width, size.height), kCGImageSourceCreateThumbnailWithTransform: true] as CFDictionary) else { throw FilmRenderError.invalid("This photo format cannot be rendered on this device.") }
        let writer = try AVAssetWriter(outputURL: destination, fileType: .mp4)
        let input = AVAssetWriterInput(mediaType: .video, outputSettings: [AVVideoCodecKey: AVVideoCodecType.h264, AVVideoWidthKey: Int(size.width), AVVideoHeightKey: Int(size.height)])
        let adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input, sourcePixelBufferAttributes: [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32ARGB, kCVPixelBufferWidthKey as String: Int(size.width), kCVPixelBufferHeightKey as String: Int(size.height), kCVPixelBufferCGImageCompatibilityKey as String: true, kCVPixelBufferCGBitmapContextCompatibilityKey as String: true])
        guard writer.canAdd(input) else { throw FilmRenderError.invalid("Cannot encode this photo.") }; writer.add(input)
        guard writer.startWriting() else { throw writer.error ?? FilmRenderError.invalid("Cannot start the movie encoder.") }; writer.startSession(atSourceTime: .zero)
        guard let pool = adaptor.pixelBufferPool else { throw FilmRenderError.invalid("The encoder has no pixel buffer pool.") }
        var optionalBuffer: CVPixelBuffer?
        guard CVPixelBufferPoolCreatePixelBuffer(nil, pool, &optionalBuffer) == kCVReturnSuccess, let buffer = optionalBuffer else { throw FilmRenderError.invalid("Cannot allocate a photo frame.") }
        CVPixelBufferLockBaseAddress(buffer, [])
        guard let ctx = CGContext(data: CVPixelBufferGetBaseAddress(buffer), width: Int(size.width), height: Int(size.height), bitsPerComponent: 8, bytesPerRow: CVPixelBufferGetBytesPerRow(buffer), space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipFirst.rawValue) else { CVPixelBufferUnlockBaseAddress(buffer, []); throw FilmRenderError.invalid("Cannot draw the photo frame.") }
        ctx.setFillColor(CGColor(gray: 0, alpha: 1)); ctx.fill(CGRect(origin: .zero, size: size))
        let scale = min(size.width / Double(image.width), size.height / Double(image.height))
        let rect = CGRect(x: (size.width - Double(image.width) * scale) / 2, y: (size.height - Double(image.height) * scale) / 2, width: Double(image.width) * scale, height: Double(image.height) * scale)
        ctx.draw(image, in: rect); CVPixelBufferUnlockBaseAddress(buffer, [])
        for frame in 0..<max(1, Int(ceil(duration * 30))) {
            try Task.checkCancellation()
            while !input.isReadyForMoreMediaData { if writer.status == .failed { throw writer.error ?? FilmRenderError.invalid("Photo encoding failed.") }; try await Task.sleep(nanoseconds: 1_000_000) }
            guard adaptor.append(buffer, withPresentationTime: time(Double(frame)/30)) else { throw writer.error ?? FilmRenderError.invalid("Photo encoding failed.") }
        }
        input.markAsFinished(); await writer.finishWriting()
        guard writer.status == .completed else { throw writer.error ?? FilmRenderError.invalid("Photo encoding failed.") }
    }
}
