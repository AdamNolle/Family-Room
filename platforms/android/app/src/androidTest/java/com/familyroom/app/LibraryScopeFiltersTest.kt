package com.familyroom.app

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class LibraryScopeFiltersTest {
    @get:Rule
    val compose = createComposeRule()

    @Test
    fun filterControlsUpdateMediaAndYearAndOfferAnAllReset() {
        val kind = mutableStateOf("all")
        val year = mutableStateOf(0)
        val moment = mutableStateOf("")
        compose.setContent {
            MaterialTheme {
                LibraryScopeFilterBar(
                    selectedKind = kind.value,
                    onKindChange = { kind.value = it },
                    selectedYear = year.value,
                    years = listOf(2025, 2024),
                    onYearChange = { year.value = it },
                    selectedMomentId = moment.value,
                    moments = listOf("summer" to "Summer at the lake"),
                    onMomentChange = { moment.value = it },
                )
            }
        }

        compose.onNodeWithTag("library.filter.media.all").assertIsSelected()
        compose.onNodeWithTag("library.filter.media.video").performClick()
        compose.onNodeWithTag("library.filter.media.video").assertIsSelected()
        compose.runOnIdle { assertEquals("video", kind.value) }
        compose.onNodeWithTag("library.filter.year").performClick()
        compose.onNodeWithTag("library.filter.year.2024").performClick()
        compose.runOnIdle { assertEquals(2024, year.value) }
        compose.onNodeWithTag("library.filter.moment").performClick()
        compose.onNodeWithTag("library.filter.moment.summer").performClick()
        compose.runOnIdle { assertEquals("summer", moment.value) }
        compose.onNodeWithTag("library.filter.media.all").performClick()
        compose.onNodeWithTag("library.filter.year").performClick()
        compose.onNodeWithTag("library.filter.year.all").performClick()
        compose.onNodeWithTag("library.filter.moment").performClick()
        compose.onNodeWithTag("library.filter.moment.all").performClick()
        compose.runOnIdle {
            assertEquals("all", kind.value)
            assertEquals(0, year.value)
            assertEquals("", moment.value)
        }
    }
}
