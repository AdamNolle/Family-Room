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
            
            Text("Stories")
                .tabItem {
                    Label("Stories", systemImage: "play.rectangle")
                }
            
            SettingsView(viewModel: viewModel)
                .tabItem {
                    Label("Settings", systemImage: "gear")
                }
        }
        .accentColor(FurnitureDesign.sageWool)
    }
}
