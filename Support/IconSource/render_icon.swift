import SwiftUI
import AppKit

// Native port of "Family Room - Icon.html": a coffee table from above with three fanned photos.
func hex(_ v: UInt32, _ a: Double = 1) -> Color { Color(.sRGB, red: Double((v >> 16) & 0xFF) / 255, green: Double((v >> 8) & 0xFF) / 255, blue: Double(v & 0xFF) / 255, opacity: a) }

struct Photo: View {
    var w: CGFloat, h: CGFloat, inset: CGFloat, imageH: CGFloat, r: CGFloat, colors: [Color]
    var body: some View {
        ZStack(alignment: .top) {
            RoundedRectangle(cornerRadius: r, style: .continuous).fill(.white).frame(width: w, height: h)
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(LinearGradient(colors: colors, startPoint: .topLeading, endPoint: .bottomTrailing))
                .frame(width: w - inset * 2, height: imageH).padding(.top, inset)
        }
        .shadow(color: hex(0x3A1F10, 0.26), radius: 11, y: 9)
    }
}

struct IconArt: View {
    var body: some View {
        ZStack {
            LinearGradient(stops: [.init(color: hex(0xF2956D), location: 0), .init(color: hex(0xE57850), location: 0.55), .init(color: hex(0xD4603D), location: 1)], startPoint: .top, endPoint: .bottom)
            RadialGradient(stops: [.init(color: hex(0xFFD9AD, 0.55), location: 0), .init(color: hex(0xFFD9AD, 0), location: 0.5)], center: UnitPoint(x: 0.5, y: 0.12), startRadius: 0, endRadius: 820)
            RoundedRectangle(cornerRadius: 174, style: .continuous)
                .fill(LinearGradient(colors: [hex(0xEDD8AF), hex(0xDCC290)], startPoint: .top, endPoint: .bottom))
                .frame(width: 768, height: 768)
                .shadow(color: hex(0x3A1F10, 0.26), radius: 11, y: 9)
            RoundedRectangle(cornerRadius: 171, style: .continuous).strokeBorder(hex(0xFFF8EC, 0.38), lineWidth: 2.5).frame(width: 762, height: 762)
            Photo(w: 346, h: 408, inset: 32, imageH: 268, r: 26, colors: [hex(0xA4C6F0), hex(0x5D82C9)])
                .rotationEffect(.degrees(-16)).offset(x: 414 - 512, y: 508 - 512)
            Photo(w: 342, h: 404, inset: 32, imageH: 264, r: 26, colors: [hex(0xBCD9A6), hex(0x6FA46A)])
                .rotationEffect(.degrees(14)).offset(x: 606 - 512, y: 532 - 512)
            Photo(w: 372, h: 432, inset: 33, imageH: 288, r: 28, colors: [hex(0xF6BD8A), hex(0xE07D4F)])
                .rotationEffect(.degrees(-3))
        }
        .frame(width: 1024, height: 1024)
        .clipped()
    }
}

/// iOS: full-bleed square; the system applies the mask.
/// macOS: Apple's grid puts an 824 pt rounded tile inside the 1024 canvas, with its own drop shadow.
struct MacIcon: View {
    var body: some View {
        IconArt()
            .clipShape(RoundedRectangle(cornerRadius: 232, style: .continuous))
            .scaleEffect(824.0 / 1024.0)
            .shadow(color: .black.opacity(0.3), radius: 10, y: 10)
            .frame(width: 1024, height: 1024)
    }
}

@MainActor func write<V: View>(_ view: V, side: CGFloat, to path: String, opaque: Bool = false) {
    let r = ImageRenderer(content: view.frame(width: 1024, height: 1024).scaleEffect(side / 1024).frame(width: side, height: side))
    r.scale = 1
    r.isOpaque = opaque
    guard let cg = r.cgImage else { print("render failed \(path)"); return }
    let rep = NSBitmapImageRep(cgImage: cg)
    try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: path))
}

let out = CommandLine.arguments[1]
MainActor.assumeIsolated {
    write(IconArt(), side: 1024, to: "\(out)/ios-1024.png", opaque: true)
    for side in [16, 32, 64, 128, 256, 512, 1024] { write(MacIcon(), side: CGFloat(side), to: "\(out)/mac-\(side).png") }
}
print("ok")
