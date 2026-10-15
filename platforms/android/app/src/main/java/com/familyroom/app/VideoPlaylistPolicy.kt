package com.familyroom.app

internal data class PlaylistStoryRef(
    val id: String,
    val publishedAssetId: String,
    val privateDraft: Boolean,
)

internal data class PlaylistMovieRef(
    val id: String,
    val kind: String,
    val deleted: Boolean,
)

/** Resolve a saved watch order to playable films without exposing drafts or stale originals. */
internal fun playablePlaylistOrder(
    storyIds: List<String>,
    stories: List<PlaylistStoryRef>,
    movies: List<PlaylistMovieRef>,
): List<Pair<String, String>> {
    val publishedStories = stories
        .filter { !it.privateDraft && it.publishedAssetId.isNotBlank() }
        .associateBy { it.id }
    val availableMovies = movies
        .filter { it.kind == "video" && !it.deleted }
        .associateBy { it.id }

    return storyIds.mapNotNull { storyId ->
        val story = publishedStories[storyId] ?: return@mapNotNull null
        val movie = availableMovies[story.publishedAssetId] ?: return@mapNotNull null
        story.id to movie.id
    }
}
