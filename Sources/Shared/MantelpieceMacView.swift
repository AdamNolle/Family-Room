import SwiftUI

struct MantelpieceMacView: View {
    @ObservedObject var viewModel: AppViewModel
    var onMomentTap: (Moment) -> Void = { _ in }

    var body: some View {
        HStack(alignment: .top, spacing: 18) {
            sidebar
            canvas
        }
        .padding(18)
    }

    // MARK: Sidebar (carved cream rail)

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: 14) {
            SectionKicker("Family", tracking: 1.0)
                .padding(.horizontal, 6)

            VStack(alignment: .leading, spacing: 4) {
                ForEach(viewModel.family) { p in
                    sidebarPersonRow(p)
                }
            }

            Hairline()
                .padding(.vertical, 4)

            SectionKicker("Places", tracking: 1.0)
                .padding(.horizontal, 6)

            VStack(alignment: .leading, spacing: 4) {
                ForEach(viewModel.places, id: \.self) { place in
                    Text(place)
                        .font(.mantelSans(13))
                        .foregroundStyle(FurnitureDesign.inkSoft)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 4)
                }
            }

            Spacer(minLength: 12)

            // Sync indicator (inset pill)
            HStack(spacing: 10) {
                Circle()
                    .fill(FurnitureDesign.good)
                    .frame(width: 8, height: 8)
                    .shadow(color: FurnitureDesign.good.opacity(0.5), radius: 4)
                Text("4 of 5 syncing")
                    .font(.mantelSans(12))
                    .foregroundStyle(FurnitureDesign.inkSoft)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .neuIn(size: .small, cornerRadius: 14)
        }
        .padding(.vertical, 20)
        .padding(.horizontal, 16)
        .frame(width: 220, alignment: .topLeading)
        .neuIn(size: .medium, cornerRadius: 22)
    }

    private func sidebarPersonRow(_ p: FamilyPerson) -> some View {
        let isSelected = viewModel.selectedPersonIDs.contains(p.id)
        return Button {
            viewModel.togglePerson(p.id)
        } label: {
            HStack(spacing: 12) {
                MantelAvatar(person: p, size: 32, ring: isSelected)
                VStack(alignment: .leading, spacing: 1) {
                    Text(p.name)
                        .font(.mantelSans(13.5, weight: .semibold))
                        .foregroundStyle(FurnitureDesign.ink)
                    Text("\(p.photoCount.formatted()) photos")
                        .font(.mantelSans(11))
                        .foregroundStyle(FurnitureDesign.inkFaint)
                }
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .background(
                Group {
                    if isSelected {
                        RoundedRectangle(cornerRadius: 14, style: .continuous)
                            .fill(FurnitureDesign.cream)
                            .neuOut(size: .small, cornerRadius: 14)
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
            searchRow
            heroBlock
            RiverOfTime(
                moments: viewModel.moments,
                filter: viewModel.selectedPersonIDs,
                height: 280,
                onTap: onMomentTap
            )
            .padding(.horizontal, 32)
            .padding(.bottom, 6)
            sharedThisWeek
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, 4)
    }

    private var searchRow: some View {
        HStack(spacing: 12) {
            HStack(spacing: 10) {
                Image(systemName: "magnifyingglass")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(FurnitureDesign.inkFaint)
                TextField("Cora — anything from this year", text: .constant(""))
                    .textFieldStyle(.plain)
                    .font(.mantelSans(14))
                    .foregroundStyle(FurnitureDesign.ink)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .frame(maxWidth: .infinity)
            .neuIn(size: .small, cornerRadius: 14)

            NeuButton(title: "+ Add photos", primary: true)
        }
        .padding(.horizontal, 24)
        .padding(.top, 14)
        .padding(.bottom, 8)
    }

    private var heroBlock: some View {
        VStack(alignment: .leading, spacing: 4) {
            SectionKicker(filterCaption, color: FurnitureDesign.warmSoft, size: 12, tracking: 0.6)
            HStack(alignment: .lastTextBaseline, spacing: 14) {
                Text("2024")
                    .font(.mantelDisplay(44, weight: .medium))
                    .foregroundStyle(FurnitureDesign.ink)
                Text("\(filteredMomentCount) moments · \(estimatedPhotoCount) photos")
                    .font(.mantelSans(15))
                    .foregroundStyle(FurnitureDesign.inkSoft)
            }
        }
        .padding(.horizontal, 32)
        .padding(.top, 20)
        .padding(.bottom, 8)
    }

    private var sharedThisWeek: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Shared this week")
                    .font(.mantelDisplay(20, weight: .medium))
                    .foregroundStyle(FurnitureDesign.ink)
                Spacer()
                Text("see all →")
                    .font(.mantelSans(12))
                    .foregroundStyle(FurnitureDesign.inkFaint)
            }
            HStack(spacing: 14) {
                ForEach(sampleShares.indices, id: \.self) { i in
                    let s = sampleShares[i]
                    SharedThisWeekCard(person: s.person, scene: s.scene, message: s.message, when: s.when, likes: s.likes, comments: s.comments)
                }
            }
        }
        .padding(.horizontal, 32)
        .padding(.top, 12)
        .padding(.bottom, 20)
    }

    // MARK: Helpers

    private var filterCaption: String {
        if viewModel.selectedPersonIDs.isEmpty {
            return "This year · everyone"
        }
        let names = viewModel.family
            .filter { viewModel.selectedPersonIDs.contains($0.id) }
            .map { $0.name }
        let joined = ListFormatter.localizedString(byJoining: names)
        return "This year, with \(joined)"
    }

    private var filteredMomentCount: Int {
        if viewModel.selectedPersonIDs.isEmpty { return viewModel.moments.count }
        return viewModel.moments.filter { !Set($0.whoIDs).intersection(viewModel.selectedPersonIDs).isEmpty }.count
    }

    private var estimatedPhotoCount: Int {
        // visual feel of the chip subtitle — ~20-30 photos/moment.
        filteredMomentCount * 23
    }

    private var sampleShares: [SharedItem] {
        [
            SharedItem(person: viewModel.family[4], scene: .bike,   message: "Iris added 4 photos",    when: "2 hours ago", likes: 4, comments: 2),
            SharedItem(person: viewModel.family[0], scene: .garden, message: "Margot added a video",   when: "yesterday",   likes: 6, comments: 1),
            SharedItem(person: viewModel.family[3], scene: .studio, message: "Theo shared a sketch",   when: "3 days ago",  likes: 2, comments: 0)
        ]
    }
}

private struct SharedItem {
    let person: FamilyPerson
    let scene: PhotoScene
    let message: String
    let when: String
    let likes: Int
    let comments: Int
}

private struct SharedThisWeekCard: View {
    let person: FamilyPerson
    let scene: PhotoScene
    let message: String
    let when: String
    let likes: Int
    let comments: Int

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            ScenePhoto(scene: scene, cornerRadius: 8)
                .frame(width: 60, height: 60)
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    MantelAvatar(person: person, size: 16)
                    Text(message)
                        .font(.mantelSans(12, weight: .semibold))
                        .foregroundStyle(FurnitureDesign.ink)
                }
                Text(when)
                    .font(.mantelSans(11))
                    .foregroundStyle(FurnitureDesign.inkFaint)
                HStack(spacing: 12) {
                    Label("\(likes)", systemImage: "heart")
                        .font(.mantelSans(11))
                        .foregroundStyle(FurnitureDesign.inkSoft)
                    Label("\(comments)", systemImage: "bubble.left")
                        .font(.mantelSans(11))
                        .foregroundStyle(FurnitureDesign.inkSoft)
                }
                .labelStyle(.titleAndIcon)
                .padding(.top, 2)
            }
            Spacer(minLength: 0)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white.opacity(0.85))
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .stroke(FurnitureDesign.hairline, lineWidth: 0.5)
        )
    }
}
