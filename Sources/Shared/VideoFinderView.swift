import SwiftUI

struct VideoFinderResponse: Decodable {
    struct Coverage: Decodable {
        let videos: Int
        let indexed: Int
        let audioIndexed: Int
        let soundEventsIndexed: Int?
        let framesIndexed: Int
    }
    struct Evidence: Decodable, Identifiable {
        let modality: String
        let startMs: UInt64
        let endMs: UInt64
        let text: String
        let confidencePercent: UInt8
        var id: String { "\(modality):\(startMs):\(text)" }
        var label: String {
            switch modality {
            case "speech": "Spoken"
            case "screen_text": "On screen"
        case "sound_event": "Possible sound"
            default: "Visual suggestion"
            }
        }
    }
    struct Match: Decodable, Identifiable {
        let roomId: String
        let assetId: String
        let startMs: UInt64
        let endMs: UInt64
        let score: UInt64
        let matchKind: String
        let evidence: [Evidence]
        let contextEvidence: [Evidence]?
        var id: String { "\(assetId):\(startMs)" }
        var displayLabel: String {
            let base = matchKind == "exact" ? "Exact phrase" : matchKind == "evidence" ? "Indexed evidence" : "Suggested match"
            return contextEvidence?.isEmpty == false ? "\(base) · nearby clue" : base
        }
    }
    let answer: String
    let matches: [Match]
    let coverage: Coverage
}

private struct VideoFinderTurn: Identifiable {
    let id = UUID()
    let prompt: String
    let modality: String
    let response: VideoFinderResponse
}

struct VideoFinderView: View {
    @EnvironmentObject private var store: LibraryStore
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var question = ""
    @State private var modality = "all"
    @State private var includeRemoteVideos = false
    @State private var conversation: [VideoFinderTurn] = []
    @State private var indexStatus: VideoIndexStatus?
    @State private var busy = false
    @State private var indexing = false
    @State private var progress = ""
    @State private var indexTask: Task<Void, Never>?

    private var room: LibraryRoom? { store.room }
    private var videos: [LibraryAsset] { room?.visibleAssets.filter { $0.kind == "video" } ?? [] }
    private var unindexedVideoCount: Int {
        indexStatus?.videoIDsNeedingIndex(modality: "all", includeRemoteVideos: includeRemoteVideos).count ?? 0
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                VStack(alignment: .leading, spacing: 8) {
                    RoomEyebrow(text: room?.name ?? "Your private library")
                    Text("Find the exact moment.")
                        .font(RoomArt.display(36))
                        .foregroundStyle(CozyTheme.ink)
                    Text("Search spoken words, sounds, on-screen text, and visible moments. Exact wording comes from transcripts or OCR; detected sounds and scene descriptions are suggestions to verify in the video.")
                        .foregroundStyle(CozyTheme.secondary)
                }

                if conversation.isEmpty {
                    VStack(alignment: .leading, spacing: 12) {
                        Label("Private, evidence-based search", systemImage: "lock.fill")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(CozyTheme.accentInk)
                        Text("Try exact words from a conversation or sign, or describe an object or action you remember seeing.")
                            .font(.callout).foregroundStyle(CozyTheme.secondary)
                        VStack(alignment: .leading, spacing: 8) {
                            suggestion("What does the birthday sign say?", modality: "frame")
                            suggestion("Find someone saying ‘I love you’", modality: "audio")
                            suggestion("Find ‘Happy birthday’ while blowing candles", modality: "all")
                            suggestion("Find ‘applause’", modality: "audio")
                            suggestion("Find a child blowing out candles", modality: "frame")
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(18)
                    .cozyPanel()
                }

                ForEach(conversation) { turn in
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Spacer(minLength: 48)
                            Text(turn.prompt)
                                .font(.body.weight(.medium))
                                .foregroundStyle(CozyTheme.ink)
                                .padding(.horizontal, 16)
                                .padding(.vertical, 12)
                                .background(CozyTheme.sidebar, in: RoundedRectangle(cornerRadius: 20))
                        }
                        HStack(alignment: .top, spacing: 10) {
                            Image(systemName: "sparkle.magnifyingglass")
                                .font(.system(size: 15, weight: .semibold))
                                .foregroundStyle(CozyTheme.accentInk)
                                .frame(width: 34, height: 34)
                                .background(CozyTheme.glassTint, in: Circle())
                            VStack(alignment: .leading, spacing: 7) {
                                Text("FAMILY ROOM · LOCAL SEARCH")
                                    .font(.caption2.weight(.semibold).monospaced())
                                    .tracking(0.7)
                                    .foregroundStyle(CozyTheme.secondary)
                                Text("Evidence scope · \(turn.modality == "audio" ? "spoken words" : turn.modality == "frame" ? "video frames" : "audio + frames")")
                                    .font(.caption).foregroundStyle(CozyTheme.secondary)
                                Text(turn.response.answer)
                                    .font(.title3)
                                    .foregroundStyle(CozyTheme.ink)
                                Text("\(turn.response.coverage.indexed) of \(turn.response.coverage.videos) videos indexed here · \(turn.response.coverage.audioIndexed) with speech · \(turn.response.coverage.soundEventsIndexed ?? 0) with sound clues · \(turn.response.coverage.framesIndexed) with frames")
                                    .font(.caption).foregroundStyle(CozyTheme.secondary)
                            }
                            Spacer(minLength: 0)
                        }
                        .padding(16)
                        .cozyPanel()

                        ForEach(turn.response.matches) { match in
                            if let asset = room?.assets.first(where: { $0.id == match.assetId }) {
                                NavigationLink {
                                    AssetDetailView(assetId: match.assetId, roomId: match.roomId,
                                                    seekToSeconds: Double(match.startMs) / 1_000)
                                } label: {
                                    HStack(alignment: .top, spacing: 16) {
                                        AssetPreview(asset: asset)
                                            .frame(width: 128, height: 88)
                                            .memoryPhotoFrame(radius: 7)
                                        VStack(alignment: .leading, spacing: 7) {
                                            let contextEvidence = match.contextEvidence ?? []
                                            let contextEvidenceIDs = Set(contextEvidence.map(\.id))
                                            Text(asset.displayTitle).font(.headline).foregroundStyle(CozyTheme.ink)
                                            HStack(spacing: 8) {
                    Text(match.displayLabel)
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(match.matchKind == "exact" ? CozyTheme.accentInk : CozyTheme.secondary)
                        .padding(.horizontal, 9)
                        .padding(.vertical, 5)
                        .background(
                            match.matchKind == "exact" ? CozyTheme.seasonalHighlight.opacity(0.35) : CozyTheme.sidebar,
                            in: Capsule()
                        )
                    Text("\(timeRange(match.startMs, match.endMs))")
                        .font(.caption.monospacedDigit().weight(.semibold))
                        .foregroundStyle(CozyTheme.secondary)
                }
                                            ForEach(match.evidence.filter { !contextEvidenceIDs.contains($0.id) }.prefix(3)) { evidence in
                                                Text("\(evidence.label): \(evidence.text)")
                                                    .font(.callout).foregroundStyle(CozyTheme.secondary)
                                                    .lineLimit(2)
                                            }
                                            if !contextEvidence.isEmpty {
                                                Text("NEARBY CLUE")
                                                    .font(.caption2.weight(.semibold).monospaced())
                                                    .tracking(0.6)
                                                    .foregroundStyle(CozyTheme.accentInk)
                                                ForEach(contextEvidence.prefix(2)) { evidence in
                                                    Text("\(evidence.label): \(evidence.text)")
                                                        .font(.callout).foregroundStyle(CozyTheme.secondary)
                                                        .lineLimit(2)
                                                }
                                            }
                                        }
                                        Spacer(minLength: 0)
                                        Image(systemName: "play.circle.fill")
                                            .foregroundStyle(CozyTheme.accentInk)
                                    }
                                    .padding(14)
                                    .cozyPanel()
                                }
                                .buttonStyle(.plain)
                        .accessibilityLabel("\(match.displayLabel), open \(asset.displayTitle) at \(timeLabel(match.startMs))")
                            }
                        }
                    }
                }

                VStack(alignment: .leading, spacing: 14) {
                    HStack(spacing: 10) {
                        Image(systemName: "sparkle.magnifyingglass")
                            .foregroundStyle(CozyTheme.accentInk)
                        TextField("Ask another question about your videos…", text: $question)
                            .textFieldStyle(.plain)
                            .onSubmit { Task { await search(question) } }
                            .accessibilityLabel("Ask your videos")
                        Button("Ask") { Task { await search(question) } }
                            .buttonStyle(CozyPrimaryButtonStyle())
                            .disabled(question.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || busy)
                    }
                    Picker("Search evidence", selection: $modality) {
                        Text("All evidence").tag("all")
                        Text("Speech + sounds").tag("audio")
                        Text("Video frames").tag("frame")
                    }
                    .pickerStyle(.segmented)
                    if indexStatus?.videos.contains(where: { !$0.local }) == true {
                        Toggle("Include remote-only videos", isOn: $includeRemoteVideos)
                            .disabled(busy || indexing)
                        .font(.callout)
                        if includeRemoteVideos {
                            Text("Selected originals are downloaded to this device and remain available offline. Storage and source network rules apply.")
                                .font(.caption).foregroundStyle(CozyTheme.secondary)
                        }
                    }
                    Text("Put exact spoken or on-screen words in quotes; surrounding context can stay outside the quotes. Follow-up clues build on your last question. A new quoted phrase starts fresh.")
                        .font(.caption).foregroundStyle(CozyTheme.secondary)
                }
                .padding(20)
                .cozyPanel()

                if busy { ProgressView("Searching your private video index…") }
                if !conversation.isEmpty {
                    Button("Clear conversation", systemImage: "xmark.circle") { conversation.removeAll() }
                        .font(.callout)
                        .foregroundStyle(CozyTheme.secondary)
                }

                VStack(alignment: .leading, spacing: 12) {
                    Text("Private video index").font(RoomArt.display(24)).foregroundStyle(CozyTheme.ink)
                    Text("This device builds a timestamped index of video frames and available speech or sound clues. The encrypted index stays here; each Room member builds their own. Local scene descriptions help find a moment but are suggestions, not verified facts.")
                        .font(.callout).foregroundStyle(CozyTheme.secondary)
                    if let indexStatus {
                        Text("\(indexStatus.videos.filter(\.indexed).count) of \(indexStatus.videos.count) videos indexed")
                            .font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary)
                    }
                    if !progress.isEmpty { Text(progress).font(.callout).foregroundStyle(CozyTheme.secondary) }
                    HStack(spacing: 12) {
                Button(indexing ? "Stop indexing" : (includeRemoteVideos ? "Index local + remote videos" : "Index local videos"), systemImage: indexing ? "stop.fill" : "viewfinder") {
                    if indexing { indexTask?.cancel() }
                    else { indexTask = Task { await indexVideos(includeRemoteVideos: includeRemoteVideos) } }
                }
                .buttonStyle(CozyPrimaryButtonStyle())
                .disabled(videos.isEmpty || (unindexedVideoCount == 0 && !indexing))
                        if indexStatus?.videos.contains(where: \.indexed) == true {
                            Button("Clear local index") { Task { await clearIndex() } }
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(20)
                .cozyPanel()
            }
            .padding(24)
            .frame(maxWidth: 950)
            .frame(maxWidth: .infinity)
        }
        .background(MemoryGalleryBackdrop())
        .navigationTitle("Ask your videos")
        .task { await loadStatus() }
        .onDisappear { indexTask?.cancel() }
    }

    @MainActor private func search(_ prompt: String, modality requestedModality: String? = nil) async {
        guard let room else { return }
        let prompt = prompt.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !prompt.isEmpty, !busy else { return }
        let requestedModality = requestedModality ?? modality
        let requiresSpeech = videoQueryRequiresSpeechIndex(prompt, modality: requestedModality)
        busy = true
        defer { busy = false }
        do {
            await loadStatus()
            let unindexedIDs = indexStatus?.videoIDsNeedingIndex(modality: requestedModality,
                                                                 includeRemoteVideos: includeRemoteVideos,
                                                                 requireSpeech: requiresSpeech) ?? []
            let unindexed = videos.filter { unindexedIDs.contains($0.id) }
            if !unindexed.isEmpty {
                progress = includeRemoteVideos
                    ? "Downloading and indexing \(unindexed.count) Room video\(unindexed.count == 1 ? "" : "s") for this search…"
                    : "Indexing \(unindexed.count) local video\(unindexed.count == 1 ? "" : "s") for this search…"
                let task = Task {
                    await indexVideos(includeAudio: requestedModality != "frame",
                                      includeFrames: requestedModality != "audio",
                                      includeRemoteVideos: includeRemoteVideos,
                                      requireSpeech: requiresSpeech)
            }
                indexTask = task
                await task.value
            }
            var request: [String: Any] = ["action": "search_video", "room_id": room.id,
                                          "query": prompt, "modality": requestedModality]
            if let previousPrompt = conversation.last?.prompt {
                request["context_query"] = previousPrompt
            }
            let result = try await store.command(request)
            let data = try JSONSerialization.data(withJSONObject: result)
            let answer = try LibraryCoding.decoder.decode(VideoFinderResponse.self, from: data)
            withAnimation(reduceMotion ? nil : .easeOut(duration: 0.18)) {
                conversation.append(VideoFinderTurn(prompt: prompt, modality: requestedModality, response: answer))
            }
            question = ""
        } catch { progress = store.readable(error) }
    }

    private func suggestion(_ prompt: String, modality requestedModality: String) -> some View {
        Button {
            question = prompt
            modality = requestedModality
            Task { await search(prompt, modality: requestedModality) }
        } label: {
            Text(prompt)
                .font(.caption.weight(.medium))
                .foregroundStyle(CozyTheme.ink)
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(CozyTheme.surface, in: RoundedRectangle(cornerRadius: 10))
                .overlay(RoundedRectangle(cornerRadius: 10).stroke(CozyTheme.line, lineWidth: 0.8))
                .fixedSize(horizontal: false, vertical: true)
        }
        .buttonStyle(.plain)
    }

    @MainActor private func loadStatus() async {
        guard let room else { return }
        do {
            let result = try await store.command(["action": "video_index_status", "room_id": room.id])
            let data = try JSONSerialization.data(withJSONObject: result)
            indexStatus = try LibraryCoding.decoder.decode(VideoIndexStatus.self, from: data)
        } catch { progress = store.readable(error) }
    }

    @MainActor private func indexVideos(includeAudio: Bool = true, includeFrames: Bool = true,
                                        includeRemoteVideos: Bool = false,
                                        requireSpeech: Bool = false) async {
        guard let room else { return }
        await loadStatus()
        let requestedModality = includeAudio ? (includeFrames ? "all" : "audio") : "frame"
        let pendingIDs = indexStatus?.videoIDsNeedingIndex(modality: requestedModality,
                                                           includeRemoteVideos: includeRemoteVideos,
                                                           requireSpeech: requireSpeech) ?? []
        let available = videos.filter { pendingIDs.contains($0.id) }
        indexing = true
        defer { indexing = false; indexTask = nil }
        var completed = 0
        var unavailable = 0
        var warnings: [String] = []
        for (number, asset) in available.enumerated() {
            if Task.isCancelled { break }
            let isLocal = indexStatus?.videos.first(where: { $0.assetId == asset.id })?.local ?? asset.local
            progress = isLocal
                ? "Analyzing video \(number + 1) of \(available.count): \(asset.displayTitle)"
                : "Downloading and indexing video \(number + 1) of \(available.count): \(asset.displayTitle)"
            do {
                let url = try await store.materialize(asset)
                let analysis = try await VideoSearchService.shared.analyze(
                    url, includeAudio: includeAudio, includeFrames: includeFrames
                )
                if Task.isCancelled { break }
                _ = try await store.command(["action": "index_video", "room_id": room.id,
                                             "asset_id": asset.id,
                                             "audio_indexed": analysis.audioIndexed,
                                             "sound_events_indexed": analysis.soundEventsIndexed,
                                             "frames_indexed": analysis.framesIndexed,
                                             "preserve_existing": true,
                                             "frame_interval_ms": analysis.frameIntervalMs,
                                             "observations": analysis.observations.map(\.commandValue)])
                completed += 1
                let analysisMessages = [
                    analysis.frameMessage.isEmpty ? nil : analysis.frameMessage,
                    includeAudio && (!analysis.audioIndexed || !analysis.soundEventsIndexed)
                        ? "\(analysis.audioMessage) \(analysis.soundMessage)" : nil
                ].compactMap { $0 }
                if !analysisMessages.isEmpty {
                    warnings.append("\(asset.displayTitle): \(analysisMessages.joined(separator: " "))")
                }
            } catch {
                unavailable += 1
                warnings.append("\(asset.displayTitle): \(store.readable(error))")
            }
        }
        let completion = Task.isCancelled
            ? "Indexing stopped. \(completed) videos indexed."
            : "\(completed) new videos indexed; \(unavailable) waiting or unavailable."
        progress = ([completion] + warnings.prefix(3)).joined(separator: " ")
        await loadStatus()
    }

    @MainActor private func clearIndex() async {
        guard let room else { return }
        do {
            _ = try await store.command(["action": "clear_video_index", "room_id": room.id])
            conversation.removeAll()
            progress = "The local video index was cleared. Originals are unchanged."
            await loadStatus()
        } catch { progress = store.readable(error) }
    }

private func timeLabel(_ milliseconds: UInt64) -> String {
        let seconds = milliseconds / 1_000
        return String(format: "%02d:%02d:%02d", seconds / 3_600, (seconds / 60) % 60, seconds % 60)
    }
}

private func timeRange(_ start: UInt64, _ end: UInt64) -> String {
    func formatted(_ milliseconds: UInt64) -> String {
        let seconds = milliseconds / 1_000
        return String(format: "%02d:%02d:%02d", seconds / 3_600, (seconds / 60) % 60, seconds % 60)
    }
    let first = formatted(start)
    guard end > start else { return "At \(first)" }
    return "\(first)–\(formatted(end))"
}
