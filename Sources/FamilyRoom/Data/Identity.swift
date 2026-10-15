import Foundation
import Combine

enum SettingsKeys {
    static let reduceSunlight = "settings.reduceSunlight"
    static let sound = "settings.sound"
    static let displayName = "identity.displayName"
    static let authorID = "identity.authorID"
}

/// Who is using this device. Stored in iCloud key-value storage so every device signed in to the same
/// Apple Account shares one author identity; falls back to UserDefaults when iCloud KVS is unavailable.
@MainActor
final class Identity: ObservableObject {
    static let shared = Identity()
    private let kvs = NSUbiquitousKeyValueStore.default
    private let defaults = UserDefaults.standard
    private var observers: [Any] = []

    @Published private(set) var authorID: String
    @Published var displayName: String {
        didSet {
            kvs.set(displayName, forKey: SettingsKeys.displayName)
            defaults.set(displayName, forKey: SettingsKeys.displayName)
            kvs.synchronize()
        }
    }

    private init() {
        let storedID = kvs.string(forKey: SettingsKeys.authorID) ?? defaults.string(forKey: SettingsKeys.authorID)
        let id = storedID ?? UUID().uuidString
        authorID = id
        displayName = kvs.string(forKey: SettingsKeys.displayName) ?? defaults.string(forKey: SettingsKeys.displayName) ?? ""
        if storedID == nil {
            kvs.set(id, forKey: SettingsKeys.authorID)
            defaults.set(id, forKey: SettingsKeys.authorID)
        }
        kvs.synchronize()
        observers.append(NotificationCenter.default.addObserver(forName: NSUbiquitousKeyValueStore.didChangeExternallyNotification, object: kvs, queue: .main) { [weak self] _ in
            Task { @MainActor in self?.reload() }
        })
    }

    private func reload() {
        if let name = kvs.string(forKey: SettingsKeys.displayName), name != displayName { displayName = name }
        if let id = kvs.string(forKey: SettingsKeys.authorID), id != authorID { authorID = id }
    }

    var nameOrFallback: String { displayName.trimmingCharacters(in: .whitespaces).isEmpty ? "Someone" : displayName }
}
