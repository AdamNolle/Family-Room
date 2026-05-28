import SwiftUI

// MARK: - Avatar

struct MantelAvatar: View {
    let person: FamilyPerson
    var size: CGFloat = 36
    var ring: Bool = false
    var dim: Bool = false

    var body: some View {
        ZStack {
            Circle()
                .fill(
                    LinearGradient(
                        colors: [person.accent, person.tone],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
                .frame(width: size, height: size)
                .overlay(
                    Group {
                        if ring {
                            Circle()
                                .stroke(Color.white, lineWidth: 2.5)
                                .padding(-2.5)
                            Circle()
                                .stroke(person.tone, lineWidth: 1.5)
                                .padding(-4.5)
                        }
                    }
                )
                .shadow(color: .black.opacity(ring ? 0 : 0.12), radius: 1, x: 0, y: 1)
            Text(person.initial)
                .font(.system(size: size * 0.42, weight: .semibold))
                .foregroundStyle(Color.white)
        }
        .opacity(dim ? 0.35 : 1)
        .animation(.easeInOut(duration: 0.22), value: dim)
        .animation(.easeInOut(duration: 0.22), value: ring)
    }
}

// MARK: - Moment plate (with caption)

struct MomentPlate: View {
    let moment: Moment
    var dim: Bool = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ScenePhoto(scene: moment.scene, cornerRadius: 10)
                .frame(width: plateWidth(moment.weight), height: plateHeight(moment.weight))
                .shadow(color: .black.opacity(0.08), radius: 2, x: 0, y: 1)
            VStack(alignment: .leading, spacing: 2) {
                Text(moment.title)
                    .font(.mantelSans(12.5, weight: .semibold))
                    .foregroundStyle(FurnitureDesign.ink)
                    .lineLimit(2)
                Text(moment.day + (moment.hero ? " · everyone" : ""))
                    .font(.mantelSans(11, weight: .medium))
                    .foregroundStyle(FurnitureDesign.inkFaint)
            }
            .frame(width: plateWidth(moment.weight), alignment: .leading)
        }
        .opacity(dim ? 0.25 : 1)
        .animation(.easeInOut(duration: 0.28), value: dim)
    }

    private func plateWidth(_ w: Int) -> CGFloat {
        [80, 110, 150, 190, 230][max(0, min(4, w - 1))]
    }
    private func plateHeight(_ w: Int) -> CGFloat {
        [70, 95, 130, 165, 200][max(0, min(4, w - 1))]
    }
}

// MARK: - River of time

struct RiverOfTime: View {
    let moments: [Moment]
    let filter: Set<String>
    var height: CGFloat = 280
    var onTap: ((Moment) -> Void)? = nil

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(alignment: .bottom, spacing: 14) {
                ForEach(moments) { m in
                    Button {
                        onTap?(m)
                    } label: {
                        MomentPlate(moment: m, dim: dimmed(m))
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 4)
            .padding(.top, 24)
            .padding(.bottom, 8)
        }
        .frame(minHeight: height)
    }

    private func dimmed(_ m: Moment) -> Bool {
        guard !filter.isEmpty else { return false }
        return Set(m.whoIDs).intersection(filter).isEmpty
    }
}

// MARK: - Section kicker (uppercase mono)

struct SectionKicker: View {
    let text: String
    var color: Color = FurnitureDesign.inkFaint
    var size: CGFloat = 11
    var tracking: CGFloat = 1.1

    init(_ text: String, color: Color = FurnitureDesign.inkFaint, size: CGFloat = 11, tracking: CGFloat = 1.1) {
        self.text = text
        self.color = color
        self.size = size
        self.tracking = tracking
    }

    var body: some View {
        Text(text.uppercased())
            .font(.mantelSans(size, weight: .semibold))
            .tracking(tracking)
            .foregroundStyle(color)
    }
}

// MARK: - Neu button + chip + card

struct NeuButton: View {
    let title: String
    var icon: String? = nil
    var primary: Bool = false
    var size: NeuButtonSize = .medium
    var action: () -> Void = {}

    enum NeuButtonSize { case small, medium, large }

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                if let icon { Image(systemName: icon).font(.system(size: fontSize + 1, weight: .medium)) }
                Text(title)
                    .font(.mantelSans(fontSize, weight: primary ? .semibold : .medium))
            }
            .foregroundStyle(primary ? FurnitureDesign.warm : FurnitureDesign.inkSoft)
            .padding(.horizontal, hPad)
            .padding(.vertical, vPad)
            .neuOut(size: .small, cornerRadius: 12)
        }
        .buttonStyle(.plain)
        .contentShape(Rectangle())
    }

    private var fontSize: CGFloat {
        switch size { case .small: 12; case .medium: 13; case .large: 14 }
    }
    private var hPad: CGFloat {
        switch size { case .small: 12; case .medium: 16; case .large: 22 }
    }
    private var vPad: CGFloat {
        switch size { case .small: 8; case .medium: 11; case .large: 14 }
    }
}

struct NeuChip: View {
    let title: String
    var active: Bool = false
    var action: () -> Void = {}

    var body: some View {
        Button(action: action) {
            Text(title)
                .font(.mantelSans(12, weight: active ? .semibold : .medium))
                .foregroundStyle(active ? FurnitureDesign.warm : FurnitureDesign.inkSoft)
                .padding(.horizontal, 12)
                .padding(.vertical, 7)
                .background(
                    Group {
                        if active {
                            Capsule().fill(FurnitureDesign.cream).neuIn(size: .small, cornerRadius: 999)
                                .clipShape(Capsule())
                        } else {
                            Capsule().fill(FurnitureDesign.cream).neuOut(size: .small, cornerRadius: 999)
                                .clipShape(Capsule())
                        }
                    }
                )
        }
        .buttonStyle(.plain)
    }
}

struct NeuCard<Content: View>: View {
    var inset: Bool = false
    var padding: CGFloat = 16
    var cornerRadius: CGFloat = 18
    @ViewBuilder var content: () -> Content

    var body: some View {
        content()
            .padding(padding)
            .modifier(NeuCardBackground(inset: inset, cornerRadius: cornerRadius))
    }
}

private struct NeuCardBackground: ViewModifier {
    let inset: Bool
    let cornerRadius: CGFloat
    func body(content: Content) -> some View {
        if inset {
            content.neuIn(size: .small, cornerRadius: cornerRadius)
        } else {
            content.neuOut(size: .small, cornerRadius: cornerRadius)
        }
    }
}

// MARK: - Hairline divider

struct Hairline: View {
    var horizontal: Bool = true
    var color: Color = FurnitureDesign.hairline

    var body: some View {
        if horizontal {
            Rectangle().fill(color).frame(height: 0.5)
        } else {
            Rectangle().fill(color).frame(width: 0.5)
        }
    }
}

// MARK: - Stat block (inset value+label tile)

struct StatBlock: View {
    let value: String
    let label: String

    var body: some View {
        VStack(alignment: .center, spacing: 2) {
            Text(value)
                .font(.mantelDisplay(20, weight: .medium))
                .foregroundStyle(FurnitureDesign.ink)
            SectionKicker(label, color: FurnitureDesign.inkFaint, size: 10, tracking: 0.6)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 10)
        .background(Color.white.opacity(0.6))
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .stroke(FurnitureDesign.hairline, lineWidth: 0.5)
        )
    }
}
