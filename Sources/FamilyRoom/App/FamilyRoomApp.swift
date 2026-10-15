import CloudKit
import SwiftUI

@main
struct FamilyRoomApp: App {
    @StateObject private var store: RoomStore
    #if os(iOS)
    @UIApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    #else
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    #endif

    init() {
        TypeFaces.register()
        _store = StateObject(wrappedValue: RoomStore())
    }

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(store)
                .environment(\.managedObjectContext, store.context)
        }
        #if os(macOS)
        .windowStyle(.hiddenTitleBar)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Add Photos from Files…") { NotificationCenter.default.post(name: .familyRoomImportFiles, object: nil) }
                    .keyboardShortcut("o")
            }
        }
        #endif
    }
}

/// Accepts CloudKit share invitations so an invited family member's Room appears in their library.
enum ShareAcceptance {
    static func accept(_ metadata: CKShare.Metadata) {
        let persistence = Persistence.shared
        guard let store = persistence.sharedStore else { return }
        persistence.container.acceptShareInvitations(from: [metadata], into: store) { _, error in
            if let error { NSLog("Share acceptance failed: \(error.localizedDescription)") }
        }
    }
}

#if os(iOS)
final class AppDelegate: NSObject, UIApplicationDelegate {
    func application(_ application: UIApplication, configurationForConnecting connectingSceneSession: UISceneSession, options: UIScene.ConnectionOptions) -> UISceneConfiguration {
        let config = UISceneConfiguration(name: nil, sessionRole: connectingSceneSession.role)
        config.delegateClass = SceneDelegate.self
        return config
    }
}

final class SceneDelegate: NSObject, UIWindowSceneDelegate {
    func windowScene(_ windowScene: UIWindowScene, userDidAcceptCloudKitShareWith cloudKitShareMetadata: CKShare.Metadata) {
        ShareAcceptance.accept(cloudKitShareMetadata)
    }
}
#else
final class AppDelegate: NSObject, NSApplicationDelegate {
    func application(_ application: NSApplication, userDidAcceptCloudKitShareWith metadata: CKShare.Metadata) {
        ShareAcceptance.accept(metadata)
    }
}
#endif
