import Foundation

enum VaultKeyStorage: Sendable {
    case keychain
    #if DEBUG && os(macOS)
    case development(DevelopmentVaultKeyStore)
    #endif

    func load() throws -> String? {
        switch self {
        case .keychain: return try VaultKeychain.load()
        #if DEBUG && os(macOS)
        case .development(let store): return try store.load()
        #endif
        }
    }

    func save(_ key: String) throws {
        switch self {
        case .keychain: try VaultKeychain.save(key)
        #if DEBUG && os(macOS)
        case .development(let store): try store.save(key)
        #endif
        }
    }
}

#if DEBUG && os(macOS)
import Darwin

/// Local development only. Kept outside the encrypted vault and its backups.
/// Release builds do not include this store and continue to use Keychain.
struct DevelopmentVaultKeyStore: Sendable {
    let file: URL

    private func failure(_ message: String) -> NSError {
        NSError(domain: "FamilyRoom.DevelopmentKey", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }

    private func validate(_ key: String) throws {
        guard key.utf8.count == 64, key.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) }) else {
            throw failure("The development recovery key is invalid. The existing library has been preserved.")
        }
    }

    func load() throws -> String? {
        let descriptor = Darwin.open(file.path, O_RDONLY | O_NOFOLLOW | O_CLOEXEC)
        if descriptor == -1 && errno == ENOENT { return nil }
        guard descriptor >= 0 else { throw failure("Cannot read the development key file.") }
        let handle = FileHandle(fileDescriptor: descriptor, closeOnDealloc: true)
        defer { try? handle.close() }
        var info = stat()
        var parentInfo = stat()
        guard fstat(descriptor, &info) == 0,
              (info.st_mode & S_IFMT) == S_IFREG,
              info.st_uid == geteuid(), (info.st_mode & 0o077) == 0, info.st_size == 64,
              lstat(file.deletingLastPathComponent().path, &parentInfo) == 0,
              (parentInfo.st_mode & S_IFMT) == S_IFDIR,
              parentInfo.st_uid == geteuid(), (parentInfo.st_mode & 0o077) == 0 else {
            throw failure("The development key must be an owner-only regular file. The existing library has been preserved.")
        }
        let data = try handle.read(upToCount: 65) ?? Data()
        guard let key = String(data: data, encoding: .utf8) else { throw failure("Cannot decode the development key.") }
        try validate(key)
        return key
    }

    func save(_ key: String) throws {
        try validate(key)
        if let existing = try load() {
            guard existing == key else { throw failure("The development key already belongs to another library. It was not overwritten.") }
            return
        }
        let parent = file.deletingLastPathComponent()
        try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
        let properties = try parent.resourceValues(forKeys: [.isSymbolicLinkKey, .isDirectoryKey])
        guard properties.isDirectory == true, properties.isSymbolicLink != true else { throw failure("The development key directory must be a local directory.") }
        var directoryInfo = stat()
        guard lstat(parent.path, &directoryInfo) == 0, directoryInfo.st_uid == geteuid(), (directoryInfo.st_mode & 0o077) == 0 else {
            throw failure("The development key directory must be accessible only by its owner.")
        }
        let descriptor = Darwin.open(file.path, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0o600)
        guard descriptor >= 0 else { throw failure("Cannot create the development key file. No existing key was replaced.") }
        let handle = FileHandle(fileDescriptor: descriptor, closeOnDealloc: true)
        do {
            try handle.write(contentsOf: Data(key.utf8))
            try handle.synchronize()
            try handle.close()
        } catch {
            try? handle.close()
            try? FileManager.default.removeItem(at: file)
            throw error
        }
    }
}
#endif
