import SwiftUI

// MARK: - Color hex init

extension Color {
    init(hex: UInt32, opacity: Double = 1) {
        let r = Double((hex >> 16) & 0xFF) / 255
        let g = Double((hex >> 8) & 0xFF) / 255
        let b = Double(hex & 0xFF) / 255
        self.init(.sRGB, red: r, green: g, blue: b, opacity: opacity)
    }
}

// MARK: - Palette

struct FurnitureDesign {
    static let cream     = Color(hex: 0xECE4D6)
    static let ink       = Color(hex: 0x2A221A)
    static let inkSoft   = Color(hex: 0x4A4035)
    static let inkMute   = Color(hex: 0x6E6358)
    static let inkFaint  = Color(hex: 0x8A7E70)
    static let warm      = Color(hex: 0xB86348)
    static let warmSoft  = Color(hex: 0xC97A5D)
    static let good      = Color(hex: 0x4A7A4A)

    static let neuLight  = Color(.sRGB, red: 1.0,  green: 0.98, blue: 0.93, opacity: 0.95)
    static let neuDark   = Color(.sRGB, red: 0.47, green: 0.37, blue: 0.235, opacity: 0.30)

    // Sentinel-thin hairlines used as 0.5pt borders/dividers throughout.
    static let hairline       = Color(.sRGB, red: 40/255, green: 30/255, blue: 20/255, opacity: 0.12)
    static let hairlineStrong = Color(.sRGB, red: 40/255, green: 30/255, blue: 20/255, opacity: 0.22)
}

// MARK: - Neumorphism

enum NeuSize {
    case small, medium, large
    var offset: CGFloat { self == .small ? 4 : (self == .large ? 12 : 8) }
    var blur:   CGFloat { self == .small ? 9 : (self == .large ? 26 : 18) }
}

struct NeuOutModifier: ViewModifier {
    var cornerRadius: CGFloat = 18
    var size: NeuSize = .medium

    func body(content: Content) -> some View {
        content
            .background(
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .fill(FurnitureDesign.cream)
                    .shadow(color: FurnitureDesign.neuLight, radius: size.blur, x: -size.offset, y: -size.offset)
                    .shadow(color: FurnitureDesign.neuDark,  radius: size.blur, x:  size.offset, y:  size.offset)
            )
    }
}

// Carved/pressed look. iOS 17+ has native inner shadows on ShapeStyle.
struct NeuInModifier: ViewModifier {
    var cornerRadius: CGFloat = 18
    var size: NeuSize = .medium

    func body(content: Content) -> some View {
        let fill = FurnitureDesign.cream
            .shadow(.inner(color: FurnitureDesign.neuDark,  radius: size.blur / 2, x:  size.offset / 2, y:  size.offset / 2))
            .shadow(.inner(color: FurnitureDesign.neuLight, radius: size.blur / 2, x: -size.offset / 2, y: -size.offset / 2))
        return content
            .background(
                RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
                    .fill(fill)
            )
    }
}

extension View {
    func neuOut(size: NeuSize = .medium, cornerRadius: CGFloat = 18) -> some View {
        modifier(NeuOutModifier(cornerRadius: cornerRadius, size: size))
    }
    func neuIn(size: NeuSize = .medium, cornerRadius: CGFloat = 18) -> some View {
        modifier(NeuInModifier(cornerRadius: cornerRadius, size: size))
    }
}

// MARK: - Typography

extension Font {
    static func mantelDisplay(_ size: CGFloat, weight: Font.Weight = .medium, italic: Bool = false) -> Font {
        let f = Font.system(size: size, weight: weight, design: .serif)
        return italic ? f.italic() : f
    }
    static func mantelSans(_ size: CGFloat, weight: Font.Weight = .regular) -> Font {
        .system(size: size, weight: weight)
    }
    static func mantelMono(_ size: CGFloat, weight: Font.Weight = .medium) -> Font {
        .system(size: size, weight: weight, design: .monospaced)
    }
}

// MARK: - Daylight Background (sunlight + grain)

struct DaylightBackground: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        ZStack {
            FurnitureDesign.cream
                .ignoresSafeArea()

            SunlightLayer(reduceMotion: reduceMotion)
                .ignoresSafeArea()
                .allowsHitTesting(false)

            CoarseGrainLayer()
                .ignoresSafeArea()
                .allowsHitTesting(false)
                .blendMode(.softLight)
                .opacity(0.32)

            FineGrainLayer()
                .ignoresSafeArea()
                .allowsHitTesting(false)
                .blendMode(.overlay)
                .opacity(0.42)
        }
    }
}

// MARK: Sunlight

private struct SunlightLayer: View {
    let reduceMotion: Bool

    var body: some View {
        if reduceMotion {
            sunlight(driftX: 0, driftY: 0, alpha: 0.85)
        } else {
            TimelineView(.animation(minimumInterval: 1.0 / 30.0)) { context in
                // 32s ease-in-out alternating loop. Phase maps to 0...1...0 over 64s.
                let t = context.date.timeIntervalSinceReferenceDate
                let cycle = (t.truncatingRemainder(dividingBy: 64)) / 32
                let phase = cycle <= 1 ? cycle : 2 - cycle
                let eased = easeInOut(phase)
                let driftX = eased * 0.02         // 0...2% horizontal nudge
                let driftY = eased * 0.01         // 0...1% vertical nudge
                let alpha  = 0.78 + (eased * 0.14)  // 0.78...0.92

                sunlight(driftX: CGFloat(driftX), driftY: CGFloat(driftY), alpha: alpha)
            }
        }
    }

    @ViewBuilder
    private func sunlight(driftX: CGFloat, driftY: CGFloat, alpha: Double) -> some View {
        ZStack {
            // Primary diagonal beam — 118°
            beam(angle: 118, stops: [
                (0,    Color.white),
                (0.28, Color.white),
                (0.36, Color(.sRGB, red: 255/255, green: 228/255, blue: 180/255, opacity: 0.78)),
                (0.42, Color(.sRGB, red: 255/255, green: 210/255, blue: 150/255, opacity: 0.62)),
                (0.52, Color.white),
                (1.0,  Color.white)
            ])
            .blendMode(.multiply)
            .offset(x: driftX * 24, y: driftY * 24)

            // Secondary thin ray — 112°
            beam(angle: 112, stops: [
                (0,    Color.white),
                (0.58, Color.white),
                (0.64, Color(.sRGB, red: 255/255, green: 225/255, blue: 170/255, opacity: 0.55)),
                (0.70, Color.white),
                (1.0,  Color.white)
            ])
            .blendMode(.multiply)
            .offset(x: -driftX * 24, y: driftY * 18)

            // Top-left ellipse warm vignette
            GeometryReader { geo in
                let w = geo.size.width
                let h = geo.size.height
                RadialGradient(
                    stops: [
                        .init(color: Color(.sRGB, red: 255/255, green: 210/255, blue: 150/255, opacity: 0.55), location: 0),
                        .init(color: Color(.sRGB, red: 255/255, green: 235/255, blue: 200/255, opacity: 0.92), location: 0.35),
                        .init(color: Color.white, location: 0.70)
                    ],
                    center: UnitPoint(x: 0.08 + driftX, y: -0.08 + driftY),
                    startRadius: 0,
                    endRadius: max(w, h) * 0.78
                )
            }
        }
        .opacity(alpha)
    }

    @ViewBuilder
    private func beam(angle: Double, stops: [(Double, Color)]) -> some View {
        let gradientStops = stops.map { Gradient.Stop(color: $0.1, location: $0.0) }
        let radians = angle * .pi / 180.0
        let dx = cos(radians) * 0.5
        let dy = sin(radians) * 0.5
        LinearGradient(
            gradient: Gradient(stops: gradientStops),
            startPoint: UnitPoint(x: 0.5 - dx, y: 0.5 - dy),
            endPoint:   UnitPoint(x: 0.5 + dx, y: 0.5 + dy)
        )
    }

    private func easeInOut(_ t: Double) -> Double {
        // smoothstep 3t² - 2t³
        let x = min(max(t, 0), 1)
        return x * x * (3 - 2 * x)
    }
}

// MARK: Grain

// A cached, Canvas-rendered noise field. Regenerates only when the view size changes.
private struct FineGrainLayer: View {
    @State private var cached: Image? = nil
    @State private var cachedSize: CGSize = .zero

    var body: some View {
        GeometryReader { geo in
            ZStack {
                if let cached, cachedSize == geo.size {
                    cached.resizable()
                } else {
                    Canvas { context, _ in
                        // Draw nothing here — placeholder until renderer fills cache.
                    }
                    .onAppear { regenerate(size: geo.size) }
                }
            }
            .onChange(of: geo.size) { _, new in regenerate(size: new) }
        }
    }

    private func regenerate(size: CGSize) {
        guard size.width > 1, size.height > 1 else { return }
        let renderer = ImageRenderer(content:
            Canvas(rendersAsynchronously: false) { ctx, sz in
                ctx.fill(Path(CGRect(origin: .zero, size: sz)), with: .color(Color(.sRGB, red: 0.47, green: 0.47, blue: 0.47, opacity: 1)))
                var seed: UInt64 = 0xFAE_4_D6
                let step: CGFloat = 1
                var y: CGFloat = 0
                while y < sz.height {
                    var x: CGFloat = 0
                    while x < sz.width {
                        seed = seed &* 6364136223846793005 &+ 1442695040888963407
                        let r = Double((seed >> 32) & 0xFFFF) / 65535.0
                        // bias toward mid-grey, with extremes for "sharpness"
                        let v = 0.35 + (r * 0.55)
                        ctx.fill(
                            Path(CGRect(x: x, y: y, width: step, height: step)),
                            with: .color(Color(.sRGB, white: v, opacity: 1))
                        )
                        x += step
                    }
                    y += step
                }
            }
            .frame(width: size.width, height: size.height)
        )
        renderer.scale = 1
        if let cg = renderer.cgImage {
            cached = Image(decorative: cg, scale: 1)
            cachedSize = size
        }
    }
}

// Coarser halftone — two staggered dot patterns. No animation, lightweight.
private struct CoarseGrainLayer: View {
    var body: some View {
        Canvas { context, size in
            let darkA = Color(.sRGB, red: 40/255, green: 30/255, blue: 20/255, opacity: 0.55)
            let darkB = Color(.sRGB, red: 40/255, green: 30/255, blue: 20/255, opacity: 0.35)
            var y: CGFloat = 0
            while y < size.height {
                var x: CGFloat = 0
                while x < size.width {
                    context.fill(Path(ellipseIn: CGRect(x: x, y: y, width: 0.6, height: 0.6)), with: .color(darkA))
                    x += 3
                }
                y += 3
            }
            y = 1.5
            while y < size.height {
                var x: CGFloat = 1.5
                while x < size.width {
                    context.fill(Path(ellipseIn: CGRect(x: x, y: y, width: 0.6, height: 0.6)), with: .color(darkB))
                    x += 5
                }
                y += 5
            }
        }
    }
}
