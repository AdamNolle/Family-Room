import AVKit
import CoreTransferable
import ImageIO
import PhotosUI
import SwiftUI
import UniformTypeIdentifiers

#if os(macOS)
  import AppKit
#endif

private enum LibrarySection: String, CaseIterable, Identifiable {
  case home = "Home"
  case library = "Library"
  case albums = "Albums"
  case people = "People"
  case theater = "Theater"
  var id: String { rawValue }
  var icon: String {
    switch self {
    case .home: "house"
    case .library: "photo.on.rectangle"
    case .albums: "rectangle.stack"
    case .people: "person.crop.circle"
    case .theater: "play.rectangle"
    }
  }
}
struct ImportedMedia: Transferable {
  var url: URL
  static var transferRepresentation: some TransferRepresentation {
    FileRepresentation(importedContentType: .data) { received in
      let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        .appendingPathExtension(received.file.pathExtension)
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
  @State private var presentedAsset: LibraryAssetOpenRequest?
  #if os(iOS)
    @Environment(\.horizontalSizeClass) private var sizeClass
  #endif
  var body: some View {
    Group {
      if store.loading {
        ProgressView("Opening your private library…").frame(
          maxWidth: .infinity, maxHeight: .infinity)
      } else if store.snapshot.rooms.isEmpty {
        WelcomeView()
      } else {
        navigation
      }
    }
    .tint(CozyTheme.accent)
    .foregroundStyle(CozyTheme.ink)
    .background(CozyTheme.canvas)
    .photosPicker(
      isPresented: $choosingPhotos, selection: $photoSelection, maxSelectionCount: 100,
      matching: .any(of: [.images, .videos]), preferredItemEncoding: .current
    )
    .fileImporter(
      isPresented: $importing, allowedContentTypes: [.data], allowsMultipleSelection: true
    ) { result in
      switch result {
      case .success(let urls):
        let destination = importDestination
        store.perform {
          guard let destination else { return }
          for url in urls {
            if url.pathExtension == "frroom" {
              let scoped = url.startAccessingSecurityScopedResource()
              defer { if scoped { url.stopAccessingSecurityScopedResource() } }
              _ = try await store.command([
                "action": "import_room", "room_id": destination, "path": url.path,
              ])
            } else {
              try await store.importURLs([url], into: destination)
            }
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
    .sheet(item: $presentedAsset) { request in
      AssetDetailView(assetId: request.assetId, roomId: request.roomId)
        .environmentObject(store)
    }
    .alert(
      "Family Room",
      isPresented: Binding(get: { store.error != nil }, set: { if !$0 { store.error = nil } })
    ) {
      Button("OK") { store.error = nil }
    } message: {
      Text(store.error ?? "")
    }
    .task(id: store.snapshot.deviceId) {
      if !store.snapshot.deviceId.isEmpty { await store.watchFolders() }
    }
    .onChange(of: store.snapshot.rooms.count) { old, new in
      if new > old, let requested = store.setupRoomId, store.room?.id == requested {
        settings = true
        store.setupRoomId = nil
      }
    }
    .task(id: store.requestedOpenAsset) {
      guard let request = store.requestedOpenAsset else { return }
      store.selectRoom(request.roomId)
      section = .library
      presentedAsset = request
      store.requestedOpenAsset = nil
    }
    #if os(macOS)
      .frame(minWidth: 850, minHeight: 600)
    #endif
  }
  @ToolbarContentBuilder private var libraryToolbar: some ToolbarContent {
    if store.room != nil {
      if store.busy {
        ToolbarItem(id: "library.progress", placement: .primaryAction) {
          ProgressView().controlSize(.small)
        }
      }
      ToolbarItem(id: "library.search", placement: .primaryAction) {
        Button {
          section = .library
          searching = true
        } label: {
          Image(systemName: "magnifyingglass")
        }.accessibilityLabel("Search memories").keyboardShortcut("f", modifiers: .command)
      }
      ToolbarItem(id: "library.add", placement: .primaryAction) {
        Menu {
          if store.room?.canContribute == true {
            Button("Import files", systemImage: "folder.badge.plus") {
              importDestination = store.room?.id
              importing = true
            }.keyboardShortcut("i", modifiers: .command)
            Button("Choose photos and videos", systemImage: "photo") {
              photoDestination = store.room?.id
              choosingPhotos = true
            }
          }
          Button("Sync now", systemImage: "arrow.triangle.2.circlepath") {
            Task { await store.replicatePending() }
          }
          Button("Room and storage settings", systemImage: "gearshape") { settings = true }
        } label: {
          Label("Add", systemImage: "plus")
        }
        .buttonStyle(CozyPrimaryButtonStyle()).accessibilityLabel("Add and manage memories")
      }
      ToolbarItem(id: "library.settings", placement: .primaryAction) {
        Button {
          settings = true
        } label: {
          Image(systemName: "gearshape")
        }
        .accessibilityLabel("Room and storage settings").accessibilityIdentifier("room.settings")
      }
    }
  }

  @ViewBuilder private var navigation: some View {
    #if os(iOS)
      if sizeClass == .compact {
        TabView(selection: Binding(get: { section ?? .home }, set: { section = $0 })) {
          ForEach(LibrarySection.allCases) { item in
            NavigationStack {
              content(item).navigationBarTitleDisplayMode(.inline).toolbar { libraryToolbar }
            }.tabItem { Label(item.rawValue, systemImage: item.icon) }.tag(item)
          }
        }
      } else {
        desktopNavigation
      }
    #else
      desktopNavigation
    #endif
  }
  private var desktopNavigation: some View {
    NavigationSplitView {
      VStack(spacing: 0) {
        HStack(spacing: 8) {
          MemoryIconTile(symbol: "house", size: 29)
          Text("Family Room").font(RoomArt.display(29, relativeTo: .title3))
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
              Label("Private Room", systemImage: "lock").font(.caption).foregroundStyle(
                CozyTheme.secondary)
            }
            Spacer()
            Image(systemName: "chevron.up.chevron.down").font(.caption)
          }.padding(14).cozyPanel(glass: true)
        }.buttonStyle(.plain).padding(.horizontal, 12).padding(.bottom, 12)
        List(LibrarySection.allCases, selection: $section) { item in
          Label {
            Text(item.rawValue)
          } icon: {
            MemoryIconTile(symbol: item.icon, size: 27, selected: section == item)
          }
          .accessibilityIdentifier("section.\(item.rawValue)").tag(item)
        }.listStyle(.sidebar).scrollContentBackground(.hidden)
        Button {
          settings = true
        } label: {
          HStack(spacing: 10) {
            Image(systemName: "externaldrive")
            VStack(alignment: .leading, spacing: 3) {
              Text("Room storage").font(.callout.weight(.medium))
              Text(
                store.snapshot.sources.contains { $0.roomId == store.room?.id }
                  ? "Sources and preservation" : "Saved on this device"
              ).font(.caption).foregroundStyle(CozyTheme.secondary)
            }
            Spacer()
            Image(systemName: "chevron.right").font(.caption)
          }.padding(14).cozyPanel()
        }.buttonStyle(.plain).padding(12)
      }.background(CozyTheme.sidebar)
        .navigationSplitViewColumnWidth(min: 210, ideal: 235, max: 290)
    } detail: {
      NavigationStack { content(section ?? .home).toolbar { libraryToolbar } }
    }
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
  @State private var selecting = false
  @State private var selectedAssetIDs = Set<String>()
  @State private var showingAlbumName = false
  @State private var albumName = ""
  @State private var showingCaptureDatePicker = false
  @State private var captureDateTarget = Date()
  @State private var showingRecentlyDeleted = false
  @State private var showingBulkDeleteConfirmation = false
  private let maxBulkSelection = 500

  private var gridAssets: [LibraryAsset] {
    guard let room = store.room else { return [] }
    guard showingRecentlyDeleted else { return store.assets }
    let term = store.search.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    return room.assets.filter { asset in
      guard asset.deleted else { return false }
      guard !term.isEmpty else { return true }
      let people = room.people.filter { asset.people.contains($0.id) }.map(\.name).joined(
        separator: " ")
      return "\(asset.filename) \(asset.caption) \(people)".lowercased().contains(term)
    }.sorted { $0.capturedAt > $1.capturedAt }
  }

  private var years: [Int] {
    Array(
      Set(
        (store.room?.visibleAssets ?? []).map {
          Calendar.current.component(.year, from: $0.date)
        })
    ).sorted(by: >)
  }

  private var selectedAssets: [LibraryAsset] {
    gridAssets.filter { selectedAssetIDs.contains($0.id) }
  }

  private var canChangeDeletion: Bool {
    guard let room = store.room else { return false }
    return canChangeDeletionForSelection(
      role: room.role,
      currentDeviceID: store.snapshot.deviceId,
      contributors: selectedAssets.map(\.contributor)
    )
  }

  private func setRecentlyDeleted(_ showTrash: Bool) {
    showingRecentlyDeleted = showTrash
    selecting = false
    selectedAssetIDs.removeAll()
    store.selectedYear = 0
    store.selectedMomentId = nil
    store.showFavorites = false
    store.mediaKind = "all"
    store.selectedPeople = []
    store.matchAllPeople = false
  }

  var body: some View {
    ScrollView {
      VStack(spacing: 0) {
        HStack(alignment: .firstTextBaseline) {
          VStack(alignment: .leading, spacing: 8) {
            RoomEyebrow(text: store.room?.name ?? "Your private library")
            Text("Your memories, together.")
              .font(RoomArt.display(32))
              .fixedSize(horizontal: false, vertical: true)
          }
          Spacer(minLength: 12)
          VStack(alignment: .trailing, spacing: 8) {
            NavigationLink {
              VideoFinderView()
            } label: {
              Label("Ask your videos", systemImage: "sparkle.magnifyingglass")
            }
            .buttonStyle(CozyPrimaryButtonStyle())
            Text("\(gridAssets.count) \(gridAssets.count == 1 ? "memory" : "memories")")
              .font(.caption.monospacedDigit())
              .foregroundStyle(CozyTheme.secondary)
          }
        }
        .padding(24)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(CozyTheme.canvas)

        if !showingRecentlyDeleted {
          PhotoSimilarityReview()
            .padding(.horizontal, 20)
            .padding(.top, 14)
        }

        if selecting {
          HStack(spacing: 14) {
            Label(
              selectedAssetIDs.count == maxBulkSelection
                ? "\(selectedAssetIDs.count) selected · maximum per batch"
                : showingRecentlyDeleted
                  ? "\(selectedAssetIDs.count) selected to restore"
                  : "\(selectedAssetIDs.count) selected",
              systemImage: "checkmark.circle.fill"
            )
            .font(.callout.weight(.semibold).monospacedDigit())
            .foregroundStyle(CozyTheme.ink)
            Spacer(minLength: 8)
            Button("Select up to 500") {
              selectedAssetIDs = Set(gridAssets.prefix(maxBulkSelection).map(\.id))
            }
            .disabled(selectedAssetIDs.count == min(gridAssets.count, maxBulkSelection))
            Button("Done") {
              selecting = false
              selectedAssetIDs.removeAll()
            }
          }
          .padding(.horizontal, 24)
          .padding(.vertical, 12)
          .background(CozyTheme.surface)
        } else {
          ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 12) {
              Button("Select", systemImage: "checkmark.circle") {
                selecting = true
              }
              .disabled(store.room?.canContribute != true || gridAssets.isEmpty)

              Button {
                setRecentlyDeleted(!showingRecentlyDeleted)
              } label: {
                Label(
                  showingRecentlyDeleted
                    ? "Back to Library"
                    : "Recently Deleted (\(store.room?.assets.filter(\.deleted).count ?? 0))",
                  systemImage: showingRecentlyDeleted ? "arrow.uturn.backward" : "trash"
                )
              }
              .buttonStyle(.bordered)

              if !showingRecentlyDeleted {
                Picker("Year", selection: $store.selectedYear) {
                  Text("All years").tag(0)
                  ForEach(years, id: \.self) { Text(String($0)) }
                }
                .frame(maxWidth: 140)

                Picker(
                  "Moment",
                  selection: Binding(
                    get: { store.selectedMomentId ?? "" },
                    set: { store.selectedMomentId = $0.isEmpty ? nil : $0 }
                  )
                ) {
                  Text("All Moments").tag("")
                  ForEach((store.room?.moments ?? []).sorted { $0.start > $1.start }) { moment in
                    Text(moment.title).tag(moment.id)
                  }
                }
                .pickerStyle(.menu)
                .frame(maxWidth: 220)

                Picker("Media", selection: $store.mediaKind) {
                  Text("All media").tag("all")
                  Text("Photos").tag("photo")
                  Text("Videos").tag("video")
                  Text("Audio").tag("audio")
                }
                .frame(maxWidth: 150)

                Toggle("Favorites", isOn: $store.showFavorites)
                  .toggleStyle(.button)
                Toggle("Photo wall", isOn: $scatter)
                  .toggleStyle(.button)

                if let room = store.room, !room.people.isEmpty {
                  Menu("People", systemImage: "person.crop.circle") {
                    ForEach(room.people) { person in
                      Button {
                        if store.selectedPeople.contains(person.id) {
                          store.selectedPeople.remove(person.id)
                        } else {
                          store.selectedPeople.insert(person.id)
                        }
                      } label: {
                        Label(
                          person.name,
                          systemImage: store.selectedPeople.contains(person.id)
                            ? "checkmark.circle.fill" : "circle"
                        )
                      }
                    }
                  }
                }

                if !store.selectedPeople.isEmpty {
                  Picker("Match People", selection: $store.matchAllPeople) {
                    Text("Any selected Person").tag(false)
                    Text("All selected People").tag(true)
                  }
                  .frame(maxWidth: 220)
                  Button("Clear People (\(store.selectedPeople.count))") {
                    store.selectedPeople = []
                  }
                }
              }
            }
            .padding()
          }
          .fixedSize(horizontal: false, vertical: true)
        }

        Group {
          if showingRecentlyDeleted && gridAssets.isEmpty {
            ContentUnavailableView(
              "Recently Deleted is empty",
              systemImage: "trash",
              description: Text("Memories moved here stay available for you to restore.")
            )
          } else if !showingRecentlyDeleted && store.room?.visibleAssets.isEmpty != false {
            ContentUnavailableView(
              "No memories yet",
              systemImage: "photo.on.rectangle",
              description: Text("Use Add to bring photos and videos into this Room.")
            )
          } else if gridAssets.isEmpty {
            ContentUnavailableView {
              Label("No matching memories", systemImage: "magnifyingglass")
            } description: {
              Text("Try another search or adjust your filters.")
            } actions: {
              Button("Clear search filters") { store.clearFilters() }
            }
          } else if scatter {
            MediaGrid(
              assets: gridAssets,
              scattered: true,
              selectedIDs: selectedAssetIDs,
              onSelect: selecting ? toggleSelection : nil
            )
            .padding(28)
          } else {
            MediaGrid(
              assets: gridAssets,
              selectedIDs: selectedAssetIDs,
              onSelect: selecting ? toggleSelection : nil
            )
            .padding(24)
          }
        }
      }
      .frame(maxWidth: 1100)
      .frame(maxWidth: .infinity)
    }
    .background(MemoryGalleryBackdrop())
    .searchable(
      text: $store.search, isPresented: $searchPresented,
      prompt: "Find a caption, filename, or Person"
    )
    .safeAreaInset(edge: .bottom, spacing: 0) {
      if selecting && !selectedAssets.isEmpty {
        VStack(spacing: 8) {
          if showingRecentlyDeleted {
            restoreBulkAction
          } else {
            ViewThatFits(in: .horizontal) {
              HStack(spacing: 10) {
                favoriteBulkAction
                personBulkAction
                correctCaptureDateBulkAction
                createAlbumBulkAction
                moveToTrashBulkAction
              }
              VStack(spacing: 8) {
                HStack(spacing: 8) {
                  favoriteBulkAction
                  personBulkAction
                  correctCaptureDateBulkAction
                }
                HStack(spacing: 8) {
                  createAlbumBulkAction.frame(maxWidth: .infinity)
                  moveToTrashBulkAction.frame(maxWidth: .infinity)
                }
              }
            }
          }
          if !showingRecentlyDeleted && !canChangeDeletion {
            Text(
              "Select only memories you contributed, or ask a Room owner or admin to remove the others."
            )
            .font(.caption)
            .foregroundStyle(CozyTheme.secondary)
            .multilineTextAlignment(.center)
          }
        }
        .frame(maxWidth: 520)
        .padding(.horizontal, 18)
        .padding(.vertical, 12)
        .background(.regularMaterial, in: Capsule())
        .padding(.bottom, 10)
        .frame(maxWidth: .infinity)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Actions for \(selectedAssets.count) selected memories")
      }
    }
    .alert("Create album", isPresented: $showingAlbumName) {
      TextField("Album name", text: $albumName)
      Button("Create") { createAlbum() }
        .disabled(albumName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
      Button("Cancel", role: .cancel) {}
    } message: {
      Text("The selected memories stay in the Room library and are added to this album.")
    }
    .confirmationDialog(
      "Move selected memories to Recently Deleted?", isPresented: $showingBulkDeleteConfirmation
    ) {
      Button(
        "Move \(selectedAssets.count) \(selectedAssets.count == 1 ? "memory" : "memories")",
        role: .destructive
      ) {
        applyBulkFields(["deleted": true])
      }
      Button("Cancel", role: .cancel) {}
    } message: {
      Text("The originals remain saved. You can restore these memories from Recently Deleted.")
    }
    .sheet(isPresented: $showingCaptureDatePicker) {
      NavigationStack {
        VStack(alignment: .leading, spacing: 20) {
          Text(
            "Choose a new calendar date for the selected memories. Each memory keeps its original time of day and relative order."
          )
          .font(.callout)
          .foregroundStyle(CozyTheme.secondary)
          DatePicker("Capture date", selection: $captureDateTarget, displayedComponents: .date)
            .datePickerStyle(.graphical)
            .labelsHidden()
          Button("Apply to \(selectedAssets.count) memories") {
            applyCaptureDateCorrection()
            showingCaptureDatePicker = false
          }
          .buttonStyle(CozyPrimaryButtonStyle())
          .frame(maxWidth: .infinity)
        }
        .padding(24)
        .navigationTitle("Correct capture date")
        .toolbar {
          ToolbarItem(placement: .cancellationAction) {
            Button("Cancel") { showingCaptureDatePicker = false }
          }
        }
      }
      .presentationDetents([.medium, .large])
    }
    .onChange(of: store.room?.id) { _, _ in
      selectedAssetIDs.removeAll()
      selecting = false
      showingRecentlyDeleted = false
      showingBulkDeleteConfirmation = false
      showingCaptureDatePicker = false
    }
  }

  @ViewBuilder
  private var favoriteBulkAction: some View {
    Button {
      applyBulkFields(["favorite": !selectedAssets.allSatisfy(\.favorite)])
    } label: {
      Label(
        selectedAssets.allSatisfy(\.favorite) ? "Unfavorite" : "Favorite",
        systemImage: selectedAssets.allSatisfy(\.favorite) ? "heart.slash" : "heart"
      )
    }
    .buttonStyle(.bordered)
  }

  @ViewBuilder
  private var personBulkAction: some View {
    if let room = store.room, !room.people.isEmpty {
      Menu {
        ForEach(room.people) { person in
          let anyTagged = selectedAssets.contains { $0.people.contains(person.id) }
          let allTagged = selectedAssets.allSatisfy { $0.people.contains(person.id) }
          if !allTagged {
            Button("Add \(person.name) to selection", systemImage: "person.crop.circle.badge.plus")
            {
              applyPersonTag(person.id, remove: false)
            }
          }
          if anyTagged {
            Button(
              "Remove \(person.name) from selection", systemImage: "person.crop.circle.badge.minus"
            ) {
              applyPersonTag(person.id, remove: true)
            }
          }
        }
      } label: {
        Label("Tag Person", systemImage: "person.crop.circle.badge.plus")
      }
      .buttonStyle(.bordered)
    }
  }

  private var createAlbumBulkAction: some View {
    Button {
      albumName = ""
      showingAlbumName = true
    } label: {
      Label("Create album", systemImage: "rectangle.stack.badge.plus")
    }
    .buttonStyle(CozyPrimaryButtonStyle())
  }

  private var correctCaptureDateBulkAction: some View {
    Button {
      captureDateTarget = selectedAssets.first?.date ?? Date()
      showingCaptureDatePicker = true
    } label: {
      Label("Correct date", systemImage: "calendar.badge.clock")
    }
    .buttonStyle(.bordered)
  }

  private var moveToTrashBulkAction: some View {
    Button(role: .destructive) {
      showingBulkDeleteConfirmation = true
    } label: {
      Label("Recently Deleted", systemImage: "trash")
    }
    .buttonStyle(.bordered)
    .disabled(!canChangeDeletion)
    .accessibilityHint(
      "Moves selected memories to Recently Deleted without removing the original files.")
  }

  private var restoreBulkAction: some View {
    Button {
      applyBulkFields(["deleted": false])
    } label: {
      Label("Restore \(selectedAssets.count)", systemImage: "arrow.uturn.backward")
    }
    .buttonStyle(CozyPrimaryButtonStyle())
    .disabled(!canChangeDeletion)
  }

  private func toggleSelection(_ assetID: String) {
    if selectedAssetIDs.contains(assetID) {
      selectedAssetIDs.remove(assetID)
    } else if selectedAssetIDs.count < maxBulkSelection {
      selectedAssetIDs.insert(assetID)
    }
  }

  private func applyBulkFields(_ fields: [String: Any]) {
    guard let room = store.room else { return }
    let ids = selectedAssetIDs.sorted()
    store.perform {
      _ = try await store.command([
        "action": "patch_assets",
        "room_id": room.id,
        "asset_ids": ids,
        "fields": fields,
      ])
      selectedAssetIDs.removeAll()
      selecting = false
    }
  }

  private func applyPersonTag(_ personID: String, remove: Bool) {
    guard let room = store.room else { return }
    let ids = selectedAssetIDs.sorted()
    store.perform {
      _ = try await store.command([
        "action": "tag_assets",
        "room_id": room.id,
        "asset_ids": ids,
        "person_id": personID,
        "remove": remove,
      ])
      selectedAssetIDs.removeAll()
      selecting = false
    }
  }

  private func applyCaptureDateCorrection() {
    guard let room = store.room, !selectedAssets.isEmpty else { return }
    let updates = selectedAssets.map { asset in
      [
        "asset_id": asset.id,
        "captured_at": correctedCaptureTimestamp(asset.capturedAt, to: captureDateTarget),
      ] as [String: Any]
    }
    store.perform {
      _ = try await store.command([
        "action": "patch_asset_dates",
        "room_id": room.id,
        "dates": updates,
      ])
      selectedAssetIDs.removeAll()
      selecting = false
    }
  }

  private func createAlbum() {
    guard let room = store.room else { return }
    let ids = selectedAssetIDs.sorted()
    let title = albumName.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !ids.isEmpty, !title.isEmpty else { return }
    store.perform {
      _ = try await store.command([
        "action": "save_album",
        "room_id": room.id,
        "title": title,
        "asset_ids": ids,
      ])
      selectedAssetIDs.removeAll()
      selecting = false
    }
  }
}

private struct PhotoSimilarityReview: View {
  @EnvironmentObject private var store: LibraryStore
  @State private var response: [String: Any]?
  @State private var isScanning = false
  @State private var failure: String?

  private var suggestions: [[String: Any]] {
    response?["suggestions"] as? [[String: Any]] ?? []
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 4) {
          Text("Possible look-alikes")
            .font(RoomArt.display(22))
            .foregroundStyle(CozyTheme.ink)
          Text("A private visual check. Compare suggestions before making changes.")
            .font(.caption)
            .foregroundStyle(CozyTheme.secondary)
            .fixedSize(horizontal: false, vertical: true)
        }
        Spacer(minLength: 4)
        Button {
          Task { await scan() }
        } label: {
          Label(isScanning ? "Checking…" : "Find similar", systemImage: "square.stack.3d.up")
        }
        .buttonStyle(.bordered)
        .disabled(isScanning || store.room == nil)
        .accessibilityIdentifier("similar-photos.scan")
      }

      if let failure {
        Text(failure)
          .font(.caption)
          .foregroundStyle(CozyTheme.secondary)
      }

      if let response {
        let indexed = response["indexed_photos"] as? Int ?? 0
        let unavailable = response["unavailable_locally"] as? Int ?? 0
        let skipped = response["unsupported_or_oversized"] as? Int ?? 0
        Text(
          "\(indexed) photos checked on this device · \(unavailable) originals unavailable · \(skipped) skipped"
        )
        .font(.caption.monospacedDigit())
        .foregroundStyle(CozyTheme.secondary)

        if suggestions.isEmpty {
          Text("No close visual matches were found in the photos checked.")
            .font(.callout)
            .foregroundStyle(CozyTheme.secondary)
        } else {
          ScrollView(.horizontal, showsIndicators: false) {
            HStack(alignment: .top, spacing: 12) {
              ForEach(Array(suggestions.prefix(30).enumerated()), id: \.offset) { index, pair in
                duplicatePair(pair)
                  .accessibilityIdentifier("similar-photos.pair.\(index)")
              }
            }
          }
          if response["comparison_limit_reached"] as? Bool == true
            || response["result_limit_reached"] as? Bool == true
          {
            Text("Showing a bounded set of matches. Run the check again after reviewing these.")
              .font(.caption)
              .foregroundStyle(CozyTheme.secondary)
          }
        }
      }
    }
    .padding(16)
    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 22, style: .continuous))
    .overlay {
      RoundedRectangle(cornerRadius: 22, style: .continuous)
        .strokeBorder(CozyTheme.line.opacity(0.72), lineWidth: 1)
    }
    .task(id: store.room?.id) {
      response = nil
      failure = nil
    }
  }

  @ViewBuilder
  private func duplicatePair(_ pair: [String: Any]) -> some View {
    let firstID = (pair["first"] as? [String: Any])?["asset_id"] as? String
    let secondID = (pair["second"] as? [String: Any])?["asset_id"] as? String
    let assets = store.room?.visibleAssets ?? []
    HStack(alignment: .top, spacing: 10) {
      if let firstID, let first = assets.first(where: { $0.id == firstID }) {
        duplicateMember(first)
      }
      if let secondID, let second = assets.first(where: { $0.id == secondID }) {
        duplicateMember(second)
      }
    }
    .padding(10)
    .background(CozyTheme.surface.opacity(0.8), in: RoundedRectangle(cornerRadius: 16))
    .overlay {
      RoundedRectangle(cornerRadius: 16)
        .strokeBorder(Color.black.opacity(0.12), lineWidth: 1)
    }
  }

  private func duplicateMember(_ asset: LibraryAsset) -> some View {
    NavigationLink {
      AssetDetailView(assetId: asset.id, roomId: asset.roomId)
    } label: {
      VStack(alignment: .leading, spacing: 6) {
        AssetPreview(asset: asset)
          .frame(width: 124, height: 88)
          .clipShape(RoundedRectangle(cornerRadius: 10))
          .overlay {
            RoundedRectangle(cornerRadius: 10)
              .strokeBorder(Color.black, lineWidth: 1)
          }
        Text(asset.displayTitle)
          .font(.caption.weight(.medium))
          .lineLimit(2)
          .foregroundStyle(CozyTheme.ink)
        Text(asset.date, style: .date)
          .font(.system(size: 10, design: .monospaced))
          .foregroundStyle(CozyTheme.secondary)
      }
      .frame(width: 124, alignment: .leading)
      .contentShape(Rectangle())
    }
    .buttonStyle(.plain)
    .accessibilityLabel(
      "Review \(asset.displayTitle), \(asset.date.formatted(date: .abbreviated, time: .omitted))")
  }

  @MainActor
  private func scan() async {
    guard let room = store.room else { return }
    isScanning = true
    failure = nil
    defer { isScanning = false }
    do {
      response = try await store.command([
        "action": "suggest_photo_duplicates",
        "room_id": room.id,
      ])
    } catch {
      failure = error.localizedDescription
    }
  }
}

struct MediaGrid: View {
  let assets: [LibraryAsset]
  var scattered = false
  var selectedIDs = Set<String>()
  var onSelect: ((String) -> Void)? = nil

  var body: some View {
    LazyVGrid(
      columns: [GridItem(.adaptive(minimum: 155, maximum: 300), spacing: scattered ? 22 : 12)],
      spacing: 22
    ) {
      ForEach(Array(assets.enumerated()), id: \.element.id) { index, asset in
        if let onSelect {
          Button {
            onSelect(asset.id)
          } label: {
            mediaCard(asset, index: index)
              .overlay(alignment: .topTrailing) {
                Image(
                  systemName: selectedIDs.contains(asset.id) ? "checkmark.circle.fill" : "circle"
                )
                .font(.title2)
                .symbolRenderingMode(.palette)
                .foregroundStyle(.white, CozyTheme.accentInk)
                .padding(9)
                .accessibilityHidden(true)
              }
          }
          .buttonStyle(.plain)
          .accessibilityLabel(
            "\(asset.displayTitle), \(asset.date.formatted(date: .abbreviated, time: .omitted)), "
              + "\(selectedIDs.contains(asset.id) ? "selected" : "not selected")"
          )
          .accessibilityAddTraits(selectedIDs.contains(asset.id) ? .isSelected : [])
        } else {
          NavigationLink {
            AssetDetailView(assetId: asset.id, roomId: asset.roomId)
          } label: {
            mediaCard(asset, index: index)
          }
          .buttonStyle(MemoryPhotoButtonStyle())
          .accessibilityLabel(
            "\(asset.displayTitle), \(asset.date.formatted(date: .abbreviated, time: .omitted)), \(asset.availability)"
          )
        }
      }
    }
  }

  @ViewBuilder
  private func mediaCard(_ asset: LibraryAsset, index: Int) -> some View {
    VStack(alignment: .leading, spacing: 9) {
      ZStack(alignment: .bottomLeading) {
        AssetPreview(asset: asset)
          .aspectRatio(4 / 3, contentMode: .fit)
          .clipped()
        if asset.kind == "video" {
          Image(systemName: "play.fill")
            .font(.caption)
            .foregroundStyle(.white)
            .padding(9)
            .background(.black.opacity(0.65), in: Circle())
            .padding(10)
        }
      }
      .memoryPhotoFrame(radius: scattered ? 2 : 8)
      .memoryPhotoMotion()

      VStack(alignment: .leading, spacing: 5) {
        if !asset.caption.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
          Text(asset.caption)
            .font(.callout.weight(.medium))
            .lineLimit(2)
            .foregroundStyle(CozyTheme.ink)
        }
        HStack {
          Text(asset.date, style: .date)
          Spacer(minLength: 4)
          if asset.favorite { Image(systemName: "heart.fill").foregroundStyle(CozyTheme.accentInk) }
          if !asset.local { Image(systemName: "icloud.and.arrow.down") }
        }
        .font(.system(.caption, design: .monospaced))
        .foregroundStyle(CozyTheme.secondary)
      }
      .padding(.horizontal, scattered ? 3 : 0)
    }
    .padding(scattered ? 8 : 0)
    .background(scattered ? CozyTheme.surface : Color.clear)
    .rotationEffect(.degrees(scattered ? Double(index % 5 - 2) * 1.3 : 0))
  }
}

struct AssetPreview: View {
  @EnvironmentObject var store: LibraryStore
  let asset: LibraryAsset
  @State private var image: CGImage?
  @State private var unavailable = false
  var body: some View {
    GeometryReader { geometry in
      ZStack {
        CozyTheme.photoBed
        if let image {
          Image(decorative: image, scale: 1).resizable().scaledToFill()
            .frame(width: geometry.size.width, height: geometry.size.height).clipped()
        } else {
          Image(
            systemName: asset.kind == "video"
              ? "play.rectangle" : asset.kind == "audio" ? "waveform" : "photo"
          ).font(.largeTitle).foregroundStyle(.secondary)
        }
        if !asset.local {
          VStack {
            Spacer()
            HStack {
              Spacer()
              Image(systemName: "icloud.and.arrow.down").padding(8).background(
                .regularMaterial, in: Circle())
            }.padding(8)
          }
        }
      }.frame(width: geometry.size.width, height: geometry.size.height).clipped()
    }
    .accessibilityLabel(asset.displayTitle)
    .task(id: asset.id) {
      guard asset.local, !unavailable else { return }
      do {
        let url = try await store.materialize(asset)
        if asset.kind == "photo" {
          image = await Task.detached {
            guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else {
              return nil as CGImage?
            }
            return CGImageSourceCreateThumbnailAtIndex(
              source, 0,
              [
                kCGImageSourceCreateThumbnailFromImageAlways: true,
                kCGImageSourceThumbnailMaxPixelSize: 600,
                kCGImageSourceCreateThumbnailWithTransform: true,
              ] as CFDictionary)
          }.value
        } else if asset.kind == "video" {
          let generator = AVAssetImageGenerator(asset: AVURLAsset(url: url))
          generator.appliesPreferredTrackTransform = true
          generator.maximumSize = CGSize(width: 600, height: 600)
          image = try? await generator.image(at: .zero).image
        }
      } catch { unavailable = true }
    }
  }
}
struct VideoSpeechCue: Decodable, Identifiable {
  let startMs: UInt64
  let endMs: UInt64
  let text: String

  var id: String { "\(startMs):\(endMs):\(text)" }

  func isActive(at milliseconds: UInt64) -> Bool {
    startMs <= milliseconds && milliseconds < endMs
  }
}

private struct VideoSpeechCaptionOverlay: View {
  let player: AVPlayer
  let cues: [VideoSpeechCue]

  private var playbackMilliseconds: UInt64? {
    let seconds = player.currentTime().seconds
    guard seconds.isFinite, seconds >= 0 else { return nil }
    return UInt64(seconds * 1_000)
  }

  var body: some View {
    TimelineView(.periodic(from: .now, by: 0.2)) { _ in
      if let playbackMilliseconds,
        let cue = cues.first(where: { $0.isActive(at: playbackMilliseconds) })
      {
        Text(cue.text)
          .font(.title3.weight(.semibold))
          .multilineTextAlignment(.center)
          .fixedSize(horizontal: false, vertical: true)
          .foregroundStyle(.white)
          .padding(.horizontal, 18)
          .padding(.vertical, 12)
          .background(.black.opacity(0.78), in: RoundedRectangle(cornerRadius: 14))
          .shadow(color: .black.opacity(0.28), radius: 12, y: 4)
          .accessibilityElement(children: .ignore)
          .accessibilityLabel("Spoken caption")
          .accessibilityValue(cue.text)
          .frame(maxWidth: 760)
          .frame(maxWidth: .infinity)
          .padding(.horizontal, 20)
          .padding(.bottom, 18)
      }
    }
  }
}

struct AssetDetailView: View {
  @EnvironmentObject var store: LibraryStore
  let assetId: String
  let roomId: String
  let seekToSeconds: Double?
  init(assetId: String, roomId: String, seekToSeconds: Double? = nil) {
    self.assetId = assetId
    self.roomId = roomId
    self.seekToSeconds = seekToSeconds
  }
  @State private var caption = ""
  @State private var comment = ""
  @State private var url: URL?
  @State private var player: AVPlayer?
  @State private var newAlbum = ""
  @State private var transcriptURL: URL?
  @State private var transcriptMessage = ""
  @State private var speechCues: [VideoSpeechCue] = []
  @State private var showSpokenCaptions = true
  var room: LibraryRoom? { store.snapshot.rooms.first { $0.id == roomId } }
  var asset: LibraryAsset? { room?.assets.first { $0.id == assetId } }
  var body: some View {
    ScrollView {
      if let asset {
        VStack(alignment: .leading, spacing: 22) {
          Group {
            if let player {
              ZStack(alignment: .bottom) {
                RoomVideoPlayer(player: player)
                if showSpokenCaptions, !speechCues.isEmpty {
                  VideoSpeechCaptionOverlay(player: player, cues: speechCues)
                }
              }.frame(minHeight: 300)
            } else if let url, asset.kind == "photo" {
              FullPhoto(url: url).frame(maxHeight: 700)
            } else {
              ContentUnavailableView(
                "Original unavailable", systemImage: "icloud.and.arrow.down",
                description: Text(asset.availability))
              Button("Try downloading original") { load(asset) }
            }
          }.frame(maxWidth: .infinity).background(Color.black.opacity(0.025))
          if !speechCues.isEmpty {
            Toggle("Show spoken captions", isOn: $showSpokenCaptions)
              .font(.caption)
              .accessibilityHint("Displays indexed speech while this video plays.")
          }
          HStack {
            Text(asset.date, style: .date).foregroundStyle(.secondary)
            Spacer()
            Button(
              asset.favorite ? "Favorited" : "Favorite",
              systemImage: asset.favorite ? "heart.fill" : "heart"
            ) { store.patch(asset, fields: ["favorite": !asset.favorite]) }.disabled(
              room?.canContribute != true)
            if let url {
              ShareLink(item: url) { Label("Export original", systemImage: "square.and.arrow.up") }
              if asset.kind == "video" && !asset.deleted {
                HStack(spacing: 12) {
                  Button { exportTranscript(asset) } label: {
                    Label("Prepare spoken captions", systemImage: "captions.bubble")
                  }
                  if let transcriptURL {
                    ShareLink(item: transcriptURL) {
                      Label("Save or share captions", systemImage: "square.and.arrow.up")
                    }
                  }
                }
                if !transcriptMessage.isEmpty {
                  Text(transcriptMessage).font(.caption).foregroundStyle(.secondary)
                }
              }
            }
          }
          TextField("Add a caption", text: $caption, axis: .vertical).font(.title2).onSubmit {
            store.patch(asset, fields: ["caption": caption])
          }.disabled(room?.canContribute != true)
          if room?.canContribute == true {
            Button("Save caption") { store.patch(asset, fields: ["caption": caption]) }
          }
          CaptionHistoryView(roomId: roomId, assetId: assetId)
          if let room, !room.people.isEmpty {
            Text("People in this memory").font(.headline)
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 130))]) {
              ForEach(room.people) { person in
                Toggle(
                  person.name,
                  isOn: Binding(
                    get: { asset.people.contains(person.id) },
                    set: { selected in
                      var tags = asset.people
                      tags.removeAll { $0 == person.id }
                      if selected { tags.append(person.id) }
                      store.patch(asset, fields: ["people": tags])
                    })
                ).toggleStyle(.button).disabled(!room.canContribute)
              }
            }
          }
          if let room, room.canContribute {
            Menu("Add to album", systemImage: "rectangle.stack.badge.plus") {
              ForEach(room.albums) { album in
                Button(album.title) {
                  store.perform {
                    _ = try await store.command([
                      "action": "save_album", "room_id": roomId, "id": album.id,
                      "title": album.title, "pinned": album.pinned,
                      "asset_ids": album.assetIds.contains(assetId)
                        ? album.assetIds : album.assetIds + [assetId],
                    ])
                  }
                }
              }
            }
          }
          Divider()
          HStack {
            Text("Notes from the Room").font(.title3.bold())
            Spacer()
            if let room {
              let liked = room.hasLiked(assetId, deviceId: store.snapshot.deviceId)
              Button(
                "\(liked ? "Unlike" : "Like") · \(room.likeCount(for: assetId))",
                systemImage: liked ? "hand.thumbsup.fill" : "hand.thumbsup"
              ) {
                store.perform {
                  _ = try await store.command([
                    "action": "set_like", "room_id": roomId, "asset_id": assetId, "liked": !liked,
                  ])
                }
              }
              .disabled(room.role == "removed")
              .accessibilityLabel(liked ? "Unlike memory" : "Like memory")
              .accessibilityValue("\(room.likeCount(for: assetId)) likes")
            }
          }
          ForEach(room?.comments.filter { $0.assetId == assetId } ?? []) { note in
            let attribution =
              note.author == store.snapshot.deviceId ? "You" : "Member \(note.author.prefix(8))"
            VStack(alignment: .leading) {
              Text(note.body)
              Text(
                "\(attribution) · \(Date(timeIntervalSince1970: Double(note.createdAt)).formatted())"
              ).font(.caption).foregroundStyle(.secondary)
            }.padding(12).frame(maxWidth: .infinity, alignment: .leading).background(
              .thinMaterial, in: RoundedRectangle(cornerRadius: 12))
          }
          HStack {
            TextField("Leave a note", text: $comment)
            Button("Post") {
              let text = comment
              store.perform {
                _ = try await store.command([
                  "action": "comment", "room_id": roomId, "asset_id": assetId, "body": text,
                ])
                comment = ""
              }
            }.disabled(comment.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
          }
          DisclosureGroup("Original and preservation") {
            VStack(alignment: .leading, spacing: 10) {
              Text(asset.availability)
              Text(
                "Original has \(asset.verifiedCopies) verified \(asset.verifiedCopies == 1 ? "copy" : "copies")."
              )
              Text(ByteCountFormatter.string(fromByteCount: Int64(asset.size), countStyle: .file))
              Text(asset.sha256).font(.caption.monospaced()).textSelection(.enabled)
            }.padding(.top, 10)
          }
          if let room,
            canChangeDeletionForSelection(
              role: room.role,
              currentDeviceID: store.snapshot.deviceId,
              contributors: [asset.contributor]
            )
          {
            Button("Move to Recently Deleted", role: .destructive) {
              store.patch(asset, fields: ["deleted": true])
            }
          }
        }.padding(24)
      }
    }.navigationTitle(asset?.displayTitle ?? "Memory")
      .task {
        if let asset {
          caption = asset.caption
          load(asset)
        }
      }
      .onDisappear { player?.pause() }
  }
  private func exportTranscript(_ asset: LibraryAsset) {
    Task {
      do {
        let result = try await store.command([
          "action": "export_video_transcript",
          "room_id": roomId,
          "asset_id": asset.id,
        ])
        let cueCount = result["cue_count"] as? Int ?? 0
        guard cueCount > 0, let webvtt = result["webvtt"] as? String else {
          transcriptURL = nil
          transcriptMessage = "No spoken captions are indexed yet. Use Ask your videos to index speech first."
          return
        }
        let title = URL(fileURLWithPath: asset.filename).deletingPathExtension().lastPathComponent
        let fileName = "\(title.isEmpty ? "video" : title)-captions-\(UUID().uuidString).vtt"
        let destination = FileManager.default.temporaryDirectory.appendingPathComponent(fileName)
        try Data(webvtt.utf8).write(to: destination, options: .atomic)
        transcriptURL = destination
        transcriptMessage = "\(cueCount) recognized speech captions are ready. Review wording against the original."
      } catch {
        transcriptURL = nil
        transcriptMessage = store.readable(error)
      }
    }
  }

  private func loadSpeechCues(for asset: LibraryAsset) async {
    guard asset.kind == "video" else {
      speechCues = []
      return
    }
    do {
      let result = try await store.command([
        "action": "export_video_transcript",
        "room_id": roomId,
        "asset_id": asset.id,
      ])
      guard let rawCues = result["cues"] as? [[String: Any]] else {
        speechCues = []
        return
      }
      let data = try JSONSerialization.data(withJSONObject: rawCues)
      speechCues = try LibraryCoding.decoder.decode([VideoSpeechCue].self, from: data)
    } catch {
      // Playback remains available when a transcript is absent or stale.
      speechCues = []
    }
  }

  private func load(_ asset: LibraryAsset) {
    Task {
      do {
        url = try await store.materialize(asset)
        if asset.kind == "video" || asset.kind == "audio", let url {
          let videoPlayer = AVPlayer(url: url)
          if let seekToSeconds {
            await videoPlayer.seek(to: CMTime(seconds: seekToSeconds, preferredTimescale: 600))
          }
          player = videoPlayer
          await loadSpeechCues(for: asset)
        }
      } catch { store.error = store.readable(error) }
    }
  }
}
private struct FullPhoto: View {
  let url: URL
  var body: some View {
    if let source = CGImageSourceCreateWithURL(url as CFURL, nil),
      let image = CGImageSourceCreateThumbnailAtIndex(
        source, 0,
        [
          kCGImageSourceCreateThumbnailFromImageAlways: true,
          kCGImageSourceThumbnailMaxPixelSize: 2400,
          kCGImageSourceCreateThumbnailWithTransform: true,
        ] as CFDictionary)
    {
      Image(decorative: image, scale: 1).resizable().scaledToFit()
    }
  }
}
private struct AlbumsLibraryView: View {
  @EnvironmentObject var store: LibraryStore
  @State private var title = ""
  @FocusState private var titleFocused: Bool
  @State private var creatingAlbum = false
  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 26) {
        MemoryPageHeading(
          room: store.room?.name, title: "Collections worth keeping.",
          detail: "Bring a few memories together, for any reason.")
        if let room = store.room {
          if room.canContribute {
            DisclosureGroup("New album", isExpanded: $creatingAlbum) {
              HStack(spacing: 12) {
                TextField("Album title", text: $title).textFieldStyle(.roundedBorder).focused(
                  $titleFocused)
                Button("Create", systemImage: "plus") {
                  titleFocused = false
                  store.perform {
                    _ = try await store.command([
                      "action": "save_album", "room_id": room.id, "title": title, "asset_ids": [],
                    ])
                    title = ""
                    creatingAlbum = false
                  }
                }.buttonStyle(CozyPrimaryButtonStyle()).disabled(
                  title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
              }
            }.padding(18).cozyPanel().onAppear { creatingAlbum = room.albums.isEmpty }
          }
          if room.albums.isEmpty {
            MemoryEmptyState(
              icon: "rectangle.stack", title: "Start a collection.",
              detail:
                "Create an album, then add memories from their detail screens. Your originals stay in the Room."
            )
        } else {
          LazyVGrid(
              columns: [GridItem(.adaptive(minimum: 220, maximum: 320), spacing: 22)],
              alignment: .leading, spacing: 26
            ) {
              ForEach(
                room.albums.sorted { lhs, rhs in
                  if lhs.pinned != rhs.pinned { return lhs.pinned }
                  return lhs.title.localizedStandardCompare(rhs.title) == .orderedAscending
                }
              ) { album in
                NavigationLink {
                  AlbumDetailView(albumId: album.id)
                } label: {
                  VStack(alignment: .leading, spacing: 12) {
                    ZStack {
                      if let cover = album.cover(in: room) {
                        AssetPreview(asset: cover)
                      } else {
                        CozyTheme.photoBed
                        Image(systemName: "rectangle.stack").font(.system(size: 34))
                          .foregroundStyle(CozyTheme.accentInk)
                      }
                    }.aspectRatio(4 / 3, contentMode: .fit).memoryPhotoFrame()
                    HStack(alignment: .firstTextBaseline) {
                      Text(album.title).font(RoomArt.display(26, relativeTo: .title2)).lineLimit(2)
                      if album.pinned {
                        Image(systemName: "pin.fill").font(.caption).foregroundStyle(
                          CozyTheme.accentInk)
                      }
                    }
                    if !album.captionText.isEmpty {
                      Text(album.captionText).font(.callout).lineLimit(2).foregroundStyle(
                        CozyTheme.secondary)
                    }
                    Text(
                      "\(album.assetIds.count) \(album.assetIds.count == 1 ? "memory" : "memories")"
                    ).font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary)
                  }.foregroundStyle(CozyTheme.ink)
                }.buttonStyle(.plain).accessibilityLabel(
                  "\(album.title), \(album.assetIds.count) \(album.assetIds.count == 1 ? "memory" : "memories")\(album.pinned ? ", pinned" : "")"
                )
              }
            }
          }
        }
      }.padding(24).frame(maxWidth: 1100, alignment: .leading).frame(maxWidth: .infinity)
    }.background(MemoryGalleryBackdrop())
  }
}
private struct AlbumDetailView: View {
  @EnvironmentObject var store: LibraryStore
  @State private var sharing = false
  @State private var downloading = false
  @State private var downloaded = 0
  @State private var downloadTotal = 0
  @State private var evicting = false
  @State private var albumTitle = ""
  @State private var albumCaption = ""
  let albumId: String
  var album: LibraryAlbum? { store.room?.albums.first { $0.id == albumId } }
  private func albumOriginals(_ album: LibraryAlbum, in room: LibraryRoom) -> [LibraryAsset] {
    let primary = room.visibleAssets.filter { album.assetIds.contains($0.id) }
    let ids = Set(primary.map(\.id) + primary.flatMap(\.components))
    return room.assets.filter { ids.contains($0.id) && !$0.deleted }
  }
  var body: some View {
    ScrollView {
      if let album, let room = store.room {
        VStack(alignment: .leading, spacing: 18) {
          if let cover = album.cover(in: room) {
            AssetPreview(asset: cover)
              .aspectRatio(16 / 9, contentMode: .fit)
              .frame(maxHeight: 420)
              .memoryPhotoFrame()
              .accessibilityLabel("Album cover: \(cover.displayTitle)")
          }
          Text(album.title).font(RoomArt.display(38, relativeTo: .largeTitle))
          if !album.captionText.isEmpty {
            Text(album.captionText).font(.body).foregroundStyle(CozyTheme.secondary)
          }
          let originals = albumOriginals(album, in: room)
          let pinnedCount = originals.filter { $0.pinnedOffline == true }.count
          let allPinned = !originals.isEmpty && pinnedCount == originals.count
          Button {
            downloading = true
            downloaded = 0
            downloadTotal = originals.count
            let action = allPinned ? "unpin_original" : "pin_original"
            let selected = originals.filter {
              allPinned ? $0.pinnedOffline == true : $0.pinnedOffline != true
            }
            store.perform {
              defer { downloading = false }
              for asset in selected {
                do {
                  _ = try await store.command([
                    "action": action, "room_id": room.id, "asset_id": asset.id,
                  ])
                  downloaded += 1
                  try await store.reload()
                } catch {
                  try? await store.reload()
                  throw error
                }
              }
            }
          } label: {
            Label(
              downloading
                ? "Saving originals…"
                : allPinned
                  ? "Unpin offline originals"
                  : "Save originals offline (\(pinnedCount)/\(originals.count))",
              systemImage: allPinned ? "pin.slash" : "arrow.down.circle"
            )
          }
          .disabled(downloading || originals.isEmpty)
          .accessibilityIdentifier("album.offline.\(album.id)")
          if downloading {
            ProgressView(value: Double(downloaded), total: Double(max(1, downloadTotal))) {
              Text("Processed \(downloaded) of \(downloadTotal) originals")
            }
          } else if !originals.isEmpty {
            Text("\(pinnedCount) of \(originals.count) originals pinned for offline use")
              .font(.caption)
              .foregroundStyle(CozyTheme.secondary)
          }
          let evictable = originals.filter {
            $0.local && $0.pinnedOffline != true && $0.verifiedCopies > 1
          }
          if !evictable.isEmpty {
            Button {
              evicting = true
              downloaded = 0
              downloadTotal = evictable.count
              store.perform {
                defer { evicting = false }
                for asset in evictable {
                  do {
                    _ = try await store.command([
                      "action": "evict_original", "room_id": room.id, "asset_id": asset.id,
                    ])
                    downloaded += 1
                    try await store.reload()
                  } catch {
                    try? await store.reload()
                    throw error
                  }
                }
              }
            } label: {
              Label(
                "Free device storage (\(evictable.count))", systemImage: "externaldrive.badge.minus"
              )
            }
            .disabled(evicting || downloading)
            .accessibilityIdentifier("album.evict.\(album.id)")
            Text("Pinned originals and the last verified copy are protected.")
              .font(.caption)
              .foregroundStyle(CozyTheme.secondary)
            if evicting {
              ProgressView(value: Double(downloaded), total: Double(max(1, downloadTotal))) {
                Text("Verified and removed \(downloaded) of \(downloadTotal) local copies")
              }
            }
          }
          Button(album.pinned ? "Unpin album" : "Pin album", systemImage: "pin") {
            store.perform {
              _ = try await store.command([
                "action": "save_album", "room_id": room.id, "id": album.id, "title": album.title,
                "asset_ids": album.assetIds, "pinned": !album.pinned,
              ])
            }
          }.disabled(!room.canContribute)
          if ["owner", "admin"].contains(room.role), room.albumOnly != true {
            Button("Share this album", systemImage: "person.badge.key.fill") { sharing = true }
          }
          MediaGrid(
            assets: album.assetIds.compactMap { id in room.visibleAssets.first { $0.id == id } })
          if room.canContribute {
            DisclosureGroup("Organize album") {
              HStack {
                TextField("Album title", text: $albumTitle)
                  .textFieldStyle(.roundedBorder)
                  .accessibilityIdentifier("album.rename")
                Button("Save name") { saveAlbum(album, in: room, title: albumTitle) }
                  .disabled(albumTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
              }
              TextField("A note about this album", text: $albumCaption, axis: .vertical)
                .lineLimit(2...5)
                .textFieldStyle(.roundedBorder)
                .accessibilityIdentifier("album.caption")
              Button("Save album note") { saveAlbum(album, in: room, caption: albumCaption) }
                .disabled(
                  albumCaption.trimmingCharacters(in: .whitespacesAndNewlines).utf8.count > 2_000)
              if album.coverAssetId != nil {
                Button("Use first memory as cover", systemImage: "photo") {
                  saveAlbum(album, in: room, resetCover: true)
                }
              }
              ForEach(Array(album.assetIds.enumerated()), id: \.element) { index, id in
                HStack {
                  Text(room.assets.first { $0.id == id }?.displayTitle ?? "Memory")
                  Spacer()
                  if album.coverAssetId == id {
                    Label("Cover", systemImage: "photo").font(.caption).foregroundStyle(
                      CozyTheme.secondary)
                  }
                  Menu("Arrange memory", systemImage: "ellipsis.circle") {
                    Button("Use as album cover", systemImage: "photo") {
                      saveAlbum(album, in: room, cover: id)
                    }.disabled(album.coverAssetId == id)
                    Button("Move earlier", systemImage: "arrow.up") {
                      var order = album.assetIds
                      order.swapAt(index, index - 1)
                      saveAlbum(album, in: room, assets: order)
                    }.disabled(index == 0)
                    Button("Move later", systemImage: "arrow.down") {
                      var order = album.assetIds
                      order.swapAt(index, index + 1)
                      saveAlbum(album, in: room, assets: order)
                    }.disabled(index + 1 == album.assetIds.count)
                    Divider()
                    Button("Remove from album", role: .destructive) {
                      saveAlbum(album, in: room, assets: album.assetIds.filter { $0 != id })
                    }
                  }
                }
              }
            }
          }
        }.padding(24)
      }
    }.navigationTitle(album?.title ?? "Album")
      .onAppear {
        albumTitle = album?.title ?? ""
        albumCaption = album?.captionText ?? ""
      }
      .sheet(isPresented: $sharing) {
        if let album, let room = store.room {
          AlbumSharingView(room: room, album: album).environmentObject(store)
        }
      }
  }
  private func saveAlbum(
    _ album: LibraryAlbum, in room: LibraryRoom, title: String? = nil,
    caption: String? = nil, assets: [String]? = nil, cover: String? = nil,
    resetCover: Bool = false
  ) {
    store.perform {
      var request: [String: Any] = [
        "action": "save_album", "room_id": room.id, "id": album.id,
        "title": title?.trimmingCharacters(in: .whitespacesAndNewlines) ?? album.title,
        "pinned": album.pinned, "asset_ids": assets ?? album.assetIds,
      ]
      if let caption { request["caption"] = caption }
      if let cover { request["cover_asset_id"] = cover }
      if resetCover { request["cover_asset_id"] = NSNull() }
      _ = try await store.command(request)
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
          Text(
            "Members receive a separately encrypted copy of this album. They can view it without access to the rest of your Room."
          )
          Text(
            "Publish again after changing the album. Previously downloaded memories remain with their recipients."
          ).font(.caption).foregroundStyle(.secondary)
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
              let result = try await store.command([
                "action": "publish_album", "room_id": room.id, "album_id": album.id,
              ])
              scopeId = result["scope_id"] as? String ?? ""
              invitation = ""
            }
          } label: {
            Label(
              publishing ? "Publishing…" : "Publish current album",
              systemImage: "lock.rectangle.stack")
          }.disabled(publishing)
          if !scopeId.isEmpty {
            Text(
              "Album published. Invite a viewer, then select this album library in Room settings to connect its storage or export an encrypted Room package."
            ).font(.callout)
          }
        }
        Section("Invite a viewer") {
          TextField("Their device code", text: $deviceCode)
          Button("Create album-only invitation") {
            publishing = true
            store.perform {
              defer { publishing = false }
              let result = try await store.command([
                "action": "invite_album", "room_id": room.id, "album_id": album.id,
                "device_id": deviceCode.trimmingCharacters(in: .whitespacesAndNewlines),
              ])
              scopeId = result["scope_id"] as? String ?? ""
              invitation = result["invitation"] as? String ?? ""
            }
          }.disabled(
            scopeId.isEmpty || deviceCode.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
              || publishing)
          if !invitation.isEmpty {
            ShareLink(item: invitation) {
              Label("Share private invitation", systemImage: "square.and.arrow.up")
            }
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
      Button("Load saved captions") {
        store.perform {
          let result = try await store.command([
            "action": "asset_history", "room_id": roomId, "asset_id": assetId,
          ])
          history = result["history"] as? [[String: Any]] ?? []
        }
      }
      ForEach(history.indices, id: \.self) { index in
        let revision = history[index]
        let caption = revision["caption"] as? String ?? ""
        let author = revision["author"] as? String ?? ""
        let attribution = author == store.snapshot.deviceId ? "You" : "Member \(author.prefix(8))"
        let clock = (revision["clock"] as? NSNumber)?.stringValue ?? ""
        let isCurrent = store.room?.assets.first(where: { $0.id == assetId })?.caption == caption
        HStack(alignment: .top, spacing: 12) {
          VStack(alignment: .leading, spacing: 4) {
            Text(caption.isEmpty ? "(No caption)" : caption)
            Text("\(attribution) · revision \(clock)")
              .font(.caption)
              .foregroundStyle(.secondary)
          }
          Spacer(minLength: 8)
          if store.room?.canContribute == true, !isCurrent {
            Button("Use this caption") {
              store.perform {
                _ = try await store.command([
                  "action": "patch_asset", "room_id": roomId, "asset_id": assetId,
                  "fields": ["caption": caption],
                ])
              }
            }
            .buttonStyle(.borderless)
          }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 10))
      }
    }
  }
}

struct MomentEditorView: View {
  @EnvironmentObject var store: LibraryStore
  @Environment(\.dismiss) private var dismiss
  private let suggestion: MomentSuggestion?
  @State private var title: String
  @State private var featured = false
  @State private var selection: Set<String>

  init(suggestion: MomentSuggestion? = nil) {
    self.suggestion = suggestion
    _title = State(initialValue: suggestion?.proposedTitle ?? "")
    _selection = State(initialValue: Set(suggestion?.assetIds ?? []))
  }
  var body: some View {
    NavigationStack {
      Form {
        if suggestion != nil {
          Section("Suggested from capture time") {
            Text(
              "Review the selected photos and videos, adjust the title or selection, then save to add this Moment."
            )
            .font(.callout)
            .foregroundStyle(.secondary)
          }
        }
        TextField("Moment title", text: $title)
        Toggle("Feature on Home", isOn: $featured)
        Section("Memories") {
          ForEach(store.room?.visibleAssets ?? []) { asset in
            Toggle(
              asset.displayTitle,
              isOn: Binding(
                get: { selection.contains(asset.id) },
                set: { if $0 { selection.insert(asset.id) } else { selection.remove(asset.id) } }))
          }
        }
      }.navigationTitle(suggestion == nil ? "Create Moment" : "Review Moment")
        .toolbar {
          ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
          ToolbarItem(placement: .confirmationAction) {
            Button("Save") {
              if let room = store.room {
                store.perform {
                  _ = try await store.command([
                    "action": "save_moment", "room_id": room.id, "title": title,
                    "featured": featured, "asset_ids": Array(selection),
                  ])
                  dismiss()
                }
              }
            }.disabled(
              title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || selection.isEmpty)
          }
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
  @FocusState private var nameFocused: Bool
  @State private var creatingPerson = false
  @State private var reviewingFaces = false
  @State private var mergeSourceID: String?
  @State private var mergeTargetID: String?
  @State private var confirmingPersonMerge = false
  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 26) {
        MemoryPageHeading(
          room: store.room?.name, title: "The people in your memories.",
          detail:
            "Name a familiar face and find your memories together. Naming a Person never invites them to your Room."
        )
        if let room = store.room {
          if room.canContribute {
            DisclosureGroup("Add a Person", isExpanded: $creatingPerson) {
              HStack(spacing: 12) {
                TextField("Person’s name", text: $name).textFieldStyle(.roundedBorder).focused(
                  $nameFocused)
                Button("Add", systemImage: "plus") {
                  nameFocused = false
                  store.perform {
                    _ = try await store.command([
                      "action": "save_person", "room_id": room.id, "name": name,
                    ])
                    name = ""
                    creatingPerson = false
                  }
                }.buttonStyle(CozyPrimaryButtonStyle()).disabled(
                  name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
              }
            }.padding(18).cozyPanel().onAppear { creatingPerson = room.people.isEmpty }
            Button {
              reviewingFaces = true
            } label: {
              Label("Review faces in saved photos", systemImage: "person.crop.rectangle.stack")
            }
            .buttonStyle(.bordered)
            .tint(CozyTheme.accentInk)
            .accessibilityHint(
              "Face crops are analyzed privately on this device. You choose each Person tag.")
          }
          if room.people.isEmpty {
            MemoryEmptyState(
              icon: "person.crop.rectangle.stack", title: "Familiar faces, gathered here.",
              detail:
                "Add a Person, then confirm their tags on a memory. You can browse all your photos without face recognition."
            )
          } else {
            if room.canContribute && room.people.count > 1 {
              Text("Control-click or press and hold a Person to merge duplicate profiles.")
                .font(.caption)
                .foregroundStyle(CozyTheme.secondary)
            }
            LazyVGrid(
              columns: [GridItem(.adaptive(minimum: 220, maximum: 320), spacing: 18)],
              alignment: .leading, spacing: 18
            ) {
              ForEach(room.people) { person in
                let memories = room.visibleAssets.filter { $0.people.contains(person.id) }
                NavigationLink {
                  ScrollView {
                    VStack(alignment: .leading, spacing: 24) {
                      MemoryPageHeading(
                        room: room.name, title: person.name,
                        detail:
                          "\(memories.count) tagged \(memories.count == 1 ? "memory" : "memories")")
                      MediaGrid(assets: memories)
                    }.padding(24)
                  }.background(MemoryGalleryBackdrop()).navigationTitle(person.name)
                } label: {
                  HStack(spacing: 16) {
                    Text(
                      person.name.split(separator: " ").prefix(2).compactMap { $0.first }.map(
                        String.init
                      ).joined().uppercased()
                    )
                    .font(.title2.weight(.medium)).foregroundStyle(CozyTheme.accentInk)
                    .frame(width: 52, height: 52).background(
                      CozyTheme.glassTint, in: RoundedRectangle(cornerRadius: 16))
                    VStack(alignment: .leading, spacing: 6) {
                      Text(person.name).font(.headline).foregroundStyle(CozyTheme.ink).lineLimit(2)
                      Text("\(memories.count) \(memories.count == 1 ? "memory" : "memories")").font(
                        .caption.monospacedDigit()
                      ).foregroundStyle(CozyTheme.secondary)
                    }
                    Spacer(minLength: 0)
                    Image(systemName: "chevron.right").font(.caption).foregroundStyle(
                      CozyTheme.secondary)
                  }.padding(18).cozyPanel()
                }.buttonStyle(.plain).accessibilityLabel(
                  "\(person.name), \(memories.count) tagged \(memories.count == 1 ? "memory" : "memories")"
                )
                .contextMenu {
                  if room.canContribute {
                    Menu("Merge into…", systemImage: "person.2") {
                      ForEach(room.people.filter { $0.id != person.id }) { target in
                        Button(target.name, systemImage: "arrow.turn.down.right") {
                          mergeSourceID = person.id
                          mergeTargetID = target.id
                          confirmingPersonMerge = true
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }.padding(24).frame(maxWidth: 1100, alignment: .leading).frame(maxWidth: .infinity)
    }
    .background(MemoryGalleryBackdrop())
    .sheet(isPresented: $reviewingFaces) {
      if let room = store.room { FaceReviewSheet(roomId: room.id).environmentObject(store) }
    }
    .confirmationDialog(
      "Merge these Person profiles?",
      isPresented: $confirmingPersonMerge,
      titleVisibility: .visible
    ) {
      Button("Merge People", role: .destructive) {
        guard let room = store.room,
          let sourceID = mergeSourceID,
          let targetID = mergeTargetID
        else { return }
        store.perform {
          _ = try await store.command([
            "action": "merge_person",
            "room_id": room.id,
            "source_person_id": sourceID,
            "target_person_id": targetID,
          ])
          try await store.reload()
        }
      }
      Button("Cancel", role: .cancel) {}
    } message: {
      Text("Memories tagged with the duplicate will appear under the Person you keep. The original photos are unchanged.")
    }
  }
}
