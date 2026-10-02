import SwiftUI
import AVKit
import PhotosUI
import UniformTypeIdentifiers
import CoreTransferable
import ImageIO
#if os(macOS)
import AppKit
#endif

private enum LibrarySection: String, CaseIterable, Identifiable {
    case home = "Home", library = "Library", albums = "Albums", people = "People", theater = "Theater"
    var id: String { rawValue }
    var icon: String { switch self { case .home: "house"; case .library: "photo.on.rectangle"; case .albums: "rectangle.stack"; case .people: "person.crop.circle"; case .theater: "play.rectangle" } }
}
struct ImportedMedia: Transferable {
    var url: URL
    static var transferRepresentation: some TransferRepresentation {
        FileRepresentation(importedContentType: .data) { received in
            let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString).appendingPathExtension(received.file.pathExtension)
            try FileManager.default.copyItem(at: received.file, to: url)
            return ImportedMedia(url: url)
        }
    }
}
struct LibraryRootView: View {
    @EnvironmentObject var store: LibraryStore
    @State private var section: LibrarySection? = .home
    @State private var importing = false
    @State private var settings = false
    @State private var searching = false
    @State private var photoSelection: [PhotosPickerItem] = []
    @State private var choosingPhotos = false
    @State private var importDestination: String?
    @State private var photoDestination: String?
    #if os(iOS)
    @Environment(\.horizontalSizeClass) private var sizeClass
    #endif
    var body: some View {
        Group {
            if store.loading { ProgressView("Opening your private library…").frame(maxWidth: .infinity, maxHeight: .infinity) }
            else if store.snapshot.rooms.isEmpty { WelcomeView() }
            else { navigation }
        }
        .tint(CozyTheme.accent)
        .foregroundStyle(CozyTheme.ink)
        .background(CozyTheme.canvas)
        .photosPicker(isPresented: $choosingPhotos, selection: $photoSelection, maxSelectionCount: 100, matching: .any(of: [.images, .videos]), preferredItemEncoding: .current)
        .fileImporter(isPresented: $importing, allowedContentTypes: [.data], allowsMultipleSelection: true) { result in
            switch result {
            case .success(let urls):
                let destination = importDestination
                store.perform {
                guard let destination else { return }
                for url in urls {
                    if url.pathExtension == "frroom" {
                        let scoped = url.startAccessingSecurityScopedResource(); defer { if scoped { url.stopAccessingSecurityScopedResource() } }
                        _ = try await store.command(["action": "import_room", "room_id": destination, "path": url.path])
                    } else { try await store.importURLs([url], into: destination) }
                }
            }
            case .failure(let error): store.error = error.localizedDescription
            }
        }
        .onChange(of: photoSelection) { _, items in
            guard !items.isEmpty else { return }
            let destination = photoDestination
            store.perform {
                guard let destination else { return }
                for item in items {
                    if let media = try await item.loadTransferable(type: ImportedMedia.self) {
                        defer { try? FileManager.default.removeItem(at: media.url) }
                        try await store.importURLs([media.url], into: destination)
                    }
                }
            }
            photoSelection = []
        }
        .sheet(isPresented: $settings) { RoomSettingsView().environmentObject(store) }
        .alert("Family Room", isPresented: Binding(get: { store.error != nil }, set: { if !$0 { store.error = nil } })) {
            Button("OK") { store.error = nil }
        } message: { Text(store.error ?? "") }
        .task(id: store.snapshot.deviceId) { if !store.snapshot.deviceId.isEmpty { await store.watchFolders() } }
        .onChange(of: store.snapshot.rooms.count) { old, new in
            if new > old, let requested = store.setupRoomId, store.room?.id == requested {
                settings = true
                store.setupRoomId = nil
            }
        }
        #if os(macOS)
        .frame(minWidth: 850, minHeight: 600)
        #endif
    }
    @ToolbarContentBuilder private var libraryToolbar: some ToolbarContent {
        if store.room != nil {
            if store.busy {
                ToolbarItem(id: "library.progress", placement: .primaryAction) { ProgressView().controlSize(.small) }
            }
            ToolbarItem(id: "library.search", placement: .primaryAction) {
                Button { section = .library; searching = true } label: {
                    Image(systemName: "magnifyingglass")
                }.accessibilityLabel("Search memories").keyboardShortcut("f", modifiers: .command)
            }
            ToolbarItem(id: "library.add", placement: .primaryAction) {
                Menu {
                    if store.room?.canContribute == true {
                        Button("Import files", systemImage: "folder.badge.plus") { importDestination = store.room?.id; importing = true }.keyboardShortcut("i", modifiers: .command)
                        Button("Choose photos and videos", systemImage: "photo") { photoDestination = store.room?.id; choosingPhotos = true }
                    }
                    Button("Sync now", systemImage: "arrow.triangle.2.circlepath") { Task { await store.replicatePending() } }
                    Button("Room and storage settings", systemImage: "gearshape") { settings = true }
                } label: { Label("Add", systemImage: "plus") }
                .buttonStyle(CozyPrimaryButtonStyle()).accessibilityLabel("Add and manage memories")
            }
            ToolbarItem(id: "library.settings", placement: .primaryAction) {
                Button { settings = true } label: { Image(systemName: "gearshape") }
                    .accessibilityLabel("Room and storage settings").accessibilityIdentifier("room.settings")
            }
        }
    }

    @ViewBuilder private var navigation: some View {
        #if os(iOS)
        if sizeClass == .compact {
            TabView(selection: Binding(get: { section ?? .home }, set: { section = $0 })) {
                ForEach(LibrarySection.allCases) { item in NavigationStack { content(item).navigationBarTitleDisplayMode(.inline).toolbar { libraryToolbar } }.tabItem { Label(item.rawValue, systemImage: item.icon) }.tag(item) }
            }
        } else { desktopNavigation }
        #else
        desktopNavigation
        #endif
    }
    private var desktopNavigation: some View {
        NavigationSplitView {
            VStack(spacing: 0) {
                HStack(spacing: 8) {
                    Image(systemName: "house").foregroundStyle(CozyTheme.accentInk)
                    Text("Family Room").font(.system(.title3, design: .serif).weight(.semibold))
                    Spacer()
                }.padding(.horizontal, 18).padding(.top, 24).padding(.bottom, 20)
                Menu {
                    ForEach(store.snapshot.rooms) { room in Button(room.name) { store.selectRoom(room.id) } }
                    Divider()
                    Button("Manage Rooms…") { settings = true }
                } label: {
                    HStack {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(store.room?.name ?? "Choose a Room").font(.headline).lineLimit(2)
                            Label("Private Room", systemImage: "lock").font(.caption).foregroundStyle(CozyTheme.secondary)
                        }
                        Spacer()
                        Image(systemName: "chevron.up.chevron.down").font(.caption)
                    }.padding(14).cozyPanel(glass: true)
                }.buttonStyle(.plain).padding(.horizontal, 12).padding(.bottom, 12)
                List(LibrarySection.allCases, selection: $section) { item in
                    Label(item.rawValue, systemImage: item.icon).tag(item)
                }.listStyle(.sidebar).scrollContentBackground(.hidden)
                Button { settings = true } label: {
                    HStack(spacing: 10) {
                        Image(systemName: "externaldrive")
                        VStack(alignment: .leading, spacing: 3) {
                            Text("Room storage").font(.callout.weight(.medium))
                            Text(store.snapshot.sources.contains { $0.roomId == store.room?.id } ? "Sources and preservation" : "Saved on this device").font(.caption).foregroundStyle(CozyTheme.secondary)
                        }
                        Spacer()
                        Image(systemName: "chevron.right").font(.caption)
                    }.padding(14).cozyPanel()
                }.buttonStyle(.plain).padding(12)
            }.background(CozyTheme.sidebar)
                .navigationSplitViewColumnWidth(min: 210, ideal: 235, max: 290)
        } detail: { NavigationStack { content(section ?? .home).toolbar { libraryToolbar } } }
    }
    @ViewBuilder private func content(_ item: LibrarySection) -> some View {
        Group {
            switch item {
            case .home: HearthLibraryView(onManageStorage: { settings = true })
            case .library: MediaLibraryView(searchPresented: $searching)
            case .albums: AlbumsLibraryView()
            case .people: PeopleLibraryView()
            case .theater: FilmLibraryView()
            }
        }.navigationTitle(item.rawValue)
    }
}

struct MediaLibraryView: View {
    @EnvironmentObject var store: LibraryStore
    @Binding var searchPresented: Bool
    @State private var scatter = false
    var years: [Int] { Array(Set((store.room?.visibleAssets ?? []).map { Calendar.current.component(.year, from: $0.date) })).sorted(by: >) }
    var body: some View {
        VStack(spacing: 0) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack {
                    Picker("Year", selection: $store.selectedYear) { Text("All years").tag(0); ForEach(years, id: \.self) { Text(String($0)).tag($0) } }.frame(maxWidth: 140)
                    Picker("Media", selection: $store.mediaKind) { Text("All media").tag("all"); Text("Photos").tag("photo"); Text("Videos").tag("video"); Text("Audio").tag("audio") }.frame(maxWidth: 150)
                    Toggle("Favorites", isOn: $store.showFavorites).toggleStyle(.button)
                    Toggle("Photo wall", isOn: $scatter).toggleStyle(.button)
                if let room = store.room, !room.people.isEmpty {
                    Menu("People", systemImage: "person.crop.circle") { ForEach(room.people) { person in
                        Button { if store.selectedPeople.contains(person.id) { store.selectedPeople.remove(person.id) } else { store.selectedPeople.insert(person.id) } } label: { Label(person.name, systemImage: store.selectedPeople.contains(person.id) ? "checkmark.circle.fill" : "circle") }
                    } }
                }
                if !store.selectedPeople.isEmpty {
                    Picker("Match People", selection: $store.matchAllPeople) { Text("Any selected Person").tag(false); Text("All selected People").tag(true) }.frame(maxWidth: 220)
                    Button("Clear People (\(store.selectedPeople.count))") { store.selectedPeople = [] }
                }
                }.padding()
            }
            ScrollView {
                if store.room?.visibleAssets.isEmpty != false {
                    ContentUnavailableView("No memories yet", systemImage: "photo.on.rectangle", description: Text("Use Add to bring photos and videos into this Room."))
                } else if store.assets.isEmpty {
                    ContentUnavailableView {
                        Label("No matching memories", systemImage: "magnifyingglass")
                    } description: {
                        Text("Try another search or adjust your filters.")
                    } actions: {
                        Button("Clear search and filters") { store.clearFilters() }
                    }
                } else {
                    MediaGrid(assets: store.assets, scattered: scatter).padding(24)
                }
            }
        }.searchable(text: $store.search, isPresented: $searchPresented, prompt: "Find a caption, filename, or Person")
    }
}
struct MediaGrid: View {
    let assets: [LibraryAsset]
    var scattered = false
    var body: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 155, maximum: 300), spacing: scattered ? 22 : 12)], spacing: 22) {
            ForEach(Array(assets.enumerated()), id: \.element.id) { index, asset in
                NavigationLink { AssetDetailView(assetId: asset.id, roomId: asset.roomId) } label: {
                    VStack(alignment: .leading, spacing: 9) {
                        ZStack(alignment: .bottomLeading) {
                            AssetPreview(asset: asset).aspectRatio(4 / 3, contentMode: .fit).clipped()
                            if asset.kind == "video" { Image(systemName: "play.fill").font(.caption).foregroundStyle(.white).padding(9).background(.black.opacity(0.65), in: Circle()).padding(10) }
                        }.clipShape(RoundedRectangle(cornerRadius: scattered ? 3 : 12))
                        VStack(alignment: .leading, spacing: 5) {
                            Text(asset.displayTitle).font(.callout.weight(.medium)).lineLimit(1).foregroundStyle(CozyTheme.ink)
                            HStack {
                                Text(asset.date, style: .date)
                                Spacer(minLength: 4)
                                if asset.favorite { Image(systemName: "heart.fill").foregroundStyle(CozyTheme.accentInk) }
                                if !asset.local { Image(systemName: "icloud.and.arrow.down") }
                            }.font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary)
                        }.padding(.horizontal, scattered ? 3 : 0)
                    }.padding(scattered ? 8 : 0)
                        .background(scattered ? CozyTheme.surface : Color.clear)
                        .rotationEffect(.degrees(scattered ? Double(index % 5 - 2) * 1.3 : 0))
                }.buttonStyle(.plain)
                .accessibilityLabel("\(asset.displayTitle), \(asset.date.formatted(date: .abbreviated, time: .omitted)), \(asset.availability)")
            }
        }
    }
}
struct AssetPreview: View {
    @EnvironmentObject var store: LibraryStore
    let asset: LibraryAsset
    @State private var image: CGImage?
    @State private var unavailable = false
    var body: some View {
        ZStack {
            CozyTheme.photoBed
            if let image { Image(decorative: image, scale: 1).resizable().scaledToFill() }
            else { Image(systemName: asset.kind == "video" ? "play.rectangle" : asset.kind == "audio" ? "waveform" : "photo").font(.largeTitle).foregroundStyle(.secondary) }
            if !asset.local { VStack { Spacer(); HStack { Spacer(); Image(systemName: "icloud.and.arrow.down").padding(8).background(.regularMaterial, in: Circle()) }.padding(8) } }
        }
        .accessibilityLabel(asset.displayTitle)
        .task(id: asset.id) {
            guard asset.local, !unavailable else { return }
            do {
                let url = try await store.materialize(asset)
                if asset.kind == "photo" {
                    image = await Task.detached {
                        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else { return nil as CGImage? }
                        return CGImageSourceCreateThumbnailAtIndex(source, 0, [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceThumbnailMaxPixelSize: 600, kCGImageSourceCreateThumbnailWithTransform: true] as CFDictionary)
                    }.value
                } else if asset.kind == "video" {
                    let generator = AVAssetImageGenerator(asset: AVURLAsset(url: url)); generator.appliesPreferredTrackTransform = true; generator.maximumSize = CGSize(width: 600, height: 600)
                    image = try? await generator.image(at: .zero).image
                }
            } catch { unavailable = true }
        }
    }
}
struct AssetDetailView: View {
    @EnvironmentObject var store: LibraryStore
    let assetId: String
    let roomId: String
    @State private var caption = ""
    @State private var comment = ""
    @State private var url: URL?
    @State private var player: AVPlayer?
    @State private var newAlbum = ""
    var room: LibraryRoom? { store.snapshot.rooms.first { $0.id == roomId } }
    var asset: LibraryAsset? { room?.assets.first { $0.id == assetId } }
    var body: some View {
        ScrollView {
            if let asset {
                VStack(alignment: .leading, spacing: 22) {
                    Group {
                        if let player { VideoPlayer(player: player).frame(minHeight: 300) }
                        else if let url, asset.kind == "photo" { FullPhoto(url: url).frame(maxHeight: 700) }
                        else { ContentUnavailableView("Original unavailable", systemImage: "icloud.and.arrow.down", description: Text(asset.availability)); Button("Try downloading original") { load(asset) } }
                    }.frame(maxWidth: .infinity).background(Color.black.opacity(0.025))
                    HStack {
                        Text(asset.date, style: .date).foregroundStyle(.secondary)
                        Spacer()
                        Button(asset.favorite ? "Favorited" : "Favorite", systemImage: asset.favorite ? "heart.fill" : "heart") { store.patch(asset, fields: ["favorite": !asset.favorite]) }.disabled(room?.canContribute != true)
                        if let url { ShareLink(item: url) { Label("Export original", systemImage: "square.and.arrow.up") } }
                    }
                    TextField("Add a caption", text: $caption, axis: .vertical).font(.title2).onSubmit { store.patch(asset, fields: ["caption": caption]) }.disabled(room?.canContribute != true)
                    if room?.canContribute == true { Button("Save caption") { store.patch(asset, fields: ["caption": caption]) } }
                    CaptionHistoryView(roomId: roomId, assetId: assetId)
                    if let room, !room.people.isEmpty {
                        Text("People in this memory").font(.headline)
                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 130))]) {
                            ForEach(room.people) { person in Toggle(person.name, isOn: Binding(get: { asset.people.contains(person.id) }, set: { selected in var tags = asset.people; tags.removeAll { $0 == person.id }; if selected { tags.append(person.id) }; store.patch(asset, fields: ["people": tags]) })).toggleStyle(.button).disabled(!room.canContribute) }
                        }
                    }
                    if let room, room.canContribute {
                        Menu("Add to album", systemImage: "rectangle.stack.badge.plus") {
                            ForEach(room.albums) { album in Button(album.title) { store.perform { _ = try await store.command(["action": "save_album", "room_id": roomId, "id": album.id, "title": album.title, "pinned": album.pinned, "asset_ids": Array(Set(album.assetIds + [assetId]))]) } } }
                        }
                    }
                    Divider()
                    Text("Notes from the Room").font(.title3.bold())
                    ForEach(room?.comments.filter { $0.assetId == assetId } ?? []) { note in VStack(alignment: .leading) { Text(note.body); Text("\(note.author.prefix(8)) · \(Date(timeIntervalSince1970: Double(note.createdAt)).formatted())").font(.caption).foregroundStyle(.secondary) }.padding(12).frame(maxWidth: .infinity, alignment: .leading).background(.thinMaterial, in: RoundedRectangle(cornerRadius: 12)) }
                    HStack { TextField("Leave a note", text: $comment); Button("Post") { let text = comment; store.perform { _ = try await store.command(["action": "comment", "room_id": roomId, "asset_id": assetId, "body": text]); comment = "" } }.disabled(comment.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty) }
                    DisclosureGroup("Original and preservation") {
                        VStack(alignment: .leading, spacing: 10) { Text(asset.availability); Text("Original has \(asset.verifiedCopies) verified \(asset.verifiedCopies == 1 ? "copy" : "copies")."); Text(ByteCountFormatter.string(fromByteCount: Int64(asset.size), countStyle: .file)); Text(asset.sha256).font(.caption.monospaced()).textSelection(.enabled) }.padding(.top, 10)
                    }
                    if room?.canContribute == true { Button("Move to Recently Deleted", role: .destructive) { store.patch(asset, fields: ["deleted": true]) } }
                }.padding(24)
            }
        }.navigationTitle(asset?.displayTitle ?? "Memory")
            .task { if let asset { caption = asset.caption; load(asset) } }
            .onDisappear { player?.pause() }
    }
    private func load(_ asset: LibraryAsset) { Task { do { url = try await store.materialize(asset); if asset.kind == "video" || asset.kind == "audio", let url { player = AVPlayer(url: url) } } catch { store.error = store.readable(error) } } }
}
private struct FullPhoto: View {
    let url: URL
    var body: some View {
        if let source = CGImageSourceCreateWithURL(url as CFURL, nil), let image = CGImageSourceCreateThumbnailAtIndex(source, 0, [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceThumbnailMaxPixelSize: 2400, kCGImageSourceCreateThumbnailWithTransform: true] as CFDictionary) { Image(decorative: image, scale: 1).resizable().scaledToFit() }
    }
}
private struct AlbumsLibraryView: View {
    @EnvironmentObject var store: LibraryStore
    @State private var title = ""
    var body: some View {
        List {
            if let room = store.room {
                if room.canContribute { Section("Create an album") { HStack { TextField("Album title", text: $title); Button("Create") { store.perform { _ = try await store.command(["action": "save_album", "room_id": room.id, "title": title, "asset_ids": []]); title = "" } }.disabled(title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty) } } }
                ForEach(room.albums.sorted { $0.pinned && !$1.pinned }) { album in NavigationLink { AlbumDetailView(albumId: album.id) } label: { Label { VStack(alignment: .leading) { Text(album.title); Text("\(album.assetIds.count) memories").font(.caption).foregroundStyle(.secondary) } } icon: { Image(systemName: album.pinned ? "pin.fill" : "rectangle.stack") } } }
                if room.albums.isEmpty { Text("Create a collection, then add memories from their detail screens.").foregroundStyle(.secondary) }
            }
        }
    }
}
private struct AlbumDetailView: View {
    @EnvironmentObject var store: LibraryStore
    @State private var sharing = false
    let albumId: String
    var album: LibraryAlbum? { store.room?.albums.first { $0.id == albumId } }
    var body: some View {
        ScrollView {
            if let album, let room = store.room {
                VStack(alignment: .leading, spacing: 18) {
                    Button(album.pinned ? "Unpin album" : "Pin album", systemImage: "pin") { store.perform { _ = try await store.command(["action": "save_album", "room_id": room.id, "id": album.id, "title": album.title, "asset_ids": album.assetIds, "pinned": !album.pinned]) } }.disabled(!room.canContribute)
                    if ["owner", "admin"].contains(room.role), room.albumOnly != true {
                        Button("Share this album", systemImage: "person.badge.key.fill") { sharing = true }
                    }
                    MediaGrid(assets: album.assetIds.compactMap { id in room.visibleAssets.first { $0.id == id } })
                    if room.canContribute {
                        DisclosureGroup("Organize album") {
                            ForEach(album.assetIds, id: \.self) { id in
                                HStack {
                                    Text(room.assets.first { $0.id == id }?.displayTitle ?? "Memory")
                                    Spacer()
                                    Button("Remove from album") {
                                        store.perform {
                                            _ = try await store.command(["action": "save_album", "room_id": room.id, "id": album.id, "title": album.title, "pinned": album.pinned, "asset_ids": album.assetIds.filter { $0 != id }])
                                        }
                                    }
                                }
                            }
                        }
                    }
                }.padding(24)
            }
        }.navigationTitle(album?.title ?? "Album")
            .sheet(isPresented: $sharing) {
                if let album, let room = store.room {
                    AlbumSharingView(room: room, album: album).environmentObject(store)
                }
            }
    }
}

private struct AlbumSharingView: View {
    @EnvironmentObject var store: LibraryStore
    @Environment(\.dismiss) var dismiss
    let room: LibraryRoom
    let album: LibraryAlbum
    @State private var deviceCode = ""
    @State private var scopeId = ""
    @State private var invitation = ""
    @State private var publishing = false
    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Text("Share only “\(album.title)”").font(.title2.weight(.semibold))
                    Text("Members receive a separately encrypted copy of this album. They can view it without access to the rest of your Room.")
                    Text("Publish again after changing the album. Previously downloaded memories remain with their recipients.").font(.caption).foregroundStyle(.secondary)
                }
                Section("Publish album") {
                    Button {
                        publishing = true
                        store.perform {
                            defer { publishing = false }
                            // Download only the originals required for this explicit publication.
                            let originals = room.assets.filter { album.assetIds.contains($0.id) && !$0.deleted }
                            let ids = Set(originals.map(\.id) + originals.flatMap(\.components))
                            for asset in room.assets where ids.contains(asset.id) && !asset.deleted {
                                _ = try await store.materialize(asset)
                            }
                            let result = try await store.command(["action": "publish_album", "room_id": room.id, "album_id": album.id])
                            scopeId = result["scope_id"] as? String ?? ""
                            invitation = ""
                        }
                    } label: {
                        Label(publishing ? "Publishing…" : "Publish current album", systemImage: "lock.rectangle.stack")
                    }.disabled(publishing)
                    if !scopeId.isEmpty {
                        Text("Album published. Invite a viewer, then select this album library in Room settings to connect its storage or export an encrypted Room package.").font(.callout)
                    }
                }
                Section("Invite a viewer") {
                    TextField("Their device code", text: $deviceCode)
                    Button("Create album-only invitation") {
                        publishing = true
                        store.perform {
                            defer { publishing = false }
                            let result = try await store.command(["action": "invite_album", "room_id": room.id, "album_id": album.id, "device_id": deviceCode.trimmingCharacters(in: .whitespacesAndNewlines)])
                            scopeId = result["scope_id"] as? String ?? ""
                            invitation = result["invitation"] as? String ?? ""
                        }
                    }.disabled(scopeId.isEmpty || deviceCode.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || publishing)
                    if !invitation.isEmpty {
                        ShareLink(item: invitation) { Label("Share private invitation", systemImage: "square.and.arrow.up") }
                        Text(invitation).font(.caption.monospaced()).textSelection(.enabled)
                    }
                }
            }.navigationTitle("Album sharing")
                .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        }
        #if os(macOS)
        .frame(minWidth: 520, idealWidth: 620, minHeight: 520)
        #endif
    }
}

private struct CaptionHistoryView: View {
    @EnvironmentObject var store: LibraryStore
    var roomId: String
    var assetId: String
    @State private var history: [[String: Any]] = []
    var body: some View {
        DisclosureGroup("Caption history") {
            Button("Load saved captions") { store.perform { let result = try await store.command(["action": "asset_history", "room_id": roomId, "asset_id": assetId]); history = result["history"] as? [[String: Any]] ?? [] } }
            ForEach(history.indices, id: \.self) { index in
                HStack { Text(history[index]["caption"] as? String ?? ""); Spacer(); if store.room?.canContribute == true { Button("Use caption") { store.perform { _ = try await store.command(["action": "patch_asset", "room_id": roomId, "asset_id": assetId, "fields": ["caption": history[index]["caption"] as? String ?? ""]]) } } } }
            }
        }
    }
}
struct MomentEditorView: View {
    @EnvironmentObject var store: LibraryStore
    @Environment(\.dismiss) private var dismiss
    @State private var title = ""
    @State private var featured = false
    @State private var selection: Set<String> = []
    var body: some View {
        NavigationStack {
            Form {
                TextField("Moment title", text: $title)
                Toggle("Feature on Home", isOn: $featured)
                Section("Memories") { ForEach(store.room?.visibleAssets ?? []) { asset in
                    Toggle(asset.displayTitle, isOn: Binding(get: { selection.contains(asset.id) }, set: { if $0 { selection.insert(asset.id) } else { selection.remove(asset.id) } }))
                } }
            }.navigationTitle("Create Moment")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
                    ToolbarItem(placement: .confirmationAction) { Button("Save") { if let room = store.room { store.perform { _ = try await store.command(["action": "save_moment", "room_id": room.id, "title": title, "featured": featured, "asset_ids": Array(selection)]); dismiss() } } }.disabled(title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || selection.isEmpty) }
                }
        }
        #if os(macOS)
        .frame(minWidth: 500, minHeight: 500)
        #endif
    }
}
private struct PeopleLibraryView: View {
    @EnvironmentObject var store: LibraryStore
    @State private var name = ""
    var body: some View {
        List {
            Section { Text("People are the faces in your memories. Naming someone does not invite them to a Room.").foregroundStyle(.secondary) }
            if let room = store.room {
                if room.canContribute { Section("Name a Person") { HStack { TextField("Person’s name", text: $name); Button("Add") { store.perform { _ = try await store.command(["action": "save_person", "room_id": room.id, "name": name]); name = "" } }.disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty) } } }
                ForEach(room.people) { person in
                    NavigationLink { ScrollView { MediaGrid(assets: room.visibleAssets.filter { $0.people.contains(person.id) }).padding(24) }.navigationTitle(person.name) } label: {
                        HStack { Text(person.name); Spacer(); Text("\(room.visibleAssets.filter { $0.people.contains(person.id) }.count) memories").foregroundStyle(.secondary) }
                    }
                }
            }
        }
    }
}
