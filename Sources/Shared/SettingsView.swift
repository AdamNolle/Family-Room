import SwiftUI

struct SettingsView: View {
    @ObservedObject var viewModel: AppViewModel

    var body: some View {
        ZStack {
            DaylightBackground()

            ScrollView {
                VStack(alignment: .leading, spacing: 28) {
                    header

                    VStack(alignment: .leading, spacing: 12) {
                        SectionKicker("Atmosphere", color: FurnitureDesign.inkFaint, size: 11, tracking: 1.1)
                            .padding(.horizontal, 24)

                        VStack(spacing: 0) {
                            settingsRow(
                                title: "Reduce sunlight motion",
                                subtitle: "Keeps the warm light steady instead of drifting.",
                                isOn: $viewModel.reduceSunlightMotion
                            )
                            Hairline()
                            settingsRow(
                                title: "Sound",
                                subtitle: "Soft analog clicks when scrubbing.",
                                isOn: $viewModel.soundEnabled
                            )
                        }
                        .padding(.vertical, 6)
                        .padding(.horizontal, 4)
                        .neuOut(size: .small, cornerRadius: 16)
                        .padding(.horizontal, 24)
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        SectionKicker("About", color: FurnitureDesign.inkFaint, size: 11, tracking: 1.1)
                            .padding(.horizontal, 24)
                        Text("Family Room \u{2014} Mantelpiece edition. A quiet room for the people who already live in your photos.")
                            .font(.mantelDisplay(15, weight: .regular, italic: true))
                            .foregroundStyle(FurnitureDesign.inkSoft)
                            .padding(.horizontal, 24)
                    }

                    Spacer(minLength: 24)
                }
                .padding(.top, 24)
                .padding(.bottom, 60)
            }
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 4) {
            SectionKicker("Settings", color: FurnitureDesign.warmSoft, size: 12, tracking: 0.6)
            Text("How the room feels")
                .font(.mantelDisplay(28, weight: .medium))
                .foregroundStyle(FurnitureDesign.ink)
        }
        .padding(.horizontal, 24)
    }

    private func settingsRow(title: String, subtitle: String, isOn: Binding<Bool>) -> some View {
        HStack(alignment: .center, spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.mantelSans(14, weight: .semibold))
                    .foregroundStyle(FurnitureDesign.ink)
                Text(subtitle)
                    .font(.mantelSans(12))
                    .foregroundStyle(FurnitureDesign.inkMute)
            }
            Spacer(minLength: 8)
            Toggle("", isOn: isOn)
                .labelsHidden()
                .tint(FurnitureDesign.warm)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
    }
}
