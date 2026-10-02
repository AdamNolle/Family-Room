import SwiftUI
import UniformTypeIdentifiers
#if os(macOS)
import AppKit
#endif

struct RoomSettingsView: View {
    @EnvironmentObject var store: LibraryStore
    @Environment(\.dismiss) private var dismiss
    @State private var roomName = ""
    @State private var invitation = ""
    @State private var deviceCode = ""
    @State private var role = "contributor"
    @State private var generatedInvitation = ""
    @State private var pendingRemoval: LibraryMember?
    @State private var accessMessage = ""
    @State private var preservedURL: URL?
    @State private var preservedMessage = ""
    @State private var recoveryKey = ""
    @State private var sourceName = ""
    @State private var sourceKind = "local"
    @State private var endpoint = ""
    @State private var token = ""
    @State private var quotaMessage = ""
    @State private var automaticFolder = ""
    @State private var historical = false
    @State private var exclude = ""
    @State private var exportURL: URL?
    @State private var importer = false
    @State private var importKind = "room"
    @State private var importRoomId: String?
    @State private var recoveryAcknowledged = UserDefaults.standard.bool(forKey: "recoveryAcknowledged")
    var body: some View {
        NavigationStack {
            Form {
                Section("Your Rooms") {
                    ForEach(store.snapshot.rooms) { room in Button { store.selectRoom(room.id) } label: { HStack { Text(room.name); Spacer(); Text(room.role.capitalized).foregroundStyle(.secondary); if store.room?.id == room.id { Image(systemName: "checkmark") } } } }
                    HStack { TextField("New Room name", text: $roomName); Button("Create") { store.createRoom(roomName); roomName = "" }.disabled(roomName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty) }
                    TextField("Invitation", text: $invitation)
                    Button("Accept invitation") { store.perform { _ = try await store.command(["action": "join", "invitation": invitation]); invitation = "" } }.disabled(invitation.isEmpty)
                }
                if let room = store.room {
                    Section("Members of \(room.name)") {
                        Text("Device code").font(.caption).foregroundStyle(.secondary)
                        Text(store.snapshot.deviceId).font(.caption.monospaced()).textSelection(.enabled)
                        ForEach(room.members) { member in
                            HStack {
                                Text(member.deviceId == store.snapshot.deviceId ? "This device" : "Device \(member.deviceId.prefix(8))")
                                Spacer()
                                Text(member.role.capitalized).foregroundStyle(.secondary)
                                if room.role == "owner", member.deviceId != room.ownerId {
                                    Menu("Manage device") {
                                        if room.albumOnly != true {
                                            ForEach(["viewer", "contributor", "admin"], id: \.self) { newRole in
                                                Button("Make \(newRole)") { changeAccess(roomId: room.id, deviceId: member.deviceId, role: newRole) }
                                                    .disabled(member.role == newRole)
                                            }
                                        }
                                        Button("Remove device", role: .destructive) { pendingRemoval = member }
                                    }
                                }
                            }
                        }
                        if room.role == "removed" {
                            Label("This device has been removed from the Room.", systemImage: "lock.slash")
                            Text("Previously downloaded memories remain available. New contributions and shared changes are blocked.").font(.caption).foregroundStyle(.secondary)
                        }
                        if (room.rejectedEdits ?? 0) > 0 {
                            DisclosureGroup("Preserved offline work (\(room.rejectedEdits ?? 0) changes)") {
                                Text("These changes remain on this device after an access change. They have not been shared.").font(.callout)
                                Text("Export a ZIP of the changes, private film drafts, and available originals. The ZIP is unencrypted; keep it private. Missing originals are listed in its manifest.").font(.caption).foregroundStyle(.secondary)
                                Button("Export preserved work") { exportPreserved(roomId: room.id) }
                                if !preservedMessage.isEmpty { Text(preservedMessage).font(.caption) }
                                if let preservedURL {
                                    ShareLink(item: preservedURL) { Label("Save preserved work", systemImage: "square.and.arrow.up") }
                                }
                            }
                        }
                        if !accessMessage.isEmpty { Text(accessMessage).font(.callout) }
                        if room.role == "owner" {
                            TextField("Member’s device code", text: $deviceCode)
                            if room.albumOnly == true {
                                Text("Album-only members have viewer access.").font(.caption)
                            } else {
                                Picker("Role", selection: $role) { Text("Contributor").tag("contributor"); Text("Viewer").tag("viewer"); Text("Admin").tag("admin") }
                            }
                            Button("Create device-bound invitation") { store.perform { let result = try await store.command(["action": "invite", "room_id": room.id, "device_id": deviceCode, "role": room.albumOnly == true ? "viewer" : role]); generatedInvitation = result["invitation"] as? String ?? "" } }.disabled(deviceCode.isEmpty)
                            if !generatedInvitation.isEmpty { Text("Send this invitation privately to the intended member.").font(.caption); Text(generatedInvitation).font(.caption.monospaced()).textSelection(.enabled); ShareLink(item: generatedInvitation) { Label("Share invitation", systemImage: "square.and.arrow.up") } }
                        }
                    }
                    Section("Storage sources") {
                        Text("The device vault keeps encrypted originals. Add an independent source for another verified copy.").font(.callout).foregroundStyle(.secondary)
                        ForEach(store.snapshot.sources.filter { $0.roomId == room.id }) { source in
                            VStack(alignment: .leading, spacing: 8) {
                                HStack { Text(source.name).font(.headline); Spacer(); Toggle("Enabled", isOn: Binding(get: { !source.paused }, set: { enabled in store.perform { _ = try await store.command(["action": "pause_source", "id": source.id, "paused": !enabled]) } })).labelsHidden() }
                            Text(source.kind.replacingOccurrences(of: "_", with: " ").capitalized).font(.caption).foregroundStyle(.secondary)
                            if source.preferred { Label("Preferred upload destination", systemImage: "checkmark.circle") }
                            else { Button("Use for new uploads") { store.perform { _ = try await store.command(["action": "prefer_source", "id": source.id]); try await store.reload(); await store.replicatePending() } } }
                                Button("Check remaining space") { store.perform { let result = try await store.command(["action": "quota", "id": source.id]); if let remaining = result["remaining"] as? UInt64 { quotaMessage = "\(source.name): \(ByteCountFormatter.string(fromByteCount: Int64(clamping: remaining), countStyle: .file)) available" } else { quotaMessage = "\(source.name) does not report a quota." } } }
                            }
                        }
                        if !quotaMessage.isEmpty { Text(quotaMessage).font(.callout) }
                        DisclosureGroup("Connect storage") {
                            TextField("Source name", text: $sourceName)
                            Picker("Provider", selection: $sourceKind) {
                                Text("Folder / mounted home server").tag("local")
                                Text("WebDAV / Nextcloud").tag("webdav")
                                Text("Google Drive").tag("google_drive")
                                Text("OneDrive").tag("onedrive"); Text("S3-compatible").tag("s3")
                                Text("Family Room gateway").tag("gateway")
                            }
                            TextField(sourceKind == "google_drive" ? "App-owned folder ID" : sourceKind == "local" ? "Folder path" : "HTTPS endpoint", text: $endpoint)
                            #if os(macOS)
                            if sourceKind == "local" { Button("Choose folder…") { if let url = selectDirectory() { endpoint = url.path } } }
                            #endif
                            if sourceKind != "local" && sourceKind != "gateway" { SecureField(sourceKind == "s3" ? "S3 credential JSON" : "Provider access token", text: $token); Text(sourceKind == "s3" ? "Use a bucket/prefix URL and credential JSON with access_key_id, secret_access_key and region; session_token and virtual_hosted are optional. S3 quota is unknown." : "Use an authorized app folder and a provider-issued token. Interactive account sign-in is still being integrated.").font(.caption).foregroundStyle(.secondary) }
                            Button("Connect source") { store.perform { _ = try await store.command(["action": "add_source", "room_id": room.id, "name": sourceName, "kind": sourceKind, "endpoint": endpoint, "token": token]); token = ""; sourceName = ""; try await store.reload(); await store.replicatePending() } }.disabled(sourceName.isEmpty || (sourceKind != "onedrive" && endpoint.isEmpty))
                        }
                        Button("Sync and verify copies now") { Task { await store.replicatePending() } }
                    }
                    Section("Automatic uploads to \(room.name)") {
                        Toggle("Also import existing Photos items", isOn: $historical)
                        TextField("Excluded file types, e.g. jpg, mp4", text: $exclude)
                        Button("Automatically share new Photos items here") { store.perform { try await PhotoLibraryMonitor.enable(store: store, roomId: room.id, includeExisting: historical, excludedExtensions: exclude.split(separator: ",").map { String($0) }) } }.disabled(!room.canContribute)
                        ForEach(store.snapshot.watches.filter { $0.roomId == room.id }) { rule in UploadRuleControls(rule: rule) }
                        #if os(macOS)
                        TextField("Watch folder", text: $automaticFolder)
                        Button("Choose source folder…") { if let url = selectDirectory() { automaticFolder = url.path } }
                        Toggle("Also import existing media", isOn: $historical)
                        Button("Start automatic uploads") { store.perform { _ = try await store.command(["action": "watch_folder", "room_id": room.id, "path": automaticFolder, "include_existing": historical, "excluded_extensions": exclude.split(separator: ",").map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }]); _ = try await store.command(["action": "scan_watches"]) } }.disabled(automaticFolder.isEmpty || !room.canContribute)
                        #endif
                        Text("New eligible media is shared with this Room while Family Room is running. Pause the rule at any time.").font(.caption).foregroundStyle(.secondary)
                        Text("File-type exclusions skip the whole Photos item, including any Live Photo components.").font(.caption).foregroundStyle(.secondary)
                    }
                    Section("Sharing and preservation") {
                        Button("Export originals and organization") {
                            let destination = store.cache.appendingPathComponent("Library-\(UUID().uuidString)", isDirectory: true)
                            store.perform {
                                for asset in room.visibleAssets where !asset.local { _ = try await store.command(["action": "fetch", "room_id": room.id, "asset_id": asset.id]) }
                                _ = try await store.command(["action": "export_library", "room_id": room.id, "destination": destination.path])
                                exportURL = destination
                            }
                        }
                        Button("Export encrypted Room package") { makeExport(roomId: room.id) }
                        Button("Import encrypted Room package") { importKind = "room"; importRoomId = room.id; importer = true }
                        Text("A Room package contains encrypted media and edits. Recipients first accept their device-bound invitation, then import the package or connect the same storage source.").font(.caption).foregroundStyle(.secondary)
                        Button("Create encrypted recovery backup") { makeExport(roomId: nil) }
                        if let exportURL { ShareLink(item: exportURL) { Label("Save or share \(exportURL.lastPathComponent)", systemImage: "square.and.arrow.up") } }
                        Button("Show recovery key") { store.perform { recoveryKey = try store.recoveryKeyForBackup() } }
                        if !recoveryKey.isEmpty { Text(recoveryKey).font(.caption.monospaced()).textSelection(.enabled); Text("Keep the key separately from the backup. It unlocks every Room in this device’s backup.").font(.caption).foregroundStyle(.secondary) }
                        Toggle("I have saved my recovery key separately", isOn: $recoveryAcknowledged).onChange(of: recoveryAcknowledged) { _, value in UserDefaults.standard.set(value, forKey: "recoveryAcknowledged") }
                        DisclosureGroup("Recently Deleted") { ForEach(room.assets.filter(\.deleted)) { asset in HStack { Text(asset.filename); Spacer(); Button("Restore") { store.patch(asset, fields: ["deleted": false]) } } } }
                    }
                    Section("Transfers") {
                        ForEach(store.snapshot.transfers.filter { $0.roomId == room.id }) { transfer in
                            VStack(alignment: .leading, spacing: 8) {
                                Text(room.assets.first { $0.id == transfer.assetId }?.filename ?? "Transfer").font(.callout)
                                ProgressView(value: transfer.progress)
                                Text(transfer.error ?? transfer.state.capitalized).font(.caption).foregroundStyle(.secondary)
                                if transfer.state != "complete" {
                                    HStack {
                                        Button("Retry") { store.perform { _ = try await store.command(["action": "retry_transfer", "id": transfer.id]) } }
                                        Button("Cancel") { store.perform { _ = try await store.command(["action": "cancel_transfer", "id": transfer.id]) } }
                                    }
                                }
                            }
                        }
                    }
                }
                Section("Atmosphere") {
                    Toggle("Reduce atmosphere and motion", isOn: $store.reduceAtmosphere).onChange(of: store.reduceAtmosphere) { _, value in UserDefaults.standard.set(value, forKey: "reduceAtmosphere") }
                    Toggle("Interface sounds", isOn: $store.soundEnabled).onChange(of: store.soundEnabled) { _, value in UserDefaults.standard.set(value, forKey: "soundEnabled") }
                }
            }.formStyle(.grouped).navigationTitle("Room and storage")
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { recoveryKey = ""; dismiss() } } }
        }
        #if os(macOS)
        .frame(minWidth: 650, minHeight: 700)
        #endif
        .confirmationDialog("Remove this device?", isPresented: Binding(
            get: { pendingRemoval != nil }, set: { if !$0 { pendingRemoval = nil } })) {
            if let member = pendingRemoval {
                Button("Remove device", role: .destructive) {
                    changeAccess(roomId: member.roomId, deviceId: member.deviceId, role: nil)
                    pendingRemoval = nil
                }
            }
        } message: {
            Text("Removal changes the Room key for future contributions. Connected storage applies the change after syncing; offline devices update when they reconnect. Previously downloaded memories cannot be removed.")
        }
        .fileImporter(isPresented: $importer, allowedContentTypes: [.data]) { result in
            if case .success(let url) = result, let roomId = importRoomId { store.perform { let scoped = url.startAccessingSecurityScopedResource(); defer { if scoped { url.stopAccessingSecurityScopedResource() } }; _ = try await store.command(["action": "import_room", "room_id": roomId, "path": url.path]) } }
        }
        .onChange(of: store.room?.id) { _, _ in
            generatedInvitation = ""
            deviceCode = ""
            preservedURL = nil
            preservedMessage = ""
            accessMessage = ""
            exportURL = nil
        }
    }

    private func changeAccess(roomId: String, deviceId: String, role: String?) {
        store.perform {
            var command: [String: Any] = ["action": role == nil ? "revoke_device" : "set_role", "room_id": roomId, "device_id": deviceId]
            if let role { command["role"] = role }
            _ = try await store.command(command)
            try await store.reload()
            accessMessage = "Saved locally. Waiting to send the access update to storage."
            let sources = store.snapshot.sources.filter { $0.roomId == roomId && !$0.paused }
            for source in sources {
                _ = try await store.command(["action": "refresh_access", "id": source.id])
            }
            if !sources.isEmpty {
                accessMessage = "Access update sent to connected storage. Offline devices update when they reconnect."
            } else {
                accessMessage = "Saved locally. Connect storage or export an updated Room package to notify other devices."
            }
            try await store.reload()
        }
    }

    private func exportPreserved(roomId: String) {
        let path = store.cache.appendingPathComponent("Preserved-work-\(UUID().uuidString).zip")
        store.perform {
            let result = try await store.command(["action": "export_preserved", "room_id": roomId, "path": path.path])
            preservedURL = path
            let exported = (result["exported"] as? NSNumber)?.intValue ?? 0
            let missing = (result["missing"] as? NSNumber)?.intValue ?? 0
            preservedMessage = "\(exported) originals exported. \(missing) originals were unavailable on this device."
        }
    }

    private func makeExport(roomId: String?) {
        let path = store.cache.appendingPathComponent(roomId == nil ? "FamilyRoom.frbackup" : "Room.frroom")
        store.perform { var command: [String: Any] = ["action": roomId == nil ? "backup" : "export_room", "path": path.path]; if let roomId { command["room_id"] = roomId }; _ = try await store.command(command); exportURL = path }
    }
    #if os(macOS)
    private func selectDirectory() -> URL? { let panel = NSOpenPanel(); panel.canChooseDirectories = true; panel.canChooseFiles = false; panel.allowsMultipleSelection = false; return panel.runModal() == .OK ? panel.url : nil }
    #endif
}

private struct UploadRuleControls: View {
    @EnvironmentObject var store: LibraryStore
    let rule: UploadRule
    @State private var excluded = ""
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(rule.path == "photos://library" ? "Photos library" : rule.path).font(.callout.weight(.medium))
            Toggle("Upload new items", isOn: Binding(get: { !rule.paused }, set: { enabled in
                store.perform { _ = try await store.command(["action": "pause_watch", "id": rule.id, "paused": !enabled]) }
            }))
            TextField("Skip file types, e.g. jpg, mp4", text: $excluded)
            Button("Save exclusions") {
                store.perform {
                    _ = try await store.command(["action": "set_watch_exclusions", "id": rule.id,
                        "excluded_extensions": excluded.split(separator: ",").map { String($0) }])
                }
            }
        }
        .onAppear { excluded = rule.excludedExtensions.joined(separator: ", ") }
        .onChange(of: rule.excludedExtensions) { _, values in excluded = values.joined(separator: ", ") }
    }
}
