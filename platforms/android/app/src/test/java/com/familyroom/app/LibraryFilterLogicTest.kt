package com.familyroom.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.Instant
import java.time.ZoneOffset

class LibraryFilterLogicTest {
    @Test
    fun momentFilterMatchesOnlyAssetsInSelectedMoment() {
        val assetIds = setOf("photo-1", "video-1")
        assertTrue(matchesMomentSelection("photo-1", "summer", assetIds))
        assertFalse(matchesMomentSelection("outside", "summer", assetIds))
        assertTrue(matchesMomentSelection("outside", "", assetIds))
    }

    @Test
    fun noPeopleSelectionKeepsEveryMemoryVisible() {
        assertTrue(matchesPeopleSelection(emptySet(), emptySet(), requireAll = true))
        assertTrue(matchesPeopleSelection(emptySet(), emptySet(), requireAll = false))
    }

    @Test
    fun anyPeopleModeMatchesAtLeastOneSelectedPerson() {
        val selected = setOf("ada", "noah")
        assertTrue(matchesPeopleSelection(setOf("ada"), selected, requireAll = false))
        assertFalse(matchesPeopleSelection(setOf("mira"), selected, requireAll = false))
    }

    @Test
    fun allPeopleModeRequiresTheEntireSelection() {
        val selected = setOf("ada", "noah")
        assertTrue(matchesPeopleSelection(selected, selected, requireAll = true))
        assertFalse(matchesPeopleSelection(setOf("ada"), selected, requireAll = true))
    }

    @Test
    fun taggingTogglesOnlyTheSelectedPerson() {
        val original = setOf("ada", "noah")
        assertEquals(setOf("ada", "noah", "mira"), togglePersonTag(original, "mira"))
        assertEquals(setOf("noah"), togglePersonTag(original, "ada"))
        assertEquals(setOf("ada", "noah"), original)
    }

    @Test
    fun photoVideoAndYearFiltersMustAllMatch() {
        val capturedAt = Instant.parse("2024-06-15T12:00:00Z").epochSecond
        assertTrue(matchesLibraryScope("video", capturedAt, "video", 2024, ZoneOffset.UTC))
        assertFalse(matchesLibraryScope("photo", capturedAt, "video", 2024, ZoneOffset.UTC))
        assertFalse(matchesLibraryScope("video", capturedAt, "all", 2025, ZoneOffset.UTC))
        assertTrue(matchesLibraryScope("photo", capturedAt, "all", 0, ZoneOffset.UTC))
    }

    @Test
    fun captureYearsAreUniqueAndNewestFirst() {
        val captures = listOf(
            Instant.parse("2023-01-01T00:00:00Z").epochSecond,
            Instant.parse("2025-01-01T00:00:00Z").epochSecond,
            Instant.parse("2023-12-31T00:00:00Z").epochSecond,
        )
        assertEquals(listOf(2025, 2023), availableCaptureYears(captures, ZoneOffset.UTC))
    }

    @Test
    fun invalidCaptureTimeDoesNotInventAYear() {
        assertNull(capturedYear(Long.MAX_VALUE, ZoneOffset.UTC))
    }
}
