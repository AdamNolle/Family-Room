import SwiftUI

struct PeopleView: View {
    @ObservedObject var viewModel: AppViewModel

    var body: some View {
        NavigationStack {
            ZStack {
                DaylightBackground()

                ScrollView {
                    VStack(alignment: .leading, spacing: 28) {
                        header

                        VStack(alignment: .leading, spacing: 14) {
                            SectionKicker("The five", color: FurnitureDesign.warmSoft, size: 12, tracking: 1.0)
                                .padding(.horizontal, 24)
                            LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 14) {
                                ForEach(viewModel.family) { person in
                                    FamilyPersonTile(person: person, selected: viewModel.selectedPersonIDs.contains(person.id)) {
                                        viewModel.togglePerson(person.id)
                                    }
                                }
                            }
                            .padding(.horizontal, 24)
                        }

                        if !viewModel.people.isEmpty {
                            VStack(alignment: .leading, spacing: 14) {
                                SectionKicker("Identified faces", color: FurnitureDesign.inkFaint, size: 11, tracking: 1.1)
                                    .padding(.horizontal, 24)

                                ScrollView(.horizontal, showsIndicators: false) {
                                    HStack(spacing: 18) {
                                        ForEach(viewModel.people) { person in
                                            NavigationLink(value: person) {
                                                LegacyPersonTile(person: person)
                                            }
                                            .buttonStyle(.plain)
                                        }
                                    }
                                    .padding(.horizontal, 24)
                                    .padding(.vertical, 6)
                                }
                            }
                        }

                        Spacer(minLength: 40)
                    }
                    .padding(.top, 24)
                }
            }
            .navigationDestination(for: PersonModel.self) { person in
                PersonDetailView(person: person)
            }
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 4) {
            SectionKicker("Family Circle", color: FurnitureDesign.warmSoft, size: 12, tracking: 0.6)
            Text("Everyone, sorted kindly")
                .font(.mantelDisplay(28, weight: .medium))
                .foregroundStyle(FurnitureDesign.ink)
            Text("Tap a face to filter the rest of the app.")
                .font(.mantelSans(13.5))
                .foregroundStyle(FurnitureDesign.inkSoft)
        }
        .padding(.horizontal, 24)
    }
}

struct FamilyPersonTile: View {
    let person: FamilyPerson
    let selected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 10) {
                MantelAvatar(person: person, size: 80, ring: selected)
                    .padding(.top, 14)
                Text(person.name)
                    .font(.mantelDisplay(17, weight: .medium))
                    .foregroundStyle(FurnitureDesign.ink)
                Text(person.role)
                    .font(.mantelSans(11.5, weight: .medium))
                    .foregroundStyle(FurnitureDesign.inkFaint)
                Text("\(person.photoCount.formatted()) photos")
                    .font(.mantelSans(11))
                    .foregroundStyle(FurnitureDesign.inkMute)
                    .padding(.bottom, 14)
            }
            .frame(maxWidth: .infinity)
            .neuOut(size: .small, cornerRadius: 18)
        }
        .buttonStyle(.plain)
        .contentShape(Rectangle())
    }
}

struct LegacyPersonTile: View {
    let person: PersonModel

    var body: some View {
        VStack(spacing: 10) {
            Circle()
                .fill(FurnitureDesign.cream)
                .frame(width: 80, height: 80)
                .neuOut(size: .small, cornerRadius: 40)
                .overlay(
                    Text(String((person.name ?? "?").prefix(1)))
                        .font(.mantelDisplay(28, weight: .medium))
                        .foregroundStyle(FurnitureDesign.inkSoft)
                )
            Text(person.name ?? "Unknown")
                .font(.mantelSans(12.5, weight: .semibold))
                .foregroundStyle(FurnitureDesign.ink)
        }
    }
}
