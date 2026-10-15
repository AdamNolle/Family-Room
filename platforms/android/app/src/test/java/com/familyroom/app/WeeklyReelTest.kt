package com.familyroom.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WeeklyReelTest {
    @Test
    fun generatedReelUsesContributionTimeAndKeepsTheLatestTwenty() {
        val now = 1_800_000_000L
        val assets = (0 until 22).map { index ->
            WeeklyReelAssetCandidate("asset-$index", "photo", now - (22 - index) * 60L)
        } + listOf(
            WeeklyReelAssetCandidate("old", "video", now - 8 * 86_400L),
            WeeklyReelAssetCandidate("future", "photo", now + 301L),
            WeeklyReelAssetCandidate("deleted", "video", now, deleted = true),
            WeeklyReelAssetCandidate("audio", "audio", now),
        )

        val selected = selectWeeklyReelAssets(assets, now)

        assertEquals(20, selected.size)
        assertEquals("asset-2", selected.first().id)
        assertEquals("asset-21", selected.last().id)
        assertTrue(selected.all { it.contributedAt in now - 7 * 86_400L..now + 300L })
    }

    @Test
    fun onlyTheNewestActivePrivateWeeklyDraftIsReused() {
        val now = 1_800_000_000L
        val stories = listOf(
            WeeklyReelStoryCandidate("old", now - 8 * 86_400L, true, true),
            WeeklyReelStoryCandidate("published", now, true, false),
            WeeklyReelStoryCandidate("published-private", now, true, true, published = true),
            WeeklyReelStoryCandidate("manual", now, false, true),
            WeeklyReelStoryCandidate("archived", now, true, true, archived = true),
            WeeklyReelStoryCandidate("saved-earlier", now - 100L, true, true),
            WeeklyReelStoryCandidate("saved-later", now - 50L, true, true),
            WeeklyReelStoryCandidate("future", now + 301L, true, true),
        )

        assertEquals("saved-later", reusableWeeklyReelDraft(stories, now)?.id)
        assertNull(reusableWeeklyReelDraft(stories.take(5), now))
    }
}
