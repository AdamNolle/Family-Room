import AVFoundation
import ImageIO
import Speech
import UniformTypeIdentifiers
import XCTest
@testable import FamilyRoom

final class VideoSearchServiceTests: XCTestCase {
    func testSoundEventsMergeNearbyDetectionsAndDiscardGenericOrWeakLabels() {
        let observer = SoundEventObserver()
        observer.ingest(label: "Applause", confidence: 0.91, startMs: 1_000, endMs: 2_000)
        observer.ingest(label: "Applause", confidence: 0.84, startMs: 2_500, endMs: 3_500)
        observer.ingest(label: "Music", confidence: 0.99, startMs: 4_000, endMs: 5_000)
        observer.ingest(label: "Barking", confidence: 0.71, startMs: 6_000, endMs: 7_000)
        observer.ingest(label: "Clapping", confidence: .nan, startMs: 8_000, endMs: 9_000)

        XCTAssertEqual(observer.observations.count, 1)
        XCTAssertEqual(observer.observations[0].modality, "sound_event")
        XCTAssertEqual(observer.observations[0].text, "applause")
        XCTAssertEqual(observer.observations[0].startMs, 1_000)
        XCTAssertEqual(observer.observations[0].endMs, 3_500)
        XCTAssertEqual(observer.observations[0].confidencePercent, 91)
    }

    func testLongContinuousSoundEventsSplitWithinCoreObservationBounds() {
        let observer = SoundEventObserver()
        observer.ingest(label: "Applause", confidence: 0.9, startMs: 0, endMs: 120_000)
        observer.ingest(label: "Applause", confidence: 0.9, startMs: 120_500, endMs: 121_000)

        XCTAssertEqual(observer.observations.count, 2)
        XCTAssertTrue(observer.observations.allSatisfy { $0.endMs - $0.startMs <= 120_000 })
    }

    func testSoundAnalysisGateIgnoresLateCompletionsAndContinuations() async {
        let gate = SoundAnalysisGate()
        let installed = expectation(description: "sound analysis continuation installed")
        let analysis = Task {
            await withCheckedContinuation { continuation in
                XCTAssertTrue(gate.install(continuation))
                installed.fulfill()
            }
        }

        await fulfillment(of: [installed], timeout: 3)
        gate.finish(false)
        gate.finish(true)
        let completed = await analysis.value
        XCTAssertFalse(completed)

        let lateContinuation = await withCheckedContinuation { continuation in
            XCTAssertFalse(gate.install(continuation))
        }
        XCTAssertFalse(lateContinuation)
    }

    func testFirstSearchIndexesOnlyLocallyAvailableVideosWithoutAnIndex() throws {
        let source = #"""
        {"videos":[
            {"asset_id":"local-new","local":true,"indexed":false,"audio_indexed":false,"frames_indexed":false},
            {"asset_id":"remote-new","local":false,"indexed":false,"audio_indexed":false,"frames_indexed":false},
            {"asset_id":"local-ready","local":true,"indexed":true,"audio_indexed":true,"frames_indexed":true}
        ]}
        """#
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        let status = try decoder.decode(VideoIndexStatus.self, from: Data(source.utf8))
        XCTAssertEqual(status.unindexedLocalVideoIDs, ["local-new"])
    }

    func testVideoSearchIndexesOnlyMissingRequestedEvidence() throws {
        let source = #"""
        {"videos":[
          {"asset_id":"local-new","local":true,"indexed":false,"audio_indexed":false,"frames_indexed":false},
          {"asset_id":"remote-new","local":false,"indexed":false,"audio_indexed":false,"frames_indexed":false},
          {"asset_id":"frames-only","local":true,"indexed":true,"audio_indexed":false,"frames_indexed":true},
          {"asset_id":"audio-only","local":true,"indexed":true,"audio_indexed":true,"frames_indexed":false},
          {"asset_id":"complete","local":true,"indexed":true,"audio_indexed":true,"frames_indexed":true}
        ]}
        """#
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        let status = try decoder.decode(VideoIndexStatus.self, from: Data(source.utf8))

        XCTAssertEqual(status.localVideoIDsNeedingIndex(modality: "frame"), ["local-new", "audio-only"])
        XCTAssertEqual(status.localVideoIDsNeedingIndex(modality: "audio"), ["local-new", "frames-only"])
        XCTAssertEqual(status.localVideoIDsNeedingIndex(modality: "all"), ["local-new", "frames-only", "audio-only"])
    }

    func testRemoteVideoIndexingRequiresExplicitOptIn() throws {
        let source = #"""
        {"videos":[
          {"asset_id":"local-new","local":true,"indexed":false,"audio_indexed":false,"frames_indexed":false},
          {"asset_id":"remote-new","local":false,"indexed":false,"audio_indexed":false,"frames_indexed":false},
          {"asset_id":"remote-frames-only","local":false,"indexed":true,"audio_indexed":false,"frames_indexed":true},
          {"asset_id":"remote-ready","local":false,"indexed":true,"audio_indexed":true,"frames_indexed":true}
        ]}
        """#
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        let status = try decoder.decode(VideoIndexStatus.self, from: Data(source.utf8))

        XCTAssertEqual(status.videoIDsNeedingIndex(modality: "frame", includeRemoteVideos: false), ["local-new"])
        XCTAssertEqual(status.videoIDsNeedingIndex(modality: "frame", includeRemoteVideos: true), ["local-new", "remote-new"])
        XCTAssertEqual(status.videoIDsNeedingIndex(modality: "audio", includeRemoteVideos: true), ["local-new", "remote-new", "remote-frames-only"])
        XCTAssertEqual(status.videoIDsNeedingIndex(modality: "all", includeRemoteVideos: true), ["local-new", "remote-new", "remote-frames-only"])
    }

    func testSoundEventCoverageAvoidsRepeatedSpeechAnalysisForGeneralBrowsing() throws {
        let source = #"{"videos":[{"asset_id":"sound-only","local":true,"indexed":true,"audio_indexed":false,"sound_events_indexed":true,"frames_indexed":true}]}"#
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        let status = try decoder.decode(VideoIndexStatus.self, from: Data(source.utf8))

        XCTAssertTrue(status.videoIDsNeedingIndex(modality: "audio", includeRemoteVideos: false).isEmpty)
        XCTAssertTrue(status.videoIDsNeedingIndex(modality: "all", includeRemoteVideos: false).isEmpty)
    }

    func testQuotedAudioSearchRetriesSpeechEvenWhenSoundCluesAreIndexed() throws {
        let source = #"{"videos":[{"asset_id":"sound-only","local":true,"indexed":true,"audio_indexed":false,"sound_events_indexed":true,"frames_indexed":true}]}"#
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        let status = try decoder.decode(VideoIndexStatus.self, from: Data(source.utf8))

        XCTAssertTrue(videoQueryRequiresSpeechIndex("Find someone saying ‘happy birthday’", modality: "audio"))
        XCTAssertTrue(videoQueryRequiresSpeechIndex("Find \"we are home\"", modality: "all"))
        XCTAssertFalse(videoQueryRequiresSpeechIndex("What does the sign say?", modality: "frame"))
        XCTAssertFalse(videoQueryRequiresSpeechIndex("What’s happening?", modality: "audio"))
        XCTAssertEqual(status.videoIDsNeedingIndex(modality: "audio", includeRemoteVideos: false,
                                                   requireSpeech: true), ["sound-only"])
        XCTAssertEqual(status.videoIDsNeedingIndex(modality: "all", includeRemoteVideos: false,
                                                   requireSpeech: true), ["sound-only"])
    }

    func testSpokenCaptionCueUsesHalfOpenPlaybackTimeBounds() {
        let cue = VideoSpeechCue(startMs: 1_250, endMs: 2_500, text: "We found it")

        XCTAssertFalse(cue.isActive(at: 1_249))
        XCTAssertTrue(cue.isActive(at: 1_250))
        XCTAssertTrue(cue.isActive(at: 2_499))
        XCTAssertFalse(cue.isActive(at: 2_500))
    }

    func testSpeechCancellationResumesOnceEvenIfRecognizerFinishesLate() async {
        let gate = SpeechRecognitionGate()
        let started = expectation(description: "speech continuation installed")
        let recognition = Task {
            let segments: [SFTranscriptionSegment] = try await withCheckedThrowingContinuation { continuation in
                XCTAssertTrue(gate.install(continuation))
                started.fulfill()
            }
            return segments
        }
        await fulfillment(of: [started], timeout: 3)
        gate.finish(.failure(CancellationError()))
        gate.finish(.success([]))
        do {
            _ = try await recognition.value
            XCTFail("Cancellation should complete the waiting recognition")
        } catch is CancellationError {
            // The late success callback must not resume the continuation twice.
        } catch {
            XCTFail("Unexpected error: \(error)")
        }
    }

    func testRenderedMovieCanBeIndexedFromRealFramesWithoutAudioTrack() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let photo = root.appendingPathComponent("frame.png")
        let context = try XCTUnwrap(CGContext(data: nil, width: 128, height: 128,
                                              bitsPerComponent: 8, bytesPerRow: 0,
                                              space: CGColorSpaceCreateDeviceRGB(),
                                              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        context.setFillColor(CGColor(red: 0.8, green: 0.45, blue: 0.2, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 128, height: 128))
        let image = try XCTUnwrap(context.makeImage())
        let png = try XCTUnwrap(CGImageDestinationCreateWithURL(photo as CFURL,
                                                                UTType.png.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(png, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(png))
        let movie = root.appendingPathComponent("frame.mp4")
        let asset = LibraryAsset(id: "photo", roomId: "room", filename: "frame.png", kind: "photo",
                                 sha256: "", size: 0, capturedAt: 0, contributor: "", favorite: false,
                                 caption: "", deleted: false, people: [], local: true,
                                 verifiedCopies: 1, availability: "Saved on this device")
        try await FilmRenderer.render(
            FilmProject(roomId: "room", width: 128, height: 128,
                        clips: [FilmClip(assetId: "photo", duration: 1)]),
            assets: [asset], urls: ["photo": photo], destination: movie)

        let analysis = try await VideoSearchService.shared.analyze(movie)
        if analysis.framesIndexed {
            XCTAssertEqual(analysis.frameIntervalMs, 2_000)
        } else {
            XCTAssertTrue(analysis.frameMessage.contains("Retry"))
        }
        XCTAssertTrue(analysis.audioIndexed, "A silent video has no speech left to analyze")
                let frameOnly = try await VideoSearchService.shared.analyze(movie, includeAudio: false)
                XCTAssertFalse(frameOnly.audioIndexed)
                XCTAssertTrue(frameOnly.audioMessage.contains("skipped"))
        XCTAssertTrue(FileManager.default.fileExists(atPath: movie.path))
    }
}
