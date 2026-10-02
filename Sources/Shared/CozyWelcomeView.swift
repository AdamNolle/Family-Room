import SwiftUI
import UniformTypeIdentifiers

struct WelcomeView: View {
    @EnvironmentObject var store: LibraryStore
    @ScaledMetric(relativeTo: .largeTitle) private var headingSize = 42
    @State private var path = "create"
    @State private var name = "Our Family Room"
    @State private var invitation = ""
    @State private var recovery = ""
    @State private var restoreFile = false
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 28) {
                HStack(spacing: 10) {
                    Image(systemName: "house").font(.title2).foregroundStyle(CozyTheme.accentInk)
                    RoomEyebrow(text: "Family Room")
                }
                VStack(alignment: .leading, spacing: 16) {
                    Text("A home for\nyour memories.")
                        .font(.system(size: headingSize, weight: .medium, design: .serif)).tracking(-1.1).fixedSize(horizontal: false, vertical: true)
                    Text("Your photos, your people, your place. Bring a private library together and choose where it lives.")
                        .font(.body).foregroundStyle(CozyTheme.secondary).lineSpacing(4).frame(maxWidth: 440, alignment: .leading)
                }
                HStack(spacing: 18) {
                    Label("Private by default", systemImage: "lock")
                    Label("Your storage", systemImage: "externaldrive")
                }.font(.caption).foregroundStyle(CozyTheme.secondary)
                Divider().overlay(CozyTheme.line)
                Picker("Get started", selection: $path) {
                    Text("Create a Room").tag("create")
                    Text("Join a Room").tag("join")
                    Text("Recover").tag("recover")
                }.pickerStyle(.segmented)
                VStack(alignment: .leading, spacing: 18) {
                    if path == "create" {
                        RoomEyebrow(text: "Start somewhere familiar")
                        Text("Who is this Room for?").font(.title2.weight(.semibold))
                        Text("A family, an extended family, close friends — every circle can have a Room.").foregroundStyle(CozyTheme.secondary)
                        TextField("Room name", text: $name).textFieldStyle(.roundedBorder).accessibilityIdentifier("onboarding.roomName")
                        Button { store.createRoom(name) } label: { Label("Create Room", systemImage: "arrow.right").frame(maxWidth: .infinity).padding(.vertical, 6) }
                            .accessibilityIdentifier("onboarding.createRoom")
                            .buttonStyle(CozyPrimaryButtonStyle()).disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || store.snapshot.deviceId.isEmpty)
                        Text("Next, choose storage and automatic sharing.").font(.caption).foregroundStyle(CozyTheme.secondary)
                    } else if path == "join" {
                        RoomEyebrow(text: "You’re invited")
                        Text("Bring your device into a Room.").font(.title2.weight(.semibold))
                        Text("Share this device code privately with the Room owner, then paste the invitation they create for you.").foregroundStyle(CozyTheme.secondary)
                        Text(store.snapshot.deviceId).font(.caption.monospaced()).textSelection(.enabled).padding(12).frame(maxWidth: .infinity, alignment: .leading).background(CozyTheme.canvas, in: RoundedRectangle(cornerRadius: 10))
                        TextField("Paste invitation", text: $invitation).textFieldStyle(.roundedBorder)
                        Button("Join Room") { store.perform { _ = try await store.command(["action": "join", "invitation": invitation]) } }
                            .buttonStyle(CozyPrimaryButtonStyle()).disabled(invitation.isEmpty || store.snapshot.deviceId.isEmpty)
                        Text("You do not need your own cloud account to join.").font(.caption).foregroundStyle(CozyTheme.secondary)
                    } else {
                        RoomEyebrow(text: "Find your way back")
                        Text("Recover your library.").font(.title2.weight(.semibold))
                        Text("Use your saved recovery key to unlock this device or restore an encrypted backup.").foregroundStyle(CozyTheme.secondary)
                        SecureField("Recovery key", text: $recovery).textFieldStyle(.roundedBorder)
                        Button("Unlock this library") { Task { await store.open(recoveryKey: recovery) } }.disabled(recovery.isEmpty)
                        Button("Restore encrypted backup") { restoreFile = true }.disabled(recovery.isEmpty || store.snapshot.deviceId.isEmpty)
                    }
                }.padding(24).frame(maxWidth: .infinity, alignment: .leading).cozyPanel(glass: true)
            }.padding(28).frame(maxWidth: 600, alignment: .leading).frame(maxWidth: .infinity)
        }.background(CozyTheme.canvas).foregroundStyle(CozyTheme.ink).tint(CozyTheme.accent)
        .fileImporter(isPresented: $restoreFile, allowedContentTypes: [.data]) { result in
            if case .success(let url) = result {
                store.perform {
                    let scoped = url.startAccessingSecurityScopedResource(); defer { if scoped { url.stopAccessingSecurityScopedResource() } }
                    _ = try await store.command(["action": "restore", "path": url.path, "recovery_key": recovery])
                }
            }
        }
    }
}
