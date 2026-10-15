import SwiftUI
import CoreText

// Design tokens ported from the "Family Room - Devices v3" design (mac3.css + mobile.css).

extension Color {
    init(hex: UInt32, opacity: Double = 1) {
        self.init(.sRGB,
                  red: Double((hex >> 16) & 0xFF) / 255,
                  green: Double((hex >> 8) & 0xFF) / 255,
                  blue: Double(hex & 0xFF) / 255,
                  opacity: opacity)
    }

    /// Parses "#RRGGBB" or "RRGGBB". Falls back to the warm accent when unreadable.
    init(hexString: String?) {
        let cleaned = (hexString ?? "").trimmingCharacters(in: CharacterSet(charactersIn: "# "))
        if let value = UInt32(cleaned, radix: 16), cleaned.count == 6 {
            self.init(hex: value)
        } else {
            self = Theme.warm
        }
    }
}

enum Theme {
    static let paper      = Color(hex: 0xECE4D6)
    static let paper2     = Color(hex: 0xF5F0E7)
    static let ink        = Color(hex: 0x2A221A)
    static let inkSoft    = Color(hex: 0x4A4035)
    static let inkMute    = Color(hex: 0x6E6358)
    static let inkFaint   = Color(hex: 0x8A7E70)
    static let warm       = Color(hex: 0xB86348)
    static let good       = Color(hex: 0x4A7A4A)
    static let hair       = Color(.sRGB, red: 40/255, green: 30/255, blue: 20/255, opacity: 0.12)
    static let hair2      = Color(.sRGB, red: 40/255, green: 30/255, blue: 20/255, opacity: 0.22)
    static let shadowInk  = Color(.sRGB, red: 40/255, green: 24/255, blue: 10/255, opacity: 1)
    static let filmBed    = Color(hex: 0x1D1712)
    static let filmBar    = Color(hex: 0x120D09)

    /// Avatar palettes, assigned round-robin as people are created.
    static let personTones: [(tone: UInt32, accent: UInt32)] = [
        (0xC97A5D, 0xF0BFA8), (0x7A6A4A, 0xD4C29A), (0x4A7A82, 0xA8D0D4),
        (0x6A5A8A, 0xC4B8D8), (0xD49060, 0xF5D4B0), (0x5D7F45, 0xC3D79F),
        (0x8E3F26, 0xE7A65B), (0x3E6A7D, 0xA9CBD4)
    ]

    static let ease = Animation.timingCurve(0.22, 1, 0.36, 1, duration: 0.45)
}

enum TypeFaces {
    static let serifName = "InstrumentSerif-Regular"
    private static var registered = false

    static var bundle: Bundle {
        #if SWIFT_PACKAGE
        return .module
        #else
        return .main
        #endif
    }

    static func register() {
        guard !registered, let url = bundle.url(forResource: "InstrumentSerif-Regular", withExtension: "ttf") else { return }
        CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
        registered = true
    }
}

extension Font {
    /// `.d` — the serif display face.
    static func display(_ size: CGFloat) -> Font {
        TypeFaces.register()
        return .custom(TypeFaces.serifName, size: size)
    }
    /// `.lbl` — small caps-style label.
    static func label(_ size: CGFloat = 11) -> Font {
        .system(size: size, weight: .semibold).width(.condensed)
    }
    static func sans(_ size: CGFloat, _ weight: Font.Weight = .regular) -> Font {
        .system(size: size, weight: weight)
    }
    static func num(_ size: CGFloat, _ weight: Font.Weight = .regular) -> Font {
        .system(size: size, weight: weight).monospacedDigit()
    }
}

extension View {
    /// Uppercase tracking label (`.lbl`). Pass `warm: true` for `.lbl.w`.
    func labelStyle(size: CGFloat = 11, warm: Bool = false, color: Color? = nil) -> some View {
        self.font(.label(size))
            .textCase(.uppercase)
            .tracking(size * 0.08)
            .foregroundStyle(color ?? (warm ? Theme.warm : Theme.inkMute))
    }

    func hairlineBorder(_ radius: CGFloat, color: Color = Theme.hair2) -> some View {
        overlay(RoundedRectangle(cornerRadius: radius, style: .continuous).strokeBorder(color, lineWidth: 0.5))
    }

    /// The warm drop shadow used under prints and cards.
    func printShadow(strength: CGFloat = 1) -> some View {
        self.shadow(color: Theme.shadowInk.opacity(0.16 * strength), radius: 15 * strength, y: 7 * strength)
            .shadow(color: Theme.shadowInk.opacity(0.08 * strength), radius: 3, y: 1)
    }

    @ViewBuilder
    func onHoverIfAvailable(_ perform: @escaping (Bool) -> Void) -> some View {
        #if os(macOS)
        self.onHover(perform: perform)
        #else
        self
        #endif
    }
}

enum DeviceWord {
    /// "iPhone", "iPad" or "Mac" — used in copy such as "worked out on this iPhone".
    static var current: String {
        #if os(macOS)
        return "Mac"
        #else
        return UIDevice.current.userInterfaceIdiom == .pad ? "iPad" : "iPhone"
        #endif
    }
}
