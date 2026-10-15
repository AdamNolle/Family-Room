import XCTest
@testable import FamilyRoom

final class MenuBarDigestTests: XCTestCase {
  func testDigestUsesRecentContributionsAcrossRoomsAndSkipsLegacyOrDeletedAssets() {
    let now: Int64 = 1_800_000_000
    let firstRoom = room(
      id: "room-one",
      name: "The Nolle Family",
      assets: [
        asset("new-photo", contributedAt: now - 60),
        asset("older-photo", contributedAt: now - 5 * 86_400),
        asset("stale-photo", contributedAt: now - 8 * 86_400),
        asset("legacy-photo", contributedAt: nil),
        asset("deleted-photo", contributedAt: now - 30, deleted: true),
        asset("recent-audio", kind: "audio", contributedAt: now - 20),
      ])
    let secondRoom = room(
      id: "room-two",
      name: "Friends",
      assets: [asset("new-video", kind: "video", contributedAt: now - 10)])
    let removedRoom = room(
      id: "room-removed",
      name: "Former Room",
      role: "removed",
      assets: [asset("removed-room-photo", contributedAt: now - 5)])

    let digest = recentMenuBarContributions(
      rooms: [firstRoom, secondRoom, removedRoom], now: now)

    XCTAssertEqual(digest.map(\.assetId), ["new-video", "new-photo", "older-photo"])
    XCTAssertEqual(digest.map(\.roomName), ["Friends", "The Nolle Family", "The Nolle Family"])
    XCTAssertEqual(digest.first?.kind, "video")
  }

  func testDigestAppliesGlobalLimitAfterSortingRoomContributions() {
    let now: Int64 = 1_800_000_000
    let assets = (0..<8).map { index in
      asset("asset-\(index)", contributedAt: now - Int64(index * 60))
    }

    let digest = recentMenuBarContributions(rooms: [room(id: "room", name: "Family", assets: assets)], now: now, limit: 3)

    XCTAssertEqual(digest.map(\.assetId), ["asset-0", "asset-1", "asset-2"])
  }

  private func room(
    id: String,
    name: String,
    role: String = "owner",
    assets: [LibraryAsset]
  ) -> LibraryRoom {
    LibraryRoom(
      id: id,
      name: name,
      ownerId: "owner",
      role: role,
      assets: assets,
      albums: [],
      moments: [],
      people: [],
      stories: [],
      comments: [],
      members: [])
  }

  private func asset(
    _ id: String,
    kind: String = "photo",
    contributedAt: Int64?,
    deleted: Bool = false
  ) -> LibraryAsset {
    LibraryAsset(
      id: id,
      roomId: "room",
      filename: "\(id).jpg",
      kind: kind,
      sha256: "digest",
      size: 1,
      capturedAt: 1,
      contributor: "device",
      favorite: false,
      caption: "",
      deleted: deleted,
      people: [],
      local: true,
      verifiedCopies: 1,
      availability: "Saved on this device",
      contributedAt: contributedAt)
  }
}
