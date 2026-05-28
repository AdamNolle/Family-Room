import SwiftUI

struct PersonDetailView: View {
    let person: PersonModel
    @EnvironmentObject var viewModel: AppViewModel
    @State private var editedName: String = ""
    @State private var isRenaming = false

    var body: some View {
        ZStack {
            DaylightBackground()

            ScrollView {
                VStack(alignment: .leading, spacing: 28) {
                    header

                    Section {
                        if mediaItems.isEmpty {
                            Text("No photos of \(person.name ?? "this person") yet.")
                                .font(.mantelSans(13.5))
                                .foregroundStyle(FurnitureDesign.inkMute)
                                .padding(.horizontal, 24)
                        } else {
                            LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible()), GridItem(.flexible())], spacing: 12) {
                                ForEach(mediaItems) { item in
                                    MediaThumbnail(item: item)
                                }
                            }
                            .padding(.horizontal, 24)
                        }
                    } header: {
                        SectionKicker("Photos", color: FurnitureDesign.inkFaint, size: 11, tracking: 1.1)
                            .padding(.horizontal, 24)
                    }
                }
                .padding(.top, 32)
                .padding(.bottom, 60)
            }
        }
        .navigationTitle("")
        #if os(iOS)
        .navigationBarTitleDisplayMode(.inline)
        #endif
        .alert("Rename", isPresented: $isRenaming) {
            TextField("Name", text: $editedName)
            Button("Cancel", role: .cancel) {}
            Button("Save") {
                person.name = editedName.isEmpty ? person.name : editedName
            }
        }
    }

    var header: some View {
        VStack(alignment: .leading, spacing: 14) {
            Circle()
                .fill(FurnitureDesign.cream)
                .frame(width: 96, height: 96)
                .neuOut(size: .small, cornerRadius: 48)
                .overlay(
                    Text(String((person.name ?? "?").prefix(1)))
                        .font(.mantelDisplay(36, weight: .medium))
                        .foregroundStyle(FurnitureDesign.inkSoft)
                )

            HStack {
                Text(person.name ?? "Unknown")
                    .font(.mantelDisplay(34, weight: .medium))
                    .foregroundStyle(FurnitureDesign.ink)
                Spacer()
                Button {
                    editedName = person.name ?? ""
                    isRenaming = true
                } label: {
                    Image(systemName: "square.and.pencil")
                        .font(.title3)
                        .foregroundStyle(FurnitureDesign.warm)
                }
            }
        }
        .padding(.horizontal, 24)
    }

    var mediaItems: [MediaItemModel] {
        // Phase 3 will populate this from FaceObservation -> MediaItemModel relationships.
        []
    }
}

struct MediaThumbnail: View {
    let item: MediaItemModel

    var body: some View {
        Rectangle()
            .fill(FurnitureDesign.cream)
            .aspectRatio(1, contentMode: .fit)
            .overlay(
                Image(systemName: item.type == "video" ? "play.rectangle" : "photo")
                    .foregroundStyle(FurnitureDesign.inkFaint)
            )
            .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 6, style: .continuous)
                    .stroke(FurnitureDesign.hairline, lineWidth: 0.5)
            )
            .shadow(color: .black.opacity(0.06), radius: 2, x: 0, y: 1)
    }
}
