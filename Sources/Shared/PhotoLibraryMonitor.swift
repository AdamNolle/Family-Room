import Foundation
import Photos

@MainActor final class PhotoLibraryMonitor: NSObject, PHPhotoLibraryChangeObserver {
    weak var store: LibraryStore?
    private var scanning = false
    private var registered = false
    init(store: LibraryStore) { self.store = store; super.init(); refreshObservation() }
    deinit { if registered { PHPhotoLibrary.shared().unregisterChangeObserver(self) } }
    func refreshObservation() {
        let hasRule = store?.snapshot.watches.contains { rule in
            rule.path == "photos://library" && !rule.paused &&
            store?.snapshot.rooms.first(where: { $0.id == rule.roomId })?.canContribute == true
        } ?? false
        let status = PHPhotoLibrary.authorizationStatus(for: .readWrite)
        let shouldObserve = hasRule && (status == .authorized || status == .limited)
        if shouldObserve && !registered {
            PHPhotoLibrary.shared().register(self)
            registered = true
        } else if !shouldObserve && registered {
            PHPhotoLibrary.shared().unregisterChangeObserver(self)
            registered = false
        }
    }
    nonisolated func photoLibraryDidChange(_ changeInstance: PHChange) { Task { @MainActor [weak self] in await self?.scan() } }
    static func enable(store: LibraryStore, roomId: String, includeExisting: Bool, excludedExtensions: [String] = []) async throws {
        guard store.snapshot.rooms.first(where: { $0.id == roomId })?.canContribute == true else {
            throw FilmRenderError.invalid("You need contribution access to enable automatic uploads.")
        }
        let status = await PHPhotoLibrary.requestAuthorization(for: .readWrite)
        guard status == .authorized || status == .limited else { throw FilmRenderError.invalid("Allow access to Photos to enable automatic uploads.") }
        let fetched = PHAsset.fetchAssets(with: nil)
        var identifiers: [String] = []
        if !includeExisting { fetched.enumerateObjects { asset, _, _ in identifiers.append(asset.localIdentifier) } }
        _ = try await store.command(["action": "watch_photos", "room_id": roomId, "seen": identifiers, "excluded_extensions": excludedExtensions])
        try await store.reload()
    }
    func scan() async {
        refreshObservation()
        guard registered, !scanning, let store, let rule = store.snapshot.watches.first(where: { $0.path == "photos://library" && !$0.paused }) else { return }
        let status = PHPhotoLibrary.authorizationStatus(for: .readWrite)
        guard status == .authorized || status == .limited else { return }
        func canContinue() -> Bool {
            let currentStatus = PHPhotoLibrary.authorizationStatus(for: .readWrite)
            return !Task.isCancelled &&
                (currentStatus == .authorized || currentStatus == .limited) &&
                store.snapshot.watches.contains(where: { $0.id == rule.id && !$0.paused && $0.roomId == rule.roomId && $0.excludedExtensions == rule.excludedExtensions }) &&
                store.snapshot.rooms.first(where: { $0.id == rule.roomId })?.canContribute == true
        }
        scanning = true
        defer { scanning = false }
        let assets = PHAsset.fetchAssets(with: nil)
        let seen = Set(rule.seen)
        var pending: [PHAsset] = []
        assets.enumerateObjects { asset, _, _ in if !seen.contains(asset.localIdentifier) { pending.append(asset) } }
        for asset in pending {
            guard canContinue() else { return }
            do {
                var paths: [String] = []
                let resources = PHAssetResource.assetResources(for: asset).filter { resource in
                    [.photo, .video, .pairedVideo, .fullSizePhoto, .fullSizeVideo, .fullSizePairedVideo, .alternatePhoto].contains(resource.type)
                }
                guard !resources.isEmpty, !resources.contains(where: { resource in
                    rule.excludedExtensions.contains(where: { ext in
                        ext.caseInsensitiveCompare(URL(fileURLWithPath: resource.originalFilename).pathExtension) == .orderedSame
                    })
                }) else { continue }
                let directory = store.cache.appendingPathComponent(UUID().uuidString)
                try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
                defer { try? FileManager.default.removeItem(at: directory) }
                for resource in resources {
                    guard canContinue() else { return }
                    let componentDirectory = directory.appendingPathComponent(UUID().uuidString)
                    try FileManager.default.createDirectory(at: componentDirectory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
                    let url = componentDirectory.appendingPathComponent(URL(fileURLWithPath: resource.originalFilename).lastPathComponent)
                    let options = PHAssetResourceRequestOptions(); options.isNetworkAccessAllowed = true
                    try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
                        PHAssetResourceManager.default().writeData(for: resource, toFile: url, options: options) { error in if let error { continuation.resume(throwing: error) } else { continuation.resume() } }
                    }
                    guard canContinue() else { return }
                    paths.append(url.path)
                }
                guard canContinue() else { return }
                let result = try await store.command(["action": "import_photo", "id": rule.id, "room_id": rule.roomId,
                    "identifier": asset.localIdentifier, "paths": paths,
                    "captured_at": Int64((asset.creationDate ?? Date()).timeIntervalSince1970)])
                if result["imported"] as? Bool != true {
                    try await store.reload()
                    guard canContinue() else { return }
                }
            } catch { store.error = store.readable(error); break }
        }
        if !pending.isEmpty { try? await store.reload(); await store.replicatePending() }
    }
}
