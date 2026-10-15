import Foundation

// Pure, testable helpers behind the Hearth: filtering, the river of time, labels.

struct PersonSummary: Hashable, Identifiable {
    var id: UUID
    var name: String
    var role: String
    var toneHex: String
    var accentHex: String
    var isFaceGroup: Bool
    var photoCount: Int

    var initial: String { String(name.trimmingCharacters(in: .whitespaces).prefix(1)).uppercased() }
}

struct MomentSummary: Hashable, Identifiable {
    var id: UUID
    var title: String
    var date: Date
    var place: String?
    var quote: String?
    var isFeatured: Bool
    var people: [PersonSummary]
    var photoCount: Int
    var videoCount: Int

    var mediaCount: Int { photoCount + videoCount }
    var peopleIDs: Set<UUID> { Set(people.map(\.id)) }

    /// 1...5 tick weight for the river, scaled by how much was captured.
    var weight: Int { min(5, max(1, Int((Double(mediaCount) + 1).squareRoot().rounded(.up)))) }
}

enum Catalog {
    static func matches(_ m: MomentSummary, selected: Set<UUID>, query: String, calendar: Calendar = .current) -> Bool {
        if !selected.isEmpty && m.peopleIDs.isDisjoint(with: selected) { return false }
        let q = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !q.isEmpty else { return true }
        if m.title.lowercased().contains(q) { return true }
        if let place = m.place, place.lowercased().contains(q) { return true }
        if dayLabel(m.date, calendar: calendar).lowercased().contains(q) { return true }
        if monthName(m.date, calendar: calendar).lowercased().hasPrefix(q) { return true }
        if String(calendar.component(.year, from: m.date)).contains(q) { return true }
        return m.people.contains { $0.name.lowercased().contains(q) }
    }

    /// The moment shown on the stage: the pinned one if it matches, else the newest match.
    static func stageMoment(_ moments: [MomentSummary], preferred: UUID?, matching: (MomentSummary) -> Bool) -> MomentSummary? {
        let active = moments.filter(matching)
        if let preferred, let hit = active.first(where: { $0.id == preferred }) { return hit }
        if let featured = active.last(where: \.isFeatured) { return featured }
        return active.max(by: { $0.date < $1.date })
    }

    static func kicker(selectedNames: [String], query: String, year: Int?) -> String {
        let q = query.trimmingCharacters(in: .whitespacesAndNewlines)
        if !q.isEmpty { return "Matching “\(q)”" }
        if !selectedNames.isEmpty { return "With \(listJoin(selectedNames))" }
        if let year, year == Calendar.current.component(.year, from: Date()) { return "This year · everyone" }
        return "Everyone"
    }

    static func listJoin(_ names: [String]) -> String {
        ListFormatter.localizedString(byJoining: names)
    }

    static func pad(_ n: Int) -> String { n < 10 ? "0\(n)" : "\(n)" }

    static func duration(_ seconds: Double) -> String {
        let s = max(0, Int(seconds.rounded()))
        return "\(s / 60):\(pad(s % 60))"
    }

    static func dayLabel(_ date: Date, calendar: Calendar = .current) -> String {
        let f = DateFormatter(); f.calendar = calendar; f.setLocalizedDateFormatFromTemplate("MMM dd")
        return f.string(from: date)
    }

    static func monthName(_ date: Date, calendar: Calendar = .current) -> String {
        let f = DateFormatter(); f.calendar = calendar; f.setLocalizedDateFormatFromTemplate("MMMM")
        return f.string(from: date)
    }

    static func longDay(_ date: Date, calendar: Calendar = .current) -> String {
        let f = DateFormatter(); f.calendar = calendar; f.setLocalizedDateFormatFromTemplate("EEEE, MMMM d")
        return f.string(from: date)
    }

    static func relative(_ date: Date, now: Date = Date()) -> String {
        let f = RelativeDateTimeFormatter(); f.unitsStyle = .full
        let interval = now.timeIntervalSince(date)
        if interval < 60 { return "just now" }
        return f.localizedString(for: date, relativeTo: now)
    }

    /// 0...1 position of a date within its year.
    static func yearFraction(_ date: Date, calendar: Calendar = .current) -> Double {
        let day = calendar.ordinality(of: .day, in: .year, for: date) ?? 1
        let days = calendar.range(of: .day, in: .year, for: date)?.count ?? 365
        return Double(day - 1) / Double(max(1, days - 1))
    }

    /// Month start fractions for the ruler, skipping the first `skip` months as the design does.
    static func monthMarks(year: Int, skip: Int = 0, calendar: Calendar = .current) -> [(label: String, fraction: Double)] {
        (1...12).dropFirst(skip).compactMap { month in
            guard let date = calendar.date(from: DateComponents(year: year, month: month, day: 1)) else { return nil }
            let f = DateFormatter(); f.calendar = calendar; f.setLocalizedDateFormatFromTemplate("MMM")
            return (f.string(from: date), yearFraction(date, calendar: calendar))
        }
    }

    /// Nearest moment to a ruler position, considering only matching moments.
    static func nearest(_ moments: [MomentSummary], to fraction: Double, matching: (MomentSummary) -> Bool, calendar: Calendar = .current) -> MomentSummary? {
        moments.filter(matching).min { abs(yearFraction($0.date, calendar: calendar) - fraction) < abs(yearFraction($1.date, calendar: calendar) - fraction) }
    }

    static func startOfDay(_ date: Date, calendar: Calendar = .current) -> Date { calendar.startOfDay(for: date) }

    static func defaultMomentTitle(_ date: Date, calendar: Calendar = .current) -> String {
        let f = DateFormatter(); f.calendar = calendar; f.setLocalizedDateFormatFromTemplate("MMMM d")
        return f.string(from: date)
    }

    static func weekStart(_ date: Date = Date(), calendar: Calendar = .current) -> Date {
        calendar.dateInterval(of: .weekOfYear, for: date)?.start ?? startOfDay(date, calendar: calendar)
    }

    static func weekLabel(_ start: Date, calendar: Calendar = .current) -> String {
        let end = calendar.date(byAdding: .day, value: 6, to: start) ?? start
        let f = DateFormatter(); f.calendar = calendar; f.setLocalizedDateFormatFromTemplate("MMM d")
        return "\(f.string(from: start)) – \(f.string(from: end))"
    }
}

/// A contribution card for "Shared this week": one person's additions on one day.
struct ContributionSummary: Hashable, Identifiable {
    var id: String
    var momentID: UUID
    var authorName: String
    var authorInitial: String
    var photoCount: Int
    var videoCount: Int
    var latest: Date
    var likes: Int
    var notes: Int

    var message: String {
        var parts: [String] = []
        if photoCount > 0 { parts.append("\(photoCount) photo\(photoCount == 1 ? "" : "s")") }
        if videoCount > 0 { parts.append(videoCount == 1 ? "a video" : "\(videoCount) videos") }
        return "\(authorName) added \(parts.joined(separator: " and "))"
    }
}
