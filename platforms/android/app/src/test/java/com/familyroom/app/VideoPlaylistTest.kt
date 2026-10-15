package com.familyroom.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VideoPlaylistTest {
    @Test fun playbackKeepsPublishedOrderAndSkipsUnavailableOrPrivateFilms() {
        val stories = listOf(
            PlaylistStoryRef("story-b", "movie-b", privateDraft = false),
            PlaylistStoryRef("draft", "movie-draft", privateDraft = true),
            PlaylistStoryRef("story-a", "movie-a", privateDraft = false),
            PlaylistStoryRef("deleted-film", "movie-deleted", privateDraft = false),
            PlaylistStoryRef("photo-film", "photo", privateDraft = false),
        )
        val movies = listOf(
            PlaylistMovieRef("movie-a", "video", deleted = false),
            PlaylistMovieRef("movie-b", "video", deleted = false),
            PlaylistMovieRef("movie-deleted", "video", deleted = true),
            PlaylistMovieRef("photo", "photo", deleted = false),
        )
        assertEquals(
            listOf("story-b" to "movie-b", "story-a" to "movie-a"),
            playablePlaylistOrder(
                storyIds = listOf("story-b", "draft", "missing", "story-a", "deleted-film", "photo-film"),
                stories = stories,
                movies = movies,
            ),
        )
    }

    @Test
    fun spokenCaptionCueUsesHalfOpenPlaybackTimeBounds() {
        val cue = SpokenCaptionCue(startMs = 1_250, endMs = 2_500, text = "We found it")

        assertFalse(cue.isActiveAt(1_249))
        assertTrue(cue.isActiveAt(1_250))
        assertTrue(cue.isActiveAt(2_499))
        assertFalse(cue.isActiveAt(2_500))
    }

}
