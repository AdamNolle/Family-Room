import SwiftUI
import AVKit

/// A short, physical transition from a saved film to its real local player.
struct TheaterScreeningView: View {
    @EnvironmentObject private var store: LibraryStore
    @Environment(\.dismiss) private var dismiss
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.scenePhase) private var scenePhase

    let project: FilmProject
    let asset: LibraryAsset
    var queue: [FilmProject] = []
    var playlistTitle: String? = nil

    @State private var stage: DiscStage = .waiting
    @State private var player: AVPlayer?
    @State private var failure: String?
    @State private var index = 0

    private enum DiscStage { case waiting, lifted, inserting, screening }
    private var currentProject: FilmProject { queue.isEmpty ? project : queue[min(index, queue.count - 1)] }
    private var currentAsset: LibraryAsset? {
        if queue.isEmpty { return asset }
        guard let id = currentProject.publishedAsset else { return nil }
        return store.room?.assets.first { $0.id == id && !$0.deleted }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                HStack(alignment: .top) {
                    VStack(alignment: .leading, spacing: 8) {
                        RoomEyebrow(text: playlistTitle.map { "Now showing · \($0)" } ?? "Now showing · \(store.room?.name ?? "Family Room")")
                        Text(currentProject.title)
                            .font(RoomArt.display(38, relativeTo: .largeTitle))
                            .foregroundStyle(CozyTheme.ink)
                    }
                    Spacer(minLength: 12)
                    Button("Close", systemImage: "xmark") { dismiss() }
                        .labelStyle(.iconOnly)
                        .buttonStyle(.bordered)
                        .accessibilityLabel("Close film")
                }

                if stage == .screening, let player {
                    RoomVideoPlayer(player: player)
                        .aspectRatio(CGFloat(max(1, currentProject.width)) / CGFloat(max(1, currentProject.height)), contentMode: .fit)
                        .clipShape(RoundedRectangle(cornerRadius: 14))
                        .overlay { RoundedRectangle(cornerRadius: 14).strokeBorder(CozyTheme.line, lineWidth: 1) }
                        .transition(reduceMotion ? .identity : .opacity.combined(with: .scale(scale: 0.97)))
                        .accessibilityLabel("Playing \(currentProject.title)")
                } else {
                    discStage
                        .accessibilityElement(children: .ignore)
                        .accessibilityLabel(failure == nil ? "Putting \(currentProject.title) into the player" : "Film unavailable")
                }

                if !queue.isEmpty {
                    HStack(spacing: 14) {
                        Text("Film \(index + 1) of \(queue.count)")
                            .font(.caption.monospacedDigit()).foregroundStyle(CozyTheme.secondary)
                        Spacer(minLength: 8)
                        Button("Previous", systemImage: "backward.end.fill") { index -= 1 }
                            .disabled(index == 0)
                        Button("Next", systemImage: "forward.end.fill") { index += 1 }
                            .disabled(index + 1 >= queue.count)
                    }
                    .buttonStyle(.bordered)
                }

                if let failure {
                    VStack(alignment: .leading, spacing: 10) {
                        Text("This film is waiting for its original.")
                            .font(RoomArt.display(28, relativeTo: .title2))
                        Text(failure).foregroundStyle(CozyTheme.secondary)
                        Button("Try again", systemImage: "arrow.clockwise") {
                            self.failure = nil
                            Task { await prepareAndPlay() }
                        }
                    }
                } else if stage != .screening {
                    HStack(spacing: 10) {
                        ProgressView().controlSize(.small)
                        Text("Preparing your film…")
                    }
                    .font(.callout)
                    .foregroundStyle(CozyTheme.secondary)
                } else {
                    Text("Your original is preserved. Playback uses a local viewing copy.")
                        .font(.caption)
                        .foregroundStyle(CozyTheme.secondary)
                }
            }
            .padding(24)
            .frame(maxWidth: 760, alignment: .leading)
            .frame(maxWidth: .infinity)
        }
        .background(MemoryGalleryBackdrop())
        .task(id: currentProject.id) { await prepareAndPlay() }
        .onReceive(NotificationCenter.default.publisher(for: AVPlayerItem.didPlayToEndTimeNotification)) { note in
            guard let ended = note.object as? AVPlayerItem, ended === player?.currentItem,
                  index + 1 < queue.count else { return }
            index += 1
        }
        .onChange(of: scenePhase) { _, phase in if phase != .active { player?.pause() } }
        .onDisappear { player?.pause() }
    }

    private var discStage: some View {
        GeometryReader { geometry in
            let width = min(geometry.size.width, 500)
            let discSize = min(width * 0.52, 190)
            ZStack {
                RoundedRectangle(cornerRadius: 23)
                    .fill(LinearGradient(colors: [CozyTheme.surface, CozyTheme.sidebar, CozyTheme.surface], startPoint: .topLeading, endPoint: .bottomTrailing))
                    .overlay { RoundedRectangle(cornerRadius: 23).strokeBorder(CozyTheme.line, lineWidth: 1) }
                    .overlay { RoundedRectangle(cornerRadius: 22).strokeBorder(.white.opacity(0.52), lineWidth: 0.7).padding(1) }
                    .shadow(color: .black.opacity(0.13), radius: 18, y: 12)
                    .frame(width: width, height: 104)
                    .offset(y: 71)
                Capsule()
                    .fill(LinearGradient(
                        colors: stage == .inserting
                            ? [CozyTheme.ink, CozyTheme.seasonalHighlight, CozyTheme.ink]
                            : [CozyTheme.ink.opacity(0.86), CozyTheme.ink, CozyTheme.ink.opacity(0.86)],
                        startPoint: .top,
                        endPoint: .bottom
                    ))
                    .frame(width: discSize * 1.24, height: stage == .inserting ? 7 : 6)
                    .overlay {
                        Capsule().strokeBorder(
                            stage == .inserting ? CozyTheme.seasonalHighlight.opacity(0.9) : .white.opacity(0.28),
                            lineWidth: 0.6
                        )
                    }
                    .overlay {
                        if stage == .inserting {
                            Capsule().fill(CozyTheme.seasonalHighlight.opacity(0.82))
                                .frame(width: discSize * 0.32, height: 3)
                                .blur(radius: 5)
                        }
                    }
                    .offset(y: 52)
                filmDisc(size: discSize)
                    .rotation3DEffect(
                        .degrees(stage == .inserting ? 76 : stage == .lifted ? -13 : 0),
                        axis: (x: 1, y: 0.08, z: 0),
                        perspective: 0.72
                    )
                    .rotationEffect(.degrees(stage == .inserting ? 570 : 0))
                    .scaleEffect(stage == .lifted ? 1.06 : stage == .inserting ? 0.68 : 1)
                    .offset(y: stage == .lifted ? -45 : stage == .inserting ? 91 : -18)
                    .opacity(stage == .inserting ? 0.08 : 1)
                    .shadow(
                        color: .black.opacity(stage == .lifted ? 0.25 : 0.17),
                        radius: stage == .lifted ? 23 : 15,
                        y: stage == .lifted ? 16 : 10
                    )
            }
            .frame(maxWidth: .infinity)
        }
        .frame(height: 266)
    }

    private func filmDisc(size: CGFloat) -> some View {
        AssetPreview(asset: currentAsset ?? asset)
            .frame(width: size, height: size)
            .clipShape(Circle())
            .overlay { Circle().strokeBorder(.white, lineWidth: 7) }
            .overlay { Circle().strokeBorder(CozyTheme.ink, lineWidth: 1.4) }
            .overlay { Circle().strokeBorder(.white.opacity(0.43), lineWidth: 0.8).padding(size * 0.13) }
            .overlay { Circle().strokeBorder(.black.opacity(0.23), lineWidth: 0.7).padding(size * 0.24) }
            .overlay {
                Circle().strokeBorder(
                    AngularGradient(
                        colors: [.white.opacity(0.82), .clear, .black.opacity(0.2), .white.opacity(0.58)],
                        center: .center
                    ),
                    lineWidth: 1.4
                )
                .padding(size * 0.095)
                .allowsHitTesting(false)
            }
            .overlay {
                Circle()
                    .fill(Color.black)
                    .frame(width: size * 0.19, height: size * 0.19)
                    .overlay { Circle().strokeBorder(.white.opacity(0.95), lineWidth: 2.4) }
                    .overlay { Circle().strokeBorder(.black.opacity(0.7), lineWidth: 0.7).padding(3) }
            }
            .shadow(color: .black.opacity(0.2), radius: 17, y: 11)
    }

    @MainActor private func prepareAndPlay() async {
        guard let currentAsset, currentAsset.kind == "video" else {
            failure = "This published item is not a playable movie."
            return
        }
        stage = .waiting
        failure = nil
        player?.pause()
        do {
            let url = try await store.materialize(currentAsset)
            guard !Task.isCancelled else { return }
            player = AVPlayer(url: url)
            if reduceMotion {
                stage = .screening
                player?.play()
                return
            }
            withAnimation(.spring(response: 0.42, dampingFraction: 0.66)) { stage = .lifted }
            try await Task.sleep(for: .milliseconds(460))
            withAnimation(.easeIn(duration: 0.62)) { stage = .inserting }
            try await Task.sleep(for: .milliseconds(620))
            withAnimation(.easeOut(duration: 0.28)) { stage = .screening }
            player?.play()
        } catch is CancellationError {
            player?.pause()
        } catch {
            failure = store.readable(error)
        }
    }
}
