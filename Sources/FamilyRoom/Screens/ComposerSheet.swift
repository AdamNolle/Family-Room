import SwiftUI

/// "Cut a short film": pick ranked clips, stitch on device, save to Stories.
struct ComposerSheet: View {
    @ObservedObject var room: Room
    var moment: Moment?          // nil = this week's reel
    var onGoStories: () -> Void
    var onClose: () -> Void

    @EnvironmentObject private var store: RoomStore
    @State private var candidates: [ClipCandidate] = []
    @State private var selected: Set<UUID> = []
    @State private var title = ""
    @State private var phase: Phase = .pick
    @State private var progress = 0.0
    @State private var error: String?
    @State private var saved: Story?
    @State private var task: Task<Void, Never>?
    @Environment(\.horizontalSizeClass) private var sizeClass

    enum Phase { case pick, stitch, done }

    private var wide: Bool { sizeClass == .regular }
    private var sourceMedia: [MediaItem] {
        if let moment { return moment.mediaList }
        let start = Catalog.weekStart()
        return room.mediaList.filter { ($0.addedAt ?? .distantPast) >= start }
    }
    private var picked: [ClipCandidate] { candidates.filter { selected.contains($0.id) } }
    private var seconds: Double { ClipRanker.totalDuration(selected: picked.count) }
    private var kicker: String { moment.map { "New story · from \($0.displayTitle)" } ?? "New story · this week" }
    private var media: [UUID: MediaItem] { Dictionary(uniqueKeysWithValues: sourceMedia.compactMap { m in m.id.map { ($0, m) } }) }

    var body: some View {
        ScrollView {
        VStack(alignment: .leading, spacing: 0) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 10) {
                    Text(kicker).labelStyle(warm: true)
                    Group {
                        switch phase {
                        case .pick: Text("Cut a short film")
                        case .stitch: Text("Stitching…")
                        case .done: (Text("Saved to ") + Text("Stories").italic())
                        }
                    }
                    .font(.display(wide ? 48 : 38)).foregroundStyle(Theme.ink)
                }
                Spacer()
                if phase != .stitch { CloseButton(action: onClose) }
            }
            .padding(.bottom, 18).overlay(alignment: .bottom) { Hairline() }.padding(.bottom, 18)
            switch phase {
            case .pick: pick
            case .stitch: stitch
            case .done: done
            }
        }
        .padding(wide ? 28 : 20).padding(.bottom, wide ? 0 : 14)
        }
        .scrollIndicators(.hidden)
        .background(Theme.paper2)
        .onAppear(perform: load)
        .onDisappear { task?.cancel() }
        .interactiveDismissDisabled(phase == .stitch)
    }

    private func load() {
        title = moment?.displayTitle ?? "This week in \(room.displayName)"
        let people = (room.people ?? []).filter { !$0.isFaceGroup }.count
        let signals = sourceMedia.map { m in
            ClipSignals(id: m.id ?? UUID(), isVideo: m.isVideo, isFavorite: m.isFavorite, faceCount: m.faces?.count ?? 0,
                        largestFace: (m.faces ?? []).map { $0.boxWidth * $0.boxHeight }.max() ?? 0,
                        capturedAt: m.capturedAt ?? Date(), duration: m.duration)
        }
        candidates = ClipRanker.rank(signals, peopleInRoom: people)
        selected = Set(candidates.prefix(6).map(\.id))
    }

    // MARK: Pick

    private var pick: some View {
        VStack(alignment: .leading, spacing: 0) {
            FieldBox(label: "Title", text: $title, placeholder: "Untitled story")
            HStack { Text("Best moments · ranked on this \(DeviceWord.current)").labelStyle(); Spacer(); Text("3 s each").labelStyle().monospacedDigit() }.padding(.top, 20).padding(.bottom, 10)
            if candidates.isEmpty {
                Text("Nothing to cut from yet. Add a few photos or videos first.").font(.sans(13)).foregroundStyle(Theme.inkMute).padding(.vertical, 20)
            }
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 12), count: wide ? 4 : 2), spacing: 12) {
                ForEach(candidates) { c in
                    let on = selected.contains(c.id)
                    Button { if on { selected.remove(c.id) } else { selected.insert(c.id) }; Feedback.tick() } label: {
                        VStack(alignment: .leading, spacing: 6) {
                            ZStack(alignment: .topTrailing) {
                                if let m = media[c.id] { MediaThumb(item: m, maxSide: 500, showVideoBadge: false) } else { Theme.hair2 }
                                Circle().fill(on ? Theme.warm : Color.black.opacity(0.35)).frame(width: 20, height: 20)
                                    .overlay(Circle().strokeBorder(.white.opacity(on ? 0 : 0.9), lineWidth: 1.5))
                                    .overlay(Image(systemName: "checkmark").font(.system(size: 11, weight: .bold)).foregroundStyle(.white).opacity(on ? 1 : 0))
                                    .padding(7)
                            }
                            .aspectRatio(16 / 9, contentMode: .fit).clipShape(RoundedRectangle(cornerRadius: 4))
                            .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(Theme.warm, lineWidth: on ? 2 : 0))
                            .overlay(alignment: .bottomLeading) { Text(Catalog.dayLabel(c.signals.capturedAt)).font(.num(10, .semibold)).foregroundStyle(.white).shadow(radius: 2).padding(8) }
                            .opacity(on ? 1 : 0.45)
                            Text(c.reason).font(.sans(11.5, .medium)).foregroundStyle(Theme.inkSoft)
                        }
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(on ? .isSelected : [])
                }
            }
            timeline.padding(.top, 18)
            HStack {
                Text("\(picked.count) clips · \(Catalog.duration(seconds))").font(.num(13)).foregroundStyle(Theme.inkSoft)
                Spacer()
                FRButton("Cancel", style: .ghost, action: onClose)
                FRButton("Stitch on this \(DeviceWord.current)", mark: "✦", disabled: picked.isEmpty || title.trimmingCharacters(in: .whitespaces).isEmpty) { start() }
            }
            .padding(.top, 18)
            if let error { Text(error).font(.sans(12.5)).foregroundStyle(Theme.warm).padding(.top, 10) }
        }
    }

    private var timeline: some View {
        HStack(spacing: 2) {
            if picked.isEmpty { Text("Pick at least one clip").font(.sans(12)).foregroundStyle(Theme.inkMute) }
            ForEach(Array(picked.enumerated()), id: \.element.id) { i, c in
                ZStack(alignment: .topLeading) {
                    if let m = media[c.id] { MediaThumb(item: m, maxSide: 200, showVideoBadge: false) }
                    Text("\(i + 1)").font(.num(10, .semibold)).foregroundStyle(.white).shadow(radius: 2).padding(.leading, 6).padding(.top, 8)
                }
                .frame(maxWidth: .infinity)
            }
        }
        .frame(height: 30).frame(maxWidth: .infinity)
        .background(Theme.ink.opacity(0.06), in: RoundedRectangle(cornerRadius: 6))
        .clipShape(RoundedRectangle(cornerRadius: 6))
        .accessibilityHidden(true)
    }

    // MARK: Stitch

    private var stitch: some View {
        VStack(alignment: .leading, spacing: 18) {
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    Capsule().fill(Theme.ink.opacity(0.1))
                    Capsule().fill(Theme.warm).frame(width: max(6, geo.size.width * progress))
                }
            }
            .frame(height: 6)
            HStack {
                Text("Clip \(min(picked.count, Int(progress * Double(picked.count) / 0.6) + 1)) of \(picked.count)").font(.num(13)).foregroundStyle(Theme.inkSoft)
                Spacer()
                Text("Nothing leaves this \(DeviceWord.current)").labelStyle()
            }
        }
        .padding(.vertical, 24)
    }

    private func start() {
        phase = .stitch; progress = 0; error = nil
        let clips = picked.compactMap { c -> StoryRenderer.Clip? in
            guard let m = media[c.id], let data = m.data else { return nil }
            return StoryRenderer.Clip(data: data, isVideo: m.isVideo, fileExtension: m.fileExtension ?? "mov")
        }
        let name = title.trimmingCharacters(in: .whitespaces)
        let subtitle = moment.map { "\(Catalog.dayLabel($0.date ?? Date())) · from \($0.displayTitle)" } ?? "\(Catalog.weekLabel(Catalog.weekStart())) · weekly reel"
        let moment = self.moment, room = self.room, weekly: Date? = moment == nil ? Catalog.weekStart() : nil
        task = Task {
            do {
                let (url, poster) = try await StoryRenderer.render(clips) { p in Task { @MainActor in progress = p } }
                let data = try Data(contentsOf: url)
                try? FileManager.default.removeItem(at: url.deletingLastPathComponent())
                let seconds = Double(clips.count) * ClipRanker.clipSeconds
                saved = store.saveStory(title: name, subtitle: subtitle, movie: data, thumbnail: poster, duration: seconds, moment: moment, room: room, weeklyReel: weekly)
                withAnimation(Theme.ease) { phase = .done }
            } catch is CancellationError {
            } catch {
                self.error = error.localizedDescription
                phase = .pick
            }
        }
    }

    // MARK: Done

    private var done: some View {
        let layout = wide ? AnyLayout(HStackLayout(alignment: .bottom, spacing: 28)) : AnyLayout(VStackLayout(alignment: .leading, spacing: 16))
        return layout {
            FilmFrame(poster: saved?.thumbnail, duration: saved?.duration ?? seconds).frame(maxWidth: wide ? 320 : .infinity)
            VStack(alignment: .leading, spacing: 8) {
                Text(saved?.displayTitle ?? title).font(.display(40)).foregroundStyle(Theme.ink)
                Text("\(picked.count) clips · \(Catalog.duration(seconds)) · \(saved?.subtitle ?? "")").font(.num(13)).foregroundStyle(Theme.inkMute)
                HStack(spacing: 8) { FRButton("Open Stories", action: onGoStories); FRButton("Done", style: .ghost, action: onClose) }.padding(.top, 16)
            }
        }
        .padding(.top, 4)
    }
}
