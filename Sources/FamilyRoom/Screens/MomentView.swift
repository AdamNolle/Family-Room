import SwiftUI

/// A single moment: title block, the light table of prints, and what everyone said.
struct MomentView: View {
    @ObservedObject var moment: Moment
    var hideBack = false
    var wide = false
    var onBack: () -> Void = {}

    @EnvironmentObject private var store: RoomStore
    @EnvironmentObject private var app: AppState
    @State private var editing = false
    @State private var title = ""
    @State private var place = ""
    @State private var quote = ""
    @State private var noteText = ""
    @State private var viewing: MediaItem?
    @State private var confirmDelete = false
    @State private var tagging = false

    private var s: MomentSummary { moment.summary }

    var body: some View {
        VStack(alignment: .leading, spacing: 26) {
            if wide {
                HStack(alignment: .top, spacing: 40) {
                    VStack(alignment: .leading, spacing: 26) { textBlock; notes }.frame(maxWidth: .infinity)
                    table.frame(maxWidth: .infinity)
                }
            } else {
                textBlock; table; notes
            }
        }
        .padding(.horizontal, wide ? 36 : 20).padding(.top, wide ? 26 : 10).padding(.bottom, wide ? 48 : 118)
        .sheet(item: $viewing) { item in MediaViewer(item: item) }
        .sheet(isPresented: $editing) { editSheet }
        .confirmationDialog("Delete this moment and its \(s.mediaCount) items?", isPresented: $confirmDelete, titleVisibility: .visible) {
            Button("Delete moment", role: .destructive) { store.delete(moment); onBack() }
        }
    }

    private var textBlock: some View {
        VStack(alignment: .leading, spacing: 0) {
            if !hideBack { BackButton(title: "The Hearth", action: onBack).padding(.bottom, 16) }
            Text([Catalog.dayLabel(s.date) + " " + String(Calendar.current.component(.year, from: s.date)), s.place ?? ""].filter { !$0.isEmpty }.joined(separator: " · "))
                .labelStyle(warm: true).monospacedDigit()
            Text(s.title).font(.display(wide ? 78 : 58)).foregroundStyle(Theme.ink).lineSpacing(-8).padding(.top, 12)
            if let quote = s.quote { Text(quote).font(.display(20)).italic().foregroundStyle(Theme.inkSoft).lineSpacing(2).padding(.top, 12) }
            who.padding(.top, 22)
            stats.padding(.top, 20)
            HStack(spacing: 8) {
                FRButton("Make a story", mark: "✦") { app.composeMoment = moment }
                FRButton(moment.liked(by: store.identity.authorID) ? "♥ \(moment.likeCount)" : "♡ \(moment.likeCount)", style: .ghost) { store.toggleLike(moment); Feedback.tick() }
                    .accessibilityLabel(moment.liked(by: store.identity.authorID) ? "Unlike" : "Like")
                Menu {
                    Button("Edit title and place") { title = moment.title ?? ""; place = moment.place ?? ""; quote = moment.quote ?? ""; editing = true }
                    Button(moment.isFeatured ? "Pinned to the Hearth" : "Pin to the Hearth") { store.setFeatured(moment) }.disabled(moment.isFeatured)
                    Button("Add photos here") { app.showImporter = true }
                    Divider()
                    Button("Delete moment", role: .destructive) { confirmDelete = true }
                } label: {
                    Image(systemName: "ellipsis").font(.system(size: 14, weight: .semibold)).foregroundStyle(Theme.ink)
                        .frame(width: 34, height: 34).background(.white.opacity(0.55), in: RoundedRectangle(cornerRadius: 9, style: .continuous)).hairlineBorder(9)
                }
                .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
                .accessibilityLabel("More")
            }
            .padding(.top, 24)
        }
    }

    private var who: some View {
        let all = (moment.room?.people ?? []).filter { !$0.isFaceGroup }
        return HStack(spacing: 14) {
            if !s.people.isEmpty { AvatarStack(people: s.people, size: 26) }
            Text(s.people.isEmpty ? "No one tagged yet" : (s.people.count == all.count && all.count > 1 ? "Everyone was here" : Catalog.listJoin(s.people.map(\.name))))
                .font(.sans(13)).foregroundStyle(Theme.inkSoft)
            Spacer()
            Menu {
                ForEach(all.sorted { $0.displayName < $1.displayName }) { p in
                    Button { store.toggle(p, in: moment) } label: {
                        Label(p.displayName, systemImage: moment.people?.contains(p) == true ? "checkmark" : "")
                    }
                }
                if all.isEmpty { Text("Add people in Family Circle first") }
            } label: { Text("Tag people").font(.sans(12.5, .semibold)).foregroundStyle(Theme.warm) }
            .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
        }
        .padding(.vertical, 14)
        .overlay(alignment: .top) { Hairline() }.overlay(alignment: .bottom) { Hairline() }
    }

    private var stats: some View {
        HStack(spacing: 0) {
            stat("\(s.photoCount)", "Photos")
            stat("\(s.videoCount)", "Videos").padding(.leading, 16).overlay(alignment: .leading) { Rectangle().fill(Theme.hair).frame(width: 0.5) }
            stat("\(s.people.count)", "People").padding(.leading, 16).overlay(alignment: .leading) { Rectangle().fill(Theme.hair).frame(width: 0.5) }
        }
    }

    private func stat(_ value: String, _ label: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(value).font(.display(40)).foregroundStyle(Theme.ink).monospacedDigit()
            Text(label).labelStyle()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    // MARK: Light table

    private var table: some View {
        let items = moment.mediaList
        let columns = wide ? 3 : 2
        return LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 16), count: columns), spacing: 18) {
            ForEach(Array(items.enumerated()), id: \.element.objectID) { i, item in
                MattedPrint(item: item, tilt: Double((i * 37) % 7) - 3) { viewing = item }
                    .contextMenu {
                        Button(item.isFavorite ? "Remove favorite" : "Favorite") { store.toggleFavorite(item) }
                        Button("Delete", role: .destructive) { store.delete(item) }
                    }
            }
            Button { app.showImporter = true } label: {
                RoundedRectangle(cornerRadius: 3).strokeBorder(Theme.hair2, style: StrokeStyle(lineWidth: 1, dash: [4, 4]))
                    .aspectRatio(1, contentMode: .fit)
                    .overlay(VStack(spacing: 6) { Image(systemName: "plus").font(.system(size: 18, weight: .medium)); Text("Add").font(.sans(12, .semibold)) }.foregroundStyle(Theme.inkMute))
            }
            .buttonStyle(PressScale())
            .padding(6).padding(.bottom, 10)
        }
        .padding(.horizontal, 6).padding(.vertical, 4)
    }

    // MARK: Notes

    private var notes: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("What everyone said").font(.display(28)).foregroundStyle(Theme.ink).padding(.bottom, 10)
            ForEach(moment.noteList, id: \.objectID) { note in
                HStack(alignment: .top, spacing: 12) {
                    NameAvatar(name: note.authorName ?? "?", size: 28)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(note.authorName ?? "Someone").font(.sans(12.5, .semibold)).foregroundStyle(Theme.ink)
                        Text(note.text ?? "").font(.sans(13.5)).foregroundStyle(Theme.inkSoft)
                    }
                    Spacer()
                    Text((note.createdAt ?? Date()).formatted(date: .omitted, time: .shortened)).font(.num(11)).foregroundStyle(Theme.inkMute)
                }
                .padding(.vertical, 12).overlay(alignment: .top) { Hairline() }
                .contextMenu { if note.authorID == store.identity.authorID { Button("Delete note", role: .destructive) { store.delete(note) } } }
            }
            HStack(spacing: 8) {
                TextField("Say something about this day", text: $noteText, axis: .vertical)
                    .textFieldStyle(.plain).font(.sans(13.5)).foregroundStyle(Theme.ink).lineLimit(1...4)
                    .padding(.horizontal, 12).padding(.vertical, 10)
                    .background(.white.opacity(0.7), in: RoundedRectangle(cornerRadius: 9, style: .continuous)).hairlineBorder(9)
                FRButton("Post", size: .small, disabled: noteText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty) {
                    store.addNote(noteText, to: moment); noteText = ""
                }
            }
            .padding(.top, 12).overlay(alignment: .top) { Hairline() }
        }
    }

    private var editSheet: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack { Text("Edit moment").font(.display(34)).foregroundStyle(Theme.ink); Spacer(); CloseButton { editing = false } }
            FieldBox(label: "Title", text: $title, placeholder: Catalog.defaultMomentTitle(moment.date ?? Date()))
            FieldBox(label: "Place", text: $place, placeholder: "Where was this?")
            VStack(alignment: .leading, spacing: 6) {
                Text("A line to remember").labelStyle()
                TextField("Something someone said, or how it felt", text: $quote, axis: .vertical)
                    .textFieldStyle(.plain).font(.sans(15)).lineLimit(2...5).foregroundStyle(Theme.ink)
                    .padding(12).background(.white, in: RoundedRectangle(cornerRadius: 9, style: .continuous)).hairlineBorder(9)
            }
            HStack { Spacer(); FRButton("Cancel", style: .ghost) { editing = false }; FRButton("Save") { store.update(moment, title: title, place: place, quote: quote); editing = false } }
        }
        .padding(24).frame(minWidth: 320, idealWidth: 460)
        .background(Theme.paper2)
        .presentationDetents([.medium, .large])
    }
}

/// A tilted matted print on the light table (`.mat`).
struct MattedPrint: View {
    @ObservedObject var item: MediaItem
    var tilt: Double
    var action: () -> Void
    @State private var hover = false
    var body: some View {
        Button(action: action) {
            MediaThumb(item: item, maxSide: 600)
                .aspectRatio(1, contentMode: .fit)
                .overlay(alignment: .topTrailing) {
                    if item.isFavorite { Image(systemName: "heart.fill").font(.system(size: 11)).foregroundStyle(.white).padding(6).shadow(radius: 2) }
                }
                .padding(.horizontal, 6).padding(.top, 6).padding(.bottom, 16)
                .background(Color(hex: 0xFBF8F2), in: RoundedRectangle(cornerRadius: 3))
                .shadow(color: Theme.shadowInk.opacity(0.12), radius: 1, y: 1)
                .shadow(color: Theme.shadowInk.opacity(hover ? 0.16 : 0.08), radius: hover ? 17 : 9, y: hover ? 10 : 6)
                .rotationEffect(.degrees(hover ? 0 : tilt))
                .scaleEffect(hover ? 1.03 : 1)
                .offset(y: hover ? -5 : 0)
                .animation(Theme.ease, value: hover)
        }
        .buttonStyle(PressScale())
        .onHoverIfAvailable { hover = $0 }
        .accessibilityLabel(item.isVideo ? "Video" : "Photo")
    }
}
