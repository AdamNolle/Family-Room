import SwiftUI

/// Paper background with the drifting sunlight beam and a fine grain, as in the design's `.sun` and `.grain`.
struct PaperBackground: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @AppStorage(SettingsKeys.reduceSunlight) private var reduceSunlight = false

    var body: some View {
        ZStack {
            Theme.paper
            if reduceMotion || reduceSunlight {
                Sunlight(phase: 0.5)
            } else {
                TimelineView(.animation(minimumInterval: 1 / 20)) { context in
                    let t = context.date.timeIntervalSinceReferenceDate
                    let cycle = t.truncatingRemainder(dividingBy: 64) / 32
                    Sunlight(phase: cycle <= 1 ? cycle : 2 - cycle)
                }
            }
            Grain()
                .blendMode(.multiply)
                .opacity(0.26)
        }
        .ignoresSafeArea()
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }
}

private struct Sunlight: View {
    let phase: Double

    var body: some View {
        let eased = phase * phase * (3 - 2 * phase)
        GeometryReader { geo in
            let w = geo.size.width, h = geo.size.height
            ZStack {
                LinearGradient(stops: [
                    .init(color: .clear, location: 0.28),
                    .init(color: Color(.sRGB, red: 1, green: 214/255, blue: 160/255, opacity: 0.45), location: 0.36),
                    .init(color: Color(.sRGB, red: 1, green: 200/255, blue: 140/255, opacity: 0.30), location: 0.42),
                    .init(color: .clear, location: 0.52)
                ], startPoint: UnitPoint(x: 0.5 - cos(118 * .pi / 180) * 0.5, y: 0.5 - sin(118 * .pi / 180) * 0.5),
                   endPoint: UnitPoint(x: 0.5 + cos(118 * .pi / 180) * 0.5, y: 0.5 + sin(118 * .pi / 180) * 0.5))
                RadialGradient(stops: [
                    .init(color: Color(.sRGB, red: 1, green: 206/255, blue: 146/255, opacity: 0.45), location: 0),
                    .init(color: Color(.sRGB, red: 1, green: 232/255, blue: 196/255, opacity: 0.15), location: 0.35),
                    .init(color: .clear, location: 0.7)
                ], center: UnitPoint(x: 0.08, y: -0.08), startRadius: 0, endRadius: max(w, h) * 0.7)
            }
            .frame(width: w * 1.08, height: h * 1.08)
            .offset(x: -w * 0.04 + w * 0.02 * eased, y: -h * 0.04 + h * 0.01 * eased)
            .opacity(0.78 + 0.16 * eased)
            .blendMode(.multiply)
        }
    }
}

/// Deterministic noise tile, rendered once and repeated.
private struct Grain: View {
    @State private var tile: Image?

    var body: some View {
        GeometryReader { _ in
            if let tile {
                tile.resizable(resizingMode: .tile)
            } else {
                Color.clear.onAppear { tile = Grain.makeTile() }
            }
        }
    }

    private static func makeTile() -> Image? {
        let size = 180
        let renderer = ImageRenderer(content: Canvas(rendersAsynchronously: false) { ctx, _ in
            var seed: UInt64 = 0x5EED_FA11
            for y in stride(from: 0, to: size, by: 2) {
                for x in stride(from: 0, to: size, by: 2) {
                    seed = seed &* 6364136223846793005 &+ 1442695040888963407
                    let r = Double((seed >> 33) & 0xFFFF) / 65535
                    let v = 0.78 + r * 0.22
                    ctx.fill(Path(CGRect(x: x, y: y, width: 2, height: 2)),
                             with: .color(Color(.sRGB, red: v, green: v * 0.985, blue: v * 0.96, opacity: 1)))
                }
            }
        }.frame(width: CGFloat(size), height: CGFloat(size)))
        renderer.scale = 1
        guard let cg = renderer.cgImage else { return nil }
        return Image(decorative: cg, scale: 1)
    }
}
