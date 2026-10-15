import AVKit
import SwiftUI

/// Full-size view of one photo or video, with favorite and delete.
struct MediaViewer: View {
    @ObservedObject var item: MediaItem
    @EnvironmentObject private var store: RoomStore
    @Environment(\.dismiss) private var dismiss
    @State private var player: AVPlayer?
    @State private var tempURL: URL?
    @State private var confirmDelete = false

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            if item.isVideo {
                if let player { VideoPlayer(player: player).ignoresSafeArea() } else { ProgressView().tint(.white) }
            } else {
                MediaThumb(item: item, maxSide: 3000, showVideoBadge: false).aspectRatio(contentMode: .fit)
            }
            VStack {
                HStack {
                    Button { dismiss() } label: { Image(systemName: "xmark").font(.system(size: 15, weight: .semibold)).foregroundStyle(.white).frame(width: 36, height: 36).background(.white.opacity(0.14), in: Circle()) }.buttonStyle(.plain)
                    Spacer()
                    Button { store.toggleFavorite(item) } label: { Image(systemName: item.isFavorite ? "heart.fill" : "heart").font(.system(size: 15, weight: .semibold)).foregroundStyle(.white).frame(width: 36, height: 36).background(.white.opacity(0.14), in: Circle()) }.buttonStyle(.plain)
                    Button { confirmDelete = true } label: { Image(systemName: "trash").font(.system(size: 15, weight: .semibold)).foregroundStyle(.white).frame(width: 36, height: 36).background(.white.opacity(0.14), in: Circle()) }.buttonStyle(.plain)
                }
                .padding(16)
                Spacer()
                HStack {
                    Text((item.capturedAt ?? Date()).formatted(date: .long, time: .shortened)).font(.num(12)).foregroundStyle(.white.opacity(0.8))
                    Spacer()
                    Text("Added by \(item.addedByName ?? "someone")").font(.sans(12)).foregroundStyle(.white.opacity(0.6))
                }
                .padding(16)
            }
        }
        .task { await loadVideo() }
        .onDisappear { player?.pause(); if let tempURL { try? FileManager.default.removeItem(at: tempURL) } }
        .confirmationDialog("Delete this \(item.isVideo ? "video" : "photo")?", isPresented: $confirmDelete, titleVisibility: .visible) {
            Button("Delete", role: .destructive) { store.delete(item); dismiss() }
        }
        #if os(macOS)
        .frame(minWidth: 720, minHeight: 480)
        #endif
    }

    private func loadVideo() async {
        guard item.isVideo, let data = item.data else { return }
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("view-\(UUID().uuidString).\(item.fileExtension ?? "mov")")
        do { try data.write(to: url); tempURL = url; let p = AVPlayer(url: url); player = p; p.play() } catch { }
    }
}

/// Plays a finished story.
struct StoryPlayer: View {
    @ObservedObject var story: Story
    @Environment(\.dismiss) private var dismiss
    @State private var player: AVPlayer?
    @State private var tempURL: URL?

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            if let player { VideoPlayer(player: player).ignoresSafeArea() } else { ProgressView().tint(.white) }
            VStack {
                HStack {
                    Button { dismiss() } label: { Image(systemName: "xmark").font(.system(size: 15, weight: .semibold)).foregroundStyle(.white).frame(width: 36, height: 36).background(.white.opacity(0.14), in: Circle()) }.buttonStyle(.plain)
                    Spacer()
                    if let tempURL { ShareLink(item: tempURL) { Image(systemName: "square.and.arrow.up").font(.system(size: 15, weight: .semibold)).foregroundStyle(.white).frame(width: 36, height: 36).background(.white.opacity(0.14), in: Circle()) }.buttonStyle(.plain) }
                }
                .padding(16)
                Spacer()
                Text(story.displayTitle).font(.display(26)).foregroundStyle(.white.opacity(0.85)).padding(16)
            }
        }
        .task {
            guard let data = story.movie else { return }
            let url = FileManager.default.temporaryDirectory.appendingPathComponent("\(story.displayTitle.replacingOccurrences(of: "/", with: "-")).mp4")
            try? data.write(to: url); tempURL = url
            let p = AVPlayer(url: url); player = p; p.play()
        }
        .onDisappear { player?.pause(); if let tempURL { try? FileManager.default.removeItem(at: tempURL) } }
        #if os(macOS)
        .frame(minWidth: 820, minHeight: 500)
        #endif
    }
}
