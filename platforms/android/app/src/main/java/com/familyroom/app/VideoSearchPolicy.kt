package com.familyroom.app

import org.json.JSONObject

internal data class VideoSearchIndexPlan(
    val includeAudio: Boolean,
    val includeFrames: Boolean,
    val includeMissingAudio: Boolean,
    val speechUnavailable: Boolean,
) {
    val shouldIndex: Boolean get() = includeAudio || includeFrames
}

/** The chosen evidence scope controls which local analyzers run for a search. */
internal fun videoSearchIndexPlan(modality: String, speechModelAvailable: Boolean): VideoSearchIndexPlan {
    val wantsAudio = modality != "frame"
    val includeAudio = wantsAudio && speechModelAvailable
    val includeFrames = modality != "audio"
    return VideoSearchIndexPlan(
        includeAudio = includeAudio,
        includeFrames = includeFrames,
        includeMissingAudio = includeAudio,
        speechUnavailable = wantsAudio && !speechModelAvailable,
    )
}

internal fun shouldExplainMissingSpeech(
    plan: VideoSearchIndexPlan,
    videos: List<VideoIndexEntry>,
    includeRemoteVideos: Boolean,
): Boolean = plan.speechUnavailable && videos.any { video ->
    (includeRemoteVideos || video.isLocal) && !video.audioIndexed
}

internal fun videoQueryRequiresSpeechIndex(query: String, modality: String): Boolean {
    if (modality == "frame") return false
    for ((open, close) in listOf('"' to '"', '“' to '”', '‘' to '’')) {
        val start = query.indexOf(open)
        val end = if (start >= 0) query.indexOf(close, start + 1) else -1
        if (start >= 0 && end > start + 1 && query.substring(start + 1, end).isNotBlank()) return true
    }

    val apostrophes = query.indices.filter { query[it] == '\'' }
    for ((open, close) in apostrophes.zipWithNext()) {
        val before = query.getOrNull(open - 1)
        val after = query.getOrNull(close + 1)
        val opensQuote = before == null || !before.isLetterOrDigit()
        val closesQuote = after == null || !after.isLetterOrDigit()
        if (opensQuote && closesQuote && query.substring(open + 1, close).isNotBlank()) return true
    }
    return false
}

/** Remote-only videos are included only after the member opts into downloading them. */
internal fun pendingVideoIndexes(
    status: JSONObject,
    includeMissingAudio: Boolean = false,
    includeRemoteVideos: Boolean = false,
): List<JSONObject> {
    val videos = status.rows("videos")
    val selected = pendingVideoIndexEntries(
        videoIndexEntries(videos),
        includeMissingAudio = includeMissingAudio,
        includeRemoteVideos = includeRemoteVideos,
    ).mapTo(mutableSetOf()) { it.assetId }
    return videos.filter { it.optString("asset_id") in selected }
}

internal fun videoIndexEntries(videos: List<JSONObject>): List<VideoIndexEntry> = videos.map { video ->
    VideoIndexEntry(
        assetId = video.optString("asset_id"),
        isLocal = video.optBoolean("local"),
        isIndexed = video.optBoolean("indexed"),
        framesIndexed = video.optBoolean("frames_indexed"),
        audioIndexed = video.optBoolean("audio_indexed"),
    )
}

internal fun pendingLocalVideoIndexes(status: JSONObject, includeMissingAudio: Boolean = false): List<JSONObject> =
    pendingVideoIndexes(status, includeMissingAudio, includeRemoteVideos = false)
