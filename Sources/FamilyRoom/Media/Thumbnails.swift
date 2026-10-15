import CoreData
import SwiftUI

/// Decoded thumbnails, keyed by object and target size, decoded off the main thread once.
final class ThumbnailCache {
    static let shared = ThumbnailCache()
    private let cache = NSCache<NSString, CGImage>()
    init() { cache.countLimit = 600 }

    func image(for key: String) -> CGImage? { cache.object(forKey: key as NSString) }
    func store(_ image: CGImage, for key: String) { cache.setObject(image, forKey: key as NSString) }
}

/// Shows a media item's thumbnail, decoding lazily. Falls back to a warm placeholder.
struct MediaThumb: View {
    @ObservedObject var item: MediaItem
    var maxSide: CGFloat = 640
    var showVideoBadge = true
    @State private var image: CGImage?

    var body: some View {
        // The fill image lives in an overlay so it can never grow the frame it is placed in.
        Theme.hair2
            .overlay {
                if let image { Image(decorative: image, scale: 1).resizable().aspectRatio(contentMode: .fill) }
            }
            .clipped()
            .overlay {
            if showVideoBadge, item.isVideo {
                VStack { Spacer(); HStack { Spacer()
                    Text(Catalog.duration(item.duration)).font(.num(10, .semibold)).foregroundStyle(.white)
                        .padding(.horizontal, 6).padding(.vertical, 3).background(.black.opacity(0.45), in: Capsule()).padding(6)
                } }
            }
            }
        .task(id: "\(item.objectID.uriRepresentation().absoluteString)-\(Int(maxSide))") { await load() }
    }

    private func load() async {
        let key = "\(item.objectID.uriRepresentation().absoluteString)-\(Int(maxSide))"
        if let cached = ThumbnailCache.shared.image(for: key) { image = cached; return }
        let data = item.thumbnail ?? (item.isVideo ? nil : item.data)
        guard let data else { return }
        let side = maxSide
        let decoded = await Task.detached(priority: .userInitiated) { MediaImporter.decode(data, maxSide: side) }.value
        if let decoded { ThumbnailCache.shared.store(decoded, for: key); image = decoded }
    }
}

/// Thumbnail for raw bytes (story posters, face crops).
struct DataThumb: View {
    let data: Data?
    var maxSide: CGFloat = 640
    @State private var image: CGImage?

    var body: some View {
        Theme.hair2
            .overlay {
                if let image { Image(decorative: image, scale: 1).resizable().aspectRatio(contentMode: .fill) }
            }
            .clipped()
        .task(id: data?.hashValue ?? 0) {
            guard let data else { image = nil; return }
            let side = maxSide
            image = await Task.detached { MediaImporter.decode(data, maxSide: side) }.value
        }
    }
}
