#if DEBUG && os(macOS)
import XCTest
@testable import FamilyRoom
import FamilyCoreBindings

final class DevelopmentVaultTests: XCTestCase {
    @MainActor func testKeyStorageFailureDoesNotCreateAnUnrecoverableCatalog() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        let keys = root.appendingPathComponent("DevelopmentKeys")
        try FileManager.default.createDirectory(at: keys, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o755])
        let store = LibraryStore(directory: root.appendingPathComponent("Vault"))
        await store.open()
        XCTAssertNotNil(store.error)
        XCTAssertFalse(FileManager.default.fileExists(atPath: store.directory.appendingPathComponent("catalog.fr").path))
        XCTAssertThrowsError(try store.recoveryKeyForBackup())
    }
    func testOwnerOnlyKeyPersistsAndRefusesReplacementOrUnsafeFiles() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        let file = root.appendingPathComponent("Keys/recovery.key")
        let store = DevelopmentVaultKeyStore(file: file)
        let key = newRecoveryKey()
        XCTAssertNil(try store.load())
        try store.save(key)
        XCTAssertEqual(try store.load(), key)
        XCTAssertEqual(try FileManager.default.attributesOfItem(atPath: file.path)[.posixPermissions] as? Int, 0o600)
        XCTAssertEqual(try FileManager.default.attributesOfItem(atPath: file.deletingLastPathComponent().path)[.posixPermissions] as? Int, 0o700)
        try store.save(key)
        XCTAssertThrowsError(try store.save(newRecoveryKey()))
        XCTAssertEqual(try store.load(), key)
        try FileManager.default.setAttributes([.posixPermissions: 0o644], ofItemAtPath: file.path)
        XCTAssertThrowsError(try store.load())
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: file.path)
        let link = root.appendingPathComponent("linked.key")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: file)
        XCTAssertThrowsError(try DevelopmentVaultKeyStore(file: link).load())
        XCTAssertThrowsError(try DevelopmentVaultKeyStore(file: link).save(key))
        XCTAssertEqual(try store.load(), key)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: file.deletingLastPathComponent().path)
        XCTAssertThrowsError(try store.load())
        try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: file.deletingLastPathComponent().path)
        XCTAssertEqual(try store.load(), key)
    }

    @MainActor func testDevelopmentLibraryReopensWithoutKeychainAndBackupExcludesLocalKey() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        let directory = root.appendingPathComponent("Vault")
        var store: LibraryStore? = LibraryStore(directory: directory)
        await store?.open()
        XCTAssertNil(store?.error)
        let key = try XCTUnwrap(store).recoveryKeyForBackup()
        let room = try await XCTUnwrap(store).command(["action": "create_room", "name": "Persistent development library"])
        let backup = root.appendingPathComponent("encrypted.frbackup")
        _ = try await XCTUnwrap(store).command(["action": "backup", "path": backup.path])
        XCTAssertNil(try Data(contentsOf: backup).range(of: Data(key.utf8)))
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/unzip")
        process.arguments = ["-Z1", backup.path]
        let pipe = Pipe(); process.standardOutput = pipe
        try process.run()
        let entries = String(decoding: pipe.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
        process.waitUntilExit()
        XCTAssertEqual(process.terminationStatus, 0)
        XCTAssertTrue(entries.contains("catalog.fr"))
        XCTAssertFalse(entries.contains("recovery.key"))
        XCTAssertFalse(entries.contains("DevelopmentKeys"))
        store = nil
        let reopened = LibraryStore(directory: directory)
        await reopened.open()
        XCTAssertNil(reopened.error)
        XCTAssertEqual(reopened.snapshot.rooms.first?.id, room["id"] as? String)
        XCTAssertEqual(try reopened.recoveryKeyForBackup(), key)
        XCTAssertNotEqual(reopened.directory, FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("FamilyRoom/Vault"))
    }
}
#endif
