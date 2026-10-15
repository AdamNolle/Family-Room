import SwiftUI

/// Calm, fluid motion. Everything settles with springs rather than linear timing, and every effect
/// falls back to a plain fade (or nothing) when Reduce Motion is on.
enum Motion {
    /// Page changes, layout shifts.
    static let gentle = Animation.spring(response: 0.55, dampingFraction: 0.86)
    /// Selection pills, toggles.
    static let snappy = Animation.spring(response: 0.38, dampingFraction: 0.78)
    /// Small celebratory bumps.
    static let pop = Animation.spring(response: 0.32, dampingFraction: 0.52)

    static func stagger(_ index: Int) -> Double { Double(min(index, 14)) * 0.035 }
}

/// Rises and fades in the first time it appears, staggered by index.
private struct Entrance: ViewModifier {
    var index: Int
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var shown = false

    func body(content: Content) -> some View {
        content
            .opacity(shown ? 1 : 0)
            .offset(y: shown || reduceMotion ? 0 : 12)
            .scaleEffect(shown || reduceMotion ? 1 : 0.985, anchor: .top)
            .onAppear {
                guard !shown else { return }
                withAnimation(reduceMotion ? .easeOut(duration: 0.2) : Motion.gentle.delay(Motion.stagger(index))) { shown = true }
            }
    }
}

/// A print dropped onto the light table: it falls from a little higher with extra tilt and settles.
private struct DropIn: ViewModifier {
    var index: Int
    var tilt: Double
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var landed = false

    func body(content: Content) -> some View {
        content
            .opacity(landed ? 1 : 0)
            .rotationEffect(.degrees(landed || reduceMotion ? 0 : tilt * 2.5 + (index.isMultiple(of: 2) ? -4 : 4)))
            .scaleEffect(landed || reduceMotion ? 1 : 1.08)
            .offset(y: landed || reduceMotion ? 0 : -18)
            .onAppear {
                guard !landed else { return }
                withAnimation(reduceMotion ? .easeOut(duration: 0.2) : .spring(response: 0.6, dampingFraction: 0.7).delay(0.08 + Motion.stagger(index) * 1.4)) { landed = true }
            }
    }
}

/// A very slow push-in and back, so a featured photo feels alive without drawing attention.
private struct Breathing: ViewModifier {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @AppStorage(SettingsKeys.reduceSunlight) private var reduceSunlight = false
    @State private var inhale = false

    func body(content: Content) -> some View {
        content
            .scaleEffect(inhale ? 1.035 : 1)
            .onAppear { breathe() }
            .onChange(of: reduceMotion || reduceSunlight) { _, _ in breathe() }
    }

    private func breathe() {
        if reduceMotion || reduceSunlight {
            withAnimation(.easeOut(duration: 0.6)) { inhale = false }
        } else {
            withAnimation(.easeInOut(duration: 14).repeatForever(autoreverses: true)) { inhale = true }
        }
    }
}

/// A quick bump when a value changes (likes, selections).
private struct Pop<V: Equatable>: ViewModifier {
    var value: V
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var bump = false

    func body(content: Content) -> some View {
        content
            .scaleEffect(bump ? 1.18 : 1)
            .onChange(of: value) { _, _ in
                guard !reduceMotion else { return }
                withAnimation(Motion.pop) { bump = true }
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.16) { withAnimation(Motion.pop) { bump = false } }
            }
    }
}

extension View {
    func entrance(_ index: Int = 0) -> some View { modifier(Entrance(index: index)) }
    func dropIn(_ index: Int, tilt: Double) -> some View { modifier(DropIn(index: index, tilt: tilt)) }
    func breathing() -> some View { modifier(Breathing()) }
    func pop<V: Equatable>(on value: V) -> some View { modifier(Pop(value: value)) }
}
