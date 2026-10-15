import SwiftUI

extension LibraryRoom {
    func films(in playlist: LibraryPlaylist) -> [FilmProject] {
        let published = latestStories.filter { !$0.isPrivateDraft && $0.publishedAsset != nil }
        return playlist.storyIds.compactMap { storyId in published.first { $0.id == storyId } }
    }
}

struct FilmPlaylistSection: View {
    @EnvironmentObject private var store: LibraryStore
    let room: LibraryRoom
    let onPlay: (LibraryPlaylist) -> Void
    let onEdit: (LibraryPlaylist) -> Void
    @State private var title = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 15) {
            HStack {
                Text("Playlists").font(RoomArt.display(32, relativeTo: .title2))
                Spacer()
                Text("\(room.curatedPlaylists.count)")
                    .font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary)
            }
            if room.canContribute {
                HStack {
                    TextField("Playlist name", text: $title).textFieldStyle(.roundedBorder)
                        .accessibilityIdentifier("playlist.title")
                    Button("Create playlist", systemImage: "plus") {
                        let name = title.trimmingCharacters(in: .whitespacesAndNewlines)
                        store.perform {
                            _ = try await store.command(["action":"save_playlist", "room_id":room.id,
                                                         "title":name, "story_ids":[]])
                            title = ""
                        }
                    }
                    .buttonStyle(CozyPrimaryButtonStyle())
                    .disabled(title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .accessibilityIdentifier("playlist.create")
                }
            }
            if room.curatedPlaylists.isEmpty {
                Text("Collect your published films in a watch order for the Room.")
                    .foregroundStyle(CozyTheme.secondary)
            }
            ForEach(room.curatedPlaylists.sorted { $0.title.localizedStandardCompare($1.title) == .orderedAscending }) { playlist in
                HStack(spacing: 14) {
                    MemoryIconTile(symbol: "play.rectangle.on.rectangle", size: 43)
                    VStack(alignment: .leading, spacing: 3) {
                        Text(playlist.title).font(RoomArt.display(23, relativeTo: .title3)).lineLimit(1)
                        Text("\(room.films(in: playlist).count) published \(room.films(in: playlist).count == 1 ? "film" : "films")")
                            .font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary)
                    }
                    Spacer(minLength: 8)
                    Button("Play all", systemImage: "play.fill") { onPlay(playlist) }
                        .disabled(room.films(in: playlist).isEmpty)
                        .accessibilityIdentifier("playlist.play.\(playlist.id)")
                    if room.canContribute {
                        Button("Edit", systemImage: "slider.horizontal.3") { onEdit(playlist) }
                            .accessibilityIdentifier("playlist.edit.\(playlist.id)")
                    }
                }
                .buttonStyle(.bordered)
                .padding(12)
                .cozyPanel()
            }
        }
        .padding(20)
        .cozyPanel()
    }
}

struct FilmPlaylistEditor: View {
    @EnvironmentObject private var store: LibraryStore
    @Environment(\.dismiss) private var dismiss
    let playlist: LibraryPlaylist
    @State private var title: String
    @State private var storyIds: [String]
    @State private var error: String?
    @State private var saving = false
    @State private var confirmDelete = false

    init(playlist: LibraryPlaylist) {
        self.playlist = playlist
        _title = State(initialValue: playlist.title)
        _storyIds = State(initialValue: playlist.storyIds)
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 19) {
                RoomEyebrow(text: "Theater · Playlist")
                Text("Arrange your films").font(RoomArt.display(36, relativeTo: .largeTitle))
                TextField("Playlist name", text: $title).textFieldStyle(.roundedBorder)
                Text("Watch order").font(RoomArt.display(25, relativeTo: .title2))
                if storyIds.isEmpty { Text("Add a published film below.").foregroundStyle(CozyTheme.secondary) }
                ForEach(storyIds.indices, id: \.self) { offset in
                    let storyId = storyIds[offset]
                    HStack(spacing: 10) {
                        Text("\(offset + 1)").font(.caption.monospacedDigit()).frame(width: 24)
                        Text(film(storyId)?.title ?? "Film unavailable")
                            .lineLimit(2)
                        Spacer()
                        Button("Move up", systemImage: "arrow.up") { storyIds.swapAt(offset, offset - 1) }
                            .disabled(offset == 0).labelStyle(.iconOnly)
                        Button("Move down", systemImage: "arrow.down") { storyIds.swapAt(offset, offset + 1) }
                            .disabled(offset + 1 == storyIds.count).labelStyle(.iconOnly)
                        Button("Remove from playlist", systemImage: "minus.circle") { storyIds.remove(at: offset) }
                            .labelStyle(.iconOnly)
                    }
                    .padding(10).cozyPanel()
                }
                Text("Add published films").font(RoomArt.display(25, relativeTo: .title2))
                if let room = store.room {
                    ForEach(room.latestStories.filter { !$0.isPrivateDraft && $0.publishedAsset != nil && !storyIds.contains($0.id) }, id: \.id) { film in
                        Button("Add \(film.title)", systemImage: "plus.circle") { storyIds.append(film.id) }
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
                if let error { Text(error).foregroundStyle(.red).font(.callout) }
                HStack {
                    Button("Save playlist", systemImage: "checkmark") { save() }
                        .buttonStyle(CozyPrimaryButtonStyle())
                        .disabled(saving || title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                        .accessibilityIdentifier("playlist.save")
                    Button("Delete playlist", systemImage: "trash", role: .destructive) { confirmDelete = true }
                        .disabled(saving)
                    Spacer()
                    Button("Close") { dismiss() }
                }
            }
            .padding(24).frame(maxWidth: 700, alignment: .leading).frame(maxWidth: .infinity)
        }
        .background(MemoryGalleryBackdrop())
        .confirmationDialog("Delete this playlist?", isPresented: $confirmDelete) {
            Button("Delete playlist", role: .destructive) { delete() }
        } message: { Text("The films and their originals stay in the Room.") }
    }

    private func film(_ id: String) -> FilmProject? {
        store.room?.latestStories.first { $0.id == id && !$0.isPrivateDraft }
    }
    private func save() {
        guard let room = store.room else { return }
        saving = true
        Task { @MainActor in
            defer { saving = false }
            do {
                _ = try await store.command(["action":"save_playlist", "room_id":room.id, "id":playlist.id,
                                             "title":title, "story_ids":storyIds])
                try await store.reload()
                dismiss()
            } catch let failure { error = store.readable(failure) }
        }
    }
    private func delete() {
        guard let room = store.room else { return }
        saving = true
        Task { @MainActor in
            defer { saving = false }
            do {
                _ = try await store.command(["action":"delete_playlist", "room_id":room.id, "id":playlist.id])
                try await store.reload()
                dismiss()
            } catch let failure { error = store.readable(failure) }
        }
    }
}
