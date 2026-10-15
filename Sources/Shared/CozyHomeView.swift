import SwiftUI

struct HearthLibraryView: View {
  @EnvironmentObject var store: LibraryStore
  var onManageStorage: () -> Void
  @State private var creatingMoment = false
  @State private var momentSuggestions: [MomentSuggestion] = []
  @State private var selectedMomentSuggestion: MomentSuggestion?
  private var room: LibraryRoom? { store.room }
  private var featured: LibraryMoment? {
    room?.moments.filter(\.featured).sorted { $0.start > $1.start }.first
  }
  private var cover: LibraryAsset? {
    if let featured,
      let photo = room?.visibleAssets.first(where: {
        featured.assetIds.contains($0.id) && $0.kind != "audio" && $0.kind != "file"
      })
    {
      return photo
    }
    return room?.visibleAssets.first(where: {
      $0.favorite && ($0.kind == "photo" || $0.kind == "video")
    }) ?? room?.visibleAssets.first(where: { $0.kind == "photo" || $0.kind == "video" })
  }
  private var highlights: [LibraryAsset] {
    guard let cover else { return [] }
    let companions = (room?.visibleAssets ?? []).filter { asset in
      asset.id != cover.id && ["photo", "video"].contains(asset.kind)
        && (featured.map { $0.assetIds.contains(asset.id) } ?? true)
    }
    return [cover] + Array(companions.prefix(4))
  }
  private var onThisDay: [LibraryAsset] {
    let ids = Set(room?.onThisDayAssetIds ?? [])
    return (room?.visibleAssets ?? []).filter { ids.contains($0.id) }
  }
  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 30) {
        VStack(alignment: .leading, spacing: 12) {
          RoomEyebrow(text: "Your people. Your memories.")
          Text(room?.name ?? "Family Room").font(RoomArt.display(40)).foregroundStyle(CozyTheme.ink)
            .fixedSize(horizontal: false, vertical: true)
          HStack(spacing: 18) {
            Label(
              "\(room?.visibleAssets.count ?? 0) \(room?.visibleAssets.count == 1 ? "memory" : "memories")",
              systemImage: "photo.on.rectangle")
            Label("Private Room", systemImage: "lock")
          }.font(.caption).foregroundStyle(CozyTheme.secondary)
        }.padding(24).frame(maxWidth: .infinity, alignment: .leading).memoryGlass()
        if room?.visibleAssets.isEmpty == true {
          VStack(alignment: .leading, spacing: 18) {
            MemoryIconTile(symbol: "photo.on.rectangle.angled", size: 68)
            Text("Your story starts here.").font(RoomArt.display(38, relativeTo: .title))
            Text(
              "Use Add to bring in your first photos and videos. Connect storage to share them with the people in this Room."
            ).foregroundStyle(CozyTheme.secondary).frame(maxWidth: 500, alignment: .leading)
            Button("Set up Room storage", action: onManageStorage).buttonStyle(
              CozyPrimaryButtonStyle())
          }.padding(30).frame(maxWidth: .infinity, alignment: .leading).cozyPanel()
        }
        if let asset = cover {
          PhotoTray(
            title: featured?.title ?? "In the spotlight",
            subtitle: asset.date.formatted(date: .abbreviated, time: .omitted)
          ) {
            MemoryHighlights(assets: highlights)
          }
        }
        if room?.canContribute == true && !momentSuggestions.isEmpty {
          VStack(alignment: .leading, spacing: 12) {
            HStack {
              RoomSectionHeading(title: "Possible Moments", detail: "Review before saving")
              Spacer()
              Image(systemName: "sparkles.rectangle.stack")
                .foregroundStyle(CozyTheme.accentInk)
            }
            Text("Grouped by capture time. Check the selection before adding a Moment.")
              .font(.callout)
              .foregroundStyle(CozyTheme.secondary)
            ForEach(momentSuggestions.prefix(3)) { suggestion in
              Button {
                selectedMomentSuggestion = suggestion
                creatingMoment = true
              } label: {
                HStack(spacing: 14) {
                  Image(systemName: "calendar.badge.clock")
                    .foregroundStyle(CozyTheme.accentInk)
                    .frame(width: 26)
                  VStack(alignment: .leading, spacing: 4) {
                    Text(suggestion.date, style: .date)
                      .font(.headline)
                      .foregroundStyle(CozyTheme.ink)
                    Text("\(suggestion.assetIds.count) photos and videos")
                      .font(.caption)
                      .foregroundStyle(CozyTheme.secondary)
                  }
                  Spacer()
                  Label("Review", systemImage: "arrow.right")
                    .font(.callout.weight(.semibold))
                    .foregroundStyle(CozyTheme.accentInk)
                }
                .padding(.vertical, 10)
                .contentShape(Rectangle())
              }
              .buttonStyle(.plain)
              Divider()
            }
          }
          .padding(20)
          .cozyPanel(glass: true)
        }
        if let room, !room.moments.isEmpty || room.canContribute {
          MomentRiverSection(room: room) {
            selectedMomentSuggestion = nil
            creatingMoment = true
          }
          .id(room.id)
        }
        let highlightedIds = Set(highlights.map(\.id))
        let recent = Array(
          (room?.visibleAssets ?? []).filter { !highlightedIds.contains($0.id) }.prefix(12))
        if !recent.isEmpty {
          PhotoTray(title: "Recently added", subtitle: "\(recent.count) memories") {
            MediaGrid(assets: recent)
          }
        }
        if !onThisDay.isEmpty {
          PhotoTray(title: "On this day", subtitle: "From earlier years") {
            MediaGrid(assets: Array(onThisDay.prefix(8)))
          }
        }
        if let last = store.snapshot.transfers.last,
          last.state != "complete" && last.state != "cancelled"
        {
          Label(
            last.error ?? "Your memories are on their way",
            systemImage: "arrow.triangle.2.circlepath"
          ).font(.callout).foregroundStyle(CozyTheme.secondary).padding(16).frame(
            maxWidth: .infinity, alignment: .leading
          ).cozyPanel(glass: true)
        }
      }.padding(24).frame(maxWidth: 1100, alignment: .leading).frame(maxWidth: .infinity)
    }.background(MemoryGalleryBackdrop())
      .task(id: "\(room?.id ?? "")-\(room?.moments.count ?? 0)") {
        await refreshMomentSuggestions()
      }
      .sheet(isPresented: $creatingMoment) {
        MomentEditorView(suggestion: selectedMomentSuggestion)
      }
  }

  @MainActor
  private func refreshMomentSuggestions() async {
    guard let room, room.canContribute else {
      momentSuggestions = []
      return
    }
    do {
      let response = try await store.command(["action": "suggest_moments", "room_id": room.id])
      let value = response["suggestions"] ?? []
      let data = try JSONSerialization.data(withJSONObject: value)
      momentSuggestions = try LibraryCoding.decoder.decode([MomentSuggestion].self, from: data)
    } catch {
      momentSuggestions = []
    }
  }
}

private struct MomentRiverSection: View {
  @EnvironmentObject private var store: LibraryStore
  let room: LibraryRoom
  let onCreate: () -> Void
  @State private var selectedYear: Int?

  private var years: [Int] {
    Array(
      Set(
        room.moments.map { moment in
          Calendar.current.component(
            .year,
            from: Date(timeIntervalSince1970: TimeInterval(moment.start))
          )
        })
    ).sorted(by: >)
  }

  private var visibleMoments: [LibraryMoment] {
    room.moments
      .filter { moment in
        guard let selectedYear else { return true }
        return Calendar.current.component(
          .year,
          from: Date(timeIntervalSince1970: TimeInterval(moment.start))
        ) == selectedYear
      }
      .sorted { $0.start < $1.start }
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      HStack(alignment: .firstTextBaseline) {
        RoomSectionHeading(title: "Moments", detail: "\(room.moments.count)")
        Spacer()
        if room.canContribute {
          Button("Create Moment", systemImage: "plus", action: onCreate)
            .font(.callout.weight(.semibold))
            .foregroundStyle(CozyTheme.accentInk)
        }
      }
      Text("Scrub through your Room’s memories in date order. Larger cards hold more memories.")
        .font(.callout)
        .foregroundStyle(CozyTheme.secondary)

      ScrollView(.horizontal, showsIndicators: false) {
        HStack(spacing: 8) {
          yearChip(title: "All years", year: nil)
          ForEach(years, id: \.self) { year in
            yearChip(title: String(year), year: year)
          }
        }
        .padding(.vertical, 2)
      }
      .accessibilityLabel("Filter Moments by year")

      if visibleMoments.isEmpty {
        Text("No Moments in this year.")
          .font(.callout)
          .foregroundStyle(CozyTheme.secondary)
          .padding(18)
          .frame(maxWidth: .infinity, alignment: .leading)
          .cozyPanel(glass: true)
      } else {
        ScrollView(.horizontal, showsIndicators: false) {
          HStack(alignment: .top, spacing: 14) {
            ForEach(visibleMoments) { moment in
              momentCard(moment)
            }
          }
          .padding(.vertical, 4)
        }
        .accessibilityLabel("Chronological Moment river")
      }
    }
  }

  private func yearChip(title: String, year: Int?) -> some View {
    let selected = selectedYear == year
    return Button {
      selectedYear = year
    } label: {
      Text(title)
        .font(.caption.weight(.semibold).monospacedDigit())
        .foregroundStyle(selected ? CozyTheme.onAccent : CozyTheme.secondary)
        .padding(.horizontal, 13)
        .padding(.vertical, 8)
        .background(selected ? CozyTheme.accent : CozyTheme.surface, in: Capsule())
        .overlay(Capsule().strokeBorder(CozyTheme.line.opacity(0.7), lineWidth: 0.7))
    }
    .buttonStyle(.plain)
    .accessibilityIdentifier("moment.river.year.\(year.map(String.init) ?? "all")")
    .accessibilityAddTraits(selected ? .isSelected : [])
  }

  private func momentCard(_ moment: LibraryMoment) -> some View {
    let count = moment.assetIds.count
    let width = min(360, max(252, 220 + CGFloat(min(count, 18)) * 7))
    let cover = room.visibleAssets.first { moment.assetIds.contains($0.id) }
    let date = Date(timeIntervalSince1970: TimeInterval(moment.start))

    return VStack(alignment: .leading, spacing: 10) {
      HStack(alignment: .center) {
        Text(date.formatted(date: .abbreviated, time: .omitted))
          .font(.caption.monospacedDigit().weight(.medium))
          .foregroundStyle(CozyTheme.secondary)
        Spacer(minLength: 8)
        if room.canContribute {
          Button {
            store.perform {
              _ = try await store.command([
                "action": "save_moment",
                "room_id": room.id,
                "id": moment.id,
                "title": moment.title,
                "asset_ids": moment.assetIds,
                "featured": !moment.featured,
              ])
            }
          } label: {
            Image(systemName: moment.featured ? "star.fill" : "star")
              .symbolRenderingMode(.hierarchical)
              .foregroundStyle(CozyTheme.accentInk)
              .frame(width: 34, height: 34)
              .background(CozyTheme.glassTint.opacity(0.65), in: Circle())
          }
          .buttonStyle(.plain)
          .accessibilityLabel(
            moment.featured
              ? "Remove \(moment.title) from Home spotlight" : "Feature \(moment.title) on Home"
          )
          .accessibilityIdentifier("moment.river.feature.\(moment.id)")
        } else if moment.featured {
          Image(systemName: "star.fill")
            .foregroundStyle(CozyTheme.accentInk)
            .accessibilityLabel("Featured on Home")
        }
      }

      NavigationLink {
        ScrollView {
          MediaGrid(assets: room.visibleAssets.filter { moment.assetIds.contains($0.id) })
            .padding(24)
        }
        .navigationTitle(moment.title)
      } label: {
        VStack(alignment: .leading, spacing: 9) {
          Group {
            if let cover {
              AssetPreview(asset: cover)
                .frame(height: 112)
                .memoryPhotoFrame(radius: 8)
            } else {
              CozyTheme.photoBed
                .overlay(Image(systemName: "photo.stack").foregroundStyle(CozyTheme.secondary))
                .frame(height: 112)
                .clipShape(RoundedRectangle(cornerRadius: 8))
            }
          }
          Text(moment.title)
            .font(RoomArt.display(23, relativeTo: .title3))
            .foregroundStyle(CozyTheme.ink)
            .lineLimit(1)
          HStack {
            Text("\(count) \(count == 1 ? "memory" : "memories")")
            Spacer()
            Image(systemName: "arrow.up.right")
          }
          .font(.caption.weight(.medium))
          .foregroundStyle(CozyTheme.secondary)
        }
        .padding(12)
        .contentShape(RoundedRectangle(cornerRadius: 16))
      }
      .buttonStyle(MemoryPhotoButtonStyle())
      .accessibilityIdentifier("moment.river.card.\(moment.id)")
    }
    .padding(12)
    .frame(width: width, alignment: .leading)
    .cozyPanel(glass: true)
  }
}
