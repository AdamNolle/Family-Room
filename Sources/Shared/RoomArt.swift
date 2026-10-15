import SwiftUI
import CoreText

enum RoomArt {
    static let bundle: Bundle = {
        #if SWIFT_PACKAGE
        if let url = Bundle.main.url(forResource: "FamilyRoom_FamilyRoom", withExtension: "bundle"), let packaged = Bundle(url: url) { return packaged }
        return .module
        #else
        return .main
        #endif
    }()
    static func registerFonts() {
        guard let url = bundle.url(forResource: "InstrumentSerif-Regular", withExtension: "ttf") else { return }
        CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
    }
    static func display(_ size: CGFloat, relativeTo style: Font.TextStyle = .largeTitle) -> Font {
        .custom("InstrumentSerif-Regular", size: size, relativeTo: style)
    }
}

/// Quiet gallery canvas. Color is confined to the photographs and Room accent.
struct MemoryGalleryBackdrop: View {
    var body: some View {
        CozyTheme.canvas.accessibilityHidden(true).allowsHitTesting(false)
    }
}

struct MemoryGlassPanel: ViewModifier {
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    func body(content: Content) -> some View {
        Group {
            if reduceTransparency {
                content.background(CozyTheme.surface, in: RoundedRectangle(cornerRadius: 16))
            } else if #available(iOS 26, macOS 26, *) {
                content.glassEffect(.regular.tint(CozyTheme.glassTint.opacity(0.16)), in: .rect(cornerRadius: 16))
            } else {
                content.background {
                    RoundedRectangle(cornerRadius: 16).fill(.regularMaterial)
                }
            }
        }
            .overlay { RoundedRectangle(cornerRadius: 16).strokeBorder(.white.opacity(0.65), lineWidth: 0.7) }
            .shadow(color: .black.opacity(0.06), radius: 16, y: 7)
    }
}

struct PhotoTray<Content: View>: View {
    let title: String
    var subtitle: String
    @ViewBuilder var content: () -> Content
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            ViewThatFits(in: .horizontal) {
                HStack(alignment: .firstTextBaseline, spacing: 16) { Text(title).font(RoomArt.display(30, relativeTo: .title)); Spacer(); metadata }
                VStack(alignment: .leading, spacing: 8) { Text(title).font(RoomArt.display(30, relativeTo: .title)); metadata }
            }
            Rectangle().fill(CozyTheme.line).frame(height: 0.5)
            content()
        }
        .foregroundStyle(CozyTheme.ink)
    }
    private var metadata: some View {
        Text(subtitle.uppercased()).font(.system(.caption, design: .monospaced)).tracking(1.1).foregroundStyle(CozyTheme.secondary)
    }
}

extension View {
    func memoryGlass() -> some View { modifier(MemoryGlassPanel()) }
}

/// A featured memory with a separate, browsable strip; never a portfolio mosaic.
struct MemoryHighlights: View {
    let assets: [LibraryAsset]
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var stackHasEntered = false
    #if os(iOS)
    @Environment(\.horizontalSizeClass) private var sizeClass
    #endif
    private var featuredHeight: CGFloat {
        #if os(macOS)
        320
        #else
        sizeClass == .compact ? 240 : 340
        #endif
    }
    var body: some View {
        if let first = assets.first {
            VStack(alignment: .leading, spacing: 18) {
                GeometryReader { geometry in
                featuredStack(first, size: geometry.size)
                }.frame(height: featuredHeight)
                if assets.count > 1 {
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 14) {
                            ForEach(Array(assets.dropFirst().prefix(4))) { asset in
                                photograph(asset, size: CGSize(width: 160, height: 108))
                            }
                        }.padding(.vertical, 3)
                    }
                }
            }
        }
    }
    private func featuredStack(_ first: LibraryAsset, size: CGSize) -> some View {
        ZStack {
            if assets.count > 2 {
                decorativePrint(assets[2], size: size)
                    .scaleEffect(0.84)
                    .rotationEffect(.degrees(3.5))
                    .offset(x: size.width * 0.12, y: -10)
            }
            if assets.count > 1 {
                decorativePrint(assets[1], size: size)
                    .scaleEffect(0.92)
                    .rotationEffect(.degrees(-2.5))
                    .offset(x: -size.width * 0.08, y: 7)
            }
            photograph(first, size: size)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .scaleEffect(reduceMotion || stackHasEntered ? 1 : 0.96)
        .rotation3DEffect(
            .degrees(reduceMotion || stackHasEntered ? 0 : -4),
            axis: (x: 1, y: -1, z: 0),
            perspective: 0.7
        )
        .offset(y: reduceMotion || stackHasEntered ? 0 : 16)
        .opacity(reduceMotion || stackHasEntered ? 1 : 0)
        .onAppear {
            guard !reduceMotion else {
                stackHasEntered = true
                return
            }
            withAnimation(.spring(response: 0.62, dampingFraction: 0.78).delay(0.08)) {
                stackHasEntered = true
            }
        }
        .onChange(of: reduceMotion) { _, isReduced in
            if isReduced { stackHasEntered = true }
        }
    }

    private func decorativePrint(_ asset: LibraryAsset, size: CGSize) -> some View {
        AssetPreview(asset: asset)
            .frame(width: max(0, size.width - 10), height: max(0, size.height - 10))
            .clipped()
            .memoryPhotoFrame()
            .accessibilityHidden(true)
            .allowsHitTesting(false)
    }

    private func photograph(_ asset: LibraryAsset, size: CGSize) -> some View {
        NavigationLink { AssetDetailView(assetId: asset.id, roomId: asset.roomId) } label: {
            AssetPreview(asset: asset)
                .frame(width: max(0, size.width - 10), height: max(0, size.height - 10))
                .clipped()
                .overlay(alignment: .bottomLeading) {
                    if asset.kind == "video" { Image(systemName: "play.fill").font(.caption).padding(10).background(.black.opacity(0.65), in: Circle()).padding(10).foregroundStyle(.white) }
                }
                .memoryPhotoFrame()
                .memoryPhotoMotion()
        }.buttonStyle(MemoryPhotoButtonStyle())
            .accessibilityLabel("Open \(asset.displayTitle), \(asset.date.formatted(date: .abbreviated, time: .omitted))")
    }
}

/// Black keyline, white mat, black outer edge. The image itself keeps its colors.
struct MemoryPhotoFrame: ViewModifier {
    var radius: CGFloat = 8
    func body(content: Content) -> some View {
        content
            .clipShape(RoundedRectangle(cornerRadius: max(0, radius - 5)))
            .overlay { RoundedRectangle(cornerRadius: max(0, radius - 5)).strokeBorder(.black, lineWidth: 0.7) }
            .padding(5)
            .background(.white, in: RoundedRectangle(cornerRadius: radius))
            .overlay { RoundedRectangle(cornerRadius: radius).strokeBorder(.black, lineWidth: 1) }
            .shadow(color: .black.opacity(0.09), radius: 6, y: 3)
    }
}

/// Movement is tied to a person pointing at or pressing a memory.
struct MemoryPhotoMotion: ViewModifier {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var hovering = false
    func body(content: Content) -> some View {
        content
            .scaleEffect(hovering && !reduceMotion ? 1.018 : 1)
            .rotation3DEffect(.degrees(hovering && !reduceMotion ? 2.2 : 0), axis: (x: 1, y: -1, z: 0), perspective: 0.45)
            .animation(.spring(response: 0.38, dampingFraction: 0.78), value: hovering)
            .onHover { hovering = $0 }
    }
}

struct MemoryPhotoButtonStyle: ButtonStyle {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed && !reduceMotion ? 0.985 : 1)
            .animation(.spring(response: 0.23, dampingFraction: 0.72), value: configuration.isPressed)
    }
}

struct MemoryIconTile: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    let symbol: String
    var size: CGFloat = 48
    var selected = false

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: size * 0.24)
                .fill(CozyTheme.ink.opacity(selected ? 0.12 : 0.07))
                .offset(y: size * 0.045)

            RoundedRectangle(cornerRadius: size * 0.24)
                .fill(
                    LinearGradient(
                        colors: selected
                            ? [CozyTheme.seasonalHighlight, CozyTheme.accent]
                            : [CozyTheme.surface, CozyTheme.glassTint],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )

            Ellipse()
                .fill(Color.white.opacity(selected ? 0.30 : 0.54))
                .frame(width: size * 0.72, height: size * 0.36)
                .blur(radius: size * 0.06)
                .offset(x: -size * 0.16, y: -size * 0.32)
                .clipShape(RoundedRectangle(cornerRadius: size * 0.24))

            RoundedRectangle(cornerRadius: size * 0.24)
                .strokeBorder(
                    LinearGradient(
                        colors: [Color.white.opacity(0.88), CozyTheme.line.opacity(0.82)],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    ),
                    lineWidth: max(0.7, size * 0.018)
                )

            Image(systemName: symbol)
                .font(.system(size: size * 0.45, weight: .semibold))
                .symbolRenderingMode(.hierarchical)
                .foregroundStyle(selected ? CozyTheme.onAccent : CozyTheme.accentInk)
                .shadow(color: .black.opacity(selected ? 0.10 : 0.07), radius: 1, y: 1)
        }
        .frame(width: size, height: size)
        .clipShape(RoundedRectangle(cornerRadius: size * 0.24))
        .shadow(color: .black.opacity(selected ? 0.13 : 0.08), radius: size * 0.10, y: size * 0.055)
        .scaleEffect(selected && !reduceMotion ? 1.055 : 1)
        .rotation3DEffect(
            .degrees(selected && !reduceMotion ? -4 : 0),
            axis: (x: 1, y: -1, z: 0),
            perspective: 0.42
        )
        .animation(.spring(response: 0.38, dampingFraction: 0.68), value: selected)
        .accessibilityHidden(true)
    }
}

extension View {
    func memoryPhotoFrame(radius: CGFloat = 8) -> some View { modifier(MemoryPhotoFrame(radius: radius)) }
    func memoryPhotoMotion() -> some View { modifier(MemoryPhotoMotion()) }
}

struct MemoryPageHeading: View {
    let room: String?
    let title: String
    let detail: String
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            RoomEyebrow(text: room ?? "Your private library")
            Text(title).font(RoomArt.display(36)).foregroundStyle(CozyTheme.ink).fixedSize(horizontal: false, vertical: true)
            Text(detail).foregroundStyle(CozyTheme.secondary).frame(maxWidth: 600, alignment: .leading)
        }.frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct MemoryEmptyState: View {
    let icon: String
    let title: String
    let detail: String
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            MemoryIconTile(symbol: icon, size: 58)
            Text(title).font(RoomArt.display(30, relativeTo: .title2)).foregroundStyle(CozyTheme.ink)
            Text(detail).foregroundStyle(CozyTheme.secondary).frame(maxWidth: 520, alignment: .leading)
        }.padding(26).frame(maxWidth: .infinity, alignment: .leading).cozyPanel()
    }
}
