import SwiftUI

struct SidebarContentView: View {
    @EnvironmentObject var viewModel: AppViewModel
    @State private var selection: SidebarSection? = .hearth
    @State private var selectedEvent: FamilyEvent? = nil

    var body: some View {
        NavigationSplitView {
            sidebar
        } detail: {
            detail
        }
        .navigationSplitViewStyle(.balanced)
    }

    var sidebar: some View {
        List(selection: $selection) {
            Section("Rooms") {
                Label("The Hearth", systemImage: "house.fill").tag(SidebarSection.hearth)
                Label("Family Circle", systemImage: "person.2.fill").tag(SidebarSection.family)
                Label("Stories", systemImage: "film.stack").tag(SidebarSection.stories)
            }
            Section("Workshop") {
                Label("Settings", systemImage: "gear").tag(SidebarSection.settings)
            }
        }
        .navigationTitle(viewModel.activeRoomName)
        .listStyle(.sidebar)
        .tint(FurnitureDesign.warm)
    }

    @ViewBuilder
    var detail: some View {
        switch selection ?? .hearth {
        case .hearth:
            HomeView(viewModel: viewModel)
                .navigationTitle("")
        case .family:
            PeopleView(viewModel: viewModel)
                .navigationTitle("Family")
        case .stories:
            StoriesView()
                .environmentObject(viewModel)
                .navigationTitle("Stories")
        case .settings:
            SettingsView(viewModel: viewModel)
                .navigationTitle("Settings")
        }
    }
}

enum SidebarSection: Hashable {
    case hearth, family, stories, settings
}
