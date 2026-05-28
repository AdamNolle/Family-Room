import SwiftUI

struct EventDetailView: View {
    let event: FamilyEvent
    @Environment(\.dismiss) var dismiss

    var body: some View {
        ZStack {
            DaylightBackground()

            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    Button {
                        dismiss()
                    } label: {
                        HStack(spacing: 6) {
                            Image(systemName: "chevron.left")
                            Text("Back")
                        }
                        .font(.mantelSans(13, weight: .semibold))
                        .foregroundStyle(FurnitureDesign.warm)
                    }
                    .buttonStyle(.plain)
                    .padding(.horizontal, 24)

                    VStack(alignment: .leading, spacing: 6) {
                        SectionKicker(event.date, color: FurnitureDesign.warmSoft, size: 12, tracking: 0.6)
                        Text(event.name)
                            .font(.mantelDisplay(34, weight: .medium))
                            .foregroundStyle(FurnitureDesign.ink)
                    }
                    .padding(.horizontal, 24)

                    LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 16) {
                        ForEach(0..<12, id: \.self) { i in
                            ScenePhoto(scene: scene(for: i), cornerRadius: 6)
                                .aspectRatio(1, contentMode: .fit)
                                .padding(6)
                                .background(Color.white)
                                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                                .shadow(color: .black.opacity(0.1), radius: 2, x: 0, y: 1)
                                .rotationEffect(.degrees(rotation(for: i)))
                        }
                    }
                    .padding(.horizontal, 24)
                }
                .padding(.top, 32)
                .padding(.bottom, 48)
            }
        }
        #if os(iOS)
        .navigationBarBackButtonHidden(true)
        #endif
    }

    private func scene(for i: Int) -> PhotoScene {
        let all = PhotoScene.allCases
        return all[i % all.count]
    }

    private func rotation(for i: Int) -> Double {
        // Deterministic gentle tilt, ±2°.
        Double((i * 37) % 7) - 3.0
    }
}
