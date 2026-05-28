import SwiftUI

@main
struct FamilyRoomApp: App {
    @StateObject private var viewModel = AppViewModel()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(viewModel)
        }
    }
}

struct ContentView: View {
    @EnvironmentObject var viewModel: AppViewModel
    #if os(iOS)
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    #endif

    var body: some View {
        rootView
            .tint(FurnitureDesign.warm)
    }

    @ViewBuilder
    private var rootView: some View {
        #if os(macOS)
        SidebarContentView()
        #else
        if horizontalSizeClass == .regular {
            SidebarContentView()
        } else {
            iPhoneTabbedView()
        }
        #endif
    }
}

struct iPhoneTabbedView: View {
    @EnvironmentObject var viewModel: AppViewModel

    var body: some View {
        TabView {
            HomeView(viewModel: viewModel)
                .tabItem {
                    Label("Home", systemImage: "house")
                }

            PeopleView(viewModel: viewModel)
                .tabItem {
                    Label("Family", systemImage: "person.2")
                }

            StoriesView()
                .tabItem {
                    Label("Stories", systemImage: "play.rectangle")
                }

            SettingsView(viewModel: viewModel)
                .tabItem {
                    Label("Settings", systemImage: "gear")
                }
        }
    }
}
