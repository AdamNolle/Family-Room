package com.familyroom.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AlbumSharingPolicyTest {
    @Test
    fun publicationIncludesOnlyAlbumOriginalsAndLivePhotoComponents() {
        val items = listOf(
            AlbumPublicationItem("photo", componentIds = listOf("motion", "deleted-motion")),
            AlbumPublicationItem("motion"),
            AlbumPublicationItem("deleted-motion", deleted = true),
            AlbumPublicationItem("deleted-photo", deleted = true, componentIds = listOf("orphan-motion")),
            AlbumPublicationItem("orphan-motion"),
            AlbumPublicationItem("unrelated"),
        )

        assertEquals(listOf("photo", "motion"), albumPublicationIds(listOf("photo", "deleted-photo"), items))
    }

    @Test
    fun onlyRoomOwnersAndAdminsCanPublishSharedAlbums() {
        assertTrue(canPublishAlbum("owner", albumOnly = false))
        assertTrue(canPublishAlbum("admin", albumOnly = false))
        assertFalse(canPublishAlbum("contributor", albumOnly = false))
        assertFalse(canPublishAlbum("viewer", albumOnly = false))
        assertFalse(canPublishAlbum("owner", albumOnly = true))
    }
}
