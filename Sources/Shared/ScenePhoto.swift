import SwiftUI

// A photo-feeling placeholder: gradient background + simple "content" shapes per scene.
// Ported from familyroom.jsx Photo({ scene }) — sunset, lake, indoor, cake, garden, bike,
// snow, stage, fall, wolf, table, studio.
struct ScenePhoto: View {
    let scene: PhotoScene
    var cornerRadius: CGFloat = 10

    var body: some View {
        GeometryReader { geo in
            let w = geo.size.width
            let h = geo.size.height
            ZStack {
                background()
                content(width: w, height: h)
                highlight()
            }
            .clipShape(RoundedRectangle(cornerRadius: cornerRadius, style: .continuous))
        }
    }

    // ── Top-left soft highlight (cheap "window light" on every photo) ──
    @ViewBuilder
    private func highlight() -> some View {
        RadialGradient(
            colors: [Color.white.opacity(0.3), .clear],
            center: UnitPoint(x: 0.3, y: 0.2),
            startRadius: 0,
            endRadius: 120
        )
        .opacity(0.12)
        .allowsHitTesting(false)
    }

    // ── Background gradient (one per scene) ──
    @ViewBuilder
    private func background() -> some View {
        switch scene {
        case .sunset:
            LinearGradient(stops: [
                .init(color: Color(hex: 0xF5C896), location: 0.0),
                .init(color: Color(hex: 0xE89968), location: 0.35),
                .init(color: Color(hex: 0xC46E52), location: 0.70),
                .init(color: Color(hex: 0x6E4838), location: 1.0)
            ], startPoint: .top, endPoint: .bottom)
        case .lake:
            LinearGradient(stops: [
                .init(color: Color(hex: 0xB8D4D6), location: 0.0),
                .init(color: Color(hex: 0x7EA8A8), location: 0.5),
                .init(color: Color(hex: 0x2C4A4F), location: 1.0)
            ], startPoint: .top, endPoint: .bottom)
        case .indoor:
            LinearGradient(stops: [
                .init(color: Color(hex: 0x6E4F3A), location: 0.0),
                .init(color: Color(hex: 0x4A3326), location: 0.5),
                .init(color: Color(hex: 0x2A1C14), location: 1.0)
            ], startPoint: .topLeading, endPoint: .bottomTrailing)
        case .cake:
            LinearGradient(stops: [
                .init(color: Color(hex: 0x8A3C3C), location: 0.0),
                .init(color: Color(hex: 0x5A2828), location: 1.0)
            ], startPoint: .top, endPoint: .bottom)
        case .garden:
            LinearGradient(stops: [
                .init(color: Color(hex: 0xC4D49A), location: 0.0),
                .init(color: Color(hex: 0x7A8A4A), location: 0.6),
                .init(color: Color(hex: 0x3A4A2A), location: 1.0)
            ], startPoint: .top, endPoint: .bottom)
        case .bike:
            LinearGradient(stops: [
                .init(color: Color(hex: 0xA0C4D4), location: 0.0),
                .init(color: Color(hex: 0x6A94A8), location: 0.5),
                .init(color: Color(hex: 0x3A5060), location: 1.0)
            ], startPoint: .top, endPoint: .bottom)
        case .snow:
            LinearGradient(stops: [
                .init(color: Color(hex: 0xE8EEF2), location: 0.0),
                .init(color: Color(hex: 0xB8C8D4), location: 0.6),
                .init(color: Color(hex: 0x6A7A8A), location: 1.0)
            ], startPoint: .top, endPoint: .bottom)
        case .stage:
            LinearGradient(stops: [
                .init(color: Color(hex: 0x2A1838), location: 0.0),
                .init(color: Color(hex: 0x6A2848), location: 0.5),
                .init(color: Color(hex: 0xC44864), location: 1.0)
            ], startPoint: .top, endPoint: .bottom)
        case .fall:
            LinearGradient(stops: [
                .init(color: Color(hex: 0xD49060), location: 0.0),
                .init(color: Color(hex: 0xB8602C), location: 0.5),
                .init(color: Color(hex: 0x6E3818), location: 1.0)
            ], startPoint: .top, endPoint: .bottom)
        case .wolf:
            LinearGradient(stops: [
                .init(color: Color(hex: 0x1A1028), location: 0.0),
                .init(color: Color(hex: 0x2A1838), location: 0.6),
                .init(color: Color(hex: 0x4A2848), location: 1.0)
            ], startPoint: .top, endPoint: .bottom)
        case .table:
            LinearGradient(stops: [
                .init(color: Color(hex: 0x6E4828), location: 0.0),
                .init(color: Color(hex: 0x4A2818), location: 1.0)
            ], startPoint: .top, endPoint: .bottom)
        case .studio:
            LinearGradient(stops: [
                .init(color: Color(hex: 0xC8B896), location: 0.0),
                .init(color: Color(hex: 0x8A7A5A), location: 1.0)
            ], startPoint: .topLeading, endPoint: .bottomTrailing)
        }
    }

    // ── Content shapes per scene ──
    @ViewBuilder
    private func content(width w: CGFloat, height h: CGFloat) -> some View {
        switch scene {
        case .sunset:
            // Sun
            Circle()
                .fill(Color(hex: 0xFFF5E0))
                .frame(width: 22, height: 22)
                .shadow(color: Color(hex: 0xFFEBC8).opacity(0.8), radius: 12)
                .position(x: w * 0.7, y: h * 0.25)
            // Bottom shading
            LinearGradient(
                colors: [.clear, Color(hex: 0x281A0F).opacity(0.55)],
                startPoint: .top, endPoint: .bottom
            )
            .frame(height: h * 0.38)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
            // Mountain silhouette
            SunsetMountains()
                .fill(Color(hex: 0x3A2418))
                .frame(height: h * 0.4)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)

        case .lake:
            // Horizon shimmer
            Rectangle()
                .fill(Color.white.opacity(0.18))
                .frame(height: h * 0.04)
                .position(x: w / 2, y: h * 0.52)
            Rectangle()
                .fill(Color.white.opacity(0.4))
                .frame(width: w * 0.6, height: 1)
                .position(x: w / 2, y: h * 0.62)
            LakeRidge()
                .fill(Color(hex: 0x1A3030))
                .frame(height: h * 0.14)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
                .offset(y: -h * 0.18)

        case .indoor:
            // Warm lamp glow
            Circle()
                .fill(RadialGradient(
                    colors: [Color(hex: 0xFFD28A), Color(hex: 0xE8A060), .clear],
                    center: .center, startRadius: 0, endRadius: 28
                ))
                .frame(width: 30, height: 30)
                .blur(radius: 2)
                .position(x: w * 0.78, y: h * 0.22)
            Circle()
                .fill(RadialGradient(
                    colors: [Color(hex: 0xFFD28A).opacity(0.4), .clear],
                    center: .center, startRadius: 0, endRadius: 30
                ))
                .frame(width: 50, height: 50)
                .blur(radius: 8)
                .position(x: w * 0.78, y: h * 0.22)

        case .cake:
            // Cake plate
            RoundedRectangle(cornerRadius: 6)
                .fill(Color(hex: 0xF0D4B0))
                .frame(width: w * 0.6, height: h * 0.32)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
            // Candles
            HStack(spacing: 4) {
                ForEach(0..<5, id: \.self) { _ in
                    ZStack(alignment: .top) {
                        Rectangle()
                            .fill(Color(hex: 0xE8C890))
                            .frame(width: 2, height: 12)
                        Capsule()
                            .fill(Color(hex: 0xFFCC66))
                            .frame(width: 5, height: 7)
                            .shadow(color: Color(hex: 0xFFC864).opacity(0.8), radius: 4)
                            .offset(y: -7)
                    }
                }
            }
            .position(x: w / 2, y: h * 0.55)

        case .garden:
            // Three flower dots
            Circle().fill(Color(hex: 0xE8A0A0)).frame(width: 14, height: 14)
                .position(x: w * 0.35, y: h * 0.27)
            Circle().fill(Color(hex: 0xF0C0C0)).frame(width: 10, height: 10)
                .position(x: w * 0.7, y: h * 0.4)
            Circle().fill(Color(hex: 0xD488A0)).frame(width: 12, height: 12)
                .position(x: w * 0.55, y: h * 0.55)

        case .bike:
            // Simple bike outline
            BikeOutline()
                .stroke(Color(hex: 0x1A2030), lineWidth: 1.5)
                .frame(width: w * 0.6, height: h * 0.3)
                .position(x: w / 2, y: h * 0.7)

        case .snow:
            ZStack {
                ForEach(0..<20, id: \.self) { i in
                    Circle()
                        .fill(Color.white.opacity(0.7))
                        .frame(width: 2, height: 2)
                        .position(
                            x: CGFloat(((i * 13) % 100)) * w / 100,
                            y: CGFloat(((i * 7) % 100)) * h / 100
                        )
                }
            }

        case .stage:
            Ellipse()
                .fill(RadialGradient(
                    colors: [Color(hex: 0xFFD28A).opacity(0.5), .clear],
                    center: .center, startRadius: 0, endRadius: 50
                ))
                .frame(width: 80, height: 60)
                .blur(radius: 8)
                .position(x: w / 2, y: h * 0.3)
            UnevenCapsule()
                .fill(Color(hex: 0x1A0820))
                .frame(width: 18, height: 30)
                .position(x: w * 0.5, y: h * 0.75)

        case .fall:
            // Falling leaves
            ZStack {
                ForEach(0..<4, id: \.self) { i in
                    LeafShape()
                        .fill(Color(hex: 0xE8A050))
                        .frame(width: 8, height: 8)
                        .rotationEffect(.degrees(Double(i) * 40))
                        .position(
                            x: w * (0.2 + CGFloat((i * 23) % 60) / 100),
                            y: h * (0.1 + CGFloat(i) * 0.22)
                        )
                }
            }

        case .wolf:
            // Moon
            Circle()
                .fill(Color(hex: 0xE8D4A0))
                .frame(width: 16, height: 16)
                .shadow(color: Color(hex: 0xE8D4A0), radius: 8)
                .position(x: w * 0.78, y: h * 0.25)
            // Jagged hills
            JaggedHills()
                .fill(Color(hex: 0x0A0512))
                .frame(height: h * 0.5)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)

        case .table:
            Circle().fill(Color(hex: 0xD8A868)).frame(width: 18, height: 18)
                .position(x: w * 0.25, y: h * 0.65)
            Circle().fill(Color(hex: 0xA85838)).frame(width: 22, height: 22)
                .position(x: w * 0.5,  y: h * 0.6)
            Circle().fill(Color(hex: 0xE8D090)).frame(width: 16, height: 16)
                .position(x: w * 0.78, y: h * 0.65)

        case .studio:
            // Canvas on easel
            Rectangle()
                .fill(Color(hex: 0xFFF8EC))
                .frame(width: w * 0.4, height: h * 0.5)
                .shadow(color: .black.opacity(0.15), radius: 3, x: 2, y: 2)
                .position(x: w * 0.4, y: h * 0.45)
            // Brush
            Rectangle()
                .fill(Color(hex: 0x3A2818))
                .frame(width: 4, height: 30)
                .rotationEffect(.degrees(20))
                .position(x: w * 0.78, y: h * 0.7)
        }
    }
}

// MARK: - Shape helpers

private struct SunsetMountains: Shape {
    func path(in rect: CGRect) -> Path {
        var p = Path()
        let w = rect.width, h = rect.height
        p.move(to: CGPoint(x: 0, y: h))
        p.addLine(to: CGPoint(x: 0, y: h * 0.55))
        p.addQuadCurve(to: CGPoint(x: w * 0.28, y: h * 0.5), control: CGPoint(x: w * 0.15, y: h * 0.4))
        p.addQuadCurve(to: CGPoint(x: w * 0.58, y: h * 0.45), control: CGPoint(x: w * 0.42, y: h * 0.3))
        p.addQuadCurve(to: CGPoint(x: w * 0.85, y: h * 0.4),  control: CGPoint(x: w * 0.72, y: h * 0.25))
        p.addQuadCurve(to: CGPoint(x: w, y: h * 0.45),        control: CGPoint(x: w * 0.95, y: h * 0.3))
        p.addLine(to: CGPoint(x: w, y: h))
        p.closeSubpath()
        return p
    }
}

private struct LakeRidge: Shape {
    func path(in rect: CGRect) -> Path {
        var p = Path()
        let w = rect.width, h = rect.height
        p.move(to: CGPoint(x: 0, y: h))
        p.addLine(to: CGPoint(x: 0, y: h * 0.55))
        p.addQuadCurve(to: CGPoint(x: w * 0.4, y: h * 0.5), control: CGPoint(x: w * 0.2, y: h * 0.3))
        p.addQuadCurve(to: CGPoint(x: w * 0.8, y: h * 0.42), control: CGPoint(x: w * 0.6, y: h * 0.15))
        p.addQuadCurve(to: CGPoint(x: w, y: h * 0.36), control: CGPoint(x: w * 0.9, y: h * 0.2))
        p.addLine(to: CGPoint(x: w, y: h))
        p.closeSubpath()
        return p
    }
}

private struct BikeOutline: Shape {
    func path(in rect: CGRect) -> Path {
        // viewBox 0 0 60 30 → fit to rect
        let scaleX = rect.width / 60
        let scaleY = rect.height / 30
        var p = Path()
        // back wheel
        p.addEllipse(in: CGRect(x: (12 - 6) * scaleX, y: (22 - 6) * scaleY, width: 12 * scaleX, height: 12 * scaleY))
        // front wheel
        p.addEllipse(in: CGRect(x: (48 - 6) * scaleX, y: (22 - 6) * scaleY, width: 12 * scaleX, height: 12 * scaleY))
        // frame
        p.move(to: CGPoint(x: 12 * scaleX, y: 22 * scaleY))
        p.addLine(to: CGPoint(x: 24 * scaleX, y: 22 * scaleY))
        p.addLine(to: CGPoint(x: 36 * scaleX, y: 8 * scaleY))
        p.addLine(to: CGPoint(x: 48 * scaleX, y: 22 * scaleY))
        p.move(to: CGPoint(x: 36 * scaleX, y: 8 * scaleY))
        p.addLine(to: CGPoint(x: 40 * scaleX, y: 22 * scaleY))
        return p
    }
}

private struct UnevenCapsule: Shape {
    func path(in rect: CGRect) -> Path {
        // "40% 40% 30% 30%" rounded blob
        let w = rect.width, h = rect.height
        let topR = min(w * 0.4, h * 0.4)
        let botR = min(w * 0.3, h * 0.3)
        var p = Path()
        p.move(to: CGPoint(x: 0, y: topR))
        p.addQuadCurve(to: CGPoint(x: topR, y: 0), control: CGPoint(x: 0, y: 0))
        p.addLine(to: CGPoint(x: w - topR, y: 0))
        p.addQuadCurve(to: CGPoint(x: w, y: topR), control: CGPoint(x: w, y: 0))
        p.addLine(to: CGPoint(x: w, y: h - botR))
        p.addQuadCurve(to: CGPoint(x: w - botR, y: h), control: CGPoint(x: w, y: h))
        p.addLine(to: CGPoint(x: botR, y: h))
        p.addQuadCurve(to: CGPoint(x: 0, y: h - botR), control: CGPoint(x: 0, y: h))
        p.closeSubpath()
        return p
    }
}

private struct LeafShape: Shape {
    func path(in rect: CGRect) -> Path {
        var p = Path()
        let w = rect.width, h = rect.height
        p.move(to: CGPoint(x: 0, y: h / 2))
        p.addQuadCurve(to: CGPoint(x: w / 2, y: 0), control: CGPoint(x: 0, y: 0))
        p.addQuadCurve(to: CGPoint(x: w, y: h / 2), control: CGPoint(x: w, y: 0))
        p.addQuadCurve(to: CGPoint(x: w / 2, y: h), control: CGPoint(x: w, y: h))
        p.addQuadCurve(to: CGPoint(x: 0, y: h / 2), control: CGPoint(x: 0, y: h))
        p.closeSubpath()
        return p
    }
}

private struct JaggedHills: Shape {
    func path(in rect: CGRect) -> Path {
        var p = Path()
        let w = rect.width, h = rect.height
        p.move(to: CGPoint(x: 0, y: h))
        p.addLine(to: CGPoint(x: 0, y: h * 0.6))
        p.addLine(to: CGPoint(x: w * 0.20, y: h * 0.52))
        p.addLine(to: CGPoint(x: w * 0.30, y: h * 0.36))
        p.addLine(to: CGPoint(x: w * 0.40, y: h * 0.48))
        p.addLine(to: CGPoint(x: w * 0.55, y: h * 0.32))
        p.addLine(to: CGPoint(x: w * 0.70, y: h * 0.44))
        p.addLine(to: CGPoint(x: w * 0.85, y: h * 0.36))
        p.addLine(to: CGPoint(x: w, y: h * 0.48))
        p.addLine(to: CGPoint(x: w, y: h))
        p.closeSubpath()
        return p
    }
}
