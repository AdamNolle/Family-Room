import Foundation

struct LibrarySnapshot: Codable {
    var version: Int
    var deviceId: String
    var rooms: [LibraryRoom]
    var sources: [LibrarySource]
    var watches: [UploadRule]
    var transfers: [LibraryTransfer]
    static let empty = LibrarySnapshot(version: 1, deviceId: "", rooms: [], sources: [], watches: [], transfers: [])
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
    var assets: [LibraryAsset]
    var albums: [LibraryAlbum]
    var moments: [LibraryMoment]
    var people: [LibraryPerson]
    var stories: [FilmProject]
    var comments: [LibraryComment]
    var members: [LibraryMember]
    var visibleAssets: [LibraryAsset] { assets.filter { !$0.deleted }.sorted { $0.capturedAt > $1.capturedAt } }
    var canContribute: Bool { ["owner", "admin", "contributor"].contains(role) }
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
    var availability: String
    var date: Date { Date(timeIntervalSince1970: TimeInterval(capturedAt)) }
    var displayTitle: String { caption.isEmpty ? filename : caption }
}
struct LibraryAlbum: Codable, Identifiable {
    var id: String
    var roomId: String
    var title: String
    var assetIds: [String]
    var pinned: Bool
}
struct LibraryPerson: Codable, Identifiable { var id: String; var roomId: String; var name: String }
struct LibraryMember: Codable, Identifiable {
    var roomId: String; var deviceId: String; var role: String; var signature: String
    var id: String { deviceId }
}
struct LibraryComment: Codable, Identifiable {
    var id: String; var assetId: String; var author: String; var body: String; var createdAt: Int64
}
struct LibrarySource: Codable, Identifiable {
    var id: String; var roomId: String; var name: String; var kind: String; var endpoint: String
    var paused: Bool; var preferred: Bool; var budget: UInt64?
}
struct UploadRule: Codable, Identifiable {
    var id: String; var roomId: String; var path: String; var paused: Bool
    var excludedExtensions: [String]; var seen: [String]
}
struct LibraryTransfer: Codable, Identifiable {
    var id: String; var roomId: String; var assetId: String; var sourceId: String
    var state: String; var transferred: UInt64; var total: UInt64; var error: String?
    var progress: Double { total == 0 ? 0 : Double(transferred) / Double(total) }
}
struct FilmKeyframe: Codable, Hashable { var time: Double; var value: Double }
struct FilmChapter: Codable, Hashable { var title: String; var time: Double }
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
        if fadeOut > 0 { multiplier *= min(1, max(0, (duration / speed - outputTime + (fadeOutOffset ?? 0)) / fadeOut)) }
        return multiplier
    }
    func opacity(at outputTime: Double) -> Double {
        min(1, max(0, value(at: outputTime * speed, keyframes: opacityKeyframes, fallback: opacity) * fadeMultiplier(at: outputTime)))
    }
    func audioGain(at outputTime: Double) -> Double {
        min(4, max(0, value(at: outputTime * speed, keyframes: volumeKeyframes, fallback: volume) * fadeMultiplier(at: outputTime)))
    }
    func resized(to sourceDuration: Double) -> FilmClip {
        guard sourceDuration.isFinite, sourceDuration >= 0.001, sourceDuration <= 86400 else { return self }
        var result = self
        result.duration = sourceDuration
        if sourceDuration < duration {
            func trim(_ frames: [FilmKeyframe], fallback: Double) -> [FilmKeyframe] {
                guard !frames.isEmpty else { return [] }
                return frames.filter { $0.time < sourceDuration }.sorted { $0.time < $1.time }
                    + [FilmKeyframe(time: sourceDuration, value: value(at: sourceDuration, keyframes: frames, fallback: fallback))]
            }
            result.opacityKeyframes = trim(opacityKeyframes, fallback: opacity)
            result.volumeKeyframes = trim(volumeKeyframes, fallback: volume)
        }
        return result
    }
    func split(at sourceTime: Double) -> (left: FilmClip, right: FilmClip)? {
        guard sourceTime.isFinite, duration.isFinite, speed.isFinite, speed > 0,
              sourceTime >= 0.001, duration - sourceTime >= 0.001 else { return nil }
        var left = self, right = self
        left.duration = sourceTime
        right.id = UUID().uuidString
        right.start += sourceTime / speed
        right.trimIn += sourceTime
        right.duration -= sourceTime
        right.fadeInOffset = (fadeInOffset ?? 0) + sourceTime / speed
        left.fadeOutOffset = (fadeOutOffset ?? 0) + right.duration / speed
        func slice(_ frames: [FilmKeyframe], from: Double, to: Double, fallback: Double) -> [FilmKeyframe] {
            guard !frames.isEmpty else { return [] }
            return [FilmKeyframe(time: 0, value: value(at: from, keyframes: frames, fallback: fallback))]
                + frames.filter { $0.time > from && $0.time < to }.sorted { $0.time < $1.time }.map { FilmKeyframe(time: $0.time - from, value: $0.value) }
                + [FilmKeyframe(time: to - from, value: value(at: to, keyframes: frames, fallback: fallback))]
        }
        left.opacityKeyframes = slice(opacityKeyframes, from: 0, to: sourceTime, fallback: opacity)
        right.opacityKeyframes = slice(opacityKeyframes, from: sourceTime, to: duration, fallback: opacity)
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
    static var decoder: JSONDecoder { let d = JSONDecoder(); d.keyDecodingStrategy = .convertFromSnakeCase; return d }
    static var encoder: JSONEncoder { let e = JSONEncoder(); e.keyEncodingStrategy = .convertToSnakeCase; return e }
}
