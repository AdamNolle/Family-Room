#if os(macOS)
import SwiftUI

struct MenuBarDigestView: View {
  @EnvironmentObject private var store: LibraryStore
  @Environment(\.openWindow) private var openWindow

  private var contributions: [MenuBarContribution] {
    recentMenuBarContributions(
      rooms: store.snapshot.rooms,
      now: Int64(Date().timeIntervalSince1970))
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      HStack(alignment: .firstTextBaseline) {
        VStack(alignment: .leading, spacing: 3) {
          RoomEyebrow(text: "A little from your Rooms")
          Text("Recent memories")
            .font(RoomArt.display(27, relativeTo: .title2))
            .foregroundStyle(CozyTheme.ink)
        }
        Spacer()
        Text("7 DAYS")
          .font(.caption2.monospaced().weight(.semibold))
          .foregroundStyle(CozyTheme.secondary)
      }

      if store.loading {
        ProgressView("Opening your private library…")
          .frame(maxWidth: .infinity, minHeight: 90)
      } else if store.snapshot.rooms.isEmpty {
        ContentUnavailableView(
          "Open Family Room",
          systemImage: "lock",
          description: Text("Open the app to unlock and check your private library."))
          .frame(maxWidth: .infinity, minHeight: 150)
      } else if contributions.isEmpty {
        ContentUnavailableView(
          "No recent additions recorded",
          systemImage: "photo.on.rectangle.angled",
          description: Text(
            "New photos and videos shared to your Rooms appear here. Older items without a contribution time are omitted."))
          .frame(maxWidth: .infinity, minHeight: 150)
      } else {
        VStack(spacing: 8) {
          ForEach(contributions) { contribution in
            Button {
              store.requestOpen(contribution)
              openWindow(id: "main")
            } label: {
              contributionRow(contribution)
            }
            .buttonStyle(.plain)
            .accessibilityLabel(
              "Open \(contribution.title), added to \(contribution.roomName) \(contribution.date.formatted(date: .abbreviated, time: .shortened))")
            .accessibilityIdentifier("menuBar.contribution.\(contribution.assetId)")
          }
        }
      }

      Button {
        openWindow(id: "main")
      } label: {
        Label("Open Family Room", systemImage: "arrow.up.right.square")
          .frame(maxWidth: .infinity, alignment: .leading)
      }
      .buttonStyle(.borderedProminent)
      .tint(CozyTheme.accent)
      .accessibilityIdentifier("menuBar.openLibrary")
    }
    .padding(18)
    .frame(width: 360)
    .background(CozyTheme.canvas)
  }

  @ViewBuilder private func contributionRow(_ contribution: MenuBarContribution) -> some View {
    HStack(spacing: 12) {
      if let asset = store.snapshot.rooms.first(where: { $0.id == contribution.roomId })?.assets
        .first(where: { $0.id == contribution.assetId })
      {
        AssetPreview(asset: asset)
          .frame(width: 52, height: 52)
          .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
          .overlay {
            RoundedRectangle(cornerRadius: 10, style: .continuous)
              .strokeBorder(.black.opacity(0.13), lineWidth: 1)
          }
      } else {
        Image(systemName: contribution.kind == "video" ? "play.rectangle" : "photo")
          .frame(width: 52, height: 52)
          .background(CozyTheme.photoBed, in: RoundedRectangle(cornerRadius: 10))
      }

      VStack(alignment: .leading, spacing: 4) {
        Text(contribution.title)
          .font(.callout.weight(.semibold))
          .foregroundStyle(CozyTheme.ink)
          .lineLimit(1)
        Text(contribution.roomName)
          .font(.caption)
          .foregroundStyle(CozyTheme.secondary)
          .lineLimit(1)
        Text(contribution.date, style: .relative)
          .font(.caption2)
          .foregroundStyle(CozyTheme.secondary)
      }
      Spacer(minLength: 4)
      Image(systemName: "chevron.right")
        .font(.caption.weight(.semibold))
        .foregroundStyle(CozyTheme.secondary)
    }
    .padding(8)
    .background(CozyTheme.surface.opacity(0.78), in: RoundedRectangle(cornerRadius: 13))
    .contentShape(RoundedRectangle(cornerRadius: 13))
  }
}
#endif
