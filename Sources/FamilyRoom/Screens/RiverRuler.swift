import SwiftUI

/// "River of time": a scrubbable year ruler with one tick per moment (`.mruler` / `.rtrack`).
struct RiverRuler: View {
    var moments: [MomentSummary]
    var year: Int
    var current: MomentSummary?
    var matches: (MomentSummary) -> Bool
    var onPick: (MomentSummary) -> Void
    var height: CGFloat = 84

    @State private var dragging = false

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("River of time").labelStyle()
                Spacer()
                Text("Drag to scrub").labelStyle(color: Theme.inkMute).monospacedDigit()
            }
            GeometryReader { geo in
                let w = geo.size.width, baseline = height - 26
                ZStack(alignment: .topLeading) {
                    Rectangle().fill(Theme.hair2).frame(height: 0.5).offset(y: baseline)
                    ForEach(Array(Catalog.monthMarks(year: year, skip: 0).enumerated()), id: \.offset) { _, mark in
                        let x = CGFloat(mark.fraction) * w
                        Rectangle().fill(Theme.hair2).frame(width: 0.5, height: 34).offset(x: x, y: height - 34)
                        Text(w > 460 ? mark.label : String(mark.label.prefix(1)))
                            .labelStyle(size: 10, color: Theme.inkMute)
                            .offset(x: x + 6, y: height - 20)
                    }
                    ForEach(moments) { m in
                        let on = m.id == current?.id
                        let x = CGFloat(Catalog.yearFraction(m.date)) * w
                        let h = CGFloat(8 + m.weight * 7)
                        RoundedRectangle(cornerRadius: 1)
                            .fill(on ? Theme.warm : Theme.inkFaint)
                            .frame(width: on ? 4 : 2, height: h)
                            .offset(x: x - (on ? 2 : 1), y: baseline - h)
                            .opacity(matches(m) ? 1 : 0.2)
                            .animation(.easeOut(duration: 0.25), value: on)
                    }
                    if let current {
                        let x = CGFloat(Catalog.yearFraction(current.date)) * w
                        Path { p in p.move(to: CGPoint(x: x, y: 0)); p.addLine(to: CGPoint(x: x, y: baseline)) }
                            .stroke(Theme.warm.opacity(0.6), style: StrokeStyle(lineWidth: 1, dash: [3, 3]))
                            .animation(Theme.ease, value: current.id)
                        Text(Catalog.dayLabel(current.date)).font(.num(11, .semibold)).foregroundStyle(Theme.warm)
                            .offset(x: min(x + 6, w - 48), y: 0)
                            .animation(Theme.ease, value: current.id)
                    }
                }
                .frame(width: w, height: height)
                .contentShape(Rectangle())
                .gesture(DragGesture(minimumDistance: 0).onChanged { value in
                    dragging = true
                    scrub(fraction: value.location.x / max(1, w))
                }.onEnded { _ in dragging = false })
            }
            .frame(height: height)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("River of time")
        .accessibilityValue(current.map { "\($0.title), \(Catalog.dayLabel($0.date))" } ?? "")
    }

    private func scrub(fraction: Double) {
        guard let best = Catalog.nearest(moments, to: min(1, max(0, fraction)), matching: matches), best.id != current?.id else { return }
        Feedback.tick()
        onPick(best)
    }
}
