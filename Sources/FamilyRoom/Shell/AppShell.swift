import PhotosUI
import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

/// Root: welcome until a Room exists, then the device-appropriate shell plus the shared sheets.
struct RootView: View {
    @EnvironmentObject private var store: RoomStore
    @StateObject private var app = AppState()
    @State private var picked: [PhotosPickerItem] = []
    @State private var fileImport = false

    var body: some View {
        Group {
            if let room = store.currentRoom {
                AppShell(room: room)
                    .environmentObject(app)
            } else {
                WelcomeView()
            }
        }
        .tint(Theme.warm)
        .preferredColorScheme(.light)
        .sheet(item: $app.composeMoment) { moment in composer(moment: moment) }
        .sheet(isPresented: $app.composeWeekly) { composer(moment: nil) }
        .storyCover(item: $app.playingStory) { StoryPlayer(story: $0) }
        .photosPicker(isPresented: $app.showImporter, selection: $picked, maxSelectionCount: 0, matching: .any(of: [.images, .videos]))
        .onChange(of: picked) { _, items in
            guard !items.isEmpty else { return }
            let batch = items; picked = []
            Task { await importPicked(batch) }
        }
        .fileImporter(isPresented: $fileImport, allowedContentTypes: [.image, .movie], allowsMultipleSelection: true) { result in
            if case .success(let urls) = result { Task { await importFiles(urls) } }
        }
        .onReceive(NotificationCenter.default.publisher(for: .familyRoomImportFiles)) { _ in fileImport = true }
        .onAppear {
            #if DEBUG
            DemoSeed.seedIfNeeded(store)
            if let room = store.currentRoom { DemoSeed.apply(app, room: room) }
            #if os(macOS)
            WindowSnapshot.scheduleIfRequested()
            #endif
            #endif
        }
        .overlay(alignment: .bottom) { ImportToast() }
    }

    @ViewBuilder
    private func composer(moment: Moment?) -> some View {
        if let room = store.currentRoom {
            ComposerSheet(room: room, moment: moment, onGoStories: { app.composeMoment = nil; app.composeWeekly = false; app.go(.stories) },
                          onClose: { app.composeMoment = nil; app.composeWeekly = false })
                .environmentObject(app)
                .presentationDetents([.large])
                #if os(macOS)
                .frame(minWidth: 780, minHeight: 560)
                #endif
        }
    }

    private func importPicked(_ items: [PhotosPickerItem]) async {
        guard let room = store.currentRoom else { return }
        let progress = Progress(totalUnitCount: Int64(items.count))
        store.importProgress = progress
        defer { store.importProgress = nil }
        let target = app.path.last.flatMap { if case .moment(let m) = $0 { return m } else { return nil } }
        for item in items {
            do {
                guard let data = try await item.loadTransferable(type: Data.self) else { continue }
                let type = item.supportedContentTypes.first
                let imported = try await MediaImporter.prepare(data: data, type: type, fileName: nil)
                _ = store.add(imported, to: room, momentOverride: target)
                store.save()
            } catch { store.lastError = error.localizedDescription }
            progress.completedUnitCount += 1
        }
    }

    private func importFiles(_ urls: [URL]) async {
        guard let room = store.currentRoom else { return }
        let progress = Progress(totalUnitCount: Int64(urls.count))
        store.importProgress = progress
        defer { store.importProgress = nil }
        for url in urls {
            let access = url.startAccessingSecurityScopedResource()
            defer { if access { url.stopAccessingSecurityScopedResource() } }
            do {
                let imported = try await MediaImporter.prepare(fileURL: url)
                _ = store.add(imported, to: room)
                store.save()
            } catch { store.lastError = error.localizedDescription }
            progress.completedUnitCount += 1
        }
    }
}

extension Notification.Name {
    static let familyRoomImportFiles = Notification.Name("familyRoom.importFiles")
}

private extension View {
    @ViewBuilder
    func storyCover<Content: View>(item: Binding<Story?>, @ViewBuilder content: @escaping (Story) -> Content) -> some View {
        #if os(iOS)
        self.fullScreenCover(item: item, content: content)
        #else
        self.sheet(item: item, content: content)
        #endif
    }
}

struct ImportToast: View {
    @EnvironmentObject private var store: RoomStore
    var body: some View {
        if let p = store.importProgress {
            HStack(spacing: 10) {
                ProgressView().controlSize(.small)
                Text("Adding \(p.completedUnitCount + 1) of \(p.totalUnitCount)…").font(.num(12.5, .medium)).foregroundStyle(Theme.ink)
            }
            .padding(.horizontal, 14).frame(height: 36)
            .background(Theme.paper2, in: Capsule()).overlay(Capsule().strokeBorder(Theme.hair2, lineWidth: 0.5))
            .shadow(color: Theme.shadowInk.opacity(0.16), radius: 12, y: 6)
            .padding(.bottom, 100)
            .transition(.move(edge: .bottom).combined(with: .opacity))
        } else if let error = store.lastError {
            Text(error).font(.sans(12.5)).foregroundStyle(.white).padding(.horizontal, 14).frame(height: 36)
                .background(Theme.warm, in: Capsule()).padding(.bottom, 100)
                .onTapGesture { store.lastError = nil }
                .task { try? await Task.sleep(nanoseconds: 5_000_000_000); store.lastError = nil }
        }
    }
}

// MARK: Shell selection

struct AppShell: View {
    @ObservedObject var room: Room
    @Environment(\.horizontalSizeClass) private var sizeClass

    var body: some View {
        #if os(macOS)
        SidebarShell(room: room)
        #else
        #if DEBUG
        if ProcessInfo.processInfo.environment["FAMILY_ROOM_SHELL"] == "duo" { DuoShell(room: room) } else { platformShell }
        #else
        platformShell
        #endif
        #endif
    }

    #if os(iOS)
    @ViewBuilder
    private var platformShell: some View {
        if sizeClass == .compact {
            PhoneShell(room: room)
        } else if UIDevice.current.userInterfaceIdiom == .phone {
            DuoShell(room: room)
        } else {
            SidebarShell(room: room)
        }
    }
    #endif
}

/// The screen for a tab or route, used by every shell.
struct ScreenBody: View {
    @ObservedObject var room: Room
    var wide: Bool
    var showSearch = true
    var showPeople = true
    @EnvironmentObject private var app: AppState

    var body: some View {
        if let route = app.path.last {
            switch route {
            case .moment(let m): MomentView(moment: m, wide: wide, onBack: app.back).id(m.objectID)
            case .person(let p): PersonView(person: p, wide: wide, onBack: app.back, onOpen: app.open).id(p.objectID)
            }
        } else {
            switch app.tab {
            case .hearth: HearthView(room: room, showSearch: showSearch, showPeople: showPeople, wide: wide, onOpen: app.open)
            case .circle: CircleView(room: room, wide: wide, onOpen: app.open)
            case .stories: StoriesView(room: room, wide: wide)
            case .settings: SettingsView(room: room, wide: wide)
            }
        }
    }
}

/// Scroll container that resets to the top and fades in when the screen changes (`.enter`).
struct Pane<Content: View>: View {
    var key: String
    @ViewBuilder var content: () -> Content
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                Color.clear.frame(height: 0).id("top")
                content()
                    .id(key)
                    .transition(reduceMotion ? .opacity : .asymmetric(insertion: .opacity.combined(with: .offset(y: 14)).combined(with: .scale(scale: 0.992, anchor: .top)), removal: .opacity.animation(.easeOut(duration: 0.12))))
            }
            .scrollIndicators(.hidden)
            .onChange(of: key) { _, _ in proxy.scrollTo("top", anchor: .top) }
            .animation(reduceMotion ? .easeOut(duration: 0.2) : Motion.gentle, value: key)
        }
    }
}

extension AppState {
    var screenKey: String {
        switch path.last {
        case .moment(let m): return "moment-\(m.objectID.uriRepresentation().absoluteString)"
        case .person(let p): return "person-\(p.objectID.uriRepresentation().absoluteString)"
        case nil: return tab.rawValue
        }
    }
}

// MARK: iPhone

struct PhoneShell: View {
    @ObservedObject var room: Room
    @EnvironmentObject private var app: AppState

    var body: some View {
        ZStack(alignment: .bottom) {
            PaperBackground()
            Pane(key: app.screenKey) { ScreenBody(room: room, wide: false) }
            TabBar()
                .padding(.horizontal, 18).padding(.bottom, 10)
        }
    }
}

struct TabBar: View {
    @EnvironmentObject private var app: AppState
    @Namespace private var pill
    var body: some View {
        HStack(spacing: 4) {
            ForEach(AppTab.allCases) { tab in
                let on = app.tab == tab && app.path.isEmpty
                Button { withAnimation(Motion.snappy) { app.go(tab) }; Feedback.tick() } label: {
                    VStack(spacing: 3) {
                        Text(tab.number).font(.label(9.5)).foregroundStyle(on ? Theme.warm : Theme.inkFaint)
                        Text(tab.title).font(.sans(12, .semibold)).foregroundStyle(on ? Theme.ink : Theme.inkSoft)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background {
                        if on {
                            RoundedRectangle(cornerRadius: 26, style: .continuous).fill(.white)
                                .shadow(color: Theme.shadowInk.opacity(0.1), radius: 1.5, y: 1)
                                .matchedGeometryEffect(id: "tab", in: pill)
                        }
                    }
                }
                .buttonStyle(.plain)
                .accessibilityLabel(tab.longTitle)
                .accessibilityAddTraits(on ? .isSelected : [])
            }
        }
        .padding(6)
        .frame(height: 62)
        .background(.ultraThinMaterial.opacity(0.9), in: RoundedRectangle(cornerRadius: 31, style: .continuous))
        .background(Theme.paper2.opacity(0.74), in: RoundedRectangle(cornerRadius: 31, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 31, style: .continuous).strokeBorder(Theme.hair2, lineWidth: 0.5))
        .shadow(color: Theme.shadowInk.opacity(0.16), radius: 15, y: 8)
        .animation(Motion.snappy, value: app.tab)
    }
}

// MARK: iPad and Mac

struct SidebarShell: View {
    @ObservedObject var room: Room
    @EnvironmentObject private var app: AppState
    @EnvironmentObject private var store: RoomStore

    var body: some View {
        ZStack {
            PaperBackground()
            HStack(spacing: 0) {
                Sidebar(room: room).frame(width: 256)
                Rectangle().fill(Theme.hair2).frame(width: 0.5)
                VStack(spacing: 0) {
                    topBar
                    GeometryReader { geo in
                        // Two-column layouts need room; the design switches at a 700 pt pane.
                        Pane(key: app.screenKey) { ScreenBody(room: room, wide: geo.size.width >= 700, showSearch: false, showPeople: false) }
                    }
                }
            }
        }
        #if os(macOS)
        .frame(minWidth: 980, minHeight: 640)
        #endif
    }

    private var topBar: some View {
        HStack(spacing: 16) {
            Text(crumb).labelStyle().lineLimit(1).frame(minWidth: 90, maxWidth: .infinity, alignment: .leading)
            SearchField(text: Binding(get: { app.query }, set: { newValue in
                // Text fields can write back an unchanged value when they gain focus; only real edits navigate.
                guard newValue != app.query else { return }
                app.query = newValue
                if !newValue.isEmpty, app.tab != .hearth || !app.path.isEmpty { app.go(.hearth) }
            }),
                        placeholder: "Names, places, months")
                .frame(minWidth: 160, maxWidth: 420)
            HStack(spacing: 8) {
                FRButton("Add photos", mark: "+") { app.showImporter = true }
                #if os(macOS)
                FRButton("Files…", style: .ghost) { NotificationCenter.default.post(name: .familyRoomImportFiles, object: nil) }
                #endif
            }
            .fixedSize()
        }
        .padding(.horizontal, 32).frame(height: 56)
        .overlay(alignment: .bottom) { Hairline() }
    }

    private var crumb: String {
        switch app.path.last {
        case .moment(let m): return "The Hearth · \(m.displayTitle)"
        case .person(let p): return "Family Circle · \(p.displayName)"
        case nil: return app.tab.longTitle
        }
    }
}

struct Sidebar: View {
    @ObservedObject var room: Room
    @EnvironmentObject private var app: AppState
    @EnvironmentObject private var store: RoomStore
    @Namespace private var navPill

    private var people: [Person] { room.peopleList.filter { !$0.isFaceGroup } }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            #if os(macOS)
            Spacer().frame(height: 28)
            #endif
            VStack(alignment: .leading, spacing: 8) {
                Text("Family Room").labelStyle()
                Text(room.displayName).font(.display(32)).foregroundStyle(Theme.ink).lineLimit(2)
            }
            .padding(.horizontal, 20).padding(.top, 26).padding(.bottom, 6)
            group("Rooms")
            nav([.hearth, .circle, .stories])
            group("Workshop")
            nav([.settings])
            HStack { Text("Family").labelStyle(); Spacer()
                if !app.selectedPeople.isEmpty { Button("Clear") { app.selectedPeople = [] }.buttonStyle(.plain).font(.sans(11.5, .medium)).foregroundStyle(Theme.warm) } }
                .padding(.horizontal, 20).padding(.top, 22).padding(.bottom, 7)
            VStack(spacing: 1) {
                ForEach(people, id: \.objectID) { p in
                    let id = p.id ?? UUID(), on = app.selectedPeople.contains(id)
                    Button { withAnimation(Motion.snappy) { app.toggle(id) }; Feedback.tick() } label: {
                        HStack(spacing: 10) {
                            AvatarView(person: p.summary, size: 28, ring: on, dim: !app.selectedPeople.isEmpty && !on)
                            Text(p.displayName).font(.sans(13.5, .medium)).foregroundStyle(Theme.ink).lineLimit(1)
                            Spacer()
                            Text(p.photoCount.formatted()).font(.num(11)).foregroundStyle(Theme.inkMute)
                        }
                        .padding(.horizontal, 10).frame(minHeight: 40)
                        .background(on ? Color.white.opacity(0.8) : .clear, in: RoundedRectangle(cornerRadius: 7, style: .continuous))
                        .overlay { if on { RoundedRectangle(cornerRadius: 7, style: .continuous).strokeBorder(Theme.hair2, lineWidth: 0.5) } }
                    }
                    .buttonStyle(.plain)
                }
                if people.isEmpty {
                    Text("Add people in Family Circle").font(.sans(12)).foregroundStyle(Theme.inkMute).padding(.horizontal, 10).padding(.vertical, 6)
                }
            }
            .padding(.horizontal, 10)
            Spacer()
            syncPill.padding(14)
        }
        .background(Theme.paper2.opacity(0.42))
        .task(id: room.objectID) { await store.sharing.refresh(for: room) }
    }

    private func group(_ title: String) -> some View {
        Text(title).labelStyle().padding(.horizontal, 20).padding(.top, 22).padding(.bottom, 7)
    }

    private func nav(_ tabs: [AppTab]) -> some View {
        VStack(spacing: 1) {
            ForEach(tabs) { tab in
                let on = app.tab == tab
                Button { withAnimation(Motion.snappy) { app.go(tab) } } label: {
                    HStack(spacing: 10) {
                        Text(tab.number).font(.label(10.5)).foregroundStyle(on ? Theme.warm : Theme.inkFaint).frame(width: 20, alignment: .leading)
                        Text(tab.longTitle).font(.sans(14, .medium)).foregroundStyle(on ? Theme.ink : Theme.inkSoft)
                        Spacer()
                    }
                    .padding(.horizontal, 10).frame(minHeight: 36)
                    .background {
                        if on {
                            RoundedRectangle(cornerRadius: 7, style: .continuous).fill(.white.opacity(0.8))
                                .overlay(RoundedRectangle(cornerRadius: 7, style: .continuous).strokeBorder(Theme.hair2, lineWidth: 0.5))
                                .matchedGeometryEffect(id: "nav", in: navPill)
                        }
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 10)
    }

    private var syncPill: some View {
        let members = store.sharing.members
        let syncing = members.filter { $0.status == .syncing || $0.status == .you }.count
        return HStack(spacing: 10) {
            Circle().fill(store.sharing.available ? Theme.good : Theme.inkFaint).frame(width: 7, height: 7).shadow(color: Theme.good.opacity(0.3), radius: 3)
            Text(store.sharing.available ? (members.isEmpty ? "Only you" : "\(syncing) of \(members.count) syncing") : "On this \(DeviceWord.current)").font(.num(12)).foregroundStyle(Theme.inkSoft)
            Spacer()
            Text("iCloud").labelStyle(size: 10)
        }
        .padding(.horizontal, 12).padding(.vertical, 10)
        .background(.white.opacity(0.5), in: RoundedRectangle(cornerRadius: 9, style: .continuous))
        .hairlineBorder(9)
    }
}

// MARK: Foldable iPhone, open

struct DuoShell: View {
    @ObservedObject var room: Room
    @EnvironmentObject private var app: AppState
    @Namespace private var dockPill

    private var split: Bool { app.tab == .hearth || app.tab == .circle }

    var body: some View {
        ZStack {
            PaperBackground()
            HStack(spacing: 0) {
                if split {
                    Pane(key: app.tab.rawValue) {
                        if app.tab == .hearth {
                            HearthView(room: room, listOnly: true) { app.duoMoment = $0; app.stageMomentID = $0.id }
                        } else {
                            CircleView(room: room, activeID: app.duoPerson?.id) { app.duoPerson = $0 }
                        }
                    }
                    .frame(width: 420)
                    Pane(key: detailKey) {
                        if app.tab == .hearth, let m = app.duoMoment ?? room.momentList.last(where: \.isFeatured) ?? room.momentList.last {
                            MomentView(moment: m, hideBack: true).id(m.objectID)
                        } else if app.tab == .circle, let p = app.duoPerson ?? room.peopleList.first(where: { !$0.isFaceGroup }) ?? room.peopleList.first {
                            PersonView(person: p, hideBack: true, onOpen: { app.duoMoment = $0; app.tab = .hearth }).id(p.objectID)
                        } else {
                            VStack { Spacer(); Text("Pick something on the left.").font(.sans(13)).foregroundStyle(Theme.inkMute); Spacer() }
                        }
                    }
                    .background(Color.white.opacity(0.22))
                } else {
                    Pane(key: app.tab.rawValue) {
                        switch app.tab {
                        case .stories: StoriesView(room: room, wide: true)
                        default: SettingsView(room: room, wide: true)
                        }
                    }
                }
                dock
            }
        }
    }

    private var detailKey: String {
        app.tab == .hearth ? "m-\(app.duoMoment?.objectID.uriRepresentation().absoluteString ?? "")" : "p-\(app.duoPerson?.objectID.uriRepresentation().absoluteString ?? "")"
    }

    private var dock: some View {
        VStack(spacing: 6) {
            ForEach(AppTab.allCases) { tab in
                let on = app.tab == tab
                Button { withAnimation(Motion.snappy) { app.go(tab) }; Feedback.tick() } label: {
                    VStack(spacing: 3) {
                        Text(tab.number).font(.label(9.5)).foregroundStyle(on ? Theme.warm : Theme.inkFaint)
                        Text(tab.title).font(.sans(11, .semibold)).foregroundStyle(on ? Theme.ink : Theme.inkSoft)
                    }
                    .frame(maxWidth: .infinity).frame(height: 58)
                    .background {
                        if on { RoundedRectangle(cornerRadius: 16, style: .continuous).fill(.white).matchedGeometryEffect(id: "dock", in: dockPill) }
                    }
                }
                .buttonStyle(.plain)
            }
        }
        .padding(8).frame(width: 68).frame(maxHeight: .infinity)
        .background(Theme.paper2.opacity(0.6))
        .overlay(alignment: .leading) { Rectangle().fill(Theme.hair2).frame(width: 0.5) }
    }
}
