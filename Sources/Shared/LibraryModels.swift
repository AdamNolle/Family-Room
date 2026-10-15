import Foundation

func correctedCaptureTimestamp(
  _ timestamp: Int64,
  to targetDate: Date,
  calendar: Calendar = .current
) -> Int64 {
  let original = Date(timeIntervalSince1970: TimeInterval(timestamp))
  let targetDay = calendar.dateComponents([.year, .month, .day], from: targetDate)
  var corrected = calendar.dateComponents([.hour, .minute, .second, .nanosecond], from: original)
  corrected.year = targetDay.year
  corrected.month = targetDay.month
  corrected.day = targetDay.day
  guard let date = calendar.date(from: corrected) else { return timestamp }
  return Int64(date.timeIntervalSince1970.rounded())
}

func canChangeDeletionForSelection(
  role: String,
  currentDeviceID: String,
  contributors: [String]
) -> Bool {
  guard !contributors.isEmpty else { return false }
  if role == "owner" || role == "admin" { return true }
  return role == "contributor" && contributors.allSatisfy { $0 == currentDeviceID }
}

struct LibrarySnapshot: Codable {
  var version: Int
  var deviceId: String
  var rooms: [LibraryRoom]
  var sources: [LibrarySource]
  var watches: [UploadRule]
  var transfers: [LibraryTransfer]
  var pendingInvitations: [LibraryPendingInvitation]? = nil
  static let empty = LibrarySnapshot(
    version: 1, deviceId: "", rooms: [], sources: [], watches: [], transfers: [],
    pendingInvitations: [])
}
struct LibraryPendingInvitation: Codable, Identifiable {
  var id: String
  var roomId: String
  var name: String
  var ownerId: String
  var role: String
  var albumOnly: Bool
  var createdAt: Int64
}
struct LibraryRoom: Codable, Identifiable {
  var id: String
  var name: String
  var ownerId: String
  var role: String
  var albumOnly: Bool? = nil
  var accessRevision: UInt64? = nil
  var keyEpoch: UInt64? = nil
    var rejectedEdits: Int? = nil
    var deviceCacheLimitBytes: UInt64? = nil
    var onThisDayAssetIds: [String]? = nil
  var assets: [LibraryAsset]
  var albums: [LibraryAlbum]
  var playlists: [LibraryPlaylist]? = nil
  var moments: [LibraryMoment]
  var people: [LibraryPerson]
  var stories: [FilmProject]
  var comments: [LibraryComment]
  var reactions: [LibraryReaction]? = nil
  var members: [LibraryMember]
  var visibleAssets: [LibraryAsset] {
    assets.filter { !$0.deleted }.sorted { $0.capturedAt > $1.capturedAt }
  }
  var curatedPlaylists: [LibraryPlaylist] { playlists ?? [] }
  var canContribute: Bool { ["owner", "admin", "contributor"].contains(role) }
  func likeCount(for assetId: String) -> Int {
    reactions?.filter { $0.assetId == assetId && $0.liked }.count ?? 0
  }
  func hasLiked(_ assetId: String, deviceId: String) -> Bool {
    reactions?.contains { $0.assetId == assetId && $0.author == deviceId && $0.liked } ?? false
  }
  var latestStories: [FilmProject] {
    var latest: [String: FilmProject] = [:]
    for project in stories { latest["\(project.id):\(project.isPrivateDraft)"] = project }
    return latest.values.sorted { $0.title < $1.title }
  }
}
struct LibraryMoment: Codable, Identifiable {
  var id: String
  var roomId: String
  var title: String
  var start: Int64
  var end: Int64
  var assetIds: [String]
  var featured: Bool
}

func libraryMomentMatchesAsset(
  assetId: String,
  selectedMomentId: String?,
  moments: [LibraryMoment]
) -> Bool {
  guard let selectedMomentId else { return true }
  return moments.first(where: { $0.id == selectedMomentId })?.assetIds.contains(assetId) == true
}

struct MomentSuggestion: Decodable, Identifiable, Hashable {
  var start: Int64
  var end: Int64
  var assetIds: [String]
  var id: String { assetIds.joined(separator: "|") }

  var date: Date { Date(timeIntervalSince1970: Double(start)) }
  var proposedTitle: String {
    "Moment · \(date.formatted(date: .abbreviated, time: .omitted))"
  }
}

struct LibraryAsset: Codable, Identifiable, Hashable {
  var id: String
  var roomId: String
  var filename: String
  var kind: String
  var sha256: String
  var size: UInt64
  var capturedAt: Int64
  var contributor: String
  var favorite: Bool
  var caption: String
  var deleted: Bool
  var people: [String]
  var components: [String] = []
  var local: Bool
  var verifiedCopies: Int
  var pinnedOffline: Bool? = nil
  var availability: String
  var contributedAt: Int64? = nil
  var date: Date { Date(timeIntervalSince1970: TimeInterval(capturedAt)) }
  var displayTitle: String { caption.isEmpty ? filename : caption }
}

struct MenuBarContribution: Equatable, Identifiable {
  var roomId: String
  var roomName: String
  var assetId: String
  var title: String
  var kind: String
  var contributedAt: Int64

  var id: String { "\(roomId):\(assetId)" }
  var date: Date { Date(timeIntervalSince1970: TimeInterval(contributedAt)) }
}

func recentMenuBarContributions(
  rooms: [LibraryRoom],
  now: Int64,
  lookback: Int64 = 7 * 86_400,
  limit: Int = 5
) -> [MenuBarContribution] {
  guard limit > 0 else { return [] }
  let earliest = now.saturatingSubtracting(max(lookback, 0))
  return rooms.flatMap { room in
    guard room.role != "removed" else { return [MenuBarContribution]() }
    return room.visibleAssets.compactMap { asset -> MenuBarContribution? in
      guard let contributedAt = asset.contributedAt,
        asset.kind == "photo" || asset.kind == "video",
        contributedAt > 0,
        contributedAt >= earliest,
        contributedAt <= now + 300
      else {
        return nil
      }
      return MenuBarContribution(
        roomId: room.id,
        roomName: room.name,
        assetId: asset.id,
        title: asset.displayTitle,
        kind: asset.kind,
        contributedAt: contributedAt)
    }
  }
  .sorted {
    if $0.contributedAt != $1.contributedAt { return $0.contributedAt > $1.contributedAt }
    return $0.id < $1.id
  }
  .prefix(limit)
  .map { $0 }
}

private extension Int64 {
  func saturatingSubtracting(_ amount: Int64) -> Int64 {
    let (value, overflow) = subtractingReportingOverflow(amount)
    return overflow ? Int64.min : value
  }
}

struct LibraryAssetOpenRequest: Equatable, Identifiable {
  var roomId: String
  var assetId: String
  var id: String { "\(roomId):\(assetId)" }
}

struct LibraryAlbum: Codable, Identifiable {
  var id: String
  var roomId: String
  var title: String
  var assetIds: [String]
  var pinned: Bool
  var caption: String? = nil
  var coverAssetId: String? = nil
  var captionText: String { caption ?? "" }
  func cover(in room: LibraryRoom) -> LibraryAsset? {
    if let coverAssetId, assetIds.contains(coverAssetId),
      let chosen = room.visibleAssets.first(where: { $0.id == coverAssetId })
    {
      return chosen
    }
    return assetIds.compactMap { id in room.visibleAssets.first { $0.id == id } }.first
  }
}
struct LibraryPlaylist: Codable, Identifiable {
  var id: String
  var roomId: String
  var title: String
  var storyIds: [String]
}
struct LibraryPerson: Codable, Identifiable {
  var id: String
  var roomId: String
  var name: String
}
struct LibraryMember: Codable, Identifiable {
  var roomId: String
  var deviceId: String
  var role: String
  var signature: String
  var id: String { deviceId }
}
struct LibraryComment: Codable, Identifiable {
  var id: String
  var assetId: String
  var author: String
  var body: String
  var createdAt: Int64
}
struct LibraryReaction: Codable, Identifiable {
  var id: String
  var assetId: String
  var author: String
  var liked: Bool
}
struct LibrarySource: Codable, Identifiable {
  var id: String
  var roomId: String
  var name: String
  var kind: String
  var endpoint: String
  var paused: Bool
  var preferred: Bool
  var overflow: Bool
  var networkPolicy: String
  var budget: UInt64?
}
struct UploadRule: Codable, Identifiable {
  var id: String
  var roomId: String
  var path: String
  var paused: Bool
  var excludedExtensions: [String]
  var seen: [String]
}
struct LibraryTransfer: Codable, Identifiable {
  var id: String
  var roomId: String
  var assetId: String
  var sourceId: String
  var state: String
  var transferred: UInt64
  var total: UInt64
  var error: String?
  var progress: Double { total == 0 ? 0 : Double(transferred) / Double(total) }
}
struct FilmKeyframe: Codable, Hashable {
  var time: Double
  var value: Double
}
struct FilmChapter: Codable, Hashable {
  var title: String
  var time: Double
}
struct FilmClip: Codable, Identifiable, Hashable {
  var id = UUID().uuidString
  var assetId: String
  var track: Int = 0
  var start: Double = 0
  var trimIn: Double = 0
  var duration: Double = 3
  var speed: Double = 1
  var volume: Double = 1
  var opacity: Double = 1
  var exposure: Double = 0
  var saturation: Double = 1
  var title: String = ""
  var fadeIn: Double = 0
  var fadeOut: Double = 0
  // Output seconds already elapsed / still remaining in the original fade.
  // Optional for projects saved before split-fade continuity was introduced.
  var fadeInOffset: Double?
  var fadeOutOffset: Double?
  var opacityKeyframes: [FilmKeyframe] = []
  var volumeKeyframes: [FilmKeyframe] = []
  var end: Double { start + duration / speed }
  func value(at time: Double, keyframes: [FilmKeyframe], fallback: Double) -> Double {
    let frames = keyframes.sorted { $0.time < $1.time }
    guard let first = frames.first else { return fallback }
    if time <= first.time { return first.value }
    for (a, b) in zip(frames, frames.dropFirst()) where time <= b.time {
      if a.time == b.time { return b.value }
      return a.value + (b.value - a.value) * ((time - a.time) / (b.time - a.time))
    }
    return frames.last?.value ?? fallback
  }
  func fadeMultiplier(at outputTime: Double) -> Double {
    var multiplier = 1.0
    if fadeIn > 0 { multiplier *= min(1, max(0, (outputTime + (fadeInOffset ?? 0)) / fadeIn)) }
    if fadeOut > 0 {
      multiplier *= min(1, max(0, (duration / speed - outputTime + (fadeOutOffset ?? 0)) / fadeOut))
    }
    return multiplier
  }
  func opacity(at outputTime: Double) -> Double {
    min(
      1,
      max(
        0,
        value(at: outputTime * speed, keyframes: opacityKeyframes, fallback: opacity)
          * fadeMultiplier(at: outputTime)))
  }
  func audioGain(at outputTime: Double) -> Double {
    min(
      4,
      max(
        0,
        value(at: outputTime * speed, keyframes: volumeKeyframes, fallback: volume)
          * fadeMultiplier(at: outputTime)))
  }
  func resized(to sourceDuration: Double) -> FilmClip {
    guard sourceDuration.isFinite, sourceDuration >= 0.001, sourceDuration <= 86400 else {
      return self
    }
    var result = self
    result.duration = sourceDuration
    if sourceDuration < duration {
      func trim(_ frames: [FilmKeyframe], fallback: Double) -> [FilmKeyframe] {
        guard !frames.isEmpty else { return [] }
        return frames.filter { $0.time < sourceDuration }.sorted { $0.time < $1.time }
          + [
            FilmKeyframe(
              time: sourceDuration,
              value: value(at: sourceDuration, keyframes: frames, fallback: fallback))
          ]
      }
      result.opacityKeyframes = trim(opacityKeyframes, fallback: opacity)
      result.volumeKeyframes = trim(volumeKeyframes, fallback: volume)
    }
    return result
  }
  func split(at sourceTime: Double) -> (left: FilmClip, right: FilmClip)? {
    guard sourceTime.isFinite, duration.isFinite, speed.isFinite, speed > 0,
      sourceTime >= 0.001, duration - sourceTime >= 0.001
    else { return nil }
    var left = self
    var right = self
    left.duration = sourceTime
    right.id = UUID().uuidString
    right.start += sourceTime / speed
    right.trimIn += sourceTime
    right.duration -= sourceTime
    right.fadeInOffset = (fadeInOffset ?? 0) + sourceTime / speed
    left.fadeOutOffset = (fadeOutOffset ?? 0) + right.duration / speed
    func slice(_ frames: [FilmKeyframe], from: Double, to: Double, fallback: Double)
      -> [FilmKeyframe]
    {
      guard !frames.isEmpty else { return [] }
      return [FilmKeyframe(time: 0, value: value(at: from, keyframes: frames, fallback: fallback))]
        + frames.filter { $0.time > from && $0.time < to }.sorted { $0.time < $1.time }.map {
          FilmKeyframe(time: $0.time - from, value: $0.value)
        }
        + [
          FilmKeyframe(time: to - from, value: value(at: to, keyframes: frames, fallback: fallback))
        ]
    }
    left.opacityKeyframes = slice(opacityKeyframes, from: 0, to: sourceTime, fallback: opacity)
    right.opacityKeyframes = slice(
      opacityKeyframes, from: sourceTime, to: duration, fallback: opacity)
    left.volumeKeyframes = slice(volumeKeyframes, from: 0, to: sourceTime, fallback: volume)
    right.volumeKeyframes = slice(volumeKeyframes, from: sourceTime, to: duration, fallback: volume)
    return (left, right)
  }
}
struct FilmProject: Codable, Identifiable, Hashable {
  var formatVersion: Int? = 1
  var id = UUID().uuidString
  var roomId: String
  var title: String = "Our film"
  var versionId: String = ""
  var editor: String = ""
  var width: Int = 1920
  var height: Int = 1080
  var clips: [FilmClip] = []
  var chapters: [FilmChapter] = []
  var publishedAsset: String?
  var privateDraft: Bool?
  var isPrivateDraft: Bool { privateDraft ?? (publishedAsset == nil) }
  var createdAt: Int64 = Int64(Date().timeIntervalSince1970)
  var autoGenerated: Bool = false
  var duration: Double { clips.map(\.end).max() ?? 0 }
  var supportsEditing: Bool { (formatVersion ?? 1) == 1 }
}

enum LibraryCoding {
  static var decoder: JSONDecoder {
    let d = JSONDecoder()
    d.keyDecodingStrategy = .convertFromSnakeCase
    return d
  }
  static var encoder: JSONEncoder {
    let e = JSONEncoder()
    e.keyEncodingStrategy = .convertToSnakeCase
    return e
  }
}
