import SwiftUI
#if os(macOS)
import AppKit
#else
import UIKit
#endif

enum CozyTheme {
    private static func adaptive(_ light: UInt32, _ dark: UInt32) -> Color {
        #if os(macOS)
        return Color(nsColor: NSColor(name: nil) { appearance in
            let value = appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua ? dark : light
            return NSColor(srgbRed: Double((value >> 16) & 255) / 255, green: Double((value >> 8) & 255) / 255, blue: Double(value & 255) / 255, alpha: 1)
        })
        #else
        return Color(uiColor: UIColor { traits in
            let value = traits.userInterfaceStyle == .dark ? dark : light
            return UIColor(red: CGFloat((value >> 16) & 255) / 255, green: CGFloat((value >> 8) & 255) / 255, blue: CGFloat(value & 255) / 255, alpha: 1)
        })
        #endif
    }
    static let canvas = adaptive(0xF7F5F1, 0x1D1C1B)
    static let sidebar = adaptive(0xF0EAE3, 0x282521)
    static let surface = adaptive(0xFFFFFF, 0x322D28)
    static let ink = adaptive(0x2C2926, 0xF6F2EC)
    static let secondary = adaptive(0x70665F, 0xC9BFB5)
    static let accent = Color(hex: 0x8A4D32)
    static let seasonalHighlight = adaptive(0xE8C1A4, 0x846149)
    static let glassTint = adaptive(0xE2EBF1, 0x303D45)
    static let onAccent = Color.white
    static let accentInk = adaptive(0x8A4D32, 0xE8B28D)
    static let line = adaptive(0xDFD7CE, 0x51473F)
    static let photoBed = adaptive(0xE9E4DE, 0x171513)
    static let radius: CGFloat = 12
}

struct CozyPanel: ViewModifier {
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    var glass = false
    func body(content: Content) -> some View {
        content.background {
            if glass && !reduceTransparency { RoundedRectangle(cornerRadius: CozyTheme.radius).fill(.regularMaterial) }
            else { RoundedRectangle(cornerRadius: CozyTheme.radius).fill(CozyTheme.surface) }
        }.overlay { RoundedRectangle(cornerRadius: CozyTheme.radius).strokeBorder(CozyTheme.line.opacity(0.8), lineWidth: 0.5) }
    }
}

/// Keep the label and fill together, including macOS's inactive-window appearance.
struct CozyPrimaryButtonStyle: ButtonStyle {
    @Environment(\.isEnabled) private var isEnabled
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.body.weight(.semibold))
            .foregroundStyle(isEnabled ? CozyTheme.onAccent : CozyTheme.secondary)
            .padding(.horizontal, 14)
            .padding(.vertical, 8)
            .frame(minHeight: minimumHeight)
            .background(isEnabled ? CozyTheme.accent : CozyTheme.sidebar, in: RoundedRectangle(cornerRadius: 8))
            .opacity(configuration.isPressed ? 0.86 : 1)
    }
    private var minimumHeight: CGFloat {
        #if os(macOS)
        36
        #else
        44
        #endif
    }
}
extension View {
    func cozyPanel(glass: Bool = false) -> some View { modifier(CozyPanel(glass: glass)) }
}
struct RoomEyebrow: View {
    let text: String
    var body: some View {
        HStack(spacing: 8) {
            Capsule().fill(CozyTheme.seasonalHighlight).frame(width: 16, height: 3).accessibilityHidden(true)
            Text(text.uppercased()).font(.system(.caption, design: .monospaced)).tracking(1.4).foregroundStyle(CozyTheme.secondary)
        }
    }
}
struct RoomSectionHeading: View {
    let title: String
    var detail: String? = nil
    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(title).font(RoomArt.display(30, relativeTo: .title2)).foregroundStyle(CozyTheme.ink)
            Spacer()
            if let detail { Text(detail).font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary) }
        }
    }
}
