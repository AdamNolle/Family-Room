package com.familyroom.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VideoIndexPolicyTest {
    private val videos = listOf(
        VideoIndexEntry("local-new", true, false, false, false),
        VideoIndexEntry("remote-new", false, false, false, false),
        VideoIndexEntry("remote-frames-only", false, true, true, false),
        VideoIndexEntry("remote-ready", false, true, true, true),
    )

    @Test fun remoteVideosAreExcludedUntilExplicitlyIncluded() {
        assertEquals(
            listOf("local-new"),
            pendingVideoIndexEntries(videos).map { it.assetId },
        )
        assertEquals(
            listOf("local-new", "remote-new"),
            pendingVideoIndexEntries(videos, includeRemoteVideos = true).map { it.assetId },
        )
    }

    @Test fun modalityRetryIncludesOnlyMissingAudioOnSelectedSources() {
        assertEquals(
            listOf("local-new", "remote-new", "remote-frames-only"),
            pendingVideoIndexEntries(videos, includeMissingAudio = true, includeRemoteVideos = true)
                .map { it.assetId },
        )
    }

    @Test fun quotedAudioRequestsNeedSpeechButFrameRequestsDoNot() {
        assertTrue(videoQueryRequiresSpeechIndex("Find someone saying ‘happy birthday’", "audio"))
        assertTrue(videoQueryRequiresSpeechIndex("Find \"we are home\"", "all"))
        assertFalse(videoQueryRequiresSpeechIndex("What does the sign say?", "frame"))
        assertFalse(videoQueryRequiresSpeechIndex("What’s happening?", "audio"))
    }

    @Test fun audioSearchIndexesMissingSpeechWithoutReprocessingFrames() {
        val plan = videoSearchIndexPlan("audio", speechModelAvailable = true)
        assertFalse(plan.includeFrames)
        assertTrue(plan.includeAudio)
        assertTrue(plan.includeMissingAudio)
        assertTrue(plan.shouldIndex)
        assertFalse(plan.speechUnavailable)
        assertEquals(
            listOf("frames-only"),
            pendingVideoIndexEntries(
                listOf(VideoIndexEntry("frames-only", true, true, true, false)),
                includeMissingAudio = plan.includeMissingAudio,
            ).map { it.assetId },
        )
    }

    @Test fun frameSearchWorksWithoutAnOfflineSpeechModel() {
        val plan = videoSearchIndexPlan("frame", speechModelAvailable = false)
        assertTrue(plan.includeFrames)
        assertFalse(plan.includeAudio)
        assertTrue(plan.shouldIndex)
        assertFalse(plan.speechUnavailable)
    }

    @Test fun allEvidenceSearchKeepsFrameIndexingWhenSpeechModelIsUnavailable() {
        val plan = videoSearchIndexPlan("all", speechModelAvailable = false)
        assertTrue(plan.includeFrames)
        assertFalse(plan.includeAudio)
        assertTrue(plan.shouldIndex)
        assertTrue(plan.speechUnavailable)
    }

    @Test fun audioOnlySearchWaitsForSpeechModelInsteadOfIndexingFramesAsAudio() {
        val plan = videoSearchIndexPlan("audio", speechModelAvailable = false)
        assertFalse(plan.includeFrames)
        assertFalse(plan.includeAudio)
        assertFalse(plan.shouldIndex)
        assertTrue(plan.speechUnavailable)
    }

    @Test fun missingSpeechNoticeOnlyCoversSelectedVideosWithoutSpeechIndexes() {
        val plan = videoSearchIndexPlan("all", speechModelAvailable = false)
        val videos = listOf(
            VideoIndexEntry("local", true, true, true, true),
            VideoIndexEntry("remote", false, true, true, false),
        )

        assertFalse(shouldExplainMissingSpeech(plan, videos, includeRemoteVideos = false))
        assertTrue(shouldExplainMissingSpeech(plan, videos, includeRemoteVideos = true))
    }
}
