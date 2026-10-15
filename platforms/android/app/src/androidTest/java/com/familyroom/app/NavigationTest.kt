package com.familyroom.app

import android.graphics.Bitmap
import android.os.SystemClock
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import android.view.inputmethod.InputMethodManager
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.json.JSONObject
import kotlinx.coroutines.runBlocking
import java.io.File

/** Runs the real Keystore/JNI catalog and native controls, without a fixture view model. */
@RunWith(AndroidJUnit4::class)
class NavigationTest {
    @get:Rule val ui = createAndroidComposeRule<MainActivity>()
    private val model get() = ViewModelProvider(ui.activity)[RoomModel::class.java]

    private fun awaitTag(tag: String) {
        ui.waitUntil(30_000) { ui.onAllNodesWithTag(tag).fetchSemanticsNodes().isNotEmpty() }
    }

    private fun capture(name: String) {
        ui.waitUntil(20_000) { ui.onAllNodesWithContentDescription("Loading preview").fetchSemanticsNodes().isEmpty() }
        val directory = InstrumentationRegistry.getArguments().getString("additionalTestOutputDir")
            ?.let(::File) ?: File(ui.activity.getExternalFilesDir(null), "ui-review")
        directory.mkdirs()
 val bitmap = InstrumentationRegistry.getInstrumentation().uiAutomation.takeScreenshot()
 try { File(directory, "$name.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) } }
 finally { bitmap.recycle() }
    }

 private fun hideKeyboard() {
        ui.runOnIdle {
 ui.activity.getSystemService(InputMethodManager::class.java)
 .hideSoftInputFromWindow(ui.activity.window.decorView.windowToken, 0)
 }
 ui.waitForIdle()
 }

 @Test fun videoFinderOpensInLibraryAndExplainsLocalCoverage() {
  ui.waitUntil(30_000) { ui.onAllNodesWithTag("room.settings").fetchSemanticsNodes().isNotEmpty() ||
   ui.onAllNodesWithTag("room.name").fetchSemanticsNodes().isNotEmpty() }
  if (ui.onAllNodesWithTag("room.name").fetchSemanticsNodes().isNotEmpty()) {
   ui.onNodeWithTag("room.name").performTextInput("Video search QA")
   ui.onNodeWithTag("room.create").performClick()
            awaitTag("settings.done")
                ui.onNodeWithText("Get your Room ready").assertExists()
   ui.onNodeWithText("Choose preferred storage").assertExists()
            ui.onNodeWithText("Choose upload folder").assertExists()
            ui.onNodeWithText("Save the recovery key").assertIsDisplayed()
            ui.onNodeWithText("Add the first photos or videos").assertIsDisplayed()
            ui.onNodeWithTag("settings.done").performClick()
        }
        ui.onNodeWithTag("section.Library").performClick()
        awaitTag("video.finder.open")
        ui.onNodeWithTag("video.finder.open").performScrollTo().performClick()
        awaitTag("video.finder.query")
        ui.onNodeWithTag("video.finder.query").assertIsDisplayed()
  ui.onNodeWithText("Frames and spoken words are analyzed on this device.", substring = true).assertExists()
        ui.onNodeWithTag("video.finder.query").performTextInput("blue bicycle")
        ui.onNodeWithTag("video.finder.search").performScrollTo().assertIsDisplayed()
        ui.onNodeWithTag("video.finder.search").assertIsEnabled()
        ui.onNodeWithTag("video.finder.search").performClick()
        ui.onNodeWithTag("video.finder.query").assertExists()
        ui.waitUntil(10_000) {
            ui.onAllNodesWithTag("video.finder.answer").fetchSemanticsNodes().isNotEmpty() ||
                ui.onAllNodesWithTag("video.finder.user-message").fetchSemanticsNodes().isNotEmpty() ||
                ui.onAllNodesWithTag("video.finder.error").fetchSemanticsNodes().isNotEmpty()
        }
        ui.onNodeWithTag("video.finder.answer").assertExists()
        val searchErrors = ui.onAllNodesWithTag("video.finder.error").fetchSemanticsNodes()
        if (searchErrors.isNotEmpty()) {
            throw AssertionError("Video search failed: ${searchErrors.joinToString { it.config.toString() }}")
        }
 ui.onNodeWithTag("video.finder.user-message").assertTextEquals("blue bicycle")
        ui.waitUntil(10_000) {
            ui.onAllNodesWithText("This Room has no videos to search yet", substring = true)
                .fetchSemanticsNodes().isNotEmpty()
        }
  ui.waitForIdle()
  SystemClock.sleep(800) // Allow the system keyboard dismissal animation to finish before the review capture.
 capture("video-finder-android")
 ui.onNodeWithTag("video.finder.query").performTextInput("What about red?")
 ui.onNodeWithTag("video.finder.search").performScrollTo().performClick()
 ui.waitUntil(10_000) {
     ui.onAllNodesWithTag("video.finder.answer").fetchSemanticsNodes().size >= 2
 }
 ui.onNodeWithTag("video.finder.clear-chat").performScrollTo().performClick()
 ui.onNodeWithTag("video.finder.answer").assertDoesNotExist()
 }

    @Test fun peopleFaceReviewRequiresAUserChoiceAndUsesOnlyLocalPhotos() {
        ui.waitUntil(30_000) { model.ready }
        assert(runBlocking {
            model.execute(JSONObject().put("action", "create_room").put("name", "Face review QA"))
        })
        ui.waitUntil(30_000) { model.room?.optString("name") == "Face review QA" }
        ui.onNodeWithTag("section.People").performClick()
        ui.onNodeWithTag("people.face_review.open").performScrollTo().performClick()
        awaitTag("people.face_review.sheet")
        awaitTag("people.face_review.no_local_photos")
        ui.onNodeWithText("A careful second look").assertExists()
        ui.onNodeWithText("names and identities are never guessed", substring = true).assertExists()
        ui.onNodeWithTag("people.face_review.scan").assertIsNotEnabled()
        ui.onNodeWithTag("people.face_review.done").performScrollTo().performClick()
    }

    @Test fun privateCatalogOnboardingAndNavigation() {
        ui.waitUntil(30_000) {
            ui.onAllNodesWithTag("room.settings").fetchSemanticsNodes().isNotEmpty() ||
                ui.onAllNodesWithText("Your private library is ready.").fetchSemanticsNodes().isNotEmpty()
        }
        if (ui.onAllNodesWithTag("room.settings").fetchSemanticsNodes().isEmpty()) {
            ui.onNodeWithTag("room.name").performTextInput("QA memory room")
            hideKeyboard()
            ui.onNodeWithTag("room.create").assertIsEnabled().performClick()
            awaitTag("settings.done")
            ui.onNodeWithTag("settings.done").performClick()
        }
        awaitTag("room.settings")
        ui.onNodeWithTag("room.create").assertDoesNotExist()
        ui.onNodeWithTag("memory.add").assertIsDisplayed()
        capture("home-android")
        for (section in listOf("Library", "Albums", "People", "Theater", "Home")) {
            ui.onNodeWithTag("section.$section").assertIsDisplayed().performClick()
            ui.onNodeWithTag("room.create").assertDoesNotExist()
            ui.onNodeWithText("Connect gateway").assertDoesNotExist()
        }
        ui.onNodeWithTag("room.settings").performClick()
        awaitTag("settings.done")
        ui.onNodeWithText("Room settings").assertIsDisplayed()
        ui.onNodeWithTag("memory.add").assertDoesNotExist()
        ui.onNodeWithTag("settings.done").performClick()
        ui.onNodeWithTag("memory.search").performClick()
        ui.onNodeWithTag("memory.query").performTextInput("missing memory")
        ui.onNodeWithText("No matching memories").assertIsDisplayed()
        hideKeyboard()
        ui.onNodeWithText("Clear").performClick()
        ui.onNodeWithTag("section.Theater").performClick()
        capture("theater-android")
        ui.activityRule.scenario.recreate()
        awaitTag("room.settings")
        ui.onNodeWithTag("settings.done").assertDoesNotExist()
        ui.onNodeWithTag("memory.add").assertIsDisplayed()
    }
}
