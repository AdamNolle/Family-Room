#if DEBUG
import CoreGraphics
import Foundation
import SwiftUI

/// Debug-only sample library for screenshots and simulator runs. Enabled with FAMILY_ROOM_DEMO=1.
/// Nothing here ships in Release builds, and the sample is never created on its own.
@MainActor
enum DemoSeed {
    static var requested: Bool { ProcessInfo.processInfo.environment["FAMILY_ROOM_DEMO"] == "1" }
    static var startScreen: String? { ProcessInfo.processInfo.environment["FAMILY_ROOM_SCREEN"] }

    static func seedIfNeeded(_ store: RoomStore) {
        guard requested, store.rooms.isEmpty else { return }
        seed(store)
    }

    static func seed(_ store: RoomStore) {
        let room = store.createRoom(named: "The Alderhouse")
        let people = [("Margot", "Grandma"), ("Henry", "Grandpa"), ("Cora", "Mom"), ("Theo", "Dad"), ("Iris", "Daughter")].map { store.createPerson(named: $0.0, role: $0.1, in: room) }
        let moments: [(Int, Int, String, [Int], String?, String?, [UInt32])] = [
            (3, 14, "Iris's first bike ride", [4, 1], "Our backyard", nil, [0xF0D48A, 0xC27A3E]),
            (4, 9, "First crocus", [0], "Margot & Henry's", nil, [0xC3D79F, 0x5D7F45]),
            (4, 22, "Dad came home", [1, 0, 2], "Margot & Henry's", nil, [0xECC99C, 0x8A5A3C]),
            (5, 5, "Sunday picnic", [0, 1, 2, 3, 4], nil, nil, [0xC3D79F, 0x5D7F45]),
            (6, 2, "Lake house", [0, 1, 2, 3, 4], "The lake house", nil, [0xA9CBD4, 0x3E6A7D]),
            (7, 4, "Backyard fireworks", [3, 4], "Our backyard", nil, [0xF2A65A, 0xB4493A]),
            (8, 9, "Dad's 72nd", [0, 1, 2, 3, 4], "The lake house", "We took the boat out at six. The sky was the color of a peach pit. Iris fell asleep on his shoulder.", [0xF3CFA6, 0xC86F52]),
            (9, 22, "Iris on stage", [4], "Iris's school", nil, [0x7A5A86, 0x24182E]),
            (10, 11, "Apple picking", [0, 2, 4], nil, nil, [0xE7A65B, 0x8E3F26]),
            (10, 31, "Halloween", [4, 3, 2], nil, nil, [0x5A4A6A, 0x1F1A24]),
            (11, 28, "Thanksgiving", [0, 1, 2, 3, 4], "Margot & Henry's", nil, [0xE9C79A, 0x9A5B3A]),
            (12, 24, "Christmas Eve", [0, 1, 2, 3, 4], "Margot & Henry's", nil, [0xECC99C, 0x8A5A3C])
        ]
        let year = Calendar.current.component(.year, from: Date())
        for (i, m) in moments.enumerated() {
            let date = Calendar.current.date(from: DateComponents(year: year, month: m.0, day: m.1, hour: 15))!
            let count = m.2 == "Dad's 72nd" ? 9 : 3 + (i * 5) % 6
            var first: MediaItem?
            for n in 0..<count {
                let data = gradientJPEG(m.6, seed: i * 31 + n)
                let captured = date.addingTimeInterval(Double(n) * 600)
                var imported = ImportedMedia(data: data, thumbnail: data, isVideo: false, capturedAt: captured, pixelSize: CGSize(width: 900, height: 600), duration: 0, fileExtension: "jpg", photoIdentifier: nil)
                imported.capturedAt = captured
                let item = store.add(imported, to: room)
                item.addedByName = people[m.3[n % m.3.count]].displayName
                item.addedAt = i >= 8 ? Date().addingTimeInterval(-Double(n + 1) * 3600 * Double(13 - i)) : date
                if first == nil { first = item }
            }
            if let moment = first?.moment {
                moment.title = m.2; moment.place = m.4; moment.quote = m.5
                moment.isFeatured = m.5 != nil
                moment.people = Set(m.3.map { people[$0] })
                if m.5 != nil {
                    for (who, text) in [(4, "best day of my life"), (0, "so happy with my whole family")] {
                        let note = Note(context: store.context)
                        note.id = UUID(); note.text = text; note.createdAt = date.addingTimeInterval(3600); note.authorName = people[who].displayName; note.authorID = "demo-\(who)"; note.moment = moment
                    }
                    for who in 0..<4 { let r = Reaction(context: store.context); r.id = UUID(); r.createdAt = date; r.authorName = people[who].displayName; r.authorID = "demo-\(who)"; r.moment = moment }
                }
            }
        }
        for name in ["Mom", "Dad", "Lucy"] { _ = store.createPerson(named: name, faceGroup: true, in: room) }
        store.save()
    }

    static func gradientJPEG(_ colors: [UInt32], seed: Int) -> Data {
        let size = CGSize(width: 900, height: 600)
        let renderer = ImageRenderer(content: ZStack {
            LinearGradient(colors: colors.map { Color(hex: $0) }, startPoint: .topLeading, endPoint: .bottomTrailing)
            RadialGradient(colors: [.white.opacity(0.7), .clear], center: UnitPoint(x: 0.3 + Double(seed % 5) * 0.1, y: 0.2 + Double(seed % 3) * 0.15), startRadius: 0, endRadius: 420)
        }.frame(width: size.width, height: size.height))
        renderer.scale = 1
        guard let cg = renderer.cgImage, let data = MediaImporter.jpeg(cg, quality: 0.7) else { return Data() }
        return data
    }

    /// Jumps to a screen for screenshots: hearth, circle, stories, settings, moment, person, face, compose.
    static func apply(_ app: AppState, room: Room) {
        guard let screen = startScreen else { return }
        switch screen {
        case "circle": app.tab = .circle
        case "stories": app.tab = .stories
        case "settings": app.tab = .settings
        case "moment": if let m = room.momentList.first(where: \.isFeatured) { app.open(m) }
        case "person": app.tab = .circle; if let p = room.peopleList.first(where: { $0.displayName == "Cora" }) { app.open(p) }
        case "face": app.tab = .circle; if let p = room.peopleList.first(where: \.isFaceGroup) { app.open(p) }
        case "compose": if let m = room.momentList.first(where: \.isFeatured) { app.open(m); app.composeMoment = m }
        case "filter": if let c = room.peopleList.first(where: { $0.displayName == "Cora" })?.id { app.selectedPeople = [c] }; app.tab = .circle
        default: break
        }
    }
}
#endif
