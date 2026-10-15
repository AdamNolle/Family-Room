package com.familyroom.app

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class RoomHighlightsTest {
    @get:Rule
    val compose = createComposeRule()

    @Test
    fun layeredSpotlightOpensFeaturedPhotoAndKeepsItsBrowseStrip() {
        val opened = mutableListOf<String>()
        val assets = listOf("featured", "back-one", "back-two").map { id ->
            JSONObject().put("id", id)
        }

        compose.setContent {
            MaterialTheme {
                RoomHighlights(
                    assets = assets,
                    thumbnail = { asset, modifier ->
                        Box(modifier.background(Color.White)) {
                            Text(asset.optString("id"))
                        }
                    },
                    onOpen = { asset -> opened += asset.optString("id") },
                )
            }
        }

        compose.onNodeWithTag("home.featured.stack").assertIsDisplayed()
        compose.onNodeWithText("back-one").assertExists()
        compose.onNodeWithText("featured").performClick()
        compose.runOnIdle { assertEquals(listOf("featured"), opened) }
    }
}
