import CoreData
import SwiftUI

/// Stories: this week's reel and the hand-cut films.
struct StoriesView: View {
    @ObservedObject var room: Room
    var wide = false

    @EnvironmentObject private var store: RoomStore
    @EnvironmentObject private var app: AppState
    @FetchRequest private var stories: FetchedResults<Story>
    @State private var confirmDelete: Story?

    init(room: Room, wide: Bool = false) {
        self.room = room; self.wide = wide
        _stories = FetchRequest(sortDescriptors: [NSSortDescriptor(key: "createdAt", ascending: false)], predicate: NSPredicate(format: "room == %@", room))
    }

    private var weekStart: Date { Catalog.weekStart() }
    private var weeklyReel: Story? { stories.first { $0.isWeeklyReel && $0.weekStart == weekStart } }
    private var handCut: [Story] { stories.filter { !$0.isWeeklyReel } }
    private var newThisWeek: [MediaItem] { (room.media ?? []).filter { ($0.addedAt ?? .distantPast) >= weekStart } }
    private var latestMoment: Moment? { room.momentList.last }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            PageHead(kicker: "Stories", size: wide ? 72 : 50, trailing: AnyView(FRButton("New story", mark: "✦") { app.composeMoment = latestMoment }.disabled(latestMoment == nil))) {
                (Text("The family ") + Text("theater").italic())
            }
            featured
            HStack { Text("Hand-cut · \(handCut.count)").labelStyle(); Spacer() }.padding(.top, 30)
            if handCut.isEmpty {
                Text("Open a moment and choose Make a story. Clips are picked and stitched on this \(DeviceWord.current).").font(.sans(13)).lineSpacing(3).foregroundStyle(Theme.inkMute).padding(.top, 12)
            } else {
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 14), count: wide ? 4 : 2), spacing: 18) {
                    ForEach(Array(handCut.enumerated()), id: \.element.objectID) { i, story in
                        filmCard(story).entrance(i)
                    }
                }
                .padding(.top, 12)
            }
        }
        .padding(.horizontal, wide ? 36 : 20).padding(.top, wide ? 26 : 10).padding(.bottom, wide ? 48 : 118)
        .confirmationDialog("Delete this story?", isPresented: Binding(get: { confirmDelete != nil }, set: { if !$0 { confirmDelete = nil } }), titleVisibility: .visible) {
            Button("Delete story", role: .destructive) { if let s = confirmDelete { store.delete(s) }; confirmDelete = nil }
        }
    }

    private func filmCard(_ story: Story) -> some View {
        let fresh = (story.createdAt ?? .distantPast) > Date().addingTimeInterval(-3600)
        return Button { app.playingStory = story } label: {
            VStack(alignment: .leading, spacing: 10) {
                FilmFrame(poster: story.thumbnail, duration: story.duration, isNew: fresh, playSize: 40)
                VStack(alignment: .leading, spacing: 2) {
                    Text(story.displayTitle).font(.sans(13.5, .semibold)).foregroundStyle(Theme.ink).lineLimit(1)
                    Text(story.subtitle ?? "").font(.sans(11.5)).foregroundStyle(Theme.inkMute).lineLimit(1)
                }
            }
        }
        .buttonStyle(PressScale())
        .contextMenu { Button("Delete story", role: .destructive) { confirmDelete = story } }
    }

    @ViewBuilder
    private var featured: some View {
        let layout = wide ? AnyLayout(HStackLayout(alignment: .bottom, spacing: 30)) : AnyLayout(VStackLayout(alignment: .leading, spacing: 16))
        layout {
            if let reel = weeklyReel {
                Button { app.playingStory = reel } label: { FilmFrame(poster: reel.thumbnail, duration: reel.duration) }.buttonStyle(PressScale()).frame(maxWidth: .infinity)
            } else {
                ZStack {
                    Theme.filmBed
                    if let cover = newThisWeek.last { MediaThumb(item: cover, maxSide: 800, showVideoBadge: false).saturation(0.6).brightness(-0.3).breathing() }
                    VStack { Rectangle().fill(Theme.filmBar).frame(height: 10); Spacer(); Rectangle().fill(Theme.filmBar).frame(height: 10) }
                    Text(newThisWeek.isEmpty ? "Nothing new yet" : "Not stitched yet").font(.display(26)).foregroundStyle(Theme.paper2.opacity(0.85))
                }
                .aspectRatio(16 / 9, contentMode: .fit).clipShape(RoundedRectangle(cornerRadius: 3)).frame(maxWidth: .infinity)
            }
            VStack(alignment: .leading, spacing: 0) {
                Text("Weekly reel · \(Catalog.weekLabel(weekStart))").labelStyle(warm: true)
                Text("This week in \(room.displayName)").font(.display(wide ? 50 : 40)).foregroundStyle(Theme.ink).lineSpacing(-6).padding(.top, 8)
                Text(weeklyReel != nil
                     ? "Stitched from \(newThisWeek.count) new \(newThisWeek.count == 1 ? "item" : "items"). Stays in your family's iCloud."
                     : newThisWeek.isEmpty ? "Add photos this week and stitch them into a short reel." : "\(newThisWeek.count) new \(newThisWeek.count == 1 ? "item" : "items") this week, ready to stitch on this \(DeviceWord.current).")
                    .font(.sans(14)).lineSpacing(4).foregroundStyle(Theme.inkSoft).padding(.top, 10)
                if weeklyReel == nil, !newThisWeek.isEmpty {
                    FRButton("Stitch this week", mark: "✦", style: .ghost, size: .small) { app.composeWeekly = true }.padding(.top, 14)
                }
            }
            .frame(maxWidth: wide ? 360 : .infinity, alignment: .leading)
        }
    }
}
