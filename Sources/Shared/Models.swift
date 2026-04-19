import Foundation

// Switching to standard classes for the prototype to ensure build stability 
// as SwiftData macros require a specific Xcode-linked environment.

class FamilyRoomModel: Identifiable {
    let id = UUID()
    var name: String
    var createdAt: Date
    var events: [EventModel] = []
    
    init(name: String = "Our Family Room") {
        self.name = name
        self.createdAt = Date()
    }
}

class EventModel: Identifiable {
    let id = UUID()
    var name: String
    var date: Date
    var category: String
    var mediaItems: [MediaItemModel] = []
    
    init(name: String, date: Date = Date(), category: String = "General") {
        self.name = name
        self.date = date
        self.category = category
    }
}

class MediaItemModel: Identifiable {
    var id: UUID
    var url: URL
    var type: String // "photo" or "video"
    var detectedFaces: [String] = [] // Person IDs
    var isFavorite: Bool = false
    
    init(url: URL, type: String) {
        self.id = UUID()
        self.url = url
        self.type = type
    }
}

class PersonModel: Identifiable {
    let id = UUID()
    var name: String
    var facePrints: [Data] = []
    
    init(name: String) {
        self.name = name
    }
}
