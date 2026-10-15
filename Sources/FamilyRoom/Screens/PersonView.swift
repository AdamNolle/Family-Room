import SwiftUI

/// One person or face group: rename, filter, and the moments they appear in.
struct PersonView: View {
    @ObservedObject var person: Person
    var hideBack = false
    var wide = false
    var onBack: () -> Void = {}
    var onOpen: (Moment) -> Void

    @EnvironmentObject private var store: RoomStore
    @EnvironmentObject private var app: AppState
    @State private var editing = false
    @State private var name = ""
    @State private var role = ""
    @State private var confirmDelete = false
    @FocusState private var nameFocus: Bool

    private var isGroup: Bool { person.isFaceGroup }
    private var id: UUID { person.id ?? UUID() }
    private var moments: [Moment] {
        var set = Set(person.moments ?? [])
        for f in person.faces ?? [] { if let m = f.media?.moment { set.insert(m) } }
        return set.sorted { ($0.date ?? .distantPast) > ($1.date ?? .distantPast) }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if !hideBack { BackButton(title: "Family Circle", action: onBack).padding(.bottom, 16) }
            header
            Text("Moments").labelStyle().padding(.top, 26)
            if moments.isEmpty {
                VStack(alignment: .leading, spacing: 12) {
                    Text("No photos of \(person.displayName) yet.").font(.display(32)).foregroundStyle(Theme.ink)
                    Text(isGroup ? "This face was found on this \(DeviceWord.current) but has not been matched to a moment yet." : "Tag \(person.displayName) on a moment, or match a face group below.")
                        .font(.sans(14)).lineSpacing(3).foregroundStyle(Theme.inkSoft).frame(maxWidth: 460, alignment: .leading)
                }
                .padding(.vertical, 44).padding(.top, 14)
                .overlay(alignment: .top) { Rectangle().fill(Theme.hair2).frame(height: 0.5).padding(.top, 14) }
                .overlay(alignment: .bottom) { Rectangle().fill(Theme.hair2).frame(height: 0.5) }
            } else {
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 14), count: wide ? 4 : 2), spacing: 18) {
                    ForEach(moments, id: \.objectID) { m in
                        Button { onOpen(m) } label: {
                            VStack(alignment: .leading, spacing: 10) {
                                PhotoSurface(media: m.cover, maxSide: 600).aspectRatio(4 / 3, contentMode: .fit).printShadow(strength: 0.6)
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(m.displayTitle).font(.sans(13, .semibold)).foregroundStyle(Theme.ink).lineLimit(1)
                                    Text(Catalog.dayLabel(m.date ?? Date())).font(.num(11)).foregroundStyle(Theme.inkMute)
                                }
                            }
                        }
                        .buttonStyle(PressScale())
                    }
                }
                .padding(.top, 12)
            }
            if isGroup { mergeSection.padding(.top, 30) }
        }
        .padding(.horizontal, wide ? 36 : 20).padding(.top, wide ? 26 : 10).padding(.bottom, wide ? 48 : 118)
        .confirmationDialog(isGroup ? "Forget this face group?" : "Remove \(person.displayName) from the Room's people?", isPresented: $confirmDelete, titleVisibility: .visible) {
            Button(isGroup ? "Forget faces" : "Remove person", role: .destructive) { store.delete(person); onBack() }
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 12) {
            if isGroup { FaceCropView(group: person, size: 84) } else { AvatarView(person: person.summary, size: 84) }
            Text(isGroup ? "Face group · found on this \(DeviceWord.current)" : [person.role ?? "", "\(person.photoCount.formatted()) photos"].filter { !$0.isEmpty }.joined(separator: " · ")).labelStyle(warm: true)
            if editing {
                VStack(alignment: .leading, spacing: 8) {
                    TextField("Name", text: $name).textFieldStyle(.plain).font(.display(44)).foregroundStyle(Theme.ink)
                        .padding(.horizontal, 10).padding(.vertical, 4)
                        .background(.white.opacity(0.7), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                        .overlay(RoundedRectangle(cornerRadius: 8, style: .continuous).strokeBorder(Theme.warm, lineWidth: 1))
                        .focused($nameFocus).onSubmit(save)
                    if !isGroup { TextField("Role", text: $role).textFieldStyle(.plain).font(.sans(15)).padding(.horizontal, 10).frame(height: 36).background(.white.opacity(0.7), in: RoundedRectangle(cornerRadius: 8, style: .continuous)).hairlineBorder(8) }
                    HStack(spacing: 8) { FRButton("Save", size: .small, action: save); FRButton("Cancel", style: .ghost, size: .small) { editing = false } }
                }
            } else {
                Text(person.displayName).font(.display(wide ? 88 : 64)).foregroundStyle(Theme.ink).lineSpacing(-8)
            }
            HStack(spacing: 8) {
                if !isGroup {
                    FRButton(app.selectedPeople.contains(id) ? "Showing \(person.displayName) ✓" : "Show only \(person.displayName)", style: app.selectedPeople.contains(id) ? .solid : .ghost, size: .small) { app.toggle(id); Feedback.tick() }
                }
                if !editing {
                    FRButton(isGroup ? "Name this face" : "Rename", style: .ghost, size: .small) { name = isGroup ? "" : person.displayName; role = person.role ?? ""; editing = true; nameFocus = true }
                    Menu {
                        Button(isGroup ? "Forget this face group" : "Remove person", role: .destructive) { confirmDelete = true }
                    } label: { Image(systemName: "ellipsis").font(.system(size: 13, weight: .semibold)).foregroundStyle(Theme.ink).frame(width: 28, height: 28).background(.white.opacity(0.55), in: RoundedRectangle(cornerRadius: 8, style: .continuous)).hairlineBorder(8) }
                    .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden).accessibilityLabel("More")
                }
            }
            .padding(.top, 2)
        }
        .padding(.bottom, 22)
        .overlay(alignment: .bottom) { Hairline() }
    }

    private func save() {
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { editing = false; return }
        if isGroup { store.promote(faceGroup: person, named: trimmed) } else { store.rename(person, to: trimmed, role: role) }
        editing = false
    }

    private var mergeSection: some View {
        let candidates = (person.room?.people ?? []).filter { !$0.isFaceGroup }.sorted { $0.displayName < $1.displayName }
        return VStack(alignment: .leading, spacing: 10) {
            Text("This is…").labelStyle()
            if candidates.isEmpty {
                Text("Name this face, or add people in Family Circle to match it to someone.").font(.sans(13)).foregroundStyle(Theme.inkMute)
            } else {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 6) {
                        ForEach(candidates, id: \.objectID) { p in
                            Button { store.merge(person, into: p); onBack() } label: {
                                HStack(spacing: 8) { AvatarView(person: p.summary, size: 24); Text(p.displayName).font(.sans(13, .medium)).foregroundStyle(Theme.ink) }
                                    .padding(.leading, 8).padding(.trailing, 12).frame(height: 36)
                                    .background(.white.opacity(0.55), in: Capsule()).overlay(Capsule().strokeBorder(Theme.hair2, lineWidth: 0.5))
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }
        }
    }
}
