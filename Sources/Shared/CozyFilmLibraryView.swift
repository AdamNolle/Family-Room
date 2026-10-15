import SwiftUI

struct FilmLibraryView: View {
    @EnvironmentObject var store: LibraryStore
    @State private var draft: FilmProject?
    @State private var opened: FilmProject?
    @State private var screening: FilmProject?
    @State private var editingPlaylist: LibraryPlaylist?
    @State private var playingPlaylist: LibraryPlaylist?
    @State private var reelToSkip: FilmProject?
    @State private var showSkipConfirmation = false
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 26) {
                VStack(alignment: .leading, spacing: 10) {
                    RoomEyebrow(text: store.room?.name ?? "Theater")
                    Text("Your Room, on film.").font(RoomArt.display(40)).foregroundStyle(CozyTheme.ink)
                    Text("Make something to watch together, and keep the originals just as they were.").foregroundStyle(CozyTheme.secondary)
                }.padding(24).frame(maxWidth: .infinity, alignment: .leading).memoryGlass()
                if let room = store.room {
                    FilmPlaylistSection(room: room, onPlay: { playingPlaylist = $0 }, onEdit: { editingPlaylist = $0 })
                    if room.canContribute {
                        ViewThatFits(in: .horizontal) {
                            HStack(spacing: 12) { creationActions(room).fixedSize(horizontal: true, vertical: false) }
                            VStack(alignment: .leading, spacing: 12) { creationActions(room) }
                        }
                    }
                    if room.latestStories.isEmpty {
                        VStack(alignment: .leading, spacing: 16) {
                            MemoryIconTile(symbol: "film.stack", size: 64)
                            Text("The first film is yours to make.").font(RoomArt.display(34, relativeTo: .title2))
                            Text("Start with a few photos or clips. Arrange them, add your soundtrack, and publish when you’re ready.").foregroundStyle(CozyTheme.secondary).frame(maxWidth: 480, alignment: .leading)
                        }.padding(26).frame(maxWidth: .infinity, alignment: .leading).cozyPanel()
                    }
                    ForEach(room.latestStories, id: \.versionId) { project in
                        ViewThatFits(in: .horizontal) {
                            HStack(spacing: 22) { cover(project, room: room).frame(width: 185, height: 118); information(project, room: room) }
                            VStack(alignment: .leading, spacing: 16) { cover(project, room: room).aspectRatio(16 / 9, contentMode: .fit); information(project, room: room) }
                        }.padding(16).cozyPanel()
                    }
                }
            }.padding(24).frame(maxWidth: 1050, alignment: .leading).frame(maxWidth: .infinity)
        }.background(MemoryGalleryBackdrop())
            .sheet(item: $draft) { FilmStudioView(project: $0).environmentObject(store) }
            .sheet(item: $opened) { FilmStudioView(project: $0).environmentObject(store) }
            .sheet(item: $screening) { project in
                if let room = store.room,
                   let id = project.publishedAsset,
                   let asset = room.assets.first(where: { $0.id == id }) {
                    TheaterScreeningView(project: project, asset: asset).environmentObject(store)
                }
            }
            .sheet(item: $editingPlaylist) { playlist in
                FilmPlaylistEditor(playlist: playlist).environmentObject(store)
            }
        .sheet(item: $playingPlaylist) { playlist in
            if let room = store.room {
                let films = room.films(in: playlist)
                if let first = films.first, let id = first.publishedAsset,
                   let asset = room.assets.first(where: { $0.id == id }) {
                    TheaterScreeningView(project: first, asset: asset, queue: films,
                                         playlistTitle: playlist.title).environmentObject(store)
                }
            }
        }
        .confirmationDialog("Skip this private reel?", isPresented: $showSkipConfirmation, titleVisibility: .visible) {
            Button("Skip reel", role: .destructive) {
                guard let project = reelToSkip, let room = store.room else { return }
                store.perform {
                    _ = try await store.command([
                        "action": "skip_auto_reel",
                        "room_id": room.id,
                        "id": project.id
                    ])
                }
                reelToSkip = nil
            }
            Button("Keep reel", role: .cancel) { reelToSkip = nil }
        } message: {
            Text("This removes only the private reel draft. Photos and videos in your Room stay in place.")
        }
    }
    @ViewBuilder private func creationActions(_ room: LibraryRoom) -> some View {
        Button { draft = FilmProject(roomId: room.id) } label: {
            Label("Create a film", systemImage: "plus")
        }.buttonStyle(CozyPrimaryButtonStyle()).accessibilityIdentifier("theater.createFilm")
        Button("Make this week’s reel", systemImage: "sparkles") { makeReel(room) }.buttonStyle(.bordered)
    }
    @ViewBuilder private func cover(_ project: FilmProject, room: LibraryRoom) -> some View {
        if let asset = room.assets.first(where: { $0.id == (project.publishedAsset ?? project.clips.first?.assetId) }) {
            AssetPreview(asset: asset).clipped().memoryPhotoFrame().memoryPhotoMotion()
        } else {
            ZStack { CozyTheme.photoBed; Image(systemName: "film").font(.largeTitle).foregroundStyle(CozyTheme.secondary) }.clipShape(RoundedRectangle(cornerRadius: 10))
        }
    }
    private func information(_ project: FilmProject, room: LibraryRoom) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            RoomEyebrow(text: project.isPrivateDraft ? "Private draft" : "Published film")
            Text(project.title).font(RoomArt.display(30, relativeTo: .title2)).foregroundStyle(CozyTheme.ink).lineLimit(2)
            Text("\(project.clips.count) clips · \(Int(project.duration)) seconds").font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary)
            HStack {
                if project.publishedAsset != nil {
                    Button("Watch", systemImage: "play.fill") { screening = project }
                        .accessibilityHint("Insert the film disc and start playback")
                }
                if room.canContribute && project.supportsEditing {
                    Button("Open studio", systemImage: "slider.horizontal.3") { opened = project }
                }
                if project.autoGenerated && project.isPrivateDraft {
                    Button("Skip reel", systemImage: "xmark") {
                        reelToSkip = project
                        showSkipConfirmation = true
                    }
                    .accessibilityHint("Remove this private reel draft without deleting its photos or videos")
                }
                if room.canContribute && project.publishedAsset != nil && !room.curatedPlaylists.isEmpty {
                    Menu("Add to playlist", systemImage: "text.badge.plus") {
                        ForEach(room.curatedPlaylists) { playlist in
                            Button(playlist.title) {
                                store.perform {
                                    _ = try await store.command(["action":"save_playlist", "room_id":room.id,
                                                                 "id":playlist.id, "title":playlist.title,
                                                                 "story_ids":playlist.storyIds + [project.id]])
                                }
                            }.disabled(playlist.storyIds.contains(project.id))
                        }
                    }
                }
            }.buttonStyle(.bordered)
            if !project.supportsEditing { Text("Update Family Room to edit this project format.").font(.caption).foregroundStyle(CozyTheme.secondary) }
        }.frame(maxWidth: .infinity, alignment: .leading)
    }
    private func makeReel(_ room: LibraryRoom) {
        let now = Int64(Date().timeIntervalSince1970)
        let cutoff = now - 7 * 86_400
        let title = "This week in \(room.name)"
        if let existing = room.latestStories.first(where: {
            $0.autoGenerated && $0.isPrivateDraft && $0.createdAt >= cutoff
        }) {
            draft = existing
            return
        }
        let recent = room.visibleAssets
            .filter { asset in
                guard let contributedAt = asset.contributedAt else { return false }
                return contributedAt >= cutoff && contributedAt <= now + 300
                    && ["photo", "video"].contains(asset.kind)
            }
            .sorted { ($0.contributedAt ?? 0) < ($1.contributedAt ?? 0) }
            .suffix(20)
        guard !recent.isEmpty else {
            store.error = "Add photos or videos to this Room this week first."
            return
        }
        var project = FilmProject(roomId: room.id, title: title, autoGenerated: true)
        project.clips = recent.enumerated().map { index, asset in
            FilmClip(assetId: asset.id, start: Double(index) * 3, duration: 3,
                     fadeIn: 0.3, fadeOut: 0.3)
        }
        store.perform {
            try await store.save(project)
            draft = project
        }
    }
}
