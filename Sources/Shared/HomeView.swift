import SwiftUI

struct HomeView: View {
    @ObservedObject var viewModel: AppViewModel
    @State private var path = NavigationPath()
    #if os(iOS)
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    #endif

    var body: some View {
        NavigationStack(path: $path) {
            ZStack {
                DaylightBackground()
                content
            }
            .navigationDestination(for: Moment.self) { moment in
                MomentDetailView(moment: moment, viewModel: viewModel)
            }
        }
    }

    @ViewBuilder
    private var content: some View {
        #if os(macOS)
        ScrollView {
            MantelpieceMacView(viewModel: viewModel, onMomentTap: openMoment)
        }
        #else
        if horizontalSizeClass == .regular {
            MantelpieceIPadView(viewModel: viewModel, onMomentTap: openMoment)
        } else {
            MantelpiecePhoneView(viewModel: viewModel, onMomentTap: openMoment)
        }
        #endif
    }

    private func openMoment(_ m: Moment) {
        path.append(m)
    }
}

// MARK: - Moment detail view

struct MomentDetailView: View {
    let moment: Moment
    @ObservedObject var viewModel: AppViewModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        ZStack {
            DaylightBackground()
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    ScenePhoto(scene: moment.scene, cornerRadius: 20)
                        .frame(height: 320)
                        .padding(.horizontal, 18)
                        .padding(.top, 8)

                    VStack(alignment: .leading, spacing: 8) {
                        SectionKicker(kicker, color: FurnitureDesign.warmSoft, size: 12, tracking: 0.6)
                        Text(moment.title)
                            .font(.mantelDisplay(30, weight: .medium))
                            .foregroundStyle(FurnitureDesign.ink)
                        Text(moment.quote ?? "We took the boat out at six. The sky was the color of a peach pit. Iris fell asleep on his shoulder.")
                            .font(.mantelDisplay(14.5, weight: .regular, italic: true))
                            .foregroundStyle(FurnitureDesign.inkSoft)
                            .padding(.top, 4)
                    }
                    .padding(.horizontal, 24)
                    .padding(.top, 20)

                    whoWasHereCard
                        .padding(.horizontal, 24)
                        .padding(.top, 16)

                    photosGrid
                        .padding(.horizontal, 24)
                        .padding(.top, 18)

                    statsRow
                        .padding(.horizontal, 24)
                        .padding(.top, 16)

                    commentsSection
                        .padding(.horizontal, 24)
                        .padding(.top, 24)
                        .padding(.bottom, 48)
                }
            }
        }
        .navigationTitle("")
        #if os(iOS)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarLeading) {
                Button {
                    dismiss()
                } label: {
                    HStack(spacing: 4) {
                        Image(systemName: "chevron.left")
                        Text("Family")
                    }
                    .font(.mantelSans(15, weight: .medium))
                    .foregroundStyle(FurnitureDesign.warmSoft)
                }
            }
        }
        #endif
    }

    private var kicker: String {
        var parts = [moment.day, "2024"]
        if let place = moment.place { parts.append(place) }
        return parts.joined(separator: " · ")
    }

    private var whoWasHere: [FamilyPerson] {
        moment.whoIDs.compactMap { id in viewModel.family.first(where: { $0.id == id }) }
    }

    private var whoWasHereCard: some View {
        HStack(spacing: 0) {
            HStack(spacing: -8) {
                ForEach(whoWasHere) { p in
                    MantelAvatar(person: p, size: 28)
                        .overlay(Circle().stroke(FurnitureDesign.cream, lineWidth: 2))
                }
            }
            Text(whoWasHere.count == viewModel.family.count ? "Everyone was here" : "\(whoWasHere.count) people")
                .font(.mantelSans(12.5))
                .foregroundStyle(FurnitureDesign.inkSoft)
                .padding(.leading, 16)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
        .background(Color.white.opacity(0.85))
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .stroke(FurnitureDesign.hairline, lineWidth: 0.5)
        )
    }

    private var photosGrid: some View {
        VStack(spacing: 6) {
            HStack(spacing: 6) {
                ScenePhoto(scene: .sunset, cornerRadius: 8)
                    .frame(height: 186)
                    .frame(maxWidth: .infinity)
                VStack(spacing: 6) {
                    ScenePhoto(scene: .lake,   cornerRadius: 8).frame(height: 90)
                    ScenePhoto(scene: .indoor, cornerRadius: 8).frame(height: 90)
                }
                .frame(width: 110)
            }
            HStack(spacing: 6) {
                ScenePhoto(scene: .cake,   cornerRadius: 8).frame(height: 70)
                ScenePhoto(scene: .garden, cornerRadius: 8).frame(height: 70)
                ScenePhoto(scene: .indoor, cornerRadius: 8).frame(height: 70)
            }
        }
    }

    private var statsRow: some View {
        HStack(spacing: 8) {
            StatBlock(value: "61",  label: "photos")
            StatBlock(value: "3",   label: "videos")
            StatBlock(value: "5.0", label: "family weight")
        }
    }

    private var commentsSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("What everyone said")
                .font(.mantelDisplay(16, weight: .medium))
                .foregroundStyle(FurnitureDesign.ink)

            commentRow(person: viewModel.family[4], text: "best day of my life",            time: "6:14 pm")
            commentRow(person: viewModel.family[0], text: "so happy with my whole family", time: "7:02 pm")
        }
    }

    private func commentRow(person: FamilyPerson, text: String, time: String) -> some View {
        HStack(alignment: .top, spacing: 10) {
            MantelAvatar(person: person, size: 28)
            VStack(alignment: .leading, spacing: 2) {
                Text(person.name)
                    .font(.mantelSans(12, weight: .semibold))
                    .foregroundStyle(FurnitureDesign.ink)
                Text(text)
                    .font(.mantelSans(13))
                    .foregroundStyle(FurnitureDesign.inkSoft)
                Text(time)
                    .font(.mantelSans(10.5))
                    .foregroundStyle(FurnitureDesign.inkFaint)
                    .padding(.top, 2)
            }
            Spacer(minLength: 0)
        }
        .padding(12)
        .background(Color.white.opacity(0.85))
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .stroke(FurnitureDesign.hairline, lineWidth: 0.5)
        )
    }
}
