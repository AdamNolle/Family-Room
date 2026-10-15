import AVKit
import SwiftUI

/// Keep SwiftUI in charge of playback state while AppKit supplies the macOS control.
struct RoomVideoPlayer: View {
    let player: AVPlayer

    var body: some View {
        #if os(macOS)
        MacVideoPlayer(player: player)
        #else
        VideoPlayer(player: player)
        #endif
    }
}

#if os(macOS)
private struct MacVideoPlayer: NSViewRepresentable {
    let player: AVPlayer

    func makeNSView(context: Context) -> AVPlayerView {
        let view = AVPlayerView()
        view.player = player
        view.controlsStyle = .floating
        return view
    }

    func updateNSView(_ view: AVPlayerView, context: Context) {
        if view.player !== player { view.player = player }
    }

    static func dismantleNSView(_ view: AVPlayerView, coordinator: ()) {
        view.player?.pause()
        view.player = nil
    }
}
#endif
