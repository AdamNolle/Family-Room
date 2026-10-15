import SwiftUI

/// Paper background: warm sunlight that drifts slowly, a few motes of dust floating in the beam,
/// and a fine film grain. Motion stops for Reduce Motion and the Workshop's "Reduce sunlight motion".
struct PaperBackground: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @AppStorage(SettingsKeys.reduceSunlight) private var reduceSunlight = false
    private var still: Bool { reduceMotion || reduceSunlight }

    var body: some View {
        ZStack {
            Theme.paper
            Sunlight(still: still)
            if !still { DustMotes() }
            FineGrain()
        }
        .ignoresSafeArea()
        .allowsHitTesting(false)
        .accessibilityHidden(true)
    }
}

/// The diagonal beam and corner glow from the design's `.sun`. The drift is a single
/// GPU-interpolated animation, so it moves continuously instead of stepping frame by frame.
private struct Sunlight: View {
    var still: Bool
    @State private var drifted = false

    var body: some View {
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
            .offset(x: -w * 0.04 + (drifted ? w * 0.02 : 0), y: -h * 0.04 + (drifted ? h * 0.01 : 0))
            .opacity(drifted ? 0.94 : 0.78)
            .blendMode(.multiply)
        }
        .onAppear { start() }
        .onChange(of: still) { _, _ in start() }
    }

    private func start() {
        if still {
            withAnimation(.easeOut(duration: 1.2)) { drifted = false }
        } else {
            drifted = false
            withAnimation(.easeInOut(duration: 32).repeatForever(autoreverses: true)) { drifted = true }
        }
    }
}

/// A handful of soft motes drifting through the sunbeam. Positions are deterministic per mote,
/// evaluated from the clock, so the motion is smooth at any frame rate.
private struct DustMotes: View {
    private struct Mote { var x, y, size, speed, sway, phase: Double }
    private static let motes: [Mote] = (0..<16).map { i in
        var seed = UInt64(i + 1) &* 0x9E3779B97F4A7C15
        func next() -> Double { seed = seed &* 6364136223846793005 &+ 1442695040888963407; return Double((seed >> 33) & 0xFFFF) / 65535 }
        return Mote(x: next(), y: next(), size: 1.5 + next() * 2.5, speed: 0.004 + next() * 0.006, sway: 0.01 + next() * 0.02, phase: next() * .pi * 2)
    }

    var body: some View {
        TimelineView(.animation(minimumInterval: 1 / 30)) { context in
            let t = context.date.timeIntervalSinceReferenceDate
            Canvas { ctx, size in
                for m in Self.motes {
                    // Drift slowly up and to the right along the beam, wrapping around.
                    let travel = (t * m.speed + m.y).truncatingRemainder(dividingBy: 1)
                    let x = (m.x + travel * 0.35 + sin(t * 0.2 + m.phase) * m.sway).truncatingRemainder(dividingBy: 1)
                    let y = 1 - travel
                    // Brightest inside the diagonal beam, fading in and out at the edges.
                    let beam = max(0, 1 - abs((x - y * 0.55) - 0.35) * 4)
                    let fade = min(travel, 1 - travel) * 4
                    let alpha = min(1, fade) * (0.25 + 0.55 * beam)
                    guard alpha > 0.02 else { continue }
                    let rect = CGRect(x: x * size.width, y: y * size.height, width: m.size, height: m.size)
                    ctx.fill(Path(ellipseIn: rect), with: .color(Color(.sRGB, red: 1, green: 0.95, blue: 0.85, opacity: alpha)))
                }
            }
        }
        .blur(radius: 0.6)
        .blendMode(.screen)
    }
}

/// Film grain at one noise sample per device pixel, so it reads as texture rather than blocks.
private struct FineGrain: View {
    @Environment(\.displayScale) private var displayScale

    var body: some View {
        if let tile = GrainTile.shared.image {
            Image(decorative: tile, scale: displayScale)
                .resizable(resizingMode: .tile)
                .blendMode(.multiply)
                .opacity(0.5)
        }
    }
}

/// One 256×256 noise tile, generated once from raw bytes.
final class GrainTile {
    static let shared = GrainTile()
    let image: CGImage?

    private init() {
        let side = 256
        var bytes = [UInt8](repeating: 255, count: side * side)
        var seed: UInt64 = 0x5EED_FA11_C0FF_EE00
        for i in 0..<bytes.count {
            seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17
            // Two samples averaged give a softer, film-like distribution around a light grey.
            let a = Double(seed & 0xFF), b = Double((seed >> 8) & 0xFF)
            let v = (a + b) / 510
            bytes[i] = UInt8(clamping: Int(255 - v * 34))
        }
        let provider = CGDataProvider(data: Data(bytes) as CFData)
        image = provider.flatMap {
            CGImage(width: side, height: side, bitsPerComponent: 8, bitsPerPixel: 8, bytesPerRow: side,
                    space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGBitmapInfo(rawValue: 0),
                    provider: $0, decode: nil, shouldInterpolate: false, intent: .defaultIntent)
        }
    }
}
