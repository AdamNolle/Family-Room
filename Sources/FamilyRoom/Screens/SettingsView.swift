import CloudKit
import SwiftUI

/// Workshop: how the room feels, who is in it, and where things live.
struct SettingsView: View {
    @ObservedObject var room: Room
    var wide = false

    @EnvironmentObject private var store: RoomStore
    @EnvironmentObject private var app: AppState
    @ObservedObject private var identity = Identity.shared
    @AppStorage(SettingsKeys.reduceSunlight) private var reduceSunlight = false
    @AppStorage(SettingsKeys.sound) private var sound = true
    @State private var roomName = ""
    @State private var invitation: RoomInvitation?
    @State private var confirmLeave = false
    @State private var confirmDeleteRoom = false
    @State private var newRoomName = ""
    @State private var creatingRoom = false

    private var sharing: RoomSharing { store.sharing }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            PageHead(kicker: "Workshop", size: wide ? 72 : 50) { (Text("How the room ") + Text("feels").italic()) }
            let layout = wide ? AnyLayout(HStackLayout(alignment: .top, spacing: 40)) : AnyLayout(VStackLayout(alignment: .leading, spacing: 30))
            layout {
                VStack(alignment: .leading, spacing: 0) {
                    Text("Atmosphere").labelStyle()
                    RowsBox {
                        SwitchRow(title: "Reduce sunlight motion", subtitle: "Keeps the warm light steady.", isOn: $reduceSunlight)
                        Hairline()
                        SwitchRow(title: "Sound", subtitle: "Soft clicks when scrubbing.", isOn: $sound)
                    }
                    .padding(.top, 12)
                    Text("You").labelStyle().padding(.top, 28)
                    RowsBox {
                        HStack(spacing: 14) {
                            NameAvatar(name: identity.nameOrFallback, size: 30)
                            VStack(alignment: .leading, spacing: 2) {
                                Text("Your name on notes and additions").font(.sans(14, .semibold)).foregroundStyle(Theme.ink)
                                TextField("Your name", text: $identity.displayName).textFieldStyle(.plain).font(.sans(13.5)).foregroundStyle(Theme.inkSoft)
                            }
                        }
                        .padding(.horizontal, 18).frame(minHeight: 60)
                    }
                    .padding(.top, 12)
                    Text("Privacy").labelStyle().padding(.top, 28)
                    Text("Faces, best clips and stories are worked out on this \(DeviceWord.current). Photos stay in your family's iCloud.")
                        .font(.display(22)).italic().lineSpacing(2).foregroundStyle(Theme.inkSoft).padding(.top, 12).frame(maxWidth: 420, alignment: .leading)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                VStack(alignment: .leading, spacing: 0) {
                    HStack { Text("Who's in the room · iCloud").labelStyle(); Spacer()
                        if sharing.available, store.isOwner(of: room) { inviteButton } }
                    members.padding(.top, 12)
                    Text(sharing.accountMessage).font(.sans(12.5)).lineSpacing(3).foregroundStyle(Theme.inkMute).padding(.top, 10)
                    if let error = sharing.error { Text(error).font(.sans(12)).foregroundStyle(Theme.warm).padding(.top, 6) }
                    Text("Rooms").labelStyle().padding(.top, 28)
                    rooms.padding(.top, 12)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .padding(.horizontal, wide ? 36 : 20).padding(.top, wide ? 26 : 10).padding(.bottom, wide ? 48 : 118)
        .task(id: room.objectID) { roomName = room.displayName; await sharing.refresh(for: room) }
        .confirmationDialog("Leave \(room.displayName)? You will stop seeing its photos on your devices.", isPresented: $confirmLeave, titleVisibility: .visible) {
            Button("Leave Room", role: .destructive) { Task { await sharing.leave(room); store.delete(room) } }
        }
        .confirmationDialog("Delete \(room.displayName) and everything in it for every member?", isPresented: $confirmDeleteRoom, titleVisibility: .visible) {
            Button("Delete Room", role: .destructive) { store.delete(room) }
        }
    }

    @ViewBuilder
    private var inviteButton: some View {
        if let invitation {
            ShareLink(item: invitation, preview: SharePreview(room.displayName, icon: Image(systemName: "house.fill"))) {
                Text("Invite").font(.sans(11.5, .medium)).foregroundStyle(Theme.warm)
            }
            .buttonStyle(.plain)
        } else {
            Button { Task { invitation = await sharing.invitation(for: room) } } label: {
                Text(sharing.busy ? "Preparing…" : "Invite").font(.sans(11.5, .medium)).foregroundStyle(Theme.warm)
            }
            .buttonStyle(.plain).disabled(sharing.busy)
        }
    }

    private var members: some View {
        RowsBox {
            if sharing.members.isEmpty {
                HStack(spacing: 14) {
                    NameAvatar(name: identity.nameOrFallback, size: 30)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(identity.nameOrFallback).font(.sans(14, .semibold)).foregroundStyle(Theme.ink)
                        Text(sharing.available ? "Only you so far" : "This \(DeviceWord.current) only").font(.sans(12.5)).foregroundStyle(Theme.inkMute)
                    }
                    Spacer()
                }
                .padding(.horizontal, 18).frame(minHeight: 60)
            }
            ForEach(Array(sharing.members.enumerated()), id: \.element.id) { i, m in
                if i > 0 { Hairline() }
                HStack(spacing: 14) {
                    NameAvatar(name: m.name, size: 30)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(m.isCurrentUser ? "\(m.name) (you)" : m.name).font(.sans(14, .semibold)).foregroundStyle(Theme.ink)
                        Text(m.isOwner ? "Owner · \(m.status == .you ? "Syncing" : m.status.rawValue)" : m.status.rawValue).font(.sans(12.5)).foregroundStyle(Theme.inkMute)
                    }
                    Spacer()
                    if m.status == .syncing || m.status == .you {
                        Circle().fill(Theme.good).frame(width: 7, height: 7).shadow(color: Theme.good.opacity(0.3), radius: 3)
                    } else if m.status == .pending, store.isOwner(of: room), let invitation {
                        ShareLink(item: invitation, preview: SharePreview(room.displayName)) { Text("Resend").font(.sans(12.5, .semibold)).foregroundStyle(Theme.ink).padding(.horizontal, 10).frame(height: 28).background(.white.opacity(0.55), in: RoundedRectangle(cornerRadius: 9, style: .continuous)).hairlineBorder(9) }.buttonStyle(.plain)
                    }
                    if store.isOwner(of: room), !m.isOwner {
                        Menu { Button("Remove from Room", role: .destructive) { Task { await sharing.removeParticipant(m, from: room) } } } label: {
                            Image(systemName: "ellipsis").font(.system(size: 12, weight: .semibold)).foregroundStyle(Theme.inkMute).frame(width: 24, height: 24)
                        }
                        .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
                    }
                }
                .padding(.horizontal, 18).frame(minHeight: 60)
            }
        }
    }

    private var rooms: some View {
        RowsBox {
            ForEach(Array(store.rooms.enumerated()), id: \.element.objectID) { i, r in
                if i > 0 { Hairline() }
                HStack(spacing: 14) {
                    Image(systemName: r == room ? "house.fill" : "house").foregroundStyle(r == room ? Theme.warm : Theme.inkMute).frame(width: 30)
                    if r == room {
                        TextField("Room name", text: $roomName).textFieldStyle(.plain).font(.sans(14, .semibold)).foregroundStyle(Theme.ink)
                            .onSubmit { store.rename(room, to: roomName) }
                    } else {
                        Button { store.select(r); app.go(.hearth) } label: { Text(r.displayName).font(.sans(14, .semibold)).foregroundStyle(Theme.ink) }.buttonStyle(.plain)
                    }
                    Spacer()
                    Text(store.isOwner(of: r) ? "Yours" : "Shared with you").font(.sans(12.5)).foregroundStyle(Theme.inkMute)
                    if r == room {
                        Menu {
                            if roomName != room.displayName { Button("Save name") { store.rename(room, to: roomName) } }
                            if store.isOwner(of: room) { Button("Delete Room", role: .destructive) { confirmDeleteRoom = true } }
                            else { Button("Leave Room", role: .destructive) { confirmLeave = true } }
                        } label: { Image(systemName: "ellipsis").font(.system(size: 12, weight: .semibold)).foregroundStyle(Theme.inkMute).frame(width: 24, height: 24) }
                        .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
                    }
                }
                .padding(.horizontal, 18).frame(minHeight: 60)
            }
            Hairline()
            HStack(spacing: 14) {
                Image(systemName: "plus").foregroundStyle(Theme.inkMute).frame(width: 30)
                TextField("New Room", text: $newRoomName).textFieldStyle(.plain).font(.sans(14)).foregroundStyle(Theme.ink)
                    .onSubmit(createRoom)
                FRButton("Create", size: .small, disabled: newRoomName.trimmingCharacters(in: .whitespaces).isEmpty, action: createRoom)
            }
            .padding(.horizontal, 18).frame(minHeight: 60)
        }
    }

    private func createRoom() {
        let name = newRoomName.trimmingCharacters(in: .whitespaces)
        guard !name.isEmpty else { return }
        store.createRoom(named: name); newRoomName = ""; app.go(.hearth)
    }
}
