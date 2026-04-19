import SwiftUI
import SwiftData

@MainActor
class AppViewModel: ObservableObject {
    @Published var activeRoomName: String = "Our Family Room"
    @Published var isSyncing: Bool = false
    @Published var selectedEvent: FamilyEvent? = nil
    @Published var selectedFabric: FabricType = .linen
    
    enum FabricType: String, CaseIterable {
        case linen = "Warm Linen"
        case wool = "Soft Wool"
        case silk = "Raw Silk"
        case wood = "Walnut Grain"
    }
    
    // Feature Rich State
    @Published var people: [PersonModel] = [
        PersonModel(name: "Mom"),
        PersonModel(name: "Dad"),
        PersonModel(name: "Lucy")
    ]
    
    @Published var recentEvents: [FamilyEvent] = [
        FamilyEvent(name: "Grandpa's Birthday", date: "Oct 12, 2025", image: "birthday"),
        FamilyEvent(name: "Summer Trip 2024", date: "Aug 15, 2024", image: "trip"),
        FamilyEvent(name: "Holiday Dinner", date: "Dec 24, 2024", image: "holiday"),
        FamilyEvent(name: "Piano Recital", date: "Mar 10, 2025", image: "music")
    ]
    
    func processNewMedia(urls: [URL]) async {
        isSyncing = true
        for url in urls {
            _ = try? await FamilyIntelligenceService.shared.analyzeMedia(url)
        }
        isSyncing = false
    }
    
    func generateHighlightStory(for event: FamilyEvent) {
        // Trigger StoryComposer logic
    }
}

struct FamilyEvent: Identifiable, Hashable {
    let id = UUID()
    let name: String
    let date: String
    let image: String
}
