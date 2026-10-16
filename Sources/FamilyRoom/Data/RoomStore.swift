import CoreData
import Foundation
import OSLog
import SwiftUI

/// Every mutation of the library funnels through here so saving and iCloud sync stay consistent.
@MainActor
final class RoomStore: ObservableObject {
    private static let log = Logger(subsystem: "com.familyroom.app", category: "RoomStore")

    let persistence: Persistence
    let identity: Identity
    let sharing: RoomSharing
    var context: NSManagedObjectContext { persistence.viewContext }

    @Published var currentRoom: Room?
    @Published var importProgress: Progress?
    @Published var faceScanProgress: Progress?
    @Published var lastError: String?
    @AppStorage("room.currentID") private var currentRoomID: String = ""

    init(persistence: Persistence = .shared, identity: Identity? = nil) {
        self.persistence = persistence
        self.identity = identity ?? .shared
        self.sharing = RoomSharing(persistence: persistence)
        restoreCurrentRoom()
        NotificationCenter.default.addObserver(forName: .NSPersistentStoreRemoteChange, object: persistence.container.persistentStoreCoordinator, queue: .main) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                if self.currentRoom == nil { self.restoreCurrentRoom() }
                self.objectWillChange.send()
            }
        }
    }

    // MARK: Inserting

    /// Creates an object in the same store as `anchor`. A Room someone joined lives in the shared store.
    /// Core Data would infer the store from the relationship at save time; assigning it up front keeps
    /// the placement explicit, as Apple's CloudKit sharing sample does. SharedRoomTests covers both cases.
    func insert<T: NSManagedObject>(_ type: T.Type, near anchor: NSManagedObject?) -> T {
        let object = T(context: context)
        if let store = anchor?.objectID.persistentStore ?? anchor.flatMap({ persistence.store(for: $0) }) {
            context.assign(object, to: store)
        }
        return object
    }

    // MARK: Rooms

    var rooms: [Room] {
        let request = NSFetchRequest<Room>(entityName: "Room")
        request.sortDescriptors = [NSSortDescriptor(key: "createdAt", ascending: true)]
        return (try? context.fetch(request)) ?? []
    }

    private func restoreCurrentRoom() {
        let all = rooms
        if let id = UUID(uuidString: currentRoomID), let hit = all.first(where: { $0.id == id }) { currentRoom = hit }
        else { currentRoom = all.first }
        if let currentRoom { currentRoomID = currentRoom.id?.uuidString ?? "" }
    }

    func select(_ room: Room) {
        currentRoom = room
        currentRoomID = room.id?.uuidString ?? ""
    }

    @discardableResult
    func createRoom(named name: String) -> Room {
        let room = Room(context: context)
        room.id = UUID(); room.name = name.trimmingCharacters(in: .whitespaces); room.createdAt = Date()
        save()
        select(room)
        return room
    }

    func rename(_ room: Room, to name: String) {
        room.name = name.trimmingCharacters(in: .whitespaces); save()
    }

    func delete(_ room: Room) {
        context.delete(room); save()
        restoreCurrentRoom()
    }

    /// After leaving a shared Room its records are purged by CloudKit; just move on to another Room.
    func roomWasLeft() {
        context.refreshAllObjects()
        currentRoom = nil
        restoreCurrentRoom()
    }

    func isOwner(of room: Room) -> Bool { persistence.isOwned(room) }

    // MARK: Moments

    func moment(in room: Room, on date: Date) -> Moment {
        let day = Catalog.startOfDay(date)
        if let existing = room.moments?.first(where: { $0.date == day }) { return existing }
        let m = insert(Moment.self, near: room)
        m.id = UUID(); m.date = day; m.createdAt = Date(); m.room = room
        return m
    }

    func update(_ moment: Moment, title: String? = nil, place: String? = nil, quote: String? = nil) {
        if let title { moment.title = title.trimmingCharacters(in: .whitespacesAndNewlines) }
        if let place { moment.place = place.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty }
        if let quote { moment.quote = quote.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty }
        save()
    }

    func setFeatured(_ moment: Moment) {
        for other in moment.room?.moments ?? [] { other.isFeatured = other == moment }
        save()
    }

    func delete(_ moment: Moment) { context.delete(moment); save() }

    func toggle(_ person: Person, in moment: Moment) {
        if moment.people?.contains(person) == true { moment.people?.remove(person) } else {
            var set = moment.people ?? []; set.insert(person); moment.people = set
        }
        save()
    }

    // MARK: Media

    func add(_ imported: ImportedMedia, to room: Room, momentOverride: Moment? = nil) -> MediaItem {
        let item = insert(MediaItem.self, near: room)
        item.id = UUID(); item.kind = imported.isVideo ? MediaItem.videoKind : MediaItem.photoKind
        item.capturedAt = imported.capturedAt; item.addedAt = Date()
        item.addedByName = identity.nameOrFallback; item.addedByID = identity.authorID
        item.width = imported.pixelSize.width; item.height = imported.pixelSize.height
        item.duration = imported.duration; item.fileExtension = imported.fileExtension
        item.photoIdentifier = imported.photoIdentifier
        item.data = imported.data; item.thumbnail = imported.thumbnail
        item.room = room
        item.moment = momentOverride ?? moment(in: room, on: imported.capturedAt)
        return item
    }

    func toggleFavorite(_ item: MediaItem) { item.isFavorite.toggle(); save() }
    func delete(_ item: MediaItem) {
        let moment = item.moment
        context.delete(item)
        context.processPendingChanges()
        if let moment, (moment.media?.count ?? 0) == 0 { context.delete(moment) }
        save()
    }
    func move(_ item: MediaItem, to moment: Moment) {
        let previous = item.moment
        item.moment = moment
        context.processPendingChanges()
        if let previous, previous != moment, (previous.media?.count ?? 0) == 0 { context.delete(previous) }
        save()
    }

    // MARK: People

    @discardableResult
    func createPerson(named name: String, role: String = "", faceGroup: Bool = false, in room: Room) -> Person {
        let p = insert(Person.self, near: room)
        p.id = UUID(); p.name = name.trimmingCharacters(in: .whitespaces); p.role = role
        p.isFaceGroup = faceGroup; p.createdAt = Date(); p.room = room
        let palette = Theme.personTones[(room.people?.count ?? 0) % Theme.personTones.count]
        p.toneHex = String(format: "%06X", palette.tone); p.accentHex = String(format: "%06X", palette.accent)
        save()
        return p
    }

    func rename(_ person: Person, to name: String, role: String? = nil) {
        person.name = name.trimmingCharacters(in: .whitespaces)
        if let role { person.role = role.trimmingCharacters(in: .whitespaces) }
        save()
    }

    /// A face group becomes a named person: its faces move, and every photo it appeared in is tagged.
    func promote(faceGroup: Person, named name: String) {
        faceGroup.name = name.trimmingCharacters(in: .whitespaces)
        faceGroup.isFaceGroup = false
        tagMoments(for: faceGroup)
        save()
    }

    func merge(_ source: Person, into target: Person) {
        for face in source.faces ?? [] { face.person = target }
        for m in source.moments ?? [] { var set = target.moments ?? []; set.insert(m); target.moments = set }
        context.delete(source)
        tagMoments(for: target)
        save()
    }

    private func tagMoments(for person: Person) {
        for face in person.faces ?? [] {
            guard let moment = face.media?.moment else { continue }
            var set = moment.people ?? []; set.insert(person); moment.people = set
        }
    }

    func delete(_ person: Person) { context.delete(person); save() }

    // MARK: Notes and likes

    func addNote(_ text: String, to moment: Moment) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        let n = insert(Note.self, near: moment)
        n.id = UUID(); n.text = trimmed; n.createdAt = Date()
        n.authorName = identity.nameOrFallback; n.authorID = identity.authorID; n.moment = moment
        save()
    }

    func delete(_ note: Note) { context.delete(note); save() }

    func toggleLike(_ moment: Moment) {
        if let mine = moment.reactions?.first(where: { $0.authorID == identity.authorID }) { context.delete(mine) } else {
            let r = insert(Reaction.self, near: moment)
            r.id = UUID(); r.createdAt = Date(); r.authorName = identity.nameOrFallback; r.authorID = identity.authorID; r.moment = moment
        }
        save()
    }

    // MARK: Stories

    @discardableResult
    func saveStory(title: String, subtitle: String, movie: Data, thumbnail: Data?, duration: Double, moment: Moment?, room: Room, weeklyReel: Date? = nil) -> Story {
        let s = insert(Story.self, near: room)
        s.id = UUID(); s.title = title; s.subtitle = subtitle; s.createdAt = Date(); s.duration = duration
        s.movie = movie; s.thumbnail = thumbnail; s.sourceMoment = moment; s.room = room
        s.isWeeklyReel = weeklyReel != nil; s.weekStart = weeklyReel
        save()
        return s
    }

    func delete(_ story: Story) { context.delete(story); save() }

    // MARK: Derived

    func contributions(in room: Room, since: Date = Date().addingTimeInterval(-7 * 86_400)) -> [ContributionSummary] {
        var groups: [String: ContributionSummary] = [:]
        for item in room.media ?? [] {
            guard let added = item.addedAt, added >= since, let moment = item.moment else { continue }
            let day = Catalog.startOfDay(added).timeIntervalSince1970
            let key = "\(item.addedByID ?? "?")|\(moment.id?.uuidString ?? "")|\(Int(day))"
            var g = groups[key] ?? ContributionSummary(id: key, momentID: moment.id ?? UUID(), authorName: item.addedByName ?? "Someone",
                                                        authorInitial: String((item.addedByName ?? "S").prefix(1)).uppercased(),
                                                        photoCount: 0, videoCount: 0, latest: added, likes: moment.likeCount, notes: moment.notes?.count ?? 0)
            if item.isVideo { g.videoCount += 1 } else { g.photoCount += 1 }
            g.latest = max(g.latest, added)
            groups[key] = g
        }
        return groups.values.sorted { $0.latest > $1.latest }
    }

    func save() { persistence.save() }
}

extension String {
    var nilIfEmpty: String? { isEmpty ? nil : self }
}
