import Foundation
import AVFoundation

class StoryComposer {
    func createHighlightReel(from mediaURLs: [URL], completion: @escaping (URL?) -> Void) {
        let composition = AVMutableComposition()
        guard let videoTrack = composition.addMutableTrack(withMediaType: .video, preferredTrackID: kCMPersistentTrackID_Invalid) else {
            completion(nil)
            return
        }
        
        var currentTime = CMTime.zero
        
        Task {
            for url in mediaURLs {
                let asset = AVAsset(url: url)
                
                do {
                    let tracks = try await asset.loadTracks(withMediaType: .video)
                    guard let assetTrack = tracks.first else { continue }
                    
                    let duration = CMTime(seconds: 3, preferredTimescale: 600)
                    let assetDuration = try await asset.load(.duration)
                    let range = CMTimeRange(start: .zero, duration: min(assetDuration, duration))
                    
                    try videoTrack.insertTimeRange(range, of: assetTrack, at: currentTime)
                    currentTime = CMTimeAdd(currentTime, range.duration)
                } catch {
                    continue
                }
            }
            completion(nil)
        }
    }
}
