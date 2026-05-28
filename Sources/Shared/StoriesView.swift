import SwiftUI

struct StoriesView: View {
    @EnvironmentObject var viewModel: AppViewModel

    var body: some View {
        NavigationStack {
            ZStack {
                DaylightBackground()

                ScrollView {
                    VStack(alignment: .leading, spacing: 28) {
                        header

                        if viewModel.stories.isEmpty {
                            emptyState
                        } else {
                            LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 18) {
                                ForEach(viewModel.stories) { story in
                                    StoryCard(story: story)
                                }
                            }
                            .padding(.horizontal, 24)
                        }
                    }
                    .padding(.top, 24)
                    .padding(.bottom, 60)
                }
            }
        }
    }

    var header: some View {
        VStack(alignment: .leading, spacing: 4) {
            SectionKicker("Memories", color: FurnitureDesign.warmSoft, size: 12, tracking: 0.6)
            Text("The family theater")
                .font(.mantelDisplay(28, weight: .medium))
                .foregroundStyle(FurnitureDesign.ink)
            Text("Hand-cut short films and a quiet weekly auto-reel.")
                .font(.mantelSans(13.5))
                .foregroundStyle(FurnitureDesign.inkSoft)
        }
        .padding(.horizontal, 24)
    }

    var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "film.stack")
                .font(.system(size: 44))
                .foregroundStyle(FurnitureDesign.inkFaint)
                .padding(.top, 36)
            Text("No stories yet")
                .font(.mantelDisplay(20, weight: .medium))
                .foregroundStyle(FurnitureDesign.ink)
            Text("Once you import family videos, you can stitch them into a cinematic highlight.")
                .font(.mantelSans(13.5))
                .foregroundStyle(FurnitureDesign.inkSoft)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 40)
            NeuButton(title: "Create Your First Story", icon: "sparkles", primary: true)
                .padding(.top, 8)
                .opacity(0.6)
                .disabled(true)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 36)
        .padding(.horizontal, 24)
    }
}

struct StoryCard: View {
    let story: SampleStory

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            ZStack {
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(FurnitureDesign.ink.opacity(0.85))
                    .aspectRatio(16.0 / 9.0, contentMode: .fit)
                Image(systemName: "play.fill")
                    .font(.title)
                    .foregroundStyle(.white.opacity(0.9))
            }
            .neuOut(size: .small, cornerRadius: 12)

            Text(story.name)
                .font(.mantelSans(13.5, weight: .semibold))
                .foregroundStyle(FurnitureDesign.ink)

            Text(story.subtitle)
                .font(.mantelSans(11.5))
                .foregroundStyle(FurnitureDesign.inkFaint)
        }
    }
}
