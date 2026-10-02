import SwiftUI

struct HearthLibraryView: View {
    @EnvironmentObject var store: LibraryStore
    var onManageStorage: () -> Void
    @ScaledMetric(relativeTo: .largeTitle) private var headingSize = 38
    @State private var creatingMoment = false
    private var room: LibraryRoom? { store.room }
    private var featured: LibraryMoment? { room?.moments.filter(\.featured).sorted { $0.start > $1.start }.first }
    private var cover: LibraryAsset? {
        if let featured, let photo = room?.visibleAssets.first(where: { featured.assetIds.contains($0.id) && $0.kind != "audio" && $0.kind != "file" }) { return photo }
        return room?.visibleAssets.first(where: { $0.favorite && ($0.kind == "photo" || $0.kind == "video") }) ?? room?.visibleAssets.first(where: { $0.kind == "photo" || $0.kind == "video" })
    }
    private var onThisDay: [LibraryAsset] {
        let calendar = Calendar.current, today = Date()
        return (room?.visibleAssets ?? []).filter { calendar.component(.month, from: $0.date) == calendar.component(.month, from: today) && calendar.component(.day, from: $0.date) == calendar.component(.day, from: today) && calendar.component(.year, from: $0.date) < calendar.component(.year, from: today) }
    }
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 30) {
                VStack(alignment: .leading, spacing: 10) {
                    RoomEyebrow(text: "A little closer, every day")
                    Text(room?.name ?? "Family Room").font(.system(size: headingSize, weight: .medium, design: .serif)).tracking(-0.6)
                    HStack(spacing: 18) {
                        Label("\(room?.visibleAssets.count ?? 0) memories", systemImage: "photo.on.rectangle")
                        Label("Private Room", systemImage: "lock")
                    }.font(.caption).foregroundStyle(CozyTheme.secondary)
                }
                if room?.visibleAssets.isEmpty == true {
                    VStack(alignment: .leading, spacing: 18) {
                        Image(systemName: "photo.on.rectangle.angled").font(.system(size: 42)).foregroundStyle(CozyTheme.accentInk)
                        Text("Your story starts here.").font(.system(.title, design: .serif))
                        Text("Use Add to bring in your first photos and videos. Connect storage to share them with the people in this Room.").foregroundStyle(CozyTheme.secondary).frame(maxWidth: 500, alignment: .leading)
                        Button("Set up Room storage", action: onManageStorage).buttonStyle(CozyPrimaryButtonStyle())
                    }.padding(30).frame(maxWidth: .infinity, alignment: .leading).cozyPanel()
                }
                if let asset = cover {
                    NavigationLink { AssetDetailView(assetId: asset.id, roomId: asset.roomId) } label: {
                        VStack(alignment: .leading, spacing: 0) {
                            AssetPreview(asset: asset).aspectRatio(16 / 9, contentMode: .fit).clipped()
                            HStack(alignment: .top) {
                                VStack(alignment: .leading, spacing: 7) {
                                    RoomEyebrow(text: featured == nil ? "From your Room" : "Featured Moment")
                                    Text(featured?.title ?? (asset.caption.isEmpty ? "A memory to revisit" : asset.caption)).font(.system(.title2, design: .serif)).foregroundStyle(CozyTheme.ink).lineLimit(2)
                                    Text(asset.date, style: .date).font(.caption).foregroundStyle(CozyTheme.secondary)
                                }
                                Spacer()
                                Image(systemName: asset.kind == "video" ? "play.circle" : "arrow.up.right").font(.title2).foregroundStyle(CozyTheme.accentInk)
                            }.padding(20)
                        }.cozyPanel().clipShape(RoundedRectangle(cornerRadius: CozyTheme.radius))
                    }.buttonStyle(.plain)
                }
                if !(room?.moments.isEmpty ?? true) || room?.canContribute == true {
                    HStack { RoomSectionHeading(title: "Moments", detail: "\(room?.moments.count ?? 0)"); if room?.canContribute == true { Button("Create Moment") { creatingMoment = true } } }
                    ForEach((room?.moments ?? []).sorted { $0.featured != $1.featured ? $0.featured : $0.start > $1.start }) { moment in
                        NavigationLink { ScrollView { MediaGrid(assets: (room?.visibleAssets ?? []).filter { moment.assetIds.contains($0.id) }).padding(24) }.navigationTitle(moment.title) } label: {
                            HStack(spacing: 14) {
                                Image(systemName: moment.featured ? "star" : "calendar").foregroundStyle(CozyTheme.accentInk).frame(width: 24)
                                VStack(alignment: .leading, spacing: 5) { Text(moment.title).font(.headline).foregroundStyle(CozyTheme.ink); Text(Date(timeIntervalSince1970: Double(moment.start)), style: .date).font(.caption).foregroundStyle(CozyTheme.secondary) }
                                Spacer()
                                Text("\(moment.assetIds.count) memories").font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary)
                                Image(systemName: "chevron.right").font(.caption).foregroundStyle(CozyTheme.secondary)
                            }.padding(.vertical, 12)
                        }.buttonStyle(.plain)
                        Divider()
                    }
                }
                let recent = Array((room?.visibleAssets ?? []).filter { $0.id != cover?.id }.prefix(12))
                if !recent.isEmpty { RoomSectionHeading(title: "Recently added", detail: "\(recent.count) memories"); MediaGrid(assets: recent) }
                if !onThisDay.isEmpty { RoomSectionHeading(title: "On this day"); MediaGrid(assets: Array(onThisDay.prefix(8))) }
                if let last = store.snapshot.transfers.last, last.state != "complete" && last.state != "cancelled" {
                    Label(last.error ?? "Your memories are on their way", systemImage: "arrow.triangle.2.circlepath").font(.callout).foregroundStyle(CozyTheme.secondary).padding(16).frame(maxWidth: .infinity, alignment: .leading).cozyPanel(glass: true)
                }
            }.padding(24).frame(maxWidth: 1100, alignment: .leading).frame(maxWidth: .infinity)
        }.background(CozyTheme.canvas)
        .sheet(isPresented: $creatingMoment) { MomentEditorView() }
    }
}
