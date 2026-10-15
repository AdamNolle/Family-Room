package com.familyroom.app

internal data class VideoIndexEntry(
    val assetId: String,
    val isLocal: Boolean,
    val isIndexed: Boolean,
    val framesIndexed: Boolean,
    val audioIndexed: Boolean,
)

internal fun pendingVideoIndexEntries(
    videos: List<VideoIndexEntry>,
    includeMissingAudio: Boolean = false,
    includeRemoteVideos: Boolean = false,
): List<VideoIndexEntry> = videos.filter { video ->
    (includeRemoteVideos || video.isLocal) &&
        (!video.isIndexed || !video.framesIndexed ||
            (includeMissingAudio && !video.audioIndexed))
}
