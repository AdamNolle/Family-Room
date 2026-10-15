import AVFoundation
import Foundation
import Speech
import SoundAnalysis
import Vision

struct VideoSearchObservation: Encodable {
    let modality: String
    let startMs: UInt64
    let endMs: UInt64
    let text: String
    let confidencePercent: UInt8

    var commandValue: [String: Any] {
        ["modality": modality, "start_ms": startMs, "end_ms": endMs,
         "text": text, "confidence_percent": confidencePercent]
    }
}

struct VideoAnalysis {
    let observations: [VideoSearchObservation]
    let audioIndexed: Bool
    let soundEventsIndexed: Bool
    let framesIndexed: Bool
    let frameIntervalMs: UInt64
    let frameMessage: String
    let audioMessage: String
    let soundMessage: String
}

struct VideoIndexStatus: Decodable {
    struct Item: Decodable {
        let assetId: String
        let local: Bool
        let indexed: Bool
        let audioIndexed: Bool
        let soundEventsIndexed: Bool?
        let framesIndexed: Bool
        let frameIntervalMs: UInt64?
    }

    let videos: [Item]

    var unindexedLocalVideoIDs: Set<String> {
        Set(videos.lazy.filter { $0.local && !$0.indexed }.map(\.assetId))
    }

    func localVideoIDsNeedingIndex(modality: String) -> Set<String> {
        videoIDsNeedingIndex(modality: modality, includeRemoteVideos: false)
    }

    func videoIDsNeedingIndex(modality: String, includeRemoteVideos: Bool,
                              requireSpeech: Bool = false) -> Set<String> {
        Set(videos.lazy.filter { item in
            let missing: Bool
            switch modality {
            case "frame": missing = !item.framesIndexed
            case "audio":
                missing = requireSpeech
                    ? !item.audioIndexed
                    : (!item.audioIndexed && item.soundEventsIndexed != true)
            default:
                let missingAudio = requireSpeech
                    ? !item.audioIndexed
                    : (!item.audioIndexed && item.soundEventsIndexed != true)
                missing = !item.framesIndexed || missingAudio
            }
            return (includeRemoteVideos || item.local) && missing
        }.map(\.assetId))
    }
}

func videoQueryRequiresSpeechIndex(_ query: String, modality: String) -> Bool {
    guard modality != "frame" else { return false }
    for (open, close) in [("\"", "\""), ("“", "”"), ("‘", "’")] {
        guard let start = query.range(of: open)?.upperBound,
              let end = query[start...].range(of: close)?.lowerBound,
              !query[start..<end].trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            continue
        }
        return true
    }

    let apostrophes = query.indices.filter { query[$0] == "'" }
    for pair in zip(apostrophes, apostrophes.dropFirst()) {
        let before = query[..<pair.0].last
        let after = query[query.index(after: pair.1)...].first
        let opensQuote = before.map { !$0.isLetter && !$0.isNumber } ?? true
        let closesQuote = after.map { !$0.isLetter && !$0.isNumber } ?? true
        if opensQuote && closesQuote &&
            !query[query.index(after: pair.0)..<pair.1]
                .trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return true
        }
    }
    return false
}

final class SpeechRecognitionGate: @unchecked Sendable {
    private let lock = NSLock()
    private var continuation: CheckedContinuation<[SFTranscriptionSegment], Error>?
    private var recognitionTask: SFSpeechRecognitionTask?
    private var timeoutTask: Task<Void, Never>?
    private var finished = false

    func install(_ continuation: CheckedContinuation<[SFTranscriptionSegment], Error>) -> Bool {
        lock.lock()
        let cancelled = finished
        if !cancelled { self.continuation = continuation }
        lock.unlock()
        if cancelled { continuation.resume(throwing: CancellationError()) }
        return !cancelled
    }

    func setRecognitionTask(_ task: SFSpeechRecognitionTask) {
        lock.lock()
        let completed = finished
        if !completed { recognitionTask = task }
        lock.unlock()
        if completed { task.cancel() }
    }

    func setTimeoutTask(_ task: Task<Void, Never>) {
        lock.lock()
        let completed = finished
        if !completed { timeoutTask = task }
        lock.unlock()
        if completed { task.cancel() }
    }

    func finish(_ result: Result<[SFTranscriptionSegment], Error>) {
        lock.lock()
        guard !finished else { lock.unlock(); return }
        finished = true
        let waiting = continuation
        continuation = nil
        let recognition = recognitionTask
        recognitionTask = nil
        let timeout = timeoutTask
        timeoutTask = nil
        lock.unlock()
        recognition?.cancel()
        timeout?.cancel()
        waiting?.resume(with: result)
    }
}

actor VideoSearchService {
    static let shared = VideoSearchService()

    func analyze(_ url: URL, includeAudio: Bool = true, includeFrames: Bool = true) async throws -> VideoAnalysis {
        let asset = AVURLAsset(url: url)
        let duration = try await asset.load(.duration).seconds
        guard duration.isFinite && duration > 0 else {
            throw NSError(domain: "FamilyRoom.VideoSearch", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "This video has no readable duration."])
        }
        let (frames, interval, framesIndexed, frameMessage): ([VideoSearchObservation], Double, Bool, String)
        if includeFrames {
            (frames, interval, framesIndexed, frameMessage) = try analyzeFrames(asset, duration: duration)
        } else {
            (frames, interval, framesIndexed, frameMessage) = ([], 0, false, "")
        }
        let speechResult: ([VideoSearchObservation], Bool, String)
        let soundResult: ([VideoSearchObservation], Bool, String)
        if includeAudio {
            speechResult = await analyzeSpeech(asset, duration: duration)
            soundResult = await analyzeSoundEvents(url)
        } else {
            speechResult = ([], false, "Speech analysis was skipped for this frame-only search.")
            soundResult = ([], false, "Sound analysis was skipped for this frame-only search.")
        }
        let (speech, audioIndexed, audioMessage) = speechResult
        let (soundEvents, soundEventsIndexed, soundMessage) = soundResult
        return VideoAnalysis(observations: (frames + speech + soundEvents).sorted { $0.startMs < $1.startMs },
                             audioIndexed: audioIndexed, soundEventsIndexed: soundEventsIndexed,
            framesIndexed: framesIndexed,
            frameIntervalMs: UInt64((interval * 1_000).rounded()),
            frameMessage: frameMessage,
                             audioMessage: audioMessage, soundMessage: soundMessage)
    }

    private func analyzeSoundEvents(_ url: URL) async -> ([VideoSearchObservation], Bool, String) {
        let asset = AVURLAsset(url: url)
        guard (try? await asset.loadTracks(withMediaType: .audio))?.isEmpty == false else {
            return ([], true, "This video has no audio track.")
        }
        do {
            let analyzer = try SNAudioFileAnalyzer(url: url)
            let request = try SNClassifySoundRequest(classifierIdentifier: .version1)
            let observer = SoundEventObserver()
            try analyzer.add(request, withObserver: observer)
            let gate = SoundAnalysisGate()
            let completed = await withTaskCancellationHandler {
                await withCheckedContinuation { continuation in
                    guard gate.install(continuation) else { return }
                    analyzer.analyze { success in gate.finish(success) }
                }
            } onCancel: {
                gate.cancel(analyzer)
            }
            guard completed else {
                return ([], false, observer.failureMessage ?? "On-device sound analysis did not complete.")
            }
            return (observer.observations, true, "Sound events were indexed on this device.")
        } catch {
            return ([], false, "Sound events could not be indexed on this device: \(error.localizedDescription)")
        }
    }

    private func analyzeFrames(_ asset: AVAsset, duration: Double) throws -> ([VideoSearchObservation], Double, Bool, String) {
        let interval = max(2, (duration / 1_500).rounded(.up))
        let generator = AVAssetImageGenerator(asset: asset)
        generator.appliesPreferredTrackTransform = true
        generator.maximumSize = CGSize(width: 960, height: 960)
        generator.requestedTimeToleranceBefore = CMTime(seconds: 0.25, preferredTimescale: 600)
        generator.requestedTimeToleranceAfter = CMTime(seconds: 0.25, preferredTimescale: 600)
        var observations: [VideoSearchObservation] = []
        let frameCount = min(1_500, Int((duration / interval).rounded(.up)))
        var decoded = 0
        var successfulRequests = 0
        var failedRequests = false
        for frame in 0..<frameCount {
            try Task.checkCancellation()
            let second = min(Double(frame) * interval, max(0, duration - 0.01))
            var actual = CMTime.zero
            guard let image = try? generator.copyCGImage(at: CMTime(seconds: second, preferredTimescale: 600), actualTime: &actual) else {
                continue
            }
            decoded += 1
            let stamp = UInt64(max(0, actual.seconds.isFinite ? actual.seconds : second) * 1_000)
            let text = VNRecognizeTextRequest()
            text.recognitionLevel = .accurate
            let labels = VNClassifyImageRequest()
            do {
                try VNImageRequestHandler(cgImage: image).perform([text])
                successfulRequests += 1
                for line in (text.results ?? []).prefix(3) {
                    guard let candidate = line.topCandidates(1).first, candidate.confidence >= 0.5 else { continue }
                    observations.append(VideoSearchObservation(
                        modality: "screen_text", startMs: stamp, endMs: stamp,
                        text: candidate.string,
                        confidencePercent: UInt8(min(100, max(0, Int(candidate.confidence * 100))))))
                }
            } catch {
                failedRequests = true
            }
            do {
                try VNImageRequestHandler(cgImage: image).perform([labels])
                successfulRequests += 1
                for label in (labels.results ?? []).filter({ $0.confidence >= 0.75 }).prefix(2) {
                    observations.append(VideoSearchObservation(
                        modality: "visual_label", startMs: stamp, endMs: stamp,
                        text: label.identifier.replacingOccurrences(of: "_", with: " "),
                        confidencePercent: UInt8(min(100, max(0, Int(label.confidence * 100))))))
                }
            } catch {
                failedRequests = true
            }
        }
        guard decoded > 0 else {
            throw NSError(domain: "FamilyRoom.VideoSearch", code: 2,
                          userInfo: [NSLocalizedDescriptionKey: "No video frames could be decoded."])
        }
        let indexed = successfulRequests > 0
        let message: String
        if !indexed {
            message = "On-device frame analysis could not run. Retry on this device."
        } else if failedRequests {
            message = "Some frame analyses failed; results may be incomplete."
        } else {
            message = ""
        }
        return (observations, interval, indexed, message)
    }

    private func analyzeSpeech(_ asset: AVAsset, duration: Double) async -> ([VideoSearchObservation], Bool, String) {
        guard (try? await asset.loadTracks(withMediaType: .audio))?.isEmpty == false else {
            return ([], true, "This video has no audio track.")
        }
        guard await speechAuthorized() else {
            return ([], false, "Speech permission is needed to index spoken words.")
        }
        guard let recognizer = SFSpeechRecognizer(locale: .current),
              recognizer.isAvailable, recognizer.supportsOnDeviceRecognition else {
            return ([], false, "On-device speech recognition is unavailable for this language or device.")
        }
        var observations: [VideoSearchObservation] = []
        do {
            var offset = 0.0
            while offset < duration {
                try Task.checkCancellation()
                let length = min(50, duration - offset)
                let audioURL = FileManager.default.temporaryDirectory
                    .appendingPathComponent("FamilyRoomSpeech-\(UUID().uuidString).m4a")
                defer { try? FileManager.default.removeItem(at: audioURL) }
                guard let exporter = AVAssetExportSession(asset: asset, presetName: AVAssetExportPresetAppleM4A) else {
                    throw NSError(domain: "FamilyRoom.VideoSearch", code: 3)
                }
                exporter.outputURL = audioURL
                exporter.outputFileType = .m4a
                exporter.timeRange = CMTimeRange(start: CMTime(seconds: offset, preferredTimescale: 600),
                                                 duration: CMTime(seconds: length, preferredTimescale: 600))
                await withCheckedContinuation { continuation in
                    exporter.exportAsynchronously { continuation.resume() }
                }
                guard exporter.status == .completed else {
                    throw exporter.error ?? NSError(domain: "FamilyRoom.VideoSearch", code: 4)
                }
                let segments = try await recognize(audioURL, recognizer: recognizer)
                observations += groupedSpeech(segments, offset: offset)
                offset += length
            }
            return (observations, true, "Spoken words were indexed on this device.")
        } catch {
            return ([], false, "Speech could not be indexed on this device: \(error.localizedDescription)")
        }
    }

    private func speechAuthorized() async -> Bool {
        switch SFSpeechRecognizer.authorizationStatus() {
        case .authorized: return true
        case .notDetermined:
            return await withCheckedContinuation { continuation in
                SFSpeechRecognizer.requestAuthorization { status in
                    continuation.resume(returning: status == .authorized)
                }
            }
        default: return false
        }
    }

    private func recognize(_ url: URL, recognizer: SFSpeechRecognizer) async throws -> [SFTranscriptionSegment] {
        let request = SFSpeechURLRecognitionRequest(url: url)
        request.requiresOnDeviceRecognition = true
        request.shouldReportPartialResults = false
        let gate = SpeechRecognitionGate()
        return try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { continuation in
                guard gate.install(continuation) else { return }
                let task = recognizer.recognitionTask(with: request) { result, error in
                    if let result, result.isFinal {
                        gate.finish(.success(result.bestTranscription.segments))
                    } else if let error {
                        gate.finish(.failure(error))
                    }
                }
                gate.setRecognitionTask(task)
                gate.setTimeoutTask(Task.detached {
                    try? await Task.sleep(for: .seconds(90))
                    gate.finish(.failure(NSError(domain: "FamilyRoom.VideoSearch", code: 5,
                                                 userInfo: [NSLocalizedDescriptionKey: "On-device speech analysis timed out."])))
                })
            }
        } onCancel: {
            gate.finish(.failure(CancellationError()))
        }
    }

    private func groupedSpeech(_ segments: [SFTranscriptionSegment], offset: Double) -> [VideoSearchObservation] {
        var groups: [VideoSearchObservation] = []
        var words: [String] = []
        var start = 0.0
        var end = 0.0
        var confidence: Float = 1
        func flush() {
            guard !words.isEmpty else { return }
            groups.append(VideoSearchObservation(
                modality: "speech", startMs: UInt64((offset + start) * 1_000),
                endMs: UInt64((offset + end) * 1_000), text: words.joined(separator: " "),
                confidencePercent: UInt8(min(100, max(0, Int(confidence * 100))))))
            words.removeAll()
            confidence = 1
        }
        for segment in segments {
            let word = segment.substring.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !word.isEmpty else { continue }
            if !words.isEmpty && (segment.timestamp - start > 4 || words.joined(separator: " ").count + word.count > 400) {
                flush()
            }
            if words.isEmpty { start = segment.timestamp }
            end = segment.timestamp + segment.duration
            confidence = min(confidence, segment.confidence)
            words.append(word)
        }
        flush()
        return groups
    }
}

final class SoundEventObserver: NSObject, SNResultsObserving, @unchecked Sendable {
    private let lock = NSLock()
    private var storedObservations: [VideoSearchObservation] = []
    private var lastIndexByLabel: [String: Int] = [:]
    private var storedFailureMessage: String?

    var observations: [VideoSearchObservation] {
        lock.lock()
        defer { lock.unlock() }
        return storedObservations.sorted { $0.startMs < $1.startMs }
    }

    var failureMessage: String? {
        lock.lock()
        defer { lock.unlock() }
        return storedFailureMessage
    }

    func request(_ request: SNRequest, didProduce result: SNResult) {
        guard let result = result as? SNClassificationResult else { return }
        let timeRange = result.timeRange
        let start = timeRange.start.seconds
        let duration = timeRange.duration.seconds
        guard start.isFinite, duration.isFinite, start >= 0, duration > 0 else { return }
        let startMs = UInt64(start * 1_000)
        let endMs = UInt64((start + duration) * 1_000)
        for candidate in result.classifications.prefix(2) {
            ingest(label: candidate.identifier, confidence: candidate.confidence,
                   startMs: startMs, endMs: endMs)
        }
    }

    func ingest(label rawLabel: String, confidence: Double, startMs: UInt64, endMs: UInt64) {
        let label = rawLabel
            .replacingOccurrences(of: "_", with: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        let genericLabels: Set<String> = [
            "speech", "music", "silence", "inside, small room",
            "inside, large room", "outside, rural or natural"
        ]
        guard confidence.isFinite, (0...1).contains(confidence), confidence >= 0.72,
              startMs <= endMs,
              !label.isEmpty, !genericLabels.contains(label) else { return }

        let confidencePercent = UInt8(min(100, max(0, Int((confidence * 100).rounded()))))
        lock.lock()
        defer { lock.unlock() }
        guard storedObservations.count < 10_000 else { return }
        if let index = lastIndexByLabel[label],
           startMs >= storedObservations[index].startMs,
           startMs <= storedObservations[index].endMs.saturatingAdd(500),
           max(storedObservations[index].endMs, endMs)
                .saturatingSubtract(storedObservations[index].startMs) <= 120_000 {
            let previous = storedObservations[index]
            storedObservations[index] = VideoSearchObservation(
                modality: "sound_event", startMs: previous.startMs,
                endMs: max(previous.endMs, endMs), text: label,
                confidencePercent: max(previous.confidencePercent, confidencePercent))
        } else {
            lastIndexByLabel[label] = storedObservations.count
            storedObservations.append(VideoSearchObservation(
                modality: "sound_event", startMs: startMs, endMs: endMs,
                text: label, confidencePercent: confidencePercent))
        }
    }

    func request(_ request: SNRequest, didFailWithError error: Error) {
        lock.lock()
        storedFailureMessage = error.localizedDescription
        lock.unlock()
    }

    func requestDidComplete(_ request: SNRequest) {}
}

final class SoundAnalysisGate: @unchecked Sendable {
    private let lock = NSLock()
    private var continuation: CheckedContinuation<Bool, Never>?
    private var finished = false

    func install(_ continuation: CheckedContinuation<Bool, Never>) -> Bool {
        lock.lock()
        let cancelled = finished
        if !cancelled { self.continuation = continuation }
        lock.unlock()
        if cancelled { continuation.resume(returning: false) }
        return !cancelled
    }

    func finish(_ completed: Bool) {
        lock.lock()
        guard !finished else { lock.unlock(); return }
        finished = true
        let waiting = continuation
        continuation = nil
        lock.unlock()
        waiting?.resume(returning: completed)
    }

    func cancel(_ analyzer: SNAudioFileAnalyzer) {
        lock.lock()
        guard !finished else { lock.unlock(); return }
        finished = true
        let waiting = continuation
        continuation = nil
        lock.unlock()
        analyzer.cancelAnalysis()
        waiting?.resume(returning: false)
    }
}

private extension UInt64 {
    func saturatingAdd(_ value: UInt64) -> UInt64 {
        addingReportingOverflow(value).overflow ? .max : self + value
    }

    func saturatingSubtract(_ value: UInt64) -> UInt64 {
        self >= value ? self - value : 0
    }
}
