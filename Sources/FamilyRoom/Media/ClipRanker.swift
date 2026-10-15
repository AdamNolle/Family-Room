import Foundation

/// Signals we actually have about an item, used to pick the best clips for a story.
struct ClipSignals: Hashable, Identifiable {
    var id: UUID
    var isVideo: Bool
    var isFavorite: Bool
    var faceCount: Int
    var largestFace: Double      // fraction of the frame covered by the biggest face, 0...1
    var capturedAt: Date
    var duration: Double
}

struct ClipCandidate: Hashable, Identifiable {
    var id: UUID { signals.id }
    var signals: ClipSignals
    var reason: String
    var score: Double
}

enum ClipRanker {
    static let clipSeconds = 3.0
    static let defaultCount = 8

    /// Ranks items by honest, on-device signals and explains the top reason for each.
    static func rank(_ items: [ClipSignals], peopleInRoom: Int, limit: Int = defaultCount) -> [ClipCandidate] {
        let scored = items.map { s -> ClipCandidate in
            var score = 0.0
            var reasons: [(Double, String)] = []
            if s.isFavorite { score += 3; reasons.append((3, "Favorite")) }
            if s.faceCount >= max(2, peopleInRoom) && peopleInRoom > 1 { score += 2.5; reasons.append((2.5, "Everyone in frame")) }
            else if s.faceCount > 1 { score += 1.5; reasons.append((1.5, "\(s.faceCount) faces")) }
            else if s.faceCount == 1 { score += 1; reasons.append((1, "A face")) }
            if s.largestFace > 0.12 { score += 1.2; reasons.append((1.2, "Close-up")) }
            if s.isVideo { score += 1.4; reasons.append((1.4, "Motion")) }
            if reasons.isEmpty { reasons.append((0, "Fresh")) }
            let reason = reasons.max(by: { $0.0 < $1.0 })!.1
            return ClipCandidate(signals: s, reason: reason, score: score)
        }
        // Stable: higher score first, then earlier capture so a story reads chronologically among ties.
        let top = scored.sorted { a, b in a.score != b.score ? a.score > b.score : a.signals.capturedAt < b.signals.capturedAt }.prefix(limit)
        return top.sorted { $0.signals.capturedAt < $1.signals.capturedAt }
    }

    static func totalDuration(selected: Int) -> Double { Double(selected) * clipSeconds }
}
