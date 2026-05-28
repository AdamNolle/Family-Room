import SwiftUI

struct MantelpiecePhoneView: View {
    @ObservedObject var viewModel: AppViewModel
    var onMomentTap: (Moment) -> Void = { _ in }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                header
                peopleRow
                todayHero
                onThisDay
                earlierThisWeek
                    .padding(.bottom, 40)
            }
            .padding(.top, 8)
        }
    }

    // MARK: Header

    private var header: some View {
        HStack(alignment: .center) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Hi Cora —")
                    .font(.mantelSans(13))
                    .foregroundStyle(FurnitureDesign.inkFaint)
                Text("Family")
                    .font(.mantelDisplay(28, weight: .medium))
                    .foregroundStyle(FurnitureDesign.ink)
            }
            Spacer()
            Button {
                // open search
            } label: {
                ZStack {
                    Color.clear.frame(width: 44, height: 44)
                    Image(systemName: "magnifyingglass")
                        .font(.system(size: 16, weight: .medium))
                        .foregroundStyle(FurnitureDesign.inkSoft)
                }
                .neuOut(size: .small, cornerRadius: 22)
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 24)
    }

    // MARK: People row

    private var peopleRow: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 16) {
                ForEach(viewModel.family) { p in
                    Button {
                        viewModel.togglePerson(p.id)
                    } label: {
                        VStack(spacing: 6) {
                            ZStack(alignment: .bottomTrailing) {
                                MantelAvatar(
                                    person: p,
                                    size: 56,
                                    ring: viewModel.selectedPersonIDs.contains(p.id),
                                    dim: !viewModel.selectedPersonIDs.isEmpty && !viewModel.selectedPersonIDs.contains(p.id)
                                )
                                if p.id == "M" || p.id == "I" {
                                    Circle()
                                        .fill(FurnitureDesign.good)
                                        .frame(width: 14, height: 14)
                                        .overlay(Circle().stroke(FurnitureDesign.cream, lineWidth: 2.5))
                                        .offset(x: 2, y: 2)
                                }
                            }
                            Text(p.nickname)
                                .font(.mantelSans(11.5, weight: .medium))
                                .foregroundStyle(FurnitureDesign.ink)
                        }
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, 24)
        }
    }

    // MARK: Today hero card

    private var todayHero: some View {
        let m = viewModel.moments.last ?? viewModel.heroMoment
        let posterID = m.whoIDs.last ?? "I"
        let poster = viewModel.family.first(where: { $0.id == posterID }) ?? viewModel.family[4]

        return VStack(alignment: .leading, spacing: 12) {
            Text("Today")
                .font(.mantelDisplay(17, weight: .medium))
                .foregroundStyle(FurnitureDesign.ink)
                .padding(.horizontal, 24)

            Button {
                onMomentTap(m)
            } label: {
                VStack(spacing: 0) {
                    ScenePhoto(scene: m.scene, cornerRadius: 0)
                        .frame(height: 220)
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(spacing: 8) {
                            MantelAvatar(person: poster, size: 20)
                            Text("\(poster.name) · 2 hours ago")
                                .font(.mantelSans(12, weight: .semibold))
                                .foregroundStyle(FurnitureDesign.ink)
                        }
                        Text(m.title)
                            .font(.mantelDisplay(17, weight: .medium))
                            .foregroundStyle(FurnitureDesign.ink)
                            .lineLimit(2)
                            .multilineTextAlignment(.leading)
                        Text("\u{201C}She didn\u{2019}t even ask for the training wheels.\u{201D} \u{2014} Theo")
                            .font(.mantelDisplay(13, weight: .regular, italic: true))
                            .foregroundStyle(FurnitureDesign.inkSoft)
                            .multilineTextAlignment(.leading)

                        Hairline().padding(.vertical, 8)

                        HStack(spacing: 14) {
                            HStack(spacing: 4) {
                                Image(systemName: "heart.fill")
                                Text("4")
                            }
                            .font(.mantelSans(13, weight: .semibold))
                            .foregroundStyle(FurnitureDesign.warmSoft)

                            HStack(spacing: 4) {
                                Image(systemName: "bubble.left")
                                Text("2 comments")
                            }
                            .font(.mantelSans(13))
                            .foregroundStyle(FurnitureDesign.inkSoft)

                            Spacer()
                            Text("4 photos")
                                .font(.mantelSans(12))
                                .foregroundStyle(FurnitureDesign.inkFaint)
                        }
                    }
                    .padding(14)
                }
                .background(FurnitureDesign.cream)
                .clipShape(RoundedRectangle(cornerRadius: 22, style: .continuous))
                .neuOut(size: .small, cornerRadius: 22)
            }
            .buttonStyle(.plain)
            .padding(.horizontal, 24)
        }
    }

    // MARK: On this day

    private var onThisDay: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("On this day")
                    .font(.mantelDisplay(17, weight: .medium))
                    .foregroundStyle(FurnitureDesign.ink)
                Spacer()
                Text("3 years")
                    .font(.mantelSans(12))
                    .foregroundStyle(FurnitureDesign.inkFaint)
            }
            .padding(.horizontal, 24)

            HStack(spacing: 10) {
                ForEach(onThisDayItems, id: \.year) { item in
                    ZStack(alignment: .bottomLeading) {
                        ScenePhoto(scene: item.scene, cornerRadius: 12)
                            .frame(height: 94)
                        Text(item.year)
                            .font(.mantelSans(11, weight: .semibold))
                            .foregroundStyle(.white)
                            .shadow(color: .black.opacity(0.4), radius: 1, x: 0, y: 1)
                            .padding(.horizontal, 8)
                            .padding(.bottom, 6)
                    }
                    .frame(maxWidth: .infinity)
                }
            }
            .padding(.horizontal, 24)
        }
    }

    private var onThisDayItems: [(year: String, scene: PhotoScene)] {
        [("2023", .garden), ("2022", .lake), ("2020", .studio)]
    }

    // MARK: Earlier this week

    private var earlierThisWeek: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Earlier this week")
                .font(.mantelDisplay(17, weight: .medium))
                .foregroundStyle(FurnitureDesign.ink)
                .padding(.horizontal, 24)

            VStack(spacing: 10) {
                EarlierRow(person: viewModel.family[0], scene: .garden, message: "Roses are coming back", day: "Mon")
                EarlierRow(person: viewModel.family[3], scene: .studio, message: "New piece in progress", day: "Sun")
            }
            .padding(.horizontal, 24)
        }
    }
}

private struct EarlierRow: View {
    let person: FamilyPerson
    let scene: PhotoScene
    let message: String
    let day: String

    var body: some View {
        HStack(spacing: 12) {
            ScenePhoto(scene: scene, cornerRadius: 10)
                .frame(width: 56, height: 56)
            VStack(alignment: .leading, spacing: 2) {
                Text(person.name)
                    .font(.mantelSans(13, weight: .semibold))
                    .foregroundStyle(FurnitureDesign.ink)
                Text(message)
                    .font(.mantelSans(12))
                    .foregroundStyle(FurnitureDesign.inkSoft)
            }
            Spacer(minLength: 0)
            Text(day)
                .font(.mantelSans(11))
                .foregroundStyle(FurnitureDesign.inkFaint)
        }
        .padding(12)
        .frame(maxWidth: .infinity)
        .neuOut(size: .small, cornerRadius: 18)
    }
}
