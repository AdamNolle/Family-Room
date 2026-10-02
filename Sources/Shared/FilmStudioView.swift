import SwiftUI
import AVKit

struct FilmStudioView: View {
    @EnvironmentObject var store: LibraryStore
    @Environment(\.dismiss) private var dismiss
    @State var project: FilmProject
    @State private var selected: String?
    @State private var player: AVPlayer?
    @State private var exported: URL?
    @State private var rendering = false
    @State private var renderStatus = ""
    @State private var undoStack: [FilmProject] = []
    @State private var redoStack: [FilmProject] = []
    @State private var autosaveTask: Task<Void, Never>?
    @State private var restoringHistory = false
    @State private var adjustingSlider = false
    @State private var hasUnpublishedChanges = false
    @State private var suppressNextAutosave = false
    @State private var studioError: String?
    @State private var splitting = false
    var room: LibraryRoom? { store.snapshot.rooms.first { $0.id == project.roomId } }
    var selectedIndex: Int? { project.clips.firstIndex { $0.id == selected } }
    var body: some View {
        NavigationStack {
            GeometryReader { geometry in
                if geometry.size.width >= 850 {
                    let timelineHeight = min(320, max(240, geometry.size.height * 0.32))
                    let previewHeight = max(140, geometry.size.height - timelineHeight - 280)
                    VStack(alignment: .leading, spacing: 16) {
                        projectHeading
                        HStack(alignment: .top, spacing: 20) {
                            ScrollView {
                                VStack(alignment: .leading, spacing: 24) {
                                    previewPane(maximumHeight: previewHeight)
                                    mediaBin
                                    chapters
                                    draftNote
                                }
                            }
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                            ScrollView { clipInspector }
                                .frame(width: 300)
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        timeline(scrollVertically: true).frame(height: timelineHeight)
                    }.padding(24).background(CozyTheme.canvas)
                } else {
                    ScrollView {
                        VStack(alignment: .leading, spacing: 24) {
                            projectHeading
                            previewPane()
                            mediaBin
                            timeline()
                            clipInspector
                            chapters
                            draftNote
                        }.padding(24)
                    }.background(CozyTheme.canvas)
                }
            }.navigationTitle("Film studio")
                .toolbar {
                    ToolbarItem(id: "film.close", placement: .cancellationAction) { Button("Close") { autosaveTask?.cancel(); Task { do { if hasUnpublishedChanges || project.isPrivateDraft { try await store.save(project) }; dismiss() } catch { studioError = store.readable(error) } } } }
                }
        }
        .onChange(of: project) { previous, value in
            if suppressNextAutosave { suppressNextAutosave = false; return }
            hasUnpublishedChanges = true
            if restoringHistory { restoringHistory = false }
            else if !adjustingSlider {
                if undoStack.last != previous { undoStack.append(previous) }
                if undoStack.count > 100 { undoStack.removeFirst() }
                redoStack = []
            }
            autosaveTask?.cancel()
            autosaveTask = Task { do { try await Task.sleep(nanoseconds: 1_200_000_000); try Task.checkCancellation(); try await store.save(value) } catch is CancellationError {} catch { studioError = store.readable(error) } }
        }
        .onDisappear { autosaveTask?.cancel(); player?.pause() }
        .alert("Film studio", isPresented: Binding(get: { studioError != nil }, set: { if !$0 { studioError = nil } })) {
            Button("OK") { studioError = nil }
        } message: { Text(studioError ?? "") }
        #if os(macOS)
        .frame(minWidth: 1000, minHeight: 780)
        #endif
    }
    private var projectHeading: some View {
        VStack(alignment: .leading, spacing: 8) {
            RoomEyebrow(text: "PRIVATE FILM STUDIO")
            TextField("Film title", text: $project.title)
                .font(.system(.title, design: .serif).weight(.medium))
                .textFieldStyle(.plain)
            historyControls
        }
    }
    private var historyControls: some View {
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 12) { historyButtons }.fixedSize(horizontal: true, vertical: false)
            VStack(alignment: .leading, spacing: 8) { historyButtons }
        }.buttonStyle(.bordered)
    }
    @ViewBuilder private var historyButtons: some View {
            Button("Undo", systemImage: "arrow.uturn.backward") { guard let last = undoStack.popLast() else { return }; restoringHistory = true; redoStack.append(project); project = last }
                .disabled(undoStack.isEmpty).keyboardShortcut("z", modifiers: .command)
                .accessibilityIdentifier("film.undo")
            Button("Redo", systemImage: "arrow.uturn.forward") { guard let last = redoStack.popLast() else { return }; restoringHistory = true; undoStack.append(project); project = last }
                .disabled(redoStack.isEmpty).keyboardShortcut("z", modifiers: [.command, .shift])
                .accessibilityIdentifier("film.redo")
            Button("Save draft") { project.autoGenerated = false; Task { do { try await store.save(project) } catch { studioError = store.readable(error) } } }
                .disabled(!hasUnpublishedChanges && !project.isPrivateDraft).keyboardShortcut("s", modifiers: .command)
                .accessibilityIdentifier("film.save")
    }
    private var chapters: some View {
        DisclosureGroup("Chapters") {
            ForEach(project.chapters.indices, id: \.self) { index in
                HStack {
                    TextField("Title", text: $project.chapters[index].title)
                    TextField("Seconds", value: $project.chapters[index].time, format: .number).frame(width: 90)
                    Button("Remove") { checkpoint(); project.chapters.remove(at: index) }
                }
            }
            Button("Add chapter") { checkpoint(); project.chapters.append(FilmChapter(title: "Chapter \(project.chapters.count + 1)", time: 0)) }
        }.padding(16).cozyPanel()
    }
    private var draftNote: some View {
        Label("Drafts autosave on this device. Publish when your film is ready for the Room.", systemImage: "lock")
            .font(.caption).foregroundStyle(CozyTheme.secondary)
    }
    private func previewPane(maximumHeight: CGFloat? = nil) -> some View {
        VStack(alignment: .leading, spacing: 14) {
            ZStack {
                Color.black
                if let player { VideoPlayer(player: player) }
                else {
                    VStack(spacing: 12) {
                        Image(systemName: "play.rectangle").font(.system(size: 36, weight: .light))
                        Text("Your film takes shape here").font(.system(.title3, design: .serif))
                        Text("Add clips below, then render a preview.").font(.caption)
                    }.foregroundStyle(.white.opacity(0.85))
                }
            }.aspectRatio(16 / 9, contentMode: .fit).clipShape(RoundedRectangle(cornerRadius: 16)).frame(maxHeight: maximumHeight)
            HStack {
                Button("Preview", systemImage: "play") { render(publish: false) }
                    .disabled(project.clips.isEmpty || rendering)
                if let exported { ShareLink(item: exported) { Label("Export", systemImage: "arrow.down.doc") } }
                Spacer()
                Text("\(Int(project.duration)) sec").font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary)
            }
            Button("Publish movie to Room", systemImage: "square.and.arrow.up") { render(publish: true) }
                .buttonStyle(CozyPrimaryButtonStyle()).disabled(project.clips.isEmpty || rendering)
            if rendering { ProgressView(renderStatus) }
        }
    }

    private var clipInspector: some View {
        VStack(alignment: .leading, spacing: 16) {
            RoomEyebrow(text: "CLIP INSPECTOR")
            if let index = selectedIndex { inspector(index) }
            else {
                Label("Select a clip in the timeline", systemImage: "slider.horizontal.3")
                    .foregroundStyle(CozyTheme.secondary).padding(.vertical, 24)
            }
        }.padding(18).cozyPanel()
    }

    private var mediaBin: some View {
        VStack(alignment: .leading, spacing: 12) {
            RoomSectionHeading(title: "Media bin")
            Text("Tap a memory to add it to the timeline.").font(.caption).foregroundStyle(CozyTheme.secondary)
            ScrollView(.horizontal) {
                HStack(alignment: .top, spacing: 12) {
                    ForEach(room?.visibleAssets ?? []) { asset in
                        Button {
                            checkpoint()
                            let clip = FilmClip(assetId: asset.id, start: project.duration)
                            project.clips.append(clip)
                            selected = clip.id
                        } label: {
                            VStack(alignment: .leading, spacing: 6) {
                                AssetPreview(asset: asset).frame(width: 132, height: 88).clipped()
                                    .clipShape(RoundedRectangle(cornerRadius: 10))
                                Text(asset.filename).font(.caption).lineLimit(1).frame(width: 132, alignment: .leading)
                            }
                        }.buttonStyle(.plain).accessibilityLabel("Add \(asset.filename) to film")
                    }
                }
            }
            if room?.visibleAssets.isEmpty != false {
                Text("Import photos or videos into this Room to begin.").font(.callout).foregroundStyle(CozyTheme.secondary)
            }
        }
    }

    private func timeline(scrollVertically: Bool = false) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            RoomSectionHeading(title: "Timeline")
            Text("Layers play together. Set start times and levels in the inspector.").font(.caption).foregroundStyle(CozyTheme.secondary)
            ScrollView(scrollVertically ? [.horizontal, .vertical] : .horizontal) {
                VStack(alignment: .leading, spacing: 8) {
                    HStack(spacing: 0) {
                        Text("Seconds").frame(width: 80, alignment: .leading)
                        ForEach(0...max(1, Int(ceil(project.duration / rulerStep))), id: \.self) { tick in
                            Text("\(Int(Double(tick) * rulerStep))").frame(width: rulerStep * 28, alignment: .leading)
                        }
                    }.font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary)
                    ForEach(0...max(2, project.clips.map(\.track).max() ?? 0), id: \.self) { track in
                        HStack(spacing: 0) {
                            Label("Layer \(track + 1)", systemImage: "square.stack.3d.up")
                                .font(.caption).frame(width: 80, alignment: .leading)
                            ZStack(alignment: .leading) {
                                RoundedRectangle(cornerRadius: 8).fill(CozyTheme.photoBed)
                                ForEach(project.clips.filter { $0.track == track }) { clip in
                                    Button { selected = clip.id } label: {
                                        Text(room?.assets.first { $0.id == clip.assetId }?.filename ?? "Clip")
                                            .font(.caption).lineLimit(1).padding(8)
                                            .frame(width: max(50, clip.duration / clip.speed * 28), height: 44)
                                            .background(selected == clip.id ? CozyTheme.accent : CozyTheme.surface, in: RoundedRectangle(cornerRadius: 7))
                                            .foregroundStyle(selected == clip.id ? Color.white : CozyTheme.ink)
                                            .overlay(RoundedRectangle(cornerRadius: 7).stroke(CozyTheme.line, lineWidth: selected == clip.id ? 0 : 1))
                                    }.buttonStyle(.plain).offset(x: clip.start * 28)
                                }
                            }.frame(width: max(500, project.duration * 28 + 80), height: 52)
                        }
                    }
                }
            }
        }.padding(18).cozyPanel()
    }

    private var rulerStep: Double { max(5, ceil(project.duration / 500) * 5) }
    private func checkpoint() { if undoStack.last != project { undoStack.append(project) }; if undoStack.count > 100 { undoStack.removeFirst() }; redoStack = [] }
    @ViewBuilder private func inspector(_ index: Int) -> some View {
        GroupBox("Selected clip") {
            VStack(alignment: .leading, spacing: 14) {
                LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], alignment: .leading, spacing: 12) {
                    numeric("Start", value: $project.clips[index].start)
                    numeric("Trim in", value: $project.clips[index].trimIn)
                    numeric("Duration", value: Binding(get: { project.clips[index].duration }, set: { value in
                        guard value.isFinite, value >= 0.001, value <= 86400 else { return }
                        project.clips[index] = project.clips[index].resized(to: value)
                    }))
                    VStack(alignment: .leading) {
                        Text("Layer \(project.clips[index].track + 1)").font(.caption).foregroundStyle(.secondary)
                        Stepper("Change layer", value: $project.clips[index].track, in: 0...15).font(.caption)
                            .accessibilityLabel("Clip layer")
                            .accessibilityValue("\(project.clips[index].track + 1)")
                    }
                }
                adjust("Speed", value: $project.clips[index].speed, range: 0.25...4)
                adjust("Volume", value: $project.clips[index].volume, range: 0...2)
                adjust("Opacity", value: $project.clips[index].opacity, range: 0...1)
                adjust("Exposure", value: $project.clips[index].exposure, range: -3...3)
                adjust("Saturation", value: $project.clips[index].saturation, range: 0...2)
                HStack { numeric("Fade in", value: $project.clips[index].fadeIn); numeric("Fade out", value: $project.clips[index].fadeOut) }
                TextField("Title overlay", text: $project.clips[index].title)
                DisclosureGroup("Keyframes") {
                    VStack(alignment: .leading, spacing: 8) {
                        Button("Fade opacity with keyframes") { checkpoint(); project.clips[index].opacityKeyframes = [FilmKeyframe(time: 0, value: 0), FilmKeyframe(time: min(1, project.clips[index].duration / 2), value: 1), FilmKeyframe(time: project.clips[index].duration, value: 0)] }
                        Button("Duck this audio layer") { checkpoint(); project.clips[index].volumeKeyframes = [FilmKeyframe(time: 0, value: 1), FilmKeyframe(time: min(1, project.clips[index].duration / 2), value: 0.2), FilmKeyframe(time: project.clips[index].duration, value: 1)] }
                    }
                    ForEach(project.clips[index].opacityKeyframes.indices, id: \.self) { frame in HStack { Text("Opacity"); numeric("Time", value: $project.clips[index].opacityKeyframes[frame].time); numeric("Value", value: $project.clips[index].opacityKeyframes[frame].value) } }
                    ForEach(project.clips[index].volumeKeyframes.indices, id: \.self) { frame in HStack { Text("Volume"); numeric("Time", value: $project.clips[index].volumeKeyframes[frame].time); numeric("Value", value: $project.clips[index].volumeKeyframes[frame].value) } }
                }
                HStack {
                    Button("Split in half", systemImage: "scissors") {
                        let before = project, clip = before.clips[index]
                        splitting = true
                        Task {
                            defer { splitting = false }
                            do {
                                let encoded = try LibraryCoding.encoder.encode(before)
                                let value = try JSONSerialization.jsonObject(with: encoded)
                                let response = try await store.command(["action": "split_story_clip", "project": value, "clip_id": clip.id, "source_time": clip.duration / 2])
                                let data = try JSONSerialization.data(withJSONObject: response["project"] as Any)
                                let result = try LibraryCoding.decoder.decode(FilmProject.self, from: data)
                                guard project == before else { studioError = "The draft changed while splitting. Try the split again."; return }
                                checkpoint(); project = result
                            } catch { studioError = store.readable(error) }
                        }
                    }.disabled(splitting || project.clips[index].duration < 0.002).accessibilityIdentifier("film.split")
                    Button("Remove clip", role: .destructive) { checkpoint(); project.clips.remove(at: index); selected = nil }
                }
            }.padding(12)
        }
    }
    private func numeric(_ title: String, value: Binding<Double>) -> some View { VStack(alignment: .leading) { Text(title).font(.caption).foregroundStyle(.secondary); TextField(title, value: value, format: .number).textFieldStyle(.roundedBorder).frame(minWidth: 65) } }
    private func adjust(_ title: String, value: Binding<Double>, range: ClosedRange<Double>) -> some View { HStack { Text(title).frame(width: 85, alignment: .leading); Slider(value: value, in: range, onEditingChanged: { editing in if editing { checkpoint() }; adjustingSlider = editing }).accessibilityLabel(title); Text(value.wrappedValue.formatted(.number.precision(.fractionLength(2)))).font(.caption.monospaced()).frame(width: 45) } }
    private func render(publish: Bool) {
        rendering = true; renderStatus = "Preparing originals…"
        Task {
            defer { rendering = false }
            do {
                guard let room else { return }
                var media: [String: URL] = [:]
                for clip in project.clips { guard let asset = room.assets.first(where: { $0.id == clip.assetId }) else { continue }; media[asset.id] = try await store.materialize(asset) }
                renderStatus = "Rendering layers and audio…"
                let destination = store.cache.appendingPathComponent("Film-\(UUID().uuidString).mp4")
                try await FilmRenderer.render(project, assets: room.assets, urls: media, destination: destination)
                exported = destination; player = AVPlayer(url: destination)
                if publish {
                    let result = try await store.command(["action": "import", "room_id": room.id, "path": destination.path])
                    autosaveTask?.cancel()
                    suppressNextAutosave = true
                    project.publishedAsset = result["id"] as? String
                    project.privateDraft = false
                    project.autoGenerated = false
                    try await store.save(project, publish: true)
                    hasUnpublishedChanges = false
                    await store.replicatePending()
                }
            } catch { studioError = store.readable(error) }
        }
    }
}
