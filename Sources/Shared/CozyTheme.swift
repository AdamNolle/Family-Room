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
    static let canvas = adaptive(0xF8F5EF, 0x17191A)
    static let sidebar = adaptive(0xEFEAE2, 0x202324)
    static let surface = adaptive(0xFFFFFF, 0x292C2D)
    static let ink = adaptive(0x302C28, 0xF5F2EB)
    static let secondary = adaptive(0x6D665E, 0xC2BCB4)
    static let accent = Color(hex: 0x99442F)
    static let onAccent = Color.white
    static let accentInk = adaptive(0x99442F, 0xF1B59C)
    static let line = adaptive(0xDDD7CF, 0x3E4243)
    static let photoBed = adaptive(0xE6E2DC, 0x121415)
    static let radius: CGFloat = 18
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
    var body: some View { Text(text.uppercased()).font(.caption.weight(.semibold)).tracking(1.6).foregroundStyle(CozyTheme.secondary) }
}
struct RoomSectionHeading: View {
    let title: String
    var detail: String? = nil
    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(title).font(.title2.weight(.semibold)).foregroundStyle(CozyTheme.ink)
            Spacer()
            if let detail { Text(detail).font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary) }
        }
    }
}
