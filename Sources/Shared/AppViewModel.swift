import SwiftUI

@MainActor
class AppViewModel: ObservableObject {
    @Published var activeRoomName: String = "The Alderhouse"
    @Published var isSyncing: Bool = false
    @Published var selectedEvent: FamilyEvent? = nil

    // Mantelpiece state — people-as-navigation filter.
    @Published var selectedPersonIDs: Set<String> = []

    @Published var reduceSunlightMotion: Bool = false
    @Published var soundEnabled: Bool = true

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

    @Published var stories: [SampleStory] = []

    // ─── The Mantelpiece family + river data ─────────────────────────────

    let family: [FamilyPerson] = [
        FamilyPerson(id: "M", name: "Margot", nickname: "Mom",  role: "Grandma",
                     tone: Color(hex: 0xC97A5D), accent: Color(hex: 0xF0BFA8), photoCount: 892),
        FamilyPerson(id: "D", name: "Henry",  nickname: "Dad",  role: "Grandpa",
                     tone: Color(hex: 0x7A6A4A), accent: Color(hex: 0xD4C29A), photoCount: 654),
        FamilyPerson(id: "C", name: "Cora",   nickname: "Cora", role: "Mom",
                     tone: Color(hex: 0x4A7A82), accent: Color(hex: 0xA8D0D4), photoCount: 1_204),
        FamilyPerson(id: "T", name: "Theo",   nickname: "Theo", role: "Dad",
                     tone: Color(hex: 0x6A5A8A), accent: Color(hex: 0xC4B8D8), photoCount: 743),
        FamilyPerson(id: "I", name: "Iris",   nickname: "Iris", role: "Daughter",
                     tone: Color(hex: 0xD49060), accent: Color(hex: 0xF5D4B0), photoCount: 2_108),
    ]

    let places: [String] = ["The lake house", "Margot & Henry's", "Our backyard", "Iris's school"]

    // Mar–Dec 2024 — ported from familyroom.jsx River() items.
    let moments: [Moment] = [
        Moment(day: "Mar 14", title: "Iris's first bike ride", weight: 5, scene: .bike,   whoIDs: ["I","D"]),
        Moment(day: "Apr 09", title: "First crocus",           weight: 2, scene: .garden, whoIDs: ["M"]),
        Moment(day: "Apr 22", title: "Dad came home",          weight: 5, scene: .indoor, whoIDs: ["D","M","C"]),
        Moment(day: "May 05", title: "Sunday picnic",          weight: 3, scene: .garden, whoIDs: ["M","D","C","T","I"]),
        Moment(day: "Jun 02", title: "Lake house",             weight: 4, scene: .lake,   whoIDs: ["M","D","C","T","I"]),
        Moment(day: "Jul 04", title: "Backyard fireworks",     weight: 4, scene: .sunset, whoIDs: ["T","I"]),
        Moment(day: "Aug 09", title: "Dad's 72nd",             weight: 5, scene: .cake,   whoIDs: ["M","D","C","T","I"], hero: true, place: "The Lake House"),
        Moment(day: "Sep 22", title: "Iris on stage",          weight: 4, scene: .stage,  whoIDs: ["I"]),
        Moment(day: "Oct 11", title: "Apple picking",          weight: 3, scene: .fall,   whoIDs: ["M","C","I"]),
        Moment(day: "Oct 31", title: "Halloween",              weight: 4, scene: .wolf,   whoIDs: ["I","T","C"]),
        Moment(day: "Nov 28", title: "Thanksgiving",           weight: 4, scene: .table,  whoIDs: ["M","D","C","T","I"]),
        Moment(day: "Dec 24", title: "Christmas Eve",          weight: 5, scene: .indoor, whoIDs: ["M","D","C","T","I"]),
    ]

    var heroMoment: Moment { moments.first(where: \.hero) ?? moments[0] }

    func togglePerson(_ id: String) {
        if selectedPersonIDs.contains(id) {
            selectedPersonIDs.remove(id)
        } else {
            selectedPersonIDs.insert(id)
        }
    }

    func processNewMedia(urls: [URL]) async {
        isSyncing = true
        for url in urls {
            _ = try? await FamilyIntelligenceService.shared.analyzeMedia(url)
        }
        isSyncing = false
    }

    func generateHighlightStory(for event: FamilyEvent) {
        // Trigger StoryComposer logic (Phase 5).
    }
}

// MARK: - Mantelpiece domain

struct FamilyPerson: Identifiable, Hashable {
    let id: String
    let name: String
    let nickname: String
    let role: String
    let tone: Color
    let accent: Color
    let photoCount: Int

    var initial: String { String(name.prefix(1)) }
}

struct Moment: Identifiable, Hashable {
    let id = UUID()
    let day: String          // "Aug 09"
    let title: String
    let weight: Int          // 1...5 → river plate size
    let scene: PhotoScene
    let whoIDs: [String]
    var hero: Bool = false
    var place: String? = nil
    var quote: String? = nil
}

enum PhotoScene: String, CaseIterable, Hashable {
    case sunset, lake, indoor, cake, garden, bike, snow, stage, fall, wolf, table, studio
}

// MARK: - Legacy lightweight models (still referenced by older screens)

struct FamilyEvent: Identifiable, Hashable {
    let id = UUID()
    let name: String
    let date: String
    let image: String
}

struct SampleStory: Identifiable, Hashable {
    let id = UUID()
    let name: String
    let subtitle: String
}
