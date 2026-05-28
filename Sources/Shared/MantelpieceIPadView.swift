import SwiftUI

struct MantelpieceIPadView: View {
    @ObservedObject var viewModel: AppViewModel
    var onMomentTap: (Moment) -> Void = { _ in }

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            sidebar
            canvas
        }
    }

    // MARK: Sidebar

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Family")
                .font(.mantelDisplay(22, weight: .medium))
                .foregroundStyle(FurnitureDesign.ink)
                .padding(.horizontal, 6)

            SectionKicker("People", tracking: 1.0)
                .padding(.horizontal, 6)

            VStack(spacing: 6) {
                ForEach(viewModel.family) { p in
                    sidebarPersonRow(p)
                }
            }

            Hairline()
                .padding(.vertical, 4)

            SectionKicker("Years", tracking: 1.0)
                .padding(.horizontal, 6)

            VStack(alignment: .leading, spacing: 6) {
                ForEach(Array(["2024", "2023", "2022", "2021", "2020", "Older"].enumerated()), id: \.element) { i, y in
                    Text(y)
                        .font(.mantelSans(13, weight: i == 0 ? .semibold : .regular))
                        .foregroundStyle(i == 0 ? FurnitureDesign.ink : FurnitureDesign.inkSoft)
                        .padding(.horizontal, 6)
                }
            }

            Spacer()
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 18)
        .frame(width: 240, alignment: .topLeading)
    }

    private func sidebarPersonRow(_ p: FamilyPerson) -> some View {
        let isSelected = viewModel.selectedPersonIDs.contains(p.id)
        return Button {
            viewModel.togglePerson(p.id)
        } label: {
            HStack(spacing: 10) {
                MantelAvatar(person: p, size: 28, ring: isSelected)
                Text(p.name)
                    .font(.mantelSans(13, weight: .medium))
                    .foregroundStyle(FurnitureDesign.ink)
                Spacer(minLength: 0)
                if p.id == "I" {
                    Text("2.1k")
                        .font(.mantelSans(10, weight: .semibold))
                        .foregroundStyle(p.tone)
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .background(
                Group {
                    if isSelected {
                        RoundedRectangle(cornerRadius: 12, style: .continuous)
                            .fill(FurnitureDesign.cream)
                            .neuOut(size: .small, cornerRadius: 12)
                    }
                }
            )
        }
        .buttonStyle(.plain)
        .contentShape(Rectangle())
    }

    // MARK: Canvas

    private var canvas: some View {
        VStack(alignment: .leading, spacing: 0) {
            canvasHeader

            RiverOfTime(
                moments: viewModel.moments,
                filter: viewModel.selectedPersonIDs,
                height: 320,
                onTap: onMomentTap
            )
            .padding(.horizontal, 8)

            scrapbookStrip
                .padding(.top, 4)
                .padding(.bottom, 18)
        }
        .padding(.horizontal, 22)
        .padding(.top, 4)
    }

    private var canvasHeader: some View {
        HStack(alignment: .bottom) {
            VStack(alignment: .leading, spacing: 4) {
                SectionKicker(headerKicker, color: FurnitureDesign.warmSoft, size: 12, tracking: 0.6)
                Text(headerTitle)
                    .font(.mantelDisplay(32, weight: .medium))
                    .foregroundStyle(FurnitureDesign.ink)
            }
            Spacer()
            HStack(spacing: 8) {
                NeuButton(title: "Slideshow")
                NeuButton(title: "+ Add", primary: true)
            }
        }
        .padding(.vertical, 14)
    }

    private var scrapbookStrip: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                HStack(spacing: 10) {
                    MantelAvatar(person: viewModel.family[4], size: 28)
                    VStack(alignment: .leading, spacing: 1) {
                        Text("Iris's scrapbook")
                            .font(.mantelSans(13, weight: .semibold))
                            .foregroundStyle(FurnitureDesign.ink)
                        Text("she made this · last edit yesterday")
                            .font(.mantelSans(11))
                            .foregroundStyle(FurnitureDesign.inkFaint)
                    }
                }
                Spacer()
                Text("9 pages →")
                    .font(.mantelSans(12))
                    .foregroundStyle(FurnitureDesign.inkFaint)
            }

            HStack(spacing: 8) {
                let scenes: [PhotoScene] = [.bike, .garden, .stage, .wolf, .snow, .fall, .lake, .sunset, .cake]
                ForEach(scenes.indices, id: \.self) { i in
                    let s = scenes[i]
                    let tilt = (i.isMultiple(of: 2) ? -1.0 : 1.0) * Double(1 + (i % 3))
                    ScenePhoto(scene: s, cornerRadius: 3)
                        .frame(height: 72)
                        .frame(maxWidth: .infinity)
                        .padding(4)
                        .background(Color.white)
                        .shadow(color: .black.opacity(0.15), radius: 1.5, x: 0, y: 1)
                        .rotationEffect(.degrees(tilt))
                }
            }
        }
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .neuOut(size: .small, cornerRadius: 20)
    }

    // MARK: Header copy

    private var headerKicker: String {
        if viewModel.selectedPersonIDs.isEmpty {
            return "Everyone · 2024"
        }
        let names = viewModel.family
            .filter { viewModel.selectedPersonIDs.contains($0.id) }
            .map { $0.name }
        return "With \(ListFormatter.localizedString(byJoining: names)) · 2024"
    }

    private var headerTitle: String {
        if viewModel.selectedPersonIDs.contains("I") && viewModel.selectedPersonIDs.count == 1 {
            return "Her year, big and small"
        }
        return "A year, in pieces"
    }
}
