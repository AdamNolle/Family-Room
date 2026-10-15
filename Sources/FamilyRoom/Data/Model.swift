import CoreData
import Foundation

// The Core Data model is declared in code so the package builds without an .xcdatamodeld.
// Every attribute is optional or defaulted, every relationship has an inverse, and nothing is
// uniquely constrained — the shape NSPersistentCloudKitContainer requires.

@objc(Room) public final class Room: NSManagedObject {
    @NSManaged public var id: UUID?
    @NSManaged public var name: String?
    @NSManaged public var createdAt: Date?
    @NSManaged public var moments: Set<Moment>?
    @NSManaged public var people: Set<Person>?
    @NSManaged public var stories: Set<Story>?
    @NSManaged public var media: Set<MediaItem>?

    var displayName: String { (name ?? "").isEmpty ? "Family Room" : name! }
    var momentList: [Moment] { (moments ?? []).sorted { ($0.date ?? .distantPast) < ($1.date ?? .distantPast) } }
    var peopleList: [Person] { (people ?? []).sorted { ($0.createdAt ?? .distantPast) < ($1.createdAt ?? .distantPast) } }
    var storyList: [Story] { (stories ?? []).sorted { ($0.createdAt ?? .distantPast) > ($1.createdAt ?? .distantPast) } }
    var mediaList: [MediaItem] { (media ?? []).sorted { ($0.capturedAt ?? .distantPast) < ($1.capturedAt ?? .distantPast) } }
}

@objc(Person) public final class Person: NSManagedObject {
    @NSManaged public var id: UUID?
    @NSManaged public var name: String?
    @NSManaged public var role: String?
    @NSManaged public var toneHex: String?
    @NSManaged public var accentHex: String?
    @NSManaged public var isFaceGroup: Bool
    @NSManaged public var createdAt: Date?
    @NSManaged public var room: Room?
    @NSManaged public var moments: Set<Moment>?
    @NSManaged public var faces: Set<FaceObservation>?

    var displayName: String { (name ?? "").isEmpty ? "Unnamed" : name! }
    var photoCount: Int {
        var items = Set<MediaItem>()
        for m in moments ?? [] { items.formUnion(m.media ?? []) }
        for f in faces ?? [] { if let item = f.media { items.insert(item) } }
        return items.count
    }
    var summary: PersonSummary {
        PersonSummary(id: id ?? UUID(), name: displayName, role: role ?? "", toneHex: toneHex ?? "B86348",
                      accentHex: accentHex ?? "F0BFA8", isFaceGroup: isFaceGroup, photoCount: photoCount)
    }
}

@objc(Moment) public final class Moment: NSManagedObject {
    @NSManaged public var id: UUID?
    @NSManaged public var title: String?
    @NSManaged public var date: Date?
    @NSManaged public var place: String?
    @NSManaged public var quote: String?
    @NSManaged public var isFeatured: Bool
    @NSManaged public var createdAt: Date?
    @NSManaged public var room: Room?
    @NSManaged public var media: Set<MediaItem>?
    @NSManaged public var people: Set<Person>?
    @NSManaged public var notes: Set<Note>?
    @NSManaged public var reactions: Set<Reaction>?
    @NSManaged public var stories: Set<Story>?

    var displayTitle: String { (title ?? "").isEmpty ? Catalog.defaultMomentTitle(date ?? Date()) : title! }
    var mediaList: [MediaItem] { (media ?? []).sorted { ($0.capturedAt ?? .distantPast) < ($1.capturedAt ?? .distantPast) } }
    var photoCount: Int { (media ?? []).filter { $0.kind != MediaItem.videoKind }.count }
    var videoCount: Int { (media ?? []).filter { $0.kind == MediaItem.videoKind }.count }
    var noteList: [Note] { (notes ?? []).sorted { ($0.createdAt ?? .distantPast) < ($1.createdAt ?? .distantPast) } }
    var peopleList: [Person] { (people ?? []).sorted { ($0.createdAt ?? .distantPast) < ($1.createdAt ?? .distantPast) } }
    var cover: MediaItem? { mediaList.first(where: \.isFavorite) ?? mediaList.first }
    var likeCount: Int { reactions?.count ?? 0 }
    func liked(by authorID: String) -> Bool { reactions?.contains { $0.authorID == authorID } ?? false }

    var summary: MomentSummary {
        MomentSummary(id: id ?? UUID(), title: displayTitle, date: date ?? Date(), place: place, quote: quote,
                      isFeatured: isFeatured, people: peopleList.map(\.summary),
                      photoCount: photoCount, videoCount: videoCount)
    }
}

@objc(MediaItem) public final class MediaItem: NSManagedObject {
    static let photoKind = "photo"
    static let videoKind = "video"

    @NSManaged public var id: UUID?
    @NSManaged public var kind: String?
    @NSManaged public var capturedAt: Date?
    @NSManaged public var addedAt: Date?
    @NSManaged public var addedByName: String?
    @NSManaged public var addedByID: String?
    @NSManaged public var isFavorite: Bool
    @NSManaged public var width: Double
    @NSManaged public var height: Double
    @NSManaged public var duration: Double
    @NSManaged public var fileExtension: String?
    @NSManaged public var photoIdentifier: String?
    @NSManaged public var faceScanned: Bool
    @NSManaged public var data: Data?
    @NSManaged public var thumbnail: Data?
    @NSManaged public var room: Room?
    @NSManaged public var moment: Moment?
    @NSManaged public var faces: Set<FaceObservation>?

    var isVideo: Bool { kind == MediaItem.videoKind }
}

@objc(FaceObservation) public final class FaceObservation: NSManagedObject {
    @NSManaged public var id: UUID?
    @NSManaged public var confidence: Double
    @NSManaged public var boxX: Double
    @NSManaged public var boxY: Double
    @NSManaged public var boxWidth: Double
    @NSManaged public var boxHeight: Double
    @NSManaged public var featurePrint: Data?
    @NSManaged public var crop: Data?
    @NSManaged public var media: MediaItem?
    @NSManaged public var person: Person?
}

@objc(Story) public final class Story: NSManagedObject {
    @NSManaged public var id: UUID?
    @NSManaged public var title: String?
    @NSManaged public var subtitle: String?
    @NSManaged public var createdAt: Date?
    @NSManaged public var duration: Double
    @NSManaged public var isWeeklyReel: Bool
    @NSManaged public var weekStart: Date?
    @NSManaged public var movie: Data?
    @NSManaged public var thumbnail: Data?
    @NSManaged public var room: Room?
    @NSManaged public var sourceMoment: Moment?

    var displayTitle: String { (title ?? "").isEmpty ? "Untitled story" : title! }
}

@objc(Note) public final class Note: NSManagedObject {
    @NSManaged public var id: UUID?
    @NSManaged public var text: String?
    @NSManaged public var createdAt: Date?
    @NSManaged public var authorName: String?
    @NSManaged public var authorID: String?
    @NSManaged public var moment: Moment?
}

@objc(Reaction) public final class Reaction: NSManagedObject {
    @NSManaged public var id: UUID?
    @NSManaged public var createdAt: Date?
    @NSManaged public var authorName: String?
    @NSManaged public var authorID: String?
    @NSManaged public var moment: Moment?
}

enum FamilyRoomModel {
    static let shared: NSManagedObjectModel = build()

    private static func attr(_ name: String, _ type: NSAttributeType, optional: Bool = true, defaultValue: Any? = nil, external: Bool = false) -> NSAttributeDescription {
        let a = NSAttributeDescription()
        a.name = name; a.attributeType = type; a.isOptional = optional
        a.defaultValue = defaultValue; a.allowsExternalBinaryDataStorage = external
        return a
    }

    private static func rel(_ name: String, to destination: String, toMany: Bool, delete: NSDeleteRule) -> NSRelationshipDescription {
        let r = NSRelationshipDescription()
        r.name = name; r.isOptional = true; r.deleteRule = delete
        r.minCount = 0; r.maxCount = toMany ? 0 : 1
        r.userInfo = ["destination": destination]
        return r
    }

    private static func build() -> NSManagedObjectModel {
        let model = NSManagedObjectModel()
        var entities: [String: NSEntityDescription] = [:]
        func entity(_ name: String, _ cls: AnyClass, _ props: [NSPropertyDescription]) {
            let e = NSEntityDescription()
            e.name = name; e.managedObjectClassName = NSStringFromClass(cls); e.properties = props
            entities[name] = e
        }
        entity("Room", Room.self, [
            attr("id", .UUIDAttributeType), attr("name", .stringAttributeType), attr("createdAt", .dateAttributeType),
            rel("moments", to: "Moment", toMany: true, delete: .cascadeDeleteRule),
            rel("people", to: "Person", toMany: true, delete: .cascadeDeleteRule),
            rel("stories", to: "Story", toMany: true, delete: .cascadeDeleteRule),
            rel("media", to: "MediaItem", toMany: true, delete: .cascadeDeleteRule)
        ])
        entity("Person", Person.self, [
            attr("id", .UUIDAttributeType), attr("name", .stringAttributeType), attr("role", .stringAttributeType),
            attr("toneHex", .stringAttributeType), attr("accentHex", .stringAttributeType),
            attr("isFaceGroup", .booleanAttributeType, optional: false, defaultValue: false), attr("createdAt", .dateAttributeType),
            rel("room", to: "Room", toMany: false, delete: .nullifyDeleteRule),
            rel("moments", to: "Moment", toMany: true, delete: .nullifyDeleteRule),
            rel("faces", to: "FaceObservation", toMany: true, delete: .nullifyDeleteRule)
        ])
        entity("Moment", Moment.self, [
            attr("id", .UUIDAttributeType), attr("title", .stringAttributeType), attr("date", .dateAttributeType),
            attr("place", .stringAttributeType), attr("quote", .stringAttributeType),
            attr("isFeatured", .booleanAttributeType, optional: false, defaultValue: false), attr("createdAt", .dateAttributeType),
            rel("room", to: "Room", toMany: false, delete: .nullifyDeleteRule),
            rel("media", to: "MediaItem", toMany: true, delete: .cascadeDeleteRule),
            rel("people", to: "Person", toMany: true, delete: .nullifyDeleteRule),
            rel("notes", to: "Note", toMany: true, delete: .cascadeDeleteRule),
            rel("reactions", to: "Reaction", toMany: true, delete: .cascadeDeleteRule),
            rel("stories", to: "Story", toMany: true, delete: .nullifyDeleteRule)
        ])
        entity("MediaItem", MediaItem.self, [
            attr("id", .UUIDAttributeType), attr("kind", .stringAttributeType), attr("capturedAt", .dateAttributeType),
            attr("addedAt", .dateAttributeType), attr("addedByName", .stringAttributeType), attr("addedByID", .stringAttributeType),
            attr("isFavorite", .booleanAttributeType, optional: false, defaultValue: false),
            attr("width", .doubleAttributeType, optional: false, defaultValue: 0), attr("height", .doubleAttributeType, optional: false, defaultValue: 0),
            attr("duration", .doubleAttributeType, optional: false, defaultValue: 0), attr("fileExtension", .stringAttributeType),
            attr("photoIdentifier", .stringAttributeType), attr("faceScanned", .booleanAttributeType, optional: false, defaultValue: false),
            attr("data", .binaryDataAttributeType, external: true), attr("thumbnail", .binaryDataAttributeType, external: true),
            rel("room", to: "Room", toMany: false, delete: .nullifyDeleteRule),
            rel("moment", to: "Moment", toMany: false, delete: .nullifyDeleteRule),
            rel("faces", to: "FaceObservation", toMany: true, delete: .cascadeDeleteRule)
        ])
        entity("FaceObservation", FaceObservation.self, [
            attr("id", .UUIDAttributeType), attr("confidence", .doubleAttributeType, optional: false, defaultValue: 0),
            attr("boxX", .doubleAttributeType, optional: false, defaultValue: 0), attr("boxY", .doubleAttributeType, optional: false, defaultValue: 0),
            attr("boxWidth", .doubleAttributeType, optional: false, defaultValue: 0), attr("boxHeight", .doubleAttributeType, optional: false, defaultValue: 0),
            attr("featurePrint", .binaryDataAttributeType), attr("crop", .binaryDataAttributeType, external: true),
            rel("media", to: "MediaItem", toMany: false, delete: .nullifyDeleteRule),
            rel("person", to: "Person", toMany: false, delete: .nullifyDeleteRule)
        ])
        entity("Story", Story.self, [
            attr("id", .UUIDAttributeType), attr("title", .stringAttributeType), attr("subtitle", .stringAttributeType),
            attr("createdAt", .dateAttributeType), attr("duration", .doubleAttributeType, optional: false, defaultValue: 0),
            attr("isWeeklyReel", .booleanAttributeType, optional: false, defaultValue: false), attr("weekStart", .dateAttributeType),
            attr("movie", .binaryDataAttributeType, external: true), attr("thumbnail", .binaryDataAttributeType, external: true),
            rel("room", to: "Room", toMany: false, delete: .nullifyDeleteRule),
            rel("sourceMoment", to: "Moment", toMany: false, delete: .nullifyDeleteRule)
        ])
        entity("Note", Note.self, [
            attr("id", .UUIDAttributeType), attr("text", .stringAttributeType), attr("createdAt", .dateAttributeType),
            attr("authorName", .stringAttributeType), attr("authorID", .stringAttributeType),
            rel("moment", to: "Moment", toMany: false, delete: .nullifyDeleteRule)
        ])
        entity("Reaction", Reaction.self, [
            attr("id", .UUIDAttributeType), attr("createdAt", .dateAttributeType),
            attr("authorName", .stringAttributeType), attr("authorID", .stringAttributeType),
            rel("moment", to: "Moment", toMany: false, delete: .nullifyDeleteRule)
        ])
        // Wire destinations and inverses.
        let inverses: [(String, String, String, String)] = [
            ("Room", "moments", "Moment", "room"), ("Room", "people", "Person", "room"),
            ("Room", "stories", "Story", "room"), ("Room", "media", "MediaItem", "room"),
            ("Moment", "media", "MediaItem", "moment"), ("Moment", "people", "Person", "moments"),
            ("Moment", "notes", "Note", "moment"), ("Moment", "reactions", "Reaction", "moment"),
            ("Moment", "stories", "Story", "sourceMoment"),
            ("MediaItem", "faces", "FaceObservation", "media"), ("Person", "faces", "FaceObservation", "person")
        ]
        for e in entities.values {
            for r in e.relationshipsByName.values {
                r.destinationEntity = entities[r.userInfo?["destination"] as? String ?? ""]
                r.userInfo = nil
            }
        }
        for (a, ar, b, br) in inverses {
            let left = entities[a]!.relationshipsByName[ar]!, right = entities[b]!.relationshipsByName[br]!
            left.inverseRelationship = right; right.inverseRelationship = left
        }
        model.entities = Array(entities.values)
        return model
    }
}

// SwiftUI identity: NSManagedObject's objectID is stable for saved objects, and `id` is our own UUID.
extension Room: Identifiable {}
extension Person: Identifiable {}
extension Moment: Identifiable {}
extension MediaItem: Identifiable {}
extension FaceObservation: Identifiable {}
extension Story: Identifiable {}
extension Note: Identifiable {}
extension Reaction: Identifiable {}
