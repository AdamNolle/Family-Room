import SwiftUI

struct FilmLibraryView: View {
    @EnvironmentObject var store: LibraryStore
    @State private var draft: FilmProject?
    @State private var opened: FilmProject?
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 26) {
                VStack(alignment: .leading, spacing: 10) {
                    RoomEyebrow(text: store.room?.name ?? "Theater")
                    Text("Your Room, on film.").font(.system(.largeTitle, design: .serif).weight(.medium))
                    Text("Make something to watch together, and keep the originals just as they were.").foregroundStyle(CozyTheme.secondary)
                }
                if let room = store.room {
                    if room.canContribute {
                        ViewThatFits(in: .horizontal) {
                            HStack(spacing: 12) { creationActions(room).fixedSize(horizontal: true, vertical: false) }
                            VStack(alignment: .leading, spacing: 12) { creationActions(room) }
                        }
                    }
                    if room.latestStories.isEmpty {
                        VStack(alignment: .leading, spacing: 16) {
                            Image(systemName: "film.stack").font(.system(size: 42)).foregroundStyle(CozyTheme.accentInk)
                            Text("The first film is yours to make.").font(.system(.title2, design: .serif))
                            Text("Start with a few photos or clips. Arrange them, add your soundtrack, and publish when you’re ready.").foregroundStyle(CozyTheme.secondary).frame(maxWidth: 480, alignment: .leading)
                        }.padding(26).frame(maxWidth: .infinity, alignment: .leading).cozyPanel()
                    }
                    ForEach(room.latestStories, id: \.versionId) { project in
                        ViewThatFits(in: .horizontal) {
                            HStack(spacing: 22) { cover(project, room: room).frame(width: 185, height: 118); information(project, room: room) }
                            VStack(alignment: .leading, spacing: 16) { cover(project, room: room).aspectRatio(16 / 9, contentMode: .fit); information(project, room: room) }
                        }.padding(16).cozyPanel()
                    }
                }
            }.padding(24).frame(maxWidth: 1050, alignment: .leading).frame(maxWidth: .infinity)
        }.background(CozyTheme.canvas)
            .sheet(item: $draft) { FilmStudioView(project: $0).environmentObject(store) }
            .sheet(item: $opened) { FilmStudioView(project: $0).environmentObject(store) }
    }
    @ViewBuilder private func creationActions(_ room: LibraryRoom) -> some View {
        Button { draft = FilmProject(roomId: room.id) } label: {
            Label("Create a film", systemImage: "plus")
        }.buttonStyle(CozyPrimaryButtonStyle()).accessibilityIdentifier("theater.createFilm")
        Button("Make this week’s reel", systemImage: "sparkles") { makeReel(room) }.buttonStyle(.bordered)
    }
    @ViewBuilder private func cover(_ project: FilmProject, room: LibraryRoom) -> some View {
        if let asset = room.assets.first(where: { $0.id == (project.publishedAsset ?? project.clips.first?.assetId) }) {
            AssetPreview(asset: asset).clipped().clipShape(RoundedRectangle(cornerRadius: 10))
        } else {
            ZStack { CozyTheme.photoBed; Image(systemName: "film").font(.largeTitle).foregroundStyle(CozyTheme.secondary) }.clipShape(RoundedRectangle(cornerRadius: 10))
        }
    }
    private func information(_ project: FilmProject, room: LibraryRoom) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            RoomEyebrow(text: project.isPrivateDraft ? "Private draft" : "Published film")
            Text(project.title).font(.system(.title2, design: .serif)).foregroundStyle(CozyTheme.ink).lineLimit(2)
            Text("\(project.clips.count) clips · \(Int(project.duration)) seconds").font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary)
            HStack {
                if let id = project.publishedAsset { NavigationLink { AssetDetailView(assetId: id, roomId: room.id) } label: { Label("Watch", systemImage: "play") } }
                if room.canContribute && project.supportsEditing { Button("Open studio", systemImage: "slider.horizontal.3") { opened = project } }
            }.buttonStyle(.bordered)
            if !project.supportsEditing { Text("Update Family Room to edit this project format.").font(.caption).foregroundStyle(CozyTheme.secondary) }
        }.frame(maxWidth: .infinity, alignment: .leading)
    }
    private func makeReel(_ room: LibraryRoom) {
        let recent = room.visibleAssets.filter { $0.date > Date().addingTimeInterval(-7 * 86400) && ["photo", "video"].contains($0.kind) }.sorted { $0.date < $1.date }.suffix(20)
        guard !recent.isEmpty else { store.error = "Add photos or videos from this week first."; return }
        var project = FilmProject(roomId: room.id, title: "This week in \(room.name)", autoGenerated: true)
        project.clips = recent.enumerated().map { index, asset in FilmClip(assetId: asset.id, start: Double(index) * 3, duration: 3, fadeIn: 0.3, fadeOut: 0.3) }
        draft = project
    }
}
