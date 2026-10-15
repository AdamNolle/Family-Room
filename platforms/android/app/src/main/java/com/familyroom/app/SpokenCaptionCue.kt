package com.familyroom.app

import org.json.JSONObject

internal data class SpokenCaptionCue(
    val startMs: Long,
    val endMs: Long,
    val text: String,
) {
    fun isActiveAt(milliseconds: Long): Boolean = startMs <= milliseconds && milliseconds < endMs
}

internal fun parseSpokenCaptionCues(response: JSONObject): List<SpokenCaptionCue> {
    val encodedCues = response.optJSONArray("cues") ?: return emptyList()
    val cues = buildList {
        for (index in 0 until minOf(encodedCues.length(), MAX_SPOKEN_CAPTION_CUES)) {
            val encoded = encodedCues.optJSONObject(index) ?: continue
            val startMs = encoded.optLong("start_ms", -1)
            val endMs = encoded.optLong("end_ms", -1)
            val text = encoded.optString("text").trim()
            if (startMs < 0 || endMs <= startMs || endMs - startMs > MAX_SPOKEN_CUE_DURATION_MS ||
                text.isEmpty() || text.length > MAX_SPOKEN_CUE_TEXT_LENGTH
            ) {
                continue
            }
            add(SpokenCaptionCue(startMs, endMs, text))
        }
    }
    return cues.sortedWith(compareBy(SpokenCaptionCue::startMs, SpokenCaptionCue::endMs, SpokenCaptionCue::text))
        .distinct()
}

private const val MAX_SPOKEN_CAPTION_CUES = 15_000
private const val MAX_SPOKEN_CUE_DURATION_MS = 120_000L
private const val MAX_SPOKEN_CUE_TEXT_LENGTH = 1_024
