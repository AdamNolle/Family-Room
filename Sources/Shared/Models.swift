import Foundation
import SwiftData
import CoreGraphics

// MARK: - FamilyRoom

@Model
final class FamilyRoomModel {
    var name: String?
    var createdAt: Date?
    @Relationship(deleteRule: .cascade, inverse: \EventModel.room)
    var events: [EventModel]?

    init(name: String? = "Our Family Room", createdAt: Date? = Date()) {
        self.name = name
        self.createdAt = createdAt
    }
}

// MARK: - Event

@Model
final class EventModel {
    var name: String?
    var date: Date?
    var category: String?
    var thumbnailSymbol: String?
    var room: FamilyRoomModel?
    @Relationship(deleteRule: .cascade, inverse: \MediaItemModel.event)
    var mediaItems: [MediaItemModel]?
    @Relationship(deleteRule: .cascade, inverse: \Story.sourceEvent)
    var stories: [Story]?

    init(
        name: String? = nil,
        date: Date? = Date(),
        category: String? = "General",
        thumbnailSymbol: String? = "photo"
    ) {
        self.name = name
        self.date = date
        self.category = category
        self.thumbnailSymbol = thumbnailSymbol
    }

    var displayDate: String {
        guard let date else { return "" }
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        return formatter.string(from: date)
    }
}

// MARK: - MediaItem

@Model
final class MediaItemModel {
    var relativePath: String?
    var type: String? // "photo" or "video"
    var isFavorite: Bool?
    var createdAt: Date?
    var event: EventModel?
    @Relationship(deleteRule: .cascade, inverse: \FaceObservation.mediaItem)
    var faces: [FaceObservation]?

    init(
        relativePath: String? = nil,
        type: String? = "photo",
        isFavorite: Bool? = false,
        createdAt: Date? = Date()
    ) {
        self.relativePath = relativePath
        self.type = type
        self.isFavorite = isFavorite
        self.createdAt = createdAt
    }

    var resolvedURL: URL? {
        guard let path = relativePath else { return nil }
        return MediaStorage.directory.appendingPathComponent(path)
    }
}

// MARK: - Person

@Model
final class PersonModel {
    var name: String?
    var createdAt: Date?
    @Relationship(deleteRule: .nullify, inverse: \FaceObservation.person)
    var faces: [FaceObservation]?

    init(name: String? = "Unknown", createdAt: Date? = Date()) {
        self.name = name
        self.createdAt = createdAt
    }

    var displayName: String { name ?? "Unknown" }
    var faceCount: Int { faces?.count ?? 0 }
}

// MARK: - FaceObservation (Phase 3 schema, forward-declared)

@Model
final class FaceObservation {
    @Attribute(.externalStorage) var featurePrintData: Data?
    var boundingBoxData: Data?
    var capturedAt: Date?
    var person: PersonModel?
    var mediaItem: MediaItemModel?

    init(
        featurePrintData: Data? = nil,
        boundingBoxData: Data? = nil,
        capturedAt: Date? = Date()
    ) {
        self.featurePrintData = featurePrintData
        self.boundingBoxData = boundingBoxData
        self.capturedAt = capturedAt
    }

    var boundingBox: CGRect? {
        guard let boundingBoxData else { return nil }
        return try? JSONDecoder().decode(CGRect.self, from: boundingBoxData)
    }

    static func encode(boundingBox: CGRect) -> Data? {
        try? JSONEncoder().encode(boundingBox)
    }
}

// MARK: - Story (Phase 5 schema, forward-declared)

@Model
final class Story {
    var name: String?
    var createdAt: Date?
    var exportedRelativePath: String?
    @Attribute(.externalStorage) var thumbnailData: Data?
    var sourceEvent: EventModel?

    init(
        name: String? = "Untitled Story",
        createdAt: Date? = Date(),
        exportedRelativePath: String? = nil
    ) {
        self.name = name
        self.createdAt = createdAt
        self.exportedRelativePath = exportedRelativePath
    }

    var resolvedURL: URL? {
        guard let path = exportedRelativePath else { return nil }
        return MediaStorage.directory.appendingPathComponent(path)
    }
}

// MARK: - Shared storage location for media + stories

enum MediaStorage {
    static var directory: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.temporaryDirectory
        let dir = base.appendingPathComponent("FamilyRoomMedia", isDirectory: true)
        if !FileManager.default.fileExists(atPath: dir.path) {
            try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        }
        return dir
    }
}
