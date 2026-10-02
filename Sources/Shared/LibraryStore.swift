import Foundation
import SwiftUI
import Security
import FamilyCoreBindings
import AudioToolbox

enum VaultKeychain {
    static let service = "com.familyroom.vault"
    static func load() throws -> String? {
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service, kSecAttrAccount as String: "recovery", kSecReturnData as String: true, kSecMatchLimit as String: kSecMatchLimitOne]
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess, let data = result as? Data, let key = String(data: data, encoding: .utf8) else {
            let detail = SecCopyErrorMessageString(status, nil) as String? ?? "Security error \(status)"
            throw NSError(domain: service, code: Int(status), userInfo: [NSLocalizedDescriptionKey: "Cannot access the vault key in Keychain: \(detail) (\(status))."])
        }
        return key
    }
    static func save(_ key: String) throws {
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service, kSecAttrAccount as String: "recovery"]
        let attributes: [String: Any] = [kSecValueData as String: Data(key.utf8), kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly]
        var status = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
        if status == errSecItemNotFound { status = SecItemAdd(query.merging(attributes) { _, new in new } as CFDictionary, nil) }
        guard status == errSecSuccess else {
            let detail = SecCopyErrorMessageString(status, nil) as String? ?? "Security error \(status)"
            throw NSError(domain: service, code: Int(status), userInfo: [NSLocalizedDescriptionKey: "Cannot save the vault key in Keychain: \(detail) (\(status))."])
        }
    }
}
actor CatalogWorker {
    let core: FamilyCore
    init(directory: URL, key: String) throws { core = try FamilyCore.open(directory: directory.path, recoveryKey: key) }
    func execute(_ request: String) throws -> String { try core.command(request: request) }
}
@MainActor final class LibraryStore: ObservableObject {
    @Published var snapshot = LibrarySnapshot.empty
    @Published var selectedRoomId: String? = UserDefaults.standard.string(forKey: "selectedRoom")
    @Published var setupRoomId: String?
    @Published var loading = true
    @Published var busy = false
    @Published var error: String?
    @Published var localURLs: [String: URL] = [:]
    @Published var search = ""
    @Published var selectedPeople: Set<String> = []
    @Published var matchAllPeople = false
    @Published var showFavorites = false
    @Published var mediaKind = "all"
    @Published var selectedYear = 0
    @Published var reduceAtmosphere = UserDefaults.standard.bool(forKey: "reduceAtmosphere")
    @Published var soundEnabled = UserDefaults.standard.bool(forKey: "soundEnabled")
    private var worker: CatalogWorker?
    private var unlockedRecoveryKey: String?
    private let keyStorage: VaultKeyStorage
    private var watching = false
    private var opening = false
    private var syncing = false
    private var photoMonitor: PhotoLibraryMonitor?
    let directory: URL
    let cache: URL
    init(directory: URL? = nil) {
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        #if DEBUG && os(macOS)
        self.directory = directory ?? support.appendingPathComponent("FamilyRoom/Development/Vault", isDirectory: true)
        keyStorage = .development(DevelopmentVaultKeyStore(file: self.directory.deletingLastPathComponent().appendingPathComponent("DevelopmentKeys/recovery.key")))
        #else
        self.directory = directory ?? support.appendingPathComponent("FamilyRoom/Vault", isDirectory: true)
        keyStorage = .keychain
        #endif
        cache = FileManager.default.temporaryDirectory.appendingPathComponent("FamilyRoom-\(UUID().uuidString)", isDirectory: true)
    }
    deinit { try? FileManager.default.removeItem(at: cache) }
    var room: LibraryRoom? { snapshot.rooms.first { $0.id == selectedRoomId } ?? snapshot.rooms.first }
    var assets: [LibraryAsset] {
        guard let room else { return [] }
        let term = search.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return room.visibleAssets.filter { asset in
            let names = room.people.filter { asset.people.contains($0.id) }.map(\.name).joined(separator: " ")
            let matchesText = term.isEmpty || "\(asset.filename) \(asset.caption) \(names)".lowercased().contains(term)
            let matchesPeople = selectedPeople.isEmpty || (matchAllPeople ? selectedPeople.isSubset(of: Set(asset.people)) : !selectedPeople.isDisjoint(with: asset.people))
            let matchesYear = selectedYear == 0 || Calendar.current.component(.year, from: asset.date) == selectedYear
            return matchesText && matchesPeople && matchesYear && (!showFavorites || asset.favorite) && (mediaKind == "all" || asset.kind == mediaKind)
        }
    }
    func open(recoveryKey: String? = nil) async {
        guard !opening else { return }
        if worker != nil { loading = false; return }
        opening = true
        defer { opening = false }
        loading = true
        do {
            let storage = keyStorage
            let existing = try await Task.detached { try recoveryKey == nil ? storage.load() : nil }.value
            let hasCatalog = FileManager.default.fileExists(atPath: directory.appendingPathComponent("catalog.fr").path)
            if existing == nil && recoveryKey == nil && hasCatalog {
                throw NSError(domain: "FamilyRoom", code: 1, userInfo: [NSLocalizedDescriptionKey: "Enter your recovery key to unlock this library."])
            }
            let key = recoveryKey ?? existing ?? newRecoveryKey()
            let persistBeforeOpening = !hasCatalog && existing != key
            // Never create an encrypted catalog before its new key is durable.
            if persistBeforeOpening { try await Task.detached { try storage.save(key) }.value }
            let root = directory
            let opened = try await Task.detached { try CatalogWorker(directory: root, key: key) }.value
            if !persistBeforeOpening && existing != key { try await Task.detached { try storage.save(key) }.value }
            worker = opened
            unlockedRecoveryKey = key
            try FileManager.default.createDirectory(at: cache, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
            try await reload()
            photoMonitor = PhotoLibraryMonitor(store: self)
            error = nil
        } catch { self.error = readable(error) }
        loading = false
    }
    func recoveryKeyForBackup() throws -> String {
        guard let key = unlockedRecoveryKey else {
            throw NSError(domain: "FamilyRoom", code: 1, userInfo: [NSLocalizedDescriptionKey: "Unlock the library before showing its recovery key."])
        }
        return key
    }
    func clearFilters() {
        selectedPeople = []
        matchAllPeople = false
        search = ""
        selectedYear = 0
        showFavorites = false
        mediaKind = "all"
    }
    func selectRoom(_ id: String) { selectedRoomId = id; clearFilters(); UserDefaults.standard.set(id, forKey: "selectedRoom") }
    @discardableResult func command(_ value: [String: Any]) async throws -> [String: Any] {
        guard let worker else { throw NSError(domain: "FamilyRoom", code: 1, userInfo: [NSLocalizedDescriptionKey: "The library is locked."]) }
        let data = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
        let response = try await worker.execute(String(decoding: data, as: UTF8.self))
        return (try JSONSerialization.jsonObject(with: Data(response.utf8))) as? [String: Any] ?? [:]
    }
    func reload() async throws {
        guard let worker else { return }
        let response = try await worker.execute("{\"action\":\"snapshot\"}")
        snapshot = try LibraryCoding.decoder.decode(LibrarySnapshot.self, from: Data(response.utf8))
        photoMonitor?.refreshObservation()
        if room?.id != selectedRoomId, let id = room?.id { selectRoom(id) }
    }
    func perform(_ body: @escaping @MainActor () async throws -> Void) {
        Task { busy = true; defer { busy = false }; do { try await body(); try await reload(); if soundEnabled { AudioServicesPlaySystemSound(1104) } } catch { self.error = readable(error) } }
    }
    func createRoom(_ name: String) {
        perform {
            let result = try await self.command(["action": "create_room", "name": name])
            if let id = result["id"] as? String {
                self.setupRoomId = id
                self.selectRoom(id)
            }
        }
    }
    func importURLs(_ urls: [URL], into roomId: String? = nil) async throws {
        guard let destination = roomId ?? room?.id else { return }
        for url in urls {
            let scoped = url.startAccessingSecurityScopedResource(); defer { if scoped { url.stopAccessingSecurityScopedResource() } }
            _ = try await command(["action": "import", "room_id": destination, "path": url.path])
        }
        try await reload()
        await replicatePending()
    }
    func replicatePending() async {
        guard !syncing else { return }
        syncing = true
        defer { syncing = false }
        for source in snapshot.sources where !source.paused {
            guard let r = snapshot.rooms.first(where: { $0.id == source.roomId }) else { continue }
            for asset in r.assets where r.canContribute && source.preferred && asset.local && !asset.deleted {
                let previous = snapshot.transfers.first { $0.sourceId == source.id && $0.assetId == asset.id }
                if previous?.state == "complete" || previous?.state == "cancelled" { continue }
                do {
                    var transferring = true
                    while transferring && !Task.isCancelled {
                        let result = try await command(["action": "replicate_step", "source_id": source.id, "asset_id": asset.id])
                        try await reload()
                        let current = snapshot.transfers.first { $0.sourceId == source.id && $0.assetId == asset.id }
                        transferring = result["state"] as? String == "queued" && current?.state != "cancelled"
                    }
                } catch { self.error = readable(error); break }
            }
            do { _ = try await command(["action": "sync_source", "id": source.id]) } catch { self.error = readable(error) }
        }
        try? await reload()
    }
    func materialize(_ asset: LibraryAsset) async throws -> URL {
        if let url = localURLs[asset.id], FileManager.default.fileExists(atPath: url.path) { return url }
        if !asset.local { _ = try await command(["action": "fetch", "room_id": asset.roomId, "asset_id": asset.id]) }
        let ext = URL(fileURLWithPath: asset.filename).pathExtension
        let destination = cache.appendingPathComponent(asset.id).appendingPathExtension(ext)
        _ = try await command(["action": "materialize", "room_id": asset.roomId, "asset_id": asset.id, "destination": destination.path])
        localURLs[asset.id] = destination
        return destination
    }
    func patch(_ asset: LibraryAsset, fields: [String: Any]) {
        perform { _ = try await self.command(["action": "patch_asset", "room_id": asset.roomId, "asset_id": asset.id, "fields": fields]) }
    }
    func save(_ project: FilmProject, publish: Bool = false) async throws {
        guard project.supportsEditing else { throw NSError(domain: "FamilyRoom", code: 1, userInfo: [NSLocalizedDescriptionKey: "Update Family Room before editing this film’s project format."]) }
        let data = try LibraryCoding.encoder.encode(project)
        _ = try await command(["action": "save_story", "project": JSONSerialization.jsonObject(with: data), "publish": publish])
        try await reload()
    }
    func watchFolders() async {
        guard !watching else { return }; watching = true
        defer { watching = false }
        while !Task.isCancelled {
            do { let result = try await command(["action": "scan_watches"]); if (result["imported"] as? Int ?? 0) > 0 { try await reload(); await replicatePending() } }
            catch { self.error = readable(error) }
            await photoMonitor?.scan()
            await replicatePending()
            do { try await Task.sleep(nanoseconds: 15_000_000_000) } catch { return }
        }
    }
    func readable(_ error: Error) -> String {
        if let error = error as? CoreError {
            switch error {
            case .Invalid(let message), .Storage(let message): return message
            case .Authentication: return "The key is incorrect or encrypted data was modified."
            case .AccessDenied: return "Your Room role does not allow this action."
            }
        }
        return error.localizedDescription
    }
}
