import SwiftUI
#if canImport(UIKit)
import UIKit
#else
import AppKit
import AudioToolbox
#endif

// MARK: Avatars

struct InitialAvatar: View {
    var initial: String
    var size: CGFloat = 28
    var tone: Color
    var accent: Color
    var ring = false
    var dim = false

    var body: some View {
        Text(initial)
            .font(.system(size: size * 0.42, weight: .semibold))
            .foregroundStyle(.white)
            .frame(width: size, height: size)
            .background(LinearGradient(colors: [accent, tone], startPoint: .topLeading, endPoint: .bottomTrailing), in: Circle())
            .overlay {
                if ring {
                    Circle().strokeBorder(Theme.paper2, lineWidth: 2).padding(-2)
                    Circle().strokeBorder(tone, lineWidth: 1.5).padding(-3.5)
                }
            }
            .shadow(color: .black.opacity(ring ? 0 : 0.12), radius: 0.5, y: 1)
            .opacity(dim ? 0.4 : 1)
            .animation(.easeInOut(duration: 0.22), value: dim)
            .animation(.easeInOut(duration: 0.22), value: ring)
    }
}

struct AvatarView: View {
    var person: PersonSummary
    var size: CGFloat = 28
    var ring = false
    var dim = false
    var body: some View {
        InitialAvatar(initial: person.initial, size: size, tone: Color(hexString: person.toneHex), accent: Color(hexString: person.accentHex), ring: ring, dim: dim)
    }
}

/// Neutral avatar for people we only know by name (iCloud members, note authors).
struct NameAvatar: View {
    var name: String
    var size: CGFloat = 28
    var body: some View {
        let palette = Theme.personTones[abs(name.hashValue) % Theme.personTones.count]
        InitialAvatar(initial: String(name.prefix(1)).uppercased(), size: size, tone: Color(hex: palette.tone), accent: Color(hex: palette.accent))
    }
}

struct AvatarStack: View {
    var people: [PersonSummary]
    var size: CGFloat = 24
    var max: Int = 5
    var body: some View {
        HStack(spacing: -6) {
            ForEach(Array(people.prefix(max))) { p in
                AvatarView(person: p, size: size)
                    .overlay(Circle().strokeBorder(Theme.paper2, lineWidth: 2))
            }
        }
    }
}

/// The large round initial used for face groups (`.fc` / `.face .c`).
struct FaceInitial: View {
    var text: String
    var size: CGFloat = 64
    var body: some View {
        Text(text)
            .font(.display(size * 0.45))
            .foregroundStyle(Theme.inkSoft)
            .frame(width: size, height: size)
            .background(Theme.paper2, in: Circle())
            .overlay(Circle().strokeBorder(Theme.hair2, lineWidth: 0.5))
            .shadow(color: Theme.shadowInk.opacity(0.08), radius: 7, y: 3)
    }
}

// MARK: Buttons

enum FRButtonStyle { case solid, ghost }
enum FRButtonSize { case regular, small }

struct FRButton: View {
    var title: String
    var mark: String? = nil
    var style: FRButtonStyle = .solid
    var size: FRButtonSize = .regular
    var disabled = false
    var action: () -> Void

    init(_ title: String, mark: String? = nil, style: FRButtonStyle = .solid, size: FRButtonSize = .regular, disabled: Bool = false, action: @escaping () -> Void) {
        self.title = title; self.mark = mark; self.style = style; self.size = size; self.disabled = disabled; self.action = action
    }

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                if let mark {
                    Text(mark).font(.system(size: 16, weight: .medium))
                        .foregroundStyle(style == .solid ? Color(hex: 0xD9A08C) : Theme.warm)
                }
                Text(title).lineLimit(1)
            }
            .font(.sans(size == .small ? 12.5 : 13.5, .semibold))
            .foregroundStyle(style == .solid ? Theme.paper2 : Theme.ink)
            .padding(.horizontal, size == .small ? 10 : 15)
            .frame(minHeight: size == .small ? 28 : 34)
            .background {
                if style == .solid { RoundedRectangle(cornerRadius: 9, style: .continuous).fill(Theme.ink) }
                else { RoundedRectangle(cornerRadius: 9, style: .continuous).fill(.white.opacity(0.55)).hairlineBorder(9) }
            }
            .opacity(disabled ? 0.35 : 1)
        }
        .buttonStyle(PressScale())
        .disabled(disabled)
    }
}

struct PressScale: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.965 : 1)
            .animation(Motion.snappy, value: configuration.isPressed)
    }
}

struct CloseButton: View {
    var action: () -> Void
    var body: some View {
        Button(action: action) {
            Image(systemName: "xmark").font(.system(size: 13, weight: .medium))
                .foregroundStyle(Theme.inkSoft)
                .frame(width: 32, height: 32)
                .background(.white.opacity(0.45), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                .hairlineBorder(8)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Close")
    }
}

struct BackButton: View {
    var title: String
    var action: () -> Void
    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) { Image(systemName: "arrow.left").font(.system(size: 12, weight: .semibold)); Text(title) }
                .font(.sans(13, .medium)).foregroundStyle(Theme.inkSoft)
                .frame(minHeight: 36)
        }
        .buttonStyle(.plain)
    }
}

// MARK: Fields

struct SearchField: View {
    @Binding var text: String
    var placeholder: String
    var tall = false
    @FocusState private var focused: Bool

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: "magnifyingglass").font(.system(size: 12, weight: .medium)).foregroundStyle(Theme.inkMute).accessibilityHidden(true)
            TextField(placeholder, text: $text)
                .textFieldStyle(.plain)
                .font(.sans(13.5))
                .foregroundStyle(Theme.ink)
                .focused($focused)
                .submitLabel(.search)
            if !text.isEmpty {
                Button("Clear") { text = "" }
                    .buttonStyle(.plain)
                    .font(.label(10.5)).foregroundStyle(Theme.inkMute)
                    .padding(.horizontal, 6).padding(.vertical, 2).hairlineBorder(5)
            }
        }
        .padding(.leading, 12).padding(.trailing, 8)
        .frame(minHeight: tall ? 44 : 34)
        .background(.white.opacity(0.55), in: RoundedRectangle(cornerRadius: tall ? 12 : 9, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: tall ? 12 : 9, style: .continuous).strokeBorder(focused ? Theme.warm : Theme.hair2, lineWidth: focused ? 1 : 0.5))
    }
}

struct FieldBox: View {
    var label: String
    @Binding var text: String
    var placeholder = ""
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label).labelStyle()
            TextField(placeholder, text: $text)
                .textFieldStyle(.plain)
                .font(.sans(15, .medium)).foregroundStyle(Theme.ink)
                .padding(.horizontal, 12).frame(minHeight: 40)
                .background(.white, in: RoundedRectangle(cornerRadius: 9, style: .continuous))
                .hairlineBorder(9)
        }
    }
}

// MARK: Layout bits

struct Hairline: View {
    var color: Color = Theme.hair
    var body: some View { Rectangle().fill(color).frame(height: 0.5) }
}

struct PageHead<Title: View>: View {
    var kicker: String
    @ViewBuilder var title: () -> Title
    var text: String? = nil
    var size: CGFloat = 50
    var trailing: AnyView? = nil

    init(kicker: String, size: CGFloat = 50, text: String? = nil, trailing: AnyView? = nil, @ViewBuilder title: @escaping () -> Title) {
        self.kicker = kicker; self.size = size; self.text = text; self.trailing = trailing; self.title = title
    }

    var body: some View {
        HStack(alignment: .bottom, spacing: 16) {
            VStack(alignment: .leading, spacing: 0) {
                Text(kicker).labelStyle(warm: true)
                title().font(.display(size)).foregroundStyle(Theme.ink).padding(.top, 10).lineSpacing(-size * 0.08)
                if let text {
                    Text(text).font(.sans(14)).lineSpacing(4).foregroundStyle(Theme.inkSoft).padding(.top, 12).frame(maxWidth: 440, alignment: .leading)
                }
            }
            if let trailing { Spacer(minLength: 0); trailing }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.bottom, 18)
        .overlay(alignment: .bottom) { Hairline() }
        .padding(.bottom, 20)
    }
}

/// Four thin corner marks around a print (`.crn`).
struct CornerCrops: View {
    var body: some View {
        GeometryReader { geo in
            let w = geo.size.width, h = geo.size.height, l: CGFloat = 10, o: CGFloat = 8
            Path { p in
                p.move(to: CGPoint(x: -o, y: -o + l)); p.addLine(to: CGPoint(x: -o, y: -o)); p.addLine(to: CGPoint(x: -o + l, y: -o))
                p.move(to: CGPoint(x: w + o - l, y: -o)); p.addLine(to: CGPoint(x: w + o, y: -o)); p.addLine(to: CGPoint(x: w + o, y: -o + l))
                p.move(to: CGPoint(x: -o, y: h + o - l)); p.addLine(to: CGPoint(x: -o, y: h + o)); p.addLine(to: CGPoint(x: -o + l, y: h + o))
                p.move(to: CGPoint(x: w + o - l, y: h + o)); p.addLine(to: CGPoint(x: w + o, y: h + o)); p.addLine(to: CGPoint(x: w + o, y: h + o - l))
            }
            .stroke(Theme.inkMute, lineWidth: 1)
        }
        .allowsHitTesting(false)
    }
}

/// A photo surface with the paper texture when empty.
struct PhotoSurface: View {
    var media: MediaItem?
    var maxSide: CGFloat = 1200
    var label: String? = nil
    var body: some View {
        ZStack(alignment: .bottomLeading) {
            if let media { MediaThumb(item: media, maxSide: maxSide) }
            else {
                LinearGradient(colors: [Color(hex: 0xDDD5C6), Color(hex: 0x7D7466)], startPoint: .topLeading, endPoint: .bottomTrailing)
                    .overlay(Image(systemName: "photo").font(.system(size: 28)).foregroundStyle(.white.opacity(0.6)))
            }
            if let label {
                Text(label).labelStyle(size: 10, color: .white.opacity(0.92)).padding(.horizontal, 12).padding(.bottom, 10)
                    .shadow(color: .black.opacity(0.35), radius: 2, y: 1)
            }
        }
        .clipShape(RoundedRectangle(cornerRadius: 2))
    }
}

/// Letterboxed film poster with the play disc (`.film`).
struct FilmFrame: View {
    var poster: Data?
    var duration: Double
    var isNew = false
    var playSize: CGFloat = 52
    var body: some View {
        ZStack {
            Theme.filmBed
            DataThumb(data: poster, maxSide: 800).saturation(0.9).brightness(-0.18)
            VStack { Rectangle().fill(Theme.filmBar).frame(height: 10); Spacer(); Rectangle().fill(Theme.filmBar).frame(height: 10) }
            Circle().fill(Theme.paper2.opacity(0.92)).frame(width: playSize, height: playSize)
                .overlay(Image(systemName: "play.fill").font(.system(size: playSize * 0.34)).foregroundStyle(Theme.ink).offset(x: playSize * 0.04))
            VStack { Spacer(); HStack { Spacer(); Text(Catalog.duration(duration)).font(.num(11, .semibold)).foregroundStyle(Theme.paper2).padding(.trailing, 12).padding(.bottom, 18) } }
            if isNew {
                VStack { HStack { Text("New").labelStyle(size: 10, color: .white).padding(.horizontal, 7).padding(.vertical, 3).background(Theme.warm, in: Capsule()).padding(.leading, 10).padding(.top, 18); Spacer() }; Spacer() }
            }
        }
        .aspectRatio(16 / 9, contentMode: .fit)
        .clipShape(RoundedRectangle(cornerRadius: 3))
    }
}

/// Toggle row in the Workshop (`.row` + `.sw`).
struct SwitchRow: View {
    var title: String
    var subtitle: String
    @Binding var isOn: Bool
    var body: some View {
        Button { isOn.toggle(); Feedback.tick() } label: {
            HStack(spacing: 14) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(title).font(.sans(14, .semibold)).foregroundStyle(Theme.ink)
                    Text(subtitle).font(.sans(12.5)).foregroundStyle(Theme.inkMute)
                }
                Spacer()
                Capsule().fill(isOn ? Theme.warm : Theme.hair2).frame(width: 40, height: 24)
                    .overlay(alignment: isOn ? .trailing : .leading) {
                        Circle().fill(.white).frame(width: 20, height: 20).shadow(color: .black.opacity(0.2), radius: 1.5, y: 1).padding(2)
                    }
                    .animation(Motion.snappy, value: isOn)
            }
            .padding(.horizontal, 18).frame(minHeight: 60)
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(.isToggle)
    }
}

struct RowsBox<Content: View>: View {
    @ViewBuilder var content: () -> Content
    var body: some View {
        VStack(spacing: 0) { content() }
            .background(.white.opacity(0.6), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
            .hairlineBorder(14)
    }
}

// MARK: Feedback

enum Feedback {
    @MainActor static func tick() {
        guard UserDefaults.standard.object(forKey: SettingsKeys.sound) as? Bool ?? true else { return }
        #if os(iOS)
        UISelectionFeedbackGenerator().selectionChanged()
        #else
        NSSound(named: "Tink")?.play()
        #endif
    }
}

// MARK: Reusable rows

/// `.irows` list row: number, date, title, avatar stack.
struct IndexRow: View {
    var number: Int
    var moment: MomentSummary
    var selected: Bool
    var dim: Bool
    var action: () -> Void
    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Text(Catalog.pad(number)).font(.label(10.5)).foregroundStyle(selected ? Theme.warm : Theme.inkFaint).frame(width: 22, alignment: .leading)
                Text(Catalog.dayLabel(moment.date)).font(.num(11.5)).foregroundStyle(Theme.inkMute).frame(width: 50, alignment: .leading)
                Text(moment.title).font(.sans(13, .medium)).foregroundStyle(Theme.ink).lineLimit(1)
                Spacer(minLength: 4)
                AvatarStack(people: Array(moment.people.prefix(2)), size: 16)
                if moment.people.count > 2 { Text("+\(moment.people.count - 2)").font(.label(10.5)).foregroundStyle(Theme.inkMute) }
            }
            .padding(.leading, 10).padding(.trailing, 6)
            .frame(minHeight: 44)
            .background(selected ? Color.white.opacity(0.62) : .clear)
            .overlay(alignment: .leading) { Rectangle().fill(Theme.warm).frame(width: 2).padding(.vertical, 8).scaleEffect(y: selected ? 1 : 0, anchor: .center) }
            .opacity(dim ? 0.28 : 1)
        }
        .buttonStyle(.plain)
        .disabled(dim)
        .animation(Motion.snappy, value: selected)
    }
}
