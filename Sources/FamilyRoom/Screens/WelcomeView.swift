import SwiftUI

/// First run: name the Room. Shared Rooms arrive on their own once an invitation is accepted.
struct WelcomeView: View {
    @EnvironmentObject private var store: RoomStore
    @State private var name = ""
    @FocusState private var focused: Bool

    var body: some View {
        ZStack {
            PaperBackground()
            VStack(alignment: .leading, spacing: 0) {
                Text("Family Room").labelStyle(warm: true).entrance(0)
                (Text("A room for the ") + Text("five of you").italic() + Text(", or the fifty."))
                    .font(.display(56)).foregroundStyle(Theme.ink).lineSpacing(-6).padding(.top, 12).entrance(1)
                Text("Photos and videos gather into moments by day. Everyone you invite sees them through iCloud, and faces, clips and stories are worked out on your own devices.")
                    .font(.sans(15)).lineSpacing(5).foregroundStyle(Theme.inkSoft).padding(.top, 16).frame(maxWidth: 480, alignment: .leading).entrance(3)
                FieldBox(label: "Name your Room", text: $name, placeholder: "The Alderhouse").padding(.top, 28).focused($focused).onSubmit(create).entrance(5)
                HStack(spacing: 8) {
                    FRButton("Open the Room", mark: "✦", disabled: name.trimmingCharacters(in: .whitespaces).isEmpty, action: create)
                    Text("Invited to a Room? Accept the invitation and it appears here.").font(.sans(12.5)).foregroundStyle(Theme.inkMute)
                }
                .padding(.top, 14)
                .entrance(6)
            }
            .padding(32).frame(maxWidth: 560, alignment: .leading)
        }
        .onAppear { focused = true }
    }

    private func create() {
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        store.createRoom(named: trimmed)
    }
}
