import SwiftUI

struct FurnitureDesign {
    static let walnutWood = Color(red: 0.35, green: 0.25, blue: 0.15)
    static let softLinen = Color(red: 0.96, green: 0.94, blue: 0.88)
    static let sageWool = Color(red: 0.45, green: 0.48, blue: 0.42)
    static let rawSilk = Color(red: 0.98, green: 0.96, blue: 0.92)
    
    struct AlbumPileModifier: ViewModifier {
        func body(content: Content) -> some View {
            ZStack {
                RoundedRectangle(cornerRadius: 4)
                    .fill(.white)
                    .shadow(color: .black.opacity(0.1), radius: 2)
                    .rotationEffect(.degrees(-3))
                    .offset(x: -4, y: -2)
                
                RoundedRectangle(cornerRadius: 4)
                    .fill(.white)
                    .shadow(color: .black.opacity(0.1), radius: 2)
                    .rotationEffect(.degrees(2))
                    .offset(x: 4, y: 1)
                
                content
                    .background(.white)
                    .clipShape(RoundedRectangle(cornerRadius: 4))
                    .shadow(color: .black.opacity(0.15), radius: 8, x: 0, y: 4)
            }
        }
    }
}

struct AdaptiveFabricBackground: View {
    let type: AppViewModel.FabricType
    
    var body: some View {
        ZStack {
            backgroundColor
                .ignoresSafeArea()
            
            // Texture overlays
            Group {
                if type == .wood {
                    WoodGrainOverlay()
                } else {
                    LinenTextureOverlay()
                }
            }
            .opacity(0.04)
            .ignoresSafeArea()
            
            // Subtle lighting
            RadialGradient(colors: [.white.opacity(0.15), .clear], center: .topLeading, startRadius: 0, endRadius: 800)
                .ignoresSafeArea()
        }
    }
    
    var backgroundColor: Color {
        switch type {
        case .linen: return FurnitureDesign.softLinen
        case .wool: return FurnitureDesign.sageWool
        case .silk: return FurnitureDesign.rawSilk
        case .wood: return FurnitureDesign.walnutWood
        }
    }
}

struct LinenTextureOverlay: View {
    var body: some View {
        Canvas { context, size in
            for x in stride(from: 0, to: size.width, by: 4) {
                context.fill(Path(CGRect(x: x, y: 0, width: 1, height: size.height)), with: .color(.black))
            }
            for y in stride(from: 0, to: size.width, by: 4) {
                context.fill(Path(CGRect(x: 0, y: y, width: size.width, height: 1)), with: .color(.black))
            }
        }
    }
}

struct WoodGrainOverlay: View {
    var body: some View {
        Canvas { context, size in
            for _ in 0...500 {
                let rect = CGRect(x: Double.random(in: 0...size.width), y: Double.random(in: 0...size.height), width: Double.random(in: 100...300), height: 1)
                context.fill(Path(rect), with: .color(.black))
            }
        }
    }
}

extension View {
    func albumStyle() -> some View {
        self.modifier(FurnitureDesign.AlbumPileModifier())
    }
}
