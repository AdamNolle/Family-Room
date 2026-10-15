package com.familyroom.app

import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextReplacement
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class MomentSuggestionsTest {
    @get:Rule
    val compose = createComposeRule()

    @Test
    fun suggestedMomentRequiresReviewAndSavesOnlyTheEditedSelection() {
        var savedTitle = ""
        var savedAssets = emptyList<String>()
        val suggestion = JSONObject()
            .put("start", 1_700_000_000L)
            .put("end", 1_700_000_240L)
            .put("asset_ids", JSONArray().put("one").put("two").put("three"))
        val assets = listOf("one", "two", "three").map { id ->
            JSONObject().put("id", id).put("filename", "$id.jpg")
        }

        compose.setContent {
            MaterialTheme {
                MomentSuggestionsPanel(
                    suggestions = listOf(suggestion),
                    assets = assets,
                    onSave = { title, ids ->
                        savedTitle = title
                        savedAssets = ids
                    },
                )
            }
        }

        compose.onNodeWithTag("moment.suggestion.card").performClick()
        compose.onNodeWithTag("moment.suggestion.title")
            .performTextReplacement("Weekend at the lake")
        compose.onNodeWithTag("moment.suggestion.asset.two").performClick()
        compose.runOnIdle {
            assertEquals("", savedTitle)
            assertTrue(savedAssets.isEmpty())
        }
        compose.onNodeWithTag("moment.suggestion.save").performClick()

        compose.runOnIdle {
            assertEquals("Weekend at the lake", savedTitle)
            assertEquals(setOf("one", "three"), savedAssets.toSet())
        }
        assertTrue(compose.onAllNodesWithText("Review Moment").fetchSemanticsNodes().isEmpty())
    }
}
