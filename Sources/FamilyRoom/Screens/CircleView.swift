import CoreData
import SwiftUI

/// Family Circle: the people in the Room, and face groups found on this device.
struct CircleView: View {
    @ObservedObject var room: Room
    var wide = false
    var activeID: UUID? = nil
    var onOpen: (Person) -> Void

    @EnvironmentObject private var store: RoomStore
    @EnvironmentObject private var app: AppState
    @FetchRequest private var people: FetchedResults<Person>
    @State private var adding = false
    @State private var newName = ""
    @State private var newRole = ""

    init(room: Room, wide: Bool = false, activeID: UUID? = nil, onOpen: @escaping (Person) -> Void) {
        self.room = room; self.wide = wide; self.activeID = activeID; self.onOpen = onOpen
        _people = FetchRequest(sortDescriptors: [NSSortDescriptor(key: "createdAt", ascending: true)], predicate: NSPredicate(format: "room == %@", room))
    }

    private var members: [Person] { people.filter { !$0.isFaceGroup } }
    private var faces: [Person] { people.filter(\.isFaceGroup) }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            PageHead(kicker: "Family Circle", size: wide ? 72 : 50,
                     text: "Tap a name to filter the whole room. Faces are grouped on this \(DeviceWord.current); nothing is sent anywhere.") {
                (Text("Everyone, sorted ") + Text("kindly").italic())
            }
            HStack { Text(members.isEmpty ? "People" : "The \(spelled(members.count))").labelStyle(); Spacer()
                Button { newName = ""; newRole = ""; adding = true } label: { Text("Add a person").font(.sans(11.5, .medium)).foregroundStyle(Theme.warm) }.buttonStyle(.plain) }
            roster.padding(.top, 10)
            HStack { Text("Found on this \(DeviceWord.current)").labelStyle(); Spacer()
                if let p = store.faceScanProgress { Text("Checking \(p.completedUnitCount) of \(p.totalUnitCount)").labelStyle().monospacedDigit() }
                else { Text("\(faces.count) \(faces.count == 1 ? "group" : "groups")").labelStyle().monospacedDigit() } }
                .padding(.top, 30)
            faceGrid.padding(.top, 14)
        }
        .padding(.horizontal, wide ? 36 : 20).padding(.top, wide ? 26 : 10).padding(.bottom, wide ? 48 : 118)
        .sheet(isPresented: $adding) { addSheet }
    }

    private func spelled(_ n: Int) -> String {
        let words = ["", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"]
        return n < words.count ? words[n] : "\(n)"
    }

    private var roster: some View {
        VStack(spacing: 0) {
            ForEach(Array(members.enumerated()), id: \.element.objectID) { i, p in
                let id = p.id ?? UUID(), on = app.selectedPeople.contains(id)
                HStack(spacing: wide ? 20 : 12) {
                    Text(Catalog.pad(i + 1)).font(.label(10.5)).foregroundStyle(on ? Theme.warm : Theme.inkFaint).frame(width: 22, alignment: .leading)
                    Button { onOpen(p) } label: {
                        HStack(spacing: 14) {
                            AvatarView(person: p.summary, size: 44, ring: on)
                            VStack(alignment: .leading, spacing: 4) {
                                Text(p.displayName).font(.display(wide ? 46 : 34)).foregroundStyle(Theme.ink).lineLimit(1)
                                Text([p.role ?? "", "\(p.photoCount.formatted()) \(p.photoCount == 1 ? "photo" : "photos")"].filter { !$0.isEmpty }.joined(separator: " · ")).labelStyle().monospacedDigit()
                            }
                        }
                        .frame(minHeight: 52)
                    }
                    .buttonStyle(.plain)
                    Spacer(minLength: 0)
                    FRButton(on ? "Filtering ✓" : "Filter", style: on ? .solid : .ghost, size: .small) { app.toggle(id); Feedback.tick() }
                }
                .padding(.vertical, 10)
                .background(activeID == id ? LinearGradient(colors: [.white.opacity(0.6), .clear], startPoint: .leading, endPoint: .trailing) : LinearGradient(colors: [.clear], startPoint: .leading, endPoint: .trailing))
                .overlay(alignment: .top) { Hairline() }
            }
            if members.isEmpty {
                Text("No one yet. Add the people who show up in your photos — this never invites anyone to the Room; members are managed in Settings.")
                    .font(.sans(13)).lineSpacing(3).foregroundStyle(Theme.inkMute).padding(.vertical, 14).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .overlay(alignment: .bottom) { Hairline() }
    }

    private var faceGrid: some View {
        let columns = [GridItem(.adaptive(minimum: 84), spacing: 20, alignment: .top)]
        return LazyVGrid(columns: columns, alignment: .leading, spacing: 20) {
            ForEach(faces, id: \.objectID) { group in
                Button { onOpen(group) } label: {
                    VStack(spacing: 4) {
                        FaceCropView(group: group, size: 64)
                            .overlay { if activeID == group.id { Circle().strokeBorder(Theme.warm, lineWidth: 2).padding(-3) } }
                            .padding(.bottom, 8)
                        Text(group.displayName).font(.sans(12.5, .semibold)).foregroundStyle(Theme.ink).lineLimit(1)
                        Text(group.photoCount > 0 ? "\(group.photoCount) \(group.photoCount == 1 ? "photo" : "photos")" : "Not yet matched").font(.num(11)).foregroundStyle(Theme.inkMute)
                    }
                    .frame(width: 84)
                }
                .buttonStyle(PressScale())
            }
            Button { Task { await store.scanFaces(in: room) } } label: {
                VStack(spacing: 4) {
                    Image(systemName: store.faceScanProgress == nil ? "plus" : "hourglass").font(.system(size: 22)).foregroundStyle(Theme.inkMute)
                        .frame(width: 64, height: 64).overlay(Circle().strokeBorder(Theme.hair2, lineWidth: 1)).padding(.bottom, 8)
                    Text(faces.isEmpty ? "Find faces" : "Check new photos").font(.sans(12.5, .semibold)).foregroundStyle(Theme.ink)
                    Text(" ").font(.num(11))
                }
                .frame(width: 84)
            }
            .buttonStyle(PressScale())
            .disabled(store.faceScanProgress != nil)
        }
    }

    private var addSheet: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack { Text("Add a person").font(.display(34)).foregroundStyle(Theme.ink); Spacer(); CloseButton { adding = false } }
            FieldBox(label: "Name", text: $newName, placeholder: "Iris")
            FieldBox(label: "Role", text: $newRole, placeholder: "Daughter, Grandpa, Friend…")
            Text("This is who appears in photos. To invite someone to see the Room, use Settings.").font(.sans(12.5)).foregroundStyle(Theme.inkMute)
            HStack { Spacer(); FRButton("Cancel", style: .ghost) { adding = false }
                FRButton("Add", disabled: newName.trimmingCharacters(in: .whitespaces).isEmpty) { store.createPerson(named: newName, role: newRole, in: room); adding = false } }
        }
        .padding(24).frame(minWidth: 320, idealWidth: 440).background(Theme.paper2)
        .presentationDetents([.medium])
    }
}

/// Shows the best face crop for a group, or its initial.
struct FaceCropView: View {
    @ObservedObject var group: Person
    var size: CGFloat
    var body: some View {
        let crop = (group.faces ?? []).max(by: { $0.confidence < $1.confidence })?.crop
        if let crop {
            DataThumb(data: crop, maxSide: 300).frame(width: size, height: size).clipShape(Circle())
                .overlay(Circle().strokeBorder(Theme.hair2, lineWidth: 0.5)).shadow(color: Theme.shadowInk.opacity(0.08), radius: 7, y: 3)
        } else {
            FaceInitial(text: group.summary.initial, size: size)
        }
    }
}
