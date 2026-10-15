import CoreData
import SwiftUI

enum AppTab: String, CaseIterable, Identifiable {
    case hearth, circle, stories, settings
    var id: String { rawValue }
    var title: String {
        switch self { case .hearth: "Home"; case .circle: "Family"; case .stories: "Stories"; case .settings: "Settings" }
    }
    var longTitle: String {
        switch self { case .hearth: "The Hearth"; case .circle: "Family Circle"; case .stories: "Stories"; case .settings: "Settings" }
    }
    var number: String { Catalog.pad(AppTab.allCases.firstIndex(of: self)! + 1) }
}

enum Route: Hashable {
    case moment(Moment)
    case person(Person)
}

/// Navigation and filter state shared by every shell (phone, iPad, foldable, Mac).
@MainActor
final class AppState: ObservableObject {
    @Published var tab: AppTab = .hearth
    @Published var path: [Route] = []
    @Published var selectedPeople: Set<UUID> = []
    @Published var query = ""
    @Published var year: Int?
    @Published var stageMomentID: UUID?
    @Published var composeMoment: Moment?
    @Published var composeWeekly = false
    @Published var playingStory: Story?
    @Published var showImporter = false
    @Published var duoMoment: Moment?
    @Published var duoPerson: Person?

    func go(_ tab: AppTab) {
        self.tab = tab
        path = []
    }

    func open(_ moment: Moment) {
        if tab != .hearth { tab = .hearth }
        path = [.moment(moment)]
    }

    func open(_ person: Person) {
        if tab != .circle { tab = .circle }
        path = [.person(person)]
    }

    func toggle(_ id: UUID) {
        if selectedPeople.contains(id) { selectedPeople.remove(id) } else { selectedPeople.insert(id) }
    }

    func back() { if !path.isEmpty { path.removeLast() } }
}
