import ImageIO
import SwiftUI

struct FaceReviewSheet: View {
    @EnvironmentObject private var store: LibraryStore
    @Environment(\.dismiss) private var dismiss
    let roomId: String

    @State private var scanning = false
    @State private var faces: [DetectedFace] = []
    @State private var progress = ""
    @State private var error = ""
    @State private var newPersonName = ""
    @State private var selectedPersonId = ""
    @State private var taggedFaceIds = Set<String>()
    @State private var taggingFaceIds = Set<String>()

    private var room: LibraryRoom? { store.snapshot.rooms.first { $0.id == roomId } }
    private var localPhotos: [LibraryAsset] {
        room?.visibleAssets.filter { $0.kind == "photo" && $0.local } ?? []
    }
    private var allPhotoCount: Int {
        room?.visibleAssets.filter { $0.kind == "photo" }.count ?? 0
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("A careful second look")
                            .font(RoomArt.display(32))
                            .foregroundStyle(CozyTheme.ink)
                        Text("Family Room finds face crops in photos already saved on this device. You decide who each face belongs to; names and tags are never guessed.")
                            .font(.callout)
                            .foregroundStyle(CozyTheme.secondary)
                    }
                    .padding(20)
                    .cozyPanel(glass: true)

                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            VStack(alignment: .leading, spacing: 4) {
                                Text("On this device")
                                    .font(.headline)
                                    .foregroundStyle(CozyTheme.ink)
                                Text("\(localPhotos.count) of \(allPhotoCount) photos are saved here")
                                    .font(.caption)
                                    .foregroundStyle(CozyTheme.secondary)
                            }
                            Spacer()
                            Button {
                                error = ""
                                faces = []
                                scanning = true
                            } label: {
                                Label(scanning ? "Scanning…" : "Find faces", systemImage: "viewfinder")
                            }
                            .buttonStyle(CozyPrimaryButtonStyle())
                            .disabled(scanning || localPhotos.isEmpty)
                            .accessibilityHint("Analyzes saved photos on this device. It does not download media or identify anyone automatically.")
                        }
                        Text("Cloud-only originals are skipped to avoid downloading your library unexpectedly.")
                            .font(.caption)
                            .foregroundStyle(CozyTheme.secondary)
                        if scanning || !progress.isEmpty {
                            Text(progress)
                                .font(.caption.monospacedDigit())
                                .foregroundStyle(CozyTheme.secondary)
                        }
                        if !error.isEmpty {
                            Text(error)
                                .font(.callout)
                                .foregroundStyle(CozyTheme.accentInk)
                        }
                    }
                    .padding(18)
                    .cozyPanel()

                    if !faces.isEmpty, let room {
                        personControls(room)
                        LazyVStack(spacing: 12) {
                            ForEach(faces.filter { !taggedFaceIds.contains($0.id) }) { face in
                                faceCard(face, room: room)
                            }
                        }
                    } else if !scanning, !progress.isEmpty {
                        MemoryEmptyState(
                            icon: "person.crop.rectangle.stack",
                            title: "No face crops to review",
                            detail: "Try photos with a clear, front-facing portrait. You can still add People and tag memories by hand."
                        )
                    }
                }
                .padding(22)
                .frame(maxWidth: 760, alignment: .leading)
                .frame(maxWidth: .infinity)
            }
            .background(MemoryGalleryBackdrop())
            .navigationTitle("Review faces")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .task(id: scanning) {
                guard scanning else { return }
                await scanLocalPhotos()
                scanning = false
            }
        }
        #if os(macOS)
        .frame(minWidth: 560, idealWidth: 720, minHeight: 580)
        #endif
    }

    @ViewBuilder
    private func personControls(_ room: LibraryRoom) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Choose a Person")
                .font(.headline)
                .foregroundStyle(CozyTheme.ink)
            if room.people.isEmpty {
                Text("Create a Person first. The new Person stays separate from Room membership.")
                    .font(.callout)
                    .foregroundStyle(CozyTheme.secondary)
            } else {
                Picker("Person", selection: $selectedPersonId) {
                    Text("Choose a Person").tag("")
                    ForEach(room.people) { person in
                        Text(person.name).tag(person.id)
                    }
                }
                .pickerStyle(.menu)
            }
            HStack(spacing: 10) {
                TextField("New Person’s name", text: $newPersonName)
                    .textFieldStyle(.roundedBorder)
                    .accessibilityLabel("New Person name")
                Button("Create") { createPerson(roomId: room.id) }
                    .buttonStyle(.bordered)
                    .disabled(newPersonName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            Text("A tag adds the photo to that Person’s memories. Naming a Person never invites them to this Room.")
                .font(.caption)
                .foregroundStyle(CozyTheme.secondary)
        }
        .padding(18)
        .cozyPanel(glass: true)
    }

    private func faceCard(_ face: DetectedFace, room: LibraryRoom) -> some View {
        let asset = room.visibleAssets.first { $0.id == face.assetId }
        return HStack(spacing: 14) {
            FaceCropPreview(data: face.crop)
                .frame(width: 82, height: 96)
                .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).stroke(CozyTheme.line, lineWidth: 1))
                .accessibilityLabel("Detected face crop")
            VStack(alignment: .leading, spacing: 5) {
                Text(asset?.displayTitle ?? "Photo")
                    .font(.callout.weight(.medium))
                    .foregroundStyle(CozyTheme.ink)
                    .lineLimit(2)
                Text(asset?.date.formatted(date: .abbreviated, time: .omitted) ?? "")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(CozyTheme.secondary)
                Text("Face detection confidence \(Int(face.confidence * 100))%")
                    .font(.caption2)
                    .foregroundStyle(CozyTheme.secondary)
            }
            Spacer(minLength: 4)
            Button("Tag") { tag(face, asset: asset, room: room) }
                .buttonStyle(.borderedProminent)
                .tint(CozyTheme.accentInk)
                .disabled(selectedPersonId.isEmpty || asset == nil || taggingFaceIds.contains(face.id))
                .accessibilityLabel("Tag this photo as the selected Person")
        }
        .padding(12)
        .cozyPanel()
    }

    @MainActor
    private func scanLocalPhotos() async {
        let photos = localPhotos
        faces = []
        var results: [DetectedFace] = []
        var failures = 0
        for (index, asset) in photos.enumerated() {
            guard !Task.isCancelled else { break }
            progress = "Checking photo \(index + 1) of \(photos.count)…"
            do {
                let url = try await store.materialize(asset)
                results += try await FamilyIntelligenceService.shared.reviewFaces(in: url, assetId: asset.id)
            } catch is CancellationError {
                break
            } catch {
                failures += 1
            }
        }
        guard !Task.isCancelled else { return }
        faces = results
        progress = "Found \(results.count) face crop\(results.count == 1 ? "" : "s") in \(photos.count - failures) readable local photos. Review each crop before adding a tag."
        if failures > 0 { error = "\(failures) photo\(failures == 1 ? "" : "s") could not be analyzed." }
        if selectedPersonId.isEmpty { selectedPersonId = room?.people.first?.id ?? "" }
    }

    private func createPerson(roomId: String) {
        let trimmed = newPersonName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        Task { @MainActor in
            do {
                let result = try await store.command(["action": "save_person", "room_id": roomId, "name": trimmed])
                selectedPersonId = result["id"] as? String ?? ""
                newPersonName = ""
                try await store.reload()
            } catch {
                self.error = store.readable(error)
            }
        }
    }

    private func tag(_ face: DetectedFace, asset: LibraryAsset?, room: LibraryRoom) {
        guard let asset, !selectedPersonId.isEmpty else { return }
        taggingFaceIds.insert(face.id)
        Task { @MainActor in
            defer { taggingFaceIds.remove(face.id) }
            do {
                var people = asset.people
                if !people.contains(selectedPersonId) { people.append(selectedPersonId) }
                _ = try await store.command(["action": "patch_asset", "room_id": room.id,
                                             "asset_id": asset.id, "fields": ["people": people]])
                try await store.reload()
                taggedFaceIds.insert(face.id)
            } catch {
                self.error = store.readable(error)
            }
        }
    }
}

private struct FaceCropPreview: View {
    let data: Data

    var body: some View {
        Group {
            if let source = CGImageSourceCreateWithData(data as CFData, nil),
               let image = CGImageSourceCreateImageAtIndex(source, 0, nil) {
                Image(decorative: image, scale: 1)
                    .resizable()
                    .scaledToFill()
            } else {
                CozyTheme.photoBed.overlay(Image(systemName: "person.crop.rectangle").foregroundStyle(CozyTheme.secondary))
            }
        }
        .background(CozyTheme.photoBed)
        .clipped()
    }
}
