package com.familyroom.app

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import androidx.test.ext.junit.runners.AndroidJUnit4

@RunWith(AndroidJUnit4::class)
class VideoSearchPolicyTest {
    @Test fun firstSearchQueuesOnlyLocalUnindexedVideos() {
        val status = JSONObject().put("videos", JSONArray()
            .put(video("local-new", local = true, indexed = false))
            .put(video("remote-new", local = false, indexed = false))
            .put(video("local-ready", local = true, indexed = true, frames = true, audio = true)))

        assertEquals(listOf("local-new"), pendingLocalVideoIndexes(status).map { it.getString("asset_id") })
    }

    @Test fun manualIndexCanRetryMissingAudioAfterFramesAreReady() {
        val status = JSONObject().put("videos", JSONArray()
            .put(video("frames-only", local = true, indexed = true, frames = true, audio = false))
            .put(video("remote-frames-only", local = false, indexed = true, frames = true, audio = false))
            .put(video("complete", local = true, indexed = true, frames = true, audio = true)))

        assertEquals(emptyList<String>(), pendingLocalVideoIndexes(status).map { it.getString("asset_id") })
        assertEquals(listOf("frames-only"), pendingLocalVideoIndexes(status, includeMissingAudio = true)
            .map { it.getString("asset_id") })
    }

    @Test fun remoteVideoIndexingRequiresExplicitOptIn() {
        val status = JSONObject().put("videos", JSONArray()
            .put(video("local-new", local = true, indexed = false))
            .put(video("remote-new", local = false, indexed = false))
            .put(video("remote-ready", local = false, indexed = true, frames = true, audio = true)))

        assertEquals(listOf("local-new"), pendingVideoIndexes(status).map { it.getString("asset_id") })
        assertEquals(listOf("local-new", "remote-new"),
            pendingVideoIndexes(status, includeRemoteVideos = true).map { it.getString("asset_id") })
    }

    private fun video(id: String, local: Boolean, indexed: Boolean,
                      frames: Boolean = false, audio: Boolean = false) =
        JSONObject().put("asset_id", id).put("local", local).put("indexed", indexed)
            .put("frames_indexed", frames).put("audio_indexed", audio)
}
