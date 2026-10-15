package com.familyroom.app

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertFalse
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class PeopleFilteringTest {
    @get:Rule
    val compose = createComposeRule()

    @Test
    fun assetPersonIdsAreReadFromRoomTags() {
        val asset = JSONObject().put("people", JSONArray().put("ada").put("noah"))
        assertTrue(matchesPeopleSelection(asset.peopleIds(), setOf("ada", "noah"), requireAll = true))
        assertFalse(matchesPeopleSelection(asset.peopleIds(), setOf("ada", "mira"), requireAll = true))
    }

    @Test
    fun personMergeDialogConfirmsTheSelectedKeptProfile() {
        val people = listOf(
            JSONObject().put("id", "ada").put("name", "Ada"),
            JSONObject().put("id", "noah").put("name", "Noah"),
        )
        val merged = mutableStateOf<Pair<String, String>?>(null)

        compose.setContent {
            MaterialTheme {
                PersonMergeDialog(
                    people = people,
                    onDismiss = {},
                    onMerge = { source, target -> merged.value = source to target },
                )
            }
        }

        compose.onNodeWithTag("people.merge.confirm").assertIsEnabled().performClick()
        compose.runOnIdle { assertEquals("ada" to "noah", merged.value) }
    }

    @Test
    fun assetPeopleMenuAddsAndRemovesTags() {
        val selected = mutableStateOf(emptySet<String>())
        val people = listOf(JSONObject().put("id", "ada").put("name", "Ada"))

        compose.setContent {
            MaterialTheme {
                AssetPeopleMenu(
                    assetPeople = selected.value,
                    people = people,
                    enabled = true,
                    onTogglePerson = { selected.value = togglePersonTag(selected.value, it) },
                    tag = "asset.people",
                )
            }
        }

        compose.onNodeWithTag("asset.people").performClick()
        compose.onNodeWithTag("asset.people.ada").performClick()
        compose.runOnIdle { assertEquals(setOf("ada"), selected.value) }
        compose.onNodeWithTag("asset.people").performClick()
        compose.onNodeWithTag("asset.people.ada").performClick()
        compose.runOnIdle { assertTrue(selected.value.isEmpty()) }
    }

    @Test
    fun libraryFilterExposesAnyAllAndClearControls() {
        val selected = mutableStateOf(setOf("ada"))
        val requireAll = mutableStateOf(false)
        val people = listOf(JSONObject().put("id", "ada").put("name", "Ada"))

        compose.setContent {
            MaterialTheme {
                PeopleFilterBar(
                    people = people,
                    selectedPeople = selected.value,
                    requireAll = requireAll.value,
                    onTogglePerson = { id ->
                        selected.value = if (id in selected.value) selected.value - id else selected.value + id
                    },
                    onRequireAllChange = { requireAll.value = it },
                    onClear = {
                        selected.value = emptySet()
                        requireAll.value = false
                    },
                )
            }
        }

        compose.onNodeWithTag("people.filter.any").assertIsSelected()
        compose.onNodeWithTag("people.filter.all").performClick()
        compose.onNodeWithTag("people.filter.all").assertIsSelected()
        compose.onNodeWithTag("people.filter.clear").performClick()
        compose.runOnIdle { assertTrue(selected.value.isEmpty()) }
        assertTrue(compose.onAllNodesWithTag("people.filter.clear").fetchSemanticsNodes().isEmpty())
        assertTrue(compose.onAllNodesWithTag("people.filter.any").fetchSemanticsNodes().isEmpty())
    }
}
