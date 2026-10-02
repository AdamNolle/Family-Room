import XCTest
@testable import FamilyRoom
import AVFoundation
import ImageIO
import UniformTypeIdentifiers

final class FilmSplitExportTests: XCTestCase {
    func testSplitExportKeepsDecodedPictureAndSoundAcrossTheCut() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let photo = root.appendingPathComponent("synthetic-red.png")
        let context = try XCTUnwrap(CGContext(data: nil, width: 128, height: 128, bitsPerComponent: 8,
            bytesPerRow: 0, space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        context.setFillColor(CGColor(red: 1, green: 0, blue: 0, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 128, height: 128))
        let image = try XCTUnwrap(context.makeImage())
        let png = try XCTUnwrap(CGImageDestinationCreateWithURL(photo as CFURL, UTType.png.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(png, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(png))
        let tone = root.appendingPathComponent("synthetic-tone.wav")
        let format = try XCTUnwrap(AVAudioFormat(standardFormatWithSampleRate: 16000, channels: 1))
        let buffer = try XCTUnwrap(AVAudioPCMBuffer(pcmFormat: format, frameCapacity: 16000))
        buffer.frameLength = 16000
        let samples = try XCTUnwrap(buffer.floatChannelData)[0]
        for i in 0..<16000 { samples[i] = Float(sin(Double(i) * 2 * .pi * 440 / 16000) * 0.4) }
        var file: AVAudioFile? = try AVAudioFile(forWriting: tone, settings: format.settings)
        try file?.write(from: buffer)
        file = nil
        func asset(_ id: String, _ name: String, _ kind: String) -> LibraryAsset {
            LibraryAsset(id: id, roomId: "room", filename: name, kind: kind, sha256: "", size: 0,
                capturedAt: 0, contributor: "", favorite: false, caption: "", deleted: false,
                people: [], local: true, verifiedCopies: 1, availability: "Saved on this device")
        }
        let video = FilmClip(assetId: "photo", duration: 1, fadeIn: 0.8, fadeOut: 0.8,
            opacityKeyframes: [FilmKeyframe(time: 0, value: 0.2), FilmKeyframe(time: 0.3, value: 1), FilmKeyframe(time: 1, value: 0.3)])
        let audio = FilmClip(assetId: "audio", track: 1, duration: 1, fadeIn: 0.8, fadeOut: 0.8,
            volumeKeyframes: [FilmKeyframe(time: 0, value: 0.8), FilmKeyframe(time: 0.3, value: 0.1), FilmKeyframe(time: 1, value: 0.7)])
        let videoSplit = try XCTUnwrap(video.split(at: 0.5)), audioSplit = try XCTUnwrap(audio.split(at: 0.5))
        let assets = [asset("photo", photo.lastPathComponent, "photo"), asset("audio", tone.lastPathComponent, "audio")]
        let unsplit = root.appendingPathComponent("before.mp4"), split = root.appendingPathComponent("after.mp4")
        for (url, clips) in [(unsplit, [video, audio]), (split, [videoSplit.left, videoSplit.right, audioSplit.left, audioSplit.right])] {
            try await FilmRenderer.render(FilmProject(roomId: "room", width: 128, height: 128, clips: clips),
                assets: assets, urls: ["photo": photo, "audio": tone], destination: url)
        }
        let before = AVURLAsset(url: unsplit), after = AVURLAsset(url: split)
        let beforeDuration = try await before.load(.duration), afterDuration = try await after.load(.duration)
        XCTAssertEqual(beforeDuration.seconds, afterDuration.seconds, accuracy: 1.0 / 30)
        func redChannel(_ movie: AVAsset, at seconds: Double) async throws -> Int {
            let generator = AVAssetImageGenerator(asset: movie)
            generator.requestedTimeToleranceBefore = .zero; generator.requestedTimeToleranceAfter = .zero
            let frame = try await generator.image(at: CMTime(seconds: seconds, preferredTimescale: 600))
            let pixel = try XCTUnwrap(CGContext(data: nil, width: 1, height: 1, bitsPerComponent: 8,
                bytesPerRow: 4, space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
            pixel.draw(frame.image, in: CGRect(x: 0, y: 0, width: 1, height: 1))
            return Int(try XCTUnwrap(pixel.data).assumingMemoryBound(to: UInt8.self)[0])
        }
        for seconds in [0.1, 0.3, 0.4667, 0.5, 0.5334, 0.7, 0.9] {
            let original = try await redChannel(before, at: seconds), edited = try await redChannel(after, at: seconds)
            XCTAssertEqual(Double(original), Double(edited), accuracy: 5, "The split must preserve the rendered fade at \(seconds)s.")
        }
        func rmsWindows(_ movie: AVAsset) async throws -> [Double] {
            let tracks = try await movie.loadTracks(withMediaType: .audio)
            let reader = try AVAssetReader(asset: movie)
            let output = AVAssetReaderTrackOutput(track: try XCTUnwrap(tracks.first), outputSettings: [
                AVFormatIDKey: kAudioFormatLinearPCM, AVLinearPCMIsFloatKey: true,
                AVLinearPCMBitDepthKey: 32, AVLinearPCMIsNonInterleaved: false
            ])
            reader.add(output); XCTAssertTrue(reader.startReading())
            var values = [Float]()
            while let sample = output.copyNextSampleBuffer() {
                guard let block = CMSampleBufferGetDataBuffer(sample) else { continue }
                var samples = [Float](repeating: 0, count: CMBlockBufferGetDataLength(block) / 4)
                let status = samples.withUnsafeMutableBytes { bytes in
                    CMBlockBufferCopyDataBytes(block, atOffset: 0, dataLength: bytes.count, destination: bytes.baseAddress!)
                }
                XCTAssertEqual(status, kCMBlockBufferNoErr); values += samples
            }
            XCTAssertEqual(reader.status, .completed)
            XCTAssertFalse(values.isEmpty)
            return (0..<10).map { index in
                let start = values.count * index / 10, end = values.count * (index + 1) / 10
                let squares = values[start..<end].reduce(0.0) { $0 + Double($1) * Double($1) }
                return sqrt(squares / Double(max(1, end - start)))
            }
        }
        let originalAudio = try await rmsWindows(before), editedAudio = try await rmsWindows(after)
        for (original, edited) in zip(originalAudio, editedAudio) { XCTAssertEqual(original, edited, accuracy: 0.005) }
        XCTAssertGreaterThan(originalAudio[4], 0.012, "Audio must be present at the cut.")
        XCTAssertLessThan(originalAudio[4], 0.06, "Fades must multiply the keyframed gain, not overwrite it.")
    }
}
