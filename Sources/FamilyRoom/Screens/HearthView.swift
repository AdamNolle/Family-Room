import CoreData
import SwiftUI

/// Home — "The Hearth". Adapts to the phone (stage + list) and to list-only use in a split layout.
struct HearthView: View {
    @ObservedObject var room: Room
    var showSearch = true
    var showPeople = true
    var listOnly = false
    var wide = false
    var onOpen: (Moment) -> Void

    @EnvironmentObject private var store: RoomStore
    @EnvironmentObject private var app: AppState
    @FetchRequest private var moments: FetchedResults<Moment>
    @FetchRequest private var people: FetchedResults<Person>
    @State private var yearMenu = false

    init(room: Room, showSearch: Bool = true, showPeople: Bool = true, listOnly: Bool = false, wide: Bool = false, onOpen: @escaping (Moment) -> Void) {
        self.room = room; self.showSearch = showSearch; self.showPeople = showPeople; self.listOnly = listOnly; self.wide = wide; self.onOpen = onOpen
        _moments = FetchRequest(sortDescriptors: [NSSortDescriptor(key: "date", ascending: true)], predicate: NSPredicate(format: "room == %@", room))
        _people = FetchRequest(sortDescriptors: [NSSortDescriptor(key: "createdAt", ascending: true)], predicate: NSPredicate(format: "room == %@ AND isFaceGroup == NO", room))
    }

    private var years: [Int] { Array(Set(moments.compactMap { $0.date.map { Calendar.current.component(.year, from: $0) } })).sorted(by: >) }
    private var year: Int { app.year.flatMap { years.contains($0) ? $0 : nil } ?? years.first ?? Calendar.current.component(.year, from: Date()) }
    private var yearMoments: [Moment] { moments.filter { $0.date.map { Calendar.current.component(.year, from: $0) == year } ?? false } }
    private var summaries: [MomentSummary] { yearMoments.map(\.summary) }
    private func matches(_ m: MomentSummary) -> Bool { Catalog.matches(m, selected: app.selectedPeople, query: app.query) }
    private var active: [MomentSummary] { summaries.filter(matches) }
    private var stage: MomentSummary? { Catalog.stageMoment(summaries, preferred: app.stageMomentID ?? app.duoMoment?.id, matching: matches) }
    private func moment(for s: MomentSummary) -> Moment? { yearMoments.first { $0.id == s.id } }

    var body: some View {
        let columns = wide && !listOnly
        VStack(alignment: .leading, spacing: 26) {
            if columns {
                HStack(alignment: .top, spacing: 40) {
                    stageSection.frame(maxWidth: .infinity)
                    VStack(alignment: .leading, spacing: 0) { head; list.padding(.top, 20) }.frame(width: 290)
                }
            } else {
                head
                if !listOnly { stageSection }
            }
            if let stage, !yearMoments.isEmpty {
                RiverRuler(moments: summaries, year: year, current: stage, matches: matches) { picked in
                    withAnimation(Motion.gentle) { app.stageMomentID = picked.id }
                    if listOnly, let m = moment(for: picked) { onOpen(m) }
                }
            }
            if !columns { list }
            shared
        }
        .padding(.horizontal, wide ? 36 : 20).padding(.top, wide ? 26 : 10).padding(.bottom, wide ? 48 : 118)
    }

    // MARK: Head

    private var head: some View {
        VStack(alignment: .leading, spacing: 0) {
            if showSearch {
                SearchField(text: $app.query, placeholder: "Names, places, months", tall: true).padding(.bottom, 18)
            }
            Text(Catalog.kicker(selectedNames: people.filter { app.selectedPeople.contains($0.id ?? UUID()) }.map(\.displayName), query: app.query, year: year)).labelStyle(warm: true)
                .contentTransition(.opacity)
                .animation(Motion.gentle, value: app.selectedPeople)
            let yearLayout = wide ? AnyLayout(VStackLayout(alignment: .leading, spacing: 10)) : AnyLayout(HStackLayout(alignment: .bottom, spacing: 14))
            yearLayout {
                Menu {
                    ForEach(years, id: \.self) { y in Button(String(y)) { app.year = y; app.stageMomentID = nil } }
                } label: {
                    Text(String(year)).font(.display(wide ? 104 : 96)).foregroundStyle(Theme.ink).lineLimit(1).fixedSize().offset(x: -4)
                }
                .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden)
                .disabled(years.count < 2)
                .accessibilityLabel("Year \(year)")
                VStack(alignment: .leading, spacing: 2) {
                    (Text("\(active.count)").font(.num(13, .semibold)).foregroundStyle(Theme.ink) + Text(" moments").foregroundStyle(Theme.inkSoft))
                        .contentTransition(.numericText())
                    Text("\(active.reduce(0) { $0 + $1.mediaCount }) photos and videos").foregroundStyle(Theme.inkSoft)
                        .contentTransition(.numericText())
                }
                .font(.num(13)).padding(.bottom, wide ? 0 : 6)
                .animation(Motion.gentle, value: active.count)
            }
            .padding(.top, 10)
            if showPeople, !people.isEmpty { peopleChips.padding(.top, 16) }
        }
    }

    private var peopleChips: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 6) {
                ForEach(people) { p in
                    let id = p.id ?? UUID(), on = app.selectedPeople.contains(id)
                    Button { withAnimation(Motion.snappy) { app.toggle(id) }; Feedback.tick() } label: {
                        HStack(spacing: 8) {
                            AvatarView(person: p.summary, size: 26, ring: on, dim: !app.selectedPeople.isEmpty && !on)
                                .pop(on: on)
                            Text(p.displayName).font(.sans(13.5, .medium)).foregroundStyle(Theme.ink)
                        }
                        .padding(.leading, 9).padding(.trailing, 14).frame(height: 44)
                        .background(on ? Color.white : Color.white.opacity(0.45), in: Capsule())
                        .overlay(Capsule().strokeBorder(Theme.hair2, lineWidth: 0.5))
                        .shadow(color: Theme.shadowInk.opacity(on ? 0.1 : 0), radius: 3, y: 2)
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(on ? .isSelected : [])
                }
            }
            .padding(.horizontal, 20).padding(.vertical, 2)
        }
        .padding(.horizontal, -20)
    }

    // MARK: Stage

    @ViewBuilder
    private var stageSection: some View {
        if let stage, let m = moment(for: stage) {
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: 12) {
                    Text("No. \(Catalog.pad((summaries.firstIndex(of: stage) ?? 0) + 1))").labelStyle(warm: true)
                    Text(Catalog.dayLabel(stage.date)).labelStyle()
                    if let place = stage.place { Text(place).labelStyle().lineLimit(1) }
                }
                .padding(.bottom, 12)
                Button { onOpen(m) } label: {
                    ZStack {
                        PhotoSurface(media: m.cover, label: m.cover == nil ? nil : "Photo · \(stage.title)")
                            .breathing()
                            .id(stage.id)
                            .transition(.asymmetric(insertion: .opacity.combined(with: .scale(scale: 1.03)), removal: .opacity))
                    }
                    .aspectRatio(3 / 2, contentMode: .fit)
                    .clipShape(RoundedRectangle(cornerRadius: 2))
                    .printShadow()
                    .overlay(CornerCrops())
                }
                .buttonStyle(PressScale())
                HStack(alignment: .bottom, spacing: 14) {
                    Text(stage.title).font(.display(wide ? 58 : 42)).foregroundStyle(Theme.ink).lineSpacing(-6).minimumScaleFactor(0.6)
                        .id("title-\(stage.id)")
                        .transition(.asymmetric(insertion: .opacity.combined(with: .offset(y: 8)), removal: .opacity))
                    Spacer(minLength: 0)
                    AvatarStack(people: stage.people, size: 22)
                }
                .padding(.top, 18)
                if let quote = stage.quote {
                    Text(quote).font(.display(20)).italic().foregroundStyle(Theme.inkSoft).lineSpacing(2).padding(.top, 12).frame(maxWidth: 420, alignment: .leading)
                }
                FRButton("Open moment · \(stage.mediaCount) \(stage.mediaCount == 1 ? "item" : "items") →", style: .ghost, size: .small) { onOpen(m) }
                    .padding(.top, 16)
            }
            .animation(Motion.gentle, value: stage.id)
        } else if moments.isEmpty {
            EmptyHearth()
        }
    }

    // MARK: List

    private var list: some View {
        VStack(spacing: 0) {
            ForEach(Array(summaries.enumerated()), id: \.element.id) { i, s in
                if i > 0 { Hairline() }
                IndexRow(number: i + 1, moment: s, selected: s.id == stage?.id, dim: !matches(s)) {
                    withAnimation(Motion.gentle) { app.stageMomentID = s.id }
                    if let m = moment(for: s) { onOpen(m) }
                }
                .entrance(i)
                .animation(Motion.gentle, value: matches(s))
            }
            if active.isEmpty, !summaries.isEmpty {
                Text("Nothing matches yet. Try a name, a place, or a month.").font(.sans(13)).foregroundStyle(Theme.inkMute).padding(.vertical, 14).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }

    // MARK: Shared this week

    private var shared: some View {
        let items = store.contributions(in: room)
        return VStack(alignment: .leading, spacing: 16) {
            HStack(alignment: .firstTextBaseline) {
                Text("Shared this week").font(.display(30)).foregroundStyle(Theme.ink)
                Spacer()
                if !items.isEmpty { Text("\(items.count)").font(.num(12)).foregroundStyle(Theme.inkMute) }
            }
            .padding(.bottom, 12).overlay(alignment: .bottom) { Hairline() }
            if items.isEmpty {
                Text("Nothing new this week. Add photos and they show up here for everyone in the Room.").font(.sans(13)).foregroundStyle(Theme.inkMute)
            } else if wide {
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 10), count: 3), spacing: 10) {
                    ForEach(Array(items.enumerated()), id: \.element.id) { i, c in card(c).entrance(i) }
                }
            } else {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 10) { ForEach(Array(items.enumerated()), id: \.element.id) { i, c in card(c).frame(width: 270).entrance(i) } }.padding(.horizontal, 20)
                }
                .padding(.horizontal, -20)
            }
        }
    }

    private func card(_ c: ContributionSummary) -> some View {
        let moment = moments.first { $0.id == c.momentID }
        return Button { if let moment { onOpen(moment) } } label: {
            HStack(alignment: .top, spacing: 14) {
                PhotoSurface(media: moment?.cover, maxSide: 200).frame(width: 56, height: 56).clipShape(RoundedRectangle(cornerRadius: 5))
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 7) { NameAvatar(name: c.authorName, size: 16); Text(c.message).font(.sans(13, .semibold)).foregroundStyle(Theme.ink).lineLimit(2) }
                    Text(Catalog.relative(c.latest)).font(.sans(11.5)).foregroundStyle(Theme.inkMute)
                    HStack(spacing: 12) {
                        Text("♥ \(c.likes)"); Text("\(c.notes) \(c.notes == 1 ? "note" : "notes")")
                    }
                    .font(.num(11.5)).foregroundStyle(Theme.inkSoft).padding(.top, 5)
                }
                Spacer(minLength: 0)
            }
            .padding(12)
            .background(.white.opacity(0.6), in: RoundedRectangle(cornerRadius: 10, style: .continuous))
            .hairlineBorder(10)
        }
        .buttonStyle(PressScale())
    }
}

struct EmptyHearth: View {
    @EnvironmentObject private var app: AppState
    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("The room is quiet.").font(.display(40)).foregroundStyle(Theme.ink)
            Text("Add photos and videos and they gather into moments by day. Everyone in the Room sees them through iCloud.")
                .font(.sans(14)).lineSpacing(4).foregroundStyle(Theme.inkSoft).frame(maxWidth: 420, alignment: .leading)
            FRButton("Add photos", mark: "+") { app.showImporter = true }
        }
        .padding(.vertical, 30)
        .frame(maxWidth: .infinity, alignment: .leading)
        .overlay(alignment: .top) { Rectangle().fill(Theme.hair2).frame(height: 0.5) }
        .overlay(alignment: .bottom) { Rectangle().fill(Theme.hair2).frame(height: 0.5) }
    }
}
