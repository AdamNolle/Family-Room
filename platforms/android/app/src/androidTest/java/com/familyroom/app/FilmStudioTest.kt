package com.familyroom.app

import android.graphics.Bitmap
import android.graphics.Color
import android.view.inputmethod.InputMethodManager
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.Calendar

/** Real native editor, encrypted JNI catalog and private revisions; no fixture view model. */
@RunWith(AndroidJUnit4::class)
class FilmStudioTest {
    @get:Rule val ui = createAndroidComposeRule<MainActivity>()
    private val model get() = ViewModelProvider(ui.activity)[RoomModel::class.java]

    private fun photoFixture(name: String): File {
        val file = File(ui.activity.cacheDir, name)
        val bitmap = Bitmap.createBitmap(160, 90, Bitmap.Config.ARGB_8888)
        try {
            bitmap.eraseColor(0xffa25b33.toInt())
            file.outputStream().use { output ->
                check(bitmap.compress(Bitmap.CompressFormat.JPEG, 95, output))
            }
        } finally {
            bitmap.recycle()
        }
        return file
    }
    private fun awaitState(predicate: () -> Boolean) { ui.waitUntil(30_000, predicate) }
    private fun scrollRoomTo(tag: String) {
        ui.onNodeWithTag("room.content").performScrollToNode(hasTestTag(tag))
    }
    private fun scrollTo(tag: String) {
        val container = listOf("film.content", "film.mediaPanel", "film.inspectorPanel").firstOrNull { name ->
            ui.onAllNodes(hasTestTag(name) and hasAnyDescendant(hasTestTag(tag))).fetchSemanticsNodes().isNotEmpty()
        }
        if (container != null) ui.onNodeWithTag(container).performScrollToNode(hasTestTag(tag))
        if (tag == "film.split") ui.onNodeWithTag(tag).performScrollTo()
    }
    private fun capture(name: String, dialog: Boolean = false) {
        val dir = InstrumentationRegistry.getArguments().getString("additionalTestOutputDir")?.let(::File)
            ?: File(ui.activity.getExternalFilesDir(null), "ui-review")
        dir.mkdirs()
        val bitmap = if (dialog) InstrumentationRegistry.getInstrumentation().uiAutomation.takeScreenshot()
            else ui.onRoot().captureToImage().asAndroidBitmap()
        bitmap.let {
            File(dir, "$name.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
        }
    }
    @Test fun homeMomentRiverFiltersByYearAndPersistsFeaturedState() {
        awaitState { model.ready }
        assertTrue(runBlocking {
            model.execute(JSONObject().put("action", "create_room").put("name", "Moment river QA"))
        })
        val roomId = model.roomId

        fun importPhoto(name: String, year: Int): String {
            val file = File(ui.activity.cacheDir, name)
            val bitmap = Bitmap.createBitmap(160, 90, Bitmap.Config.ARGB_8888)
            try {
                bitmap.eraseColor(if (year == 2024) 0xffa25b33.toInt() else 0xff627c94.toInt())
                file.outputStream().use { assertTrue(bitmap.compress(Bitmap.CompressFormat.JPEG, 95, it)) }
            } finally {
                bitmap.recycle()
            }
            val capturedAt = Calendar.getInstance().apply {
                clear()
                set(year, Calendar.JUNE, 15, 12, 0)
            }.timeInMillis / 1_000
            return runBlocking {
                try {
                    val result = model.command(
                        JSONObject().put("action", "import")
                            .put("room_id", roomId)
                            .put("path", file.absolutePath)
                            .put("captured_at", capturedAt),
                    )
                    result.getString("id")
                } finally {
                    file.delete()
                }
            }
        }

        val asset2024 = importPhoto("moment-river-2024.jpg", 2024)
        val asset2025 = importPhoto("moment-river-2025.jpg", 2025)
        runBlocking {
            model.command(
                JSONObject().put("action", "save_moment")
                    .put("room_id", roomId)
                    .put("id", "moment-river-2024")
                    .put("title", "Summer picnic")
                    .put("asset_ids", org.json.JSONArray().put(asset2024))
                    .put("featured", false),
            )
            model.command(
                JSONObject().put("action", "save_moment")
                    .put("room_id", roomId)
                    .put("id", "moment-river-2025")
                    .put("title", "Garden afternoon")
                    .put("asset_ids", org.json.JSONArray().put(asset2025))
                    .put("featured", true),
            )
        model.reload()
        val storedYears = model.room!!.rows("moments").map { moment ->
            Calendar.getInstance().apply { timeInMillis = moment.getLong("start") * 1_000L }
                .get(Calendar.YEAR)
        }.toSet()
        assertEquals(setOf(2024, 2025), storedYears)
    }

        ui.onNodeWithTag("section.Home").performClick()
        scrollRoomTo("moment.river.panel")
        awaitState {
            ui.onAllNodesWithTag("moment.river.card.moment-river-2024").fetchSemanticsNodes().isNotEmpty() &&
                ui.onAllNodesWithTag("moment.river.card.moment-river-2025").fetchSemanticsNodes().isNotEmpty()
        }
        ui.onNodeWithTag("moment.river.year.2024").performClick()
        ui.waitForIdle()
        ui.onNodeWithTag("moment.river.year.2024").assertIsSelected()
        ui.onNodeWithTag("moment.river.card.moment-river-2024").performScrollTo().assertIsDisplayed()
        ui.onNodeWithTag("moment.river.count").assertTextEquals("1")
        ui.onNodeWithTag("moment.river.card.moment-river-2025").assertIsNotDisplayed()
        ui.onNodeWithTag("moment.river.year.all").performClick()
        ui.onNodeWithTag("moment.river.feature.moment-river-2025")
            .performScrollTo().assertIsDisplayed().performClick()
        awaitState {
            model.room?.rows("moments")?.firstOrNull { it.optString("id") == "moment-river-2025" }
                ?.optBoolean("featured") == false
        }
    }

    @Test fun similarPhotoReviewFindsCloseLocalPhotosWithoutChangingOriginals() {
        awaitState { model.ready }
        assertTrue(runBlocking {
            model.execute(JSONObject().put("action", "create_room").put("name", "Look-alike QA"))
        })
        val roomId = model.roomId

        fun importGradient(name: String, lift: Int): String {
            val file = File(ui.activity.cacheDir, name)
            val bitmap = Bitmap.createBitmap(90, 80, Bitmap.Config.ARGB_8888)
            try {
                for (y in 0 until 80) {
                    for (x in 0 until 90) {
                        val level = (x * 2 + lift).coerceIn(0, 255)
                        bitmap.setPixel(x, y, Color.rgb(level, level, level))
                    }
                }
                file.outputStream().use {
                    assertTrue(bitmap.compress(Bitmap.CompressFormat.PNG, 100, it))
                }
            } finally {
                bitmap.recycle()
            }
            return runBlocking {
                try {
                    model.command(
                        JSONObject().put("action", "import")
                            .put("room_id", roomId)
                            .put("path", file.absolutePath),
                    ).getString("id")
                } finally {
                    file.delete()
                }
            }
        }

        val firstId = importGradient("look-alike-a.png", 0)
        val secondId = importGradient("look-alike-b.png", 20)
        assertNotEquals(firstId, secondId)
        runBlocking { model.reload() }

        ui.onNodeWithTag("section.Library").performClick()
        ui.onNodeWithTag("library.similar_photos.scan").performClick()
        awaitState {
            ui.onAllNodesWithTag("library.similar_photos.pair.0")
                .fetchSemanticsNodes().isNotEmpty()
        }
        ui.onNodeWithTag("library.similar_photos.pair.0").assertIsDisplayed()
        assertEquals(
            2,
            model.room?.rows("assets")?.count { !it.optBoolean("deleted") },
        )
    }

    @Test fun sharedLikeTogglesFromNativeLibrary() {
        awaitState { model.ready }
        val source = photoFixture("like-fixture.jpg")
        val assetId = runBlocking {
            assertTrue(model.execute(JSONObject().put("action", "create_room").put("name", "Like QA")))
            val asset = model.command(JSONObject().put("action", "import")
                .put("room_id", model.roomId).put("path", source.absolutePath))
            model.reload()
            asset.getString("id")
        }
        source.delete()
        ui.onNodeWithTag("section.Library").performClick()
        scrollRoomTo("memory.card.$assetId")
        ui.onNodeWithTag("memory.like.$assetId").performScrollTo().performClick()
        awaitState { model.room?.rows("reactions")?.count { it.optBoolean("liked") } == 1 }
        ui.onNodeWithTag("memory.like.$assetId").performScrollTo().performClick()
        awaitState { model.room?.rows("reactions")?.count { it.optBoolean("liked") } == 0 }
    }

    @Test fun familyNotePostsWithAuthorAndAppearsOnItsMemory() {
        awaitState { model.ready }
        val source = photoFixture("note-fixture.jpg")
        val assetId = runBlocking {
            assertTrue(model.execute(JSONObject().put("action", "create_room").put("name", "Notes QA")))
            val asset = model.command(JSONObject().put("action", "import")
                .put("room_id", model.roomId).put("path", source.absolutePath))
            model.reload()
            asset.getString("id")
        }
        source.delete()

        ui.onNodeWithTag("section.Library").performClick()
        scrollRoomTo("memory.card.$assetId")
        val fieldTag = "memory.comment.input.$assetId"
        val postTag = "memory.comment.post.$assetId"
        ui.onNodeWithTag(fieldTag).performScrollTo().performTextInput("Save this for Grandma’s visit")
        ui.onNodeWithTag(postTag).performScrollTo().assertIsEnabled().performClick()
        awaitState {
            model.room?.rows("comments")?.any {
                it.optString("asset_id") == assetId &&
                    it.optString("body") == "Save this for Grandma’s visit" &&
                    it.optString("author").isNotBlank()
            } == true
        }
        ui.onNodeWithText("Save this for Grandma’s visit").assertExists()
        ui.onNodeWithText("You", useUnmergedTree = true).assertExists()
    }

    @Test fun captionHistoryReviewRestoresARevisionAsANewCaption() {
        awaitState { model.ready }
        val source = photoFixture("caption-history-fixture.jpg")
        val assetId = runBlocking {
            assertTrue(model.execute(JSONObject().put("action", "create_room").put("name", "Caption history QA")))
            val asset = model.command(JSONObject().put("action", "import")
                .put("room_id", model.roomId).put("path", source.absolutePath))
            model.command(JSONObject().put("action", "patch_asset").put("room_id", model.roomId)
                .put("asset_id", asset.getString("id")).put("fields", JSONObject().put("caption", "Earlier family caption")))
            model.command(JSONObject().put("action", "patch_asset").put("room_id", model.roomId)
                .put("asset_id", asset.getString("id")).put("fields", JSONObject().put("caption", "Current family caption")))
            model.reload()
            asset.getString("id")
        }
        source.delete()

        ui.onNodeWithTag("section.Library").performClick()
        scrollRoomTo("memory.card.$assetId")
        ui.onNodeWithTag("memory.caption.history.$assetId").performScrollTo().performClick()
 val restoreTag = "memory.caption.history.use.$assetId.0"
 awaitState { ui.onAllNodesWithTag(restoreTag).fetchSemanticsNodes().isNotEmpty() }
 ui.onNodeWithTag(restoreTag).performScrollTo().assertIsEnabled().performClick()
        awaitState {
            model.room?.rows("assets")?.firstOrNull { it.optString("id") == assetId }
                ?.optString("caption") == "Earlier family caption"
        }
        val revisions = runBlocking {
            model.command(JSONObject().put("action", "asset_history")
                .put("room_id", model.roomId).put("asset_id", assetId)).optJSONArray("history")
        }
        assertEquals(3, revisions?.length())
    }

    @Test fun editsReopenPrivatelyAndSurviveActivityRecreation() {
        awaitState { model.ready }
        val source = File(ui.activity.cacheDir, "studio-test.png")
        Bitmap.createBitmap(160, 90, Bitmap.Config.ARGB_8888).let { bitmap ->
            bitmap.eraseColor(0xffa25b33.toInt())
            source.outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
            bitmap.recycle()
        }
        val asset = runBlocking {
            withContext(Dispatchers.Main) {
                assertTrue(model.execute(JSONObject().put("action", "create_room").put("name", "Native film QA")))
                val result = model.command(JSONObject().put("action", "import").put("room_id", model.roomId).put("path", source.absolutePath))
                model.reload(); result.getString("id")
            }
        }
        source.delete()
        // Core Room creation bypasses onboarding intentionally; every film action below is native UI.
        ui.onNodeWithTag("section.Theater").performClick()
        ui.onNodeWithTag("theater.createFilm").performScrollTo().performClick()
        awaitState { model.studio.project != null && !model.studio.preparing }
        val id = model.studio.project!!.id
        scrollTo("film.add.$asset")
        ui.onNodeWithTag("film.add.$asset").performClick()
        awaitState { model.studio.project?.clips?.size == 1 && !model.studio.preparing }
        assertEquals(3.0, model.studio.project!!.duration, 0.000001)
        scrollTo("film.split")
        capture("film-inspector-android")
        ui.onNodeWithTag("film.split").performClick()
        awaitState { model.studio.project?.clips?.size == 2 && !model.studio.preparing }
        assertEquals(listOf(0.0, 1.5), model.studio.project!!.clips.map { it.start })
        assertEquals(listOf(0.0, 1.5), model.studio.project!!.clips.map { it.trimIn })
        ui.onNodeWithTag("film.undo").assertIsDisplayed().assertIsEnabled()
        ui.onNodeWithTag("film.undo").performClick()
        ui.waitForIdle()
        assertEquals("Undo history: undo=${model.studio.undoCount}, redo=${model.studio.redoCount}, preparing=${model.studio.preparing}, role=${model.studio.room?.optString("role")}, error=${model.studio.error}", 1, model.studio.project?.clips?.size)
        ui.onNodeWithTag("film.redo").performClick()
        awaitState { model.studio.project?.clips?.size == 2 }
        scrollTo("film.title")
        ui.onNodeWithTag("film.title").performTextReplacement("Private native film")
        ui.onNodeWithTag("film.save").performClick()
        awaitState { !model.studio.dirty && !model.studio.saving }
        val saved = model.room!!.rows("stories").last { it.getString("id") == id }
        assertTrue(saved.getBoolean("private_draft"))
        assertTrue(saved.isNull("published_asset"))
        assertEquals(1, saved.getInt("format_version"))
        ui.runOnIdle {
            ui.activity.getSystemService(InputMethodManager::class.java).hideSoftInputFromWindow(ui.activity.window.decorView.windowToken, 0)
        }
        ui.waitForIdle()
        scrollTo("film.preview")
        capture("film-studio-android")
        ui.activityRule.scenario.recreate()
        awaitState { ui.onAllNodesWithTag("film.title").fetchSemanticsNodes().isNotEmpty() }
        assertEquals("Private native film", model.studio.project!!.title)
        ui.onNodeWithTag("film.close").performClick()
        awaitState { model.studio.project == null }
        ui.onNodeWithTag("theater.open.$id").performScrollTo().performClick()
        awaitState { model.studio.project?.id == id }
        assertEquals(2, model.studio.project!!.clips.size)
        assertEquals(3.0, model.studio.project!!.duration, 0.000001)
        scrollTo("film.size")
        ui.onNodeWithTag("film.size").performClick()
        ui.onNodeWithTag("film.size.854.480").performClick()
        scrollTo("film.render")
        ui.onNodeWithTag("film.render").assertIsEnabled().performClick()
        ui.waitUntil(90_000) { !model.studio.rendering && (model.studio.renderedFile != null || model.studio.error != null) }
        assertNull(model.studio.error)
        assertTrue(model.studio.renderedFile!!.length() > 1000)
        assertEquals("Rendering a private preview must not contribute an original.", 1, model.room!!.rows("assets").size)
        ui.onNodeWithTag("film.export").assertIsEnabled()
        scrollTo("film.publish")
        ui.onNodeWithTag("film.publish").performClick()
        awaitState { !model.studio.preparing && model.studio.project?.privateDraft == false }
        assertNull(model.studio.error)
        val published = model.room!!.rows("stories").last { it.getString("id") == id && !it.optBoolean("private_draft") }
        val movie = model.room!!.rows("assets").first { it.getString("id") == published.getString("published_asset") }
        assertEquals("video", movie.getString("kind"))
        assertTrue(movie.getBoolean("local"))
        val indexed = runBlocking {
            model.command(
                JSONObject()
                    .put("action", "index_video")
                    .put("room_id", movie.getString("room_id"))
                    .put("asset_id", movie.getString("id"))
                    .put("audio_indexed", true)
                    .put("frames_indexed", false)
                    .put(
                        "observations",
                        JSONArray().put(
                            JSONObject()
                                .put("modality", "speech")
                                .put("start_ms", 0)
                                .put("end_ms", 120_000)
                                .put("text", "A caption from the shared local index")
                                .put("confidence_percent", 99),
                        ),
                    ),
            )
        }
        assertTrue(indexed.optBoolean("indexed"))
        ui.onNodeWithTag("film.close").performClick()
        awaitState { model.studio.project == null }
        ui.onNodeWithTag("theater.watch.$id").performScrollTo().performClick()
        ui.onNodeWithTag("theater.disc").assertExists()
        ui.waitUntil(15_000) { ui.onAllNodesWithTag("theater.artwork").fetchSemanticsNodes().isNotEmpty() }
        capture("theater-disc-android", dialog = true)
        ui.waitUntil(30_000) { ui.onAllNodesWithTag("theater.movie").fetchSemanticsNodes().isNotEmpty() }
        ui.onNodeWithTag("theater.movie").assertIsDisplayed()
        ui.waitUntil(5_000) { ui.onAllNodesWithTag("theater.caption.active").fetchSemanticsNodes().isNotEmpty() }
        ui.onNodeWithText("A caption from the shared local index").assertExists()
        ui.onNodeWithTag("theater.captions.toggle").performClick()
        ui.onNodeWithTag("theater.caption.active").assertDoesNotExist()
        ui.onNodeWithTag("theater.captions.toggle").performClick()
        ui.onNodeWithTag("theater.caption.active").assertExists()
        ui.onNodeWithTag("theater.close").performClick()

        ui.onNodeWithTag("playlist.title").performScrollTo().performTextInput("Friday films")
        ui.onNodeWithTag("playlist.create").performClick()
        awaitState { model.room!!.rows("playlists").any { it.optString("title") == "Friday films" } }
        val playlist = model.room!!.rows("playlists").first { it.optString("title") == "Friday films" }
 ui.onNodeWithTag("playlist.edit.${playlist.optString("id")}").performScrollTo().performClick()
        awaitState { ui.onAllNodesWithTag("collection.name").fetchSemanticsNodes().isNotEmpty() }
        ui.onNodeWithTag("playlist.add.$id").performScrollTo().performClick()
        ui.onNodeWithTag("collection.save").performScrollTo().performClick()
        awaitState {
            playlistFilms(model.room!!, model.room!!.rows("playlists").first { it.optString("id") == playlist.optString("id") })
                .any { it.first.optString("id") == id }
        }
        assertEquals(listOf(id), playlistFilms(model.room!!, model.room!!.rows("playlists").first { it.optString("id") == playlist.optString("id") }).map { it.first.optString("id") })
        ui.onNodeWithTag("playlist.play.${playlist.optString("id")}").performScrollTo().performClick()
        ui.waitUntil(30_000) { ui.onAllNodesWithTag("theater.movie").fetchSemanticsNodes().isNotEmpty() }
        ui.onNodeWithTag("theater.close").performClick()

        ui.onNodeWithTag("section.Albums").performClick()
        scrollRoomTo("collection.panel.album")
        ui.onNodeWithTag("album.title").performScrollTo().performTextInput("Autumn afternoon")
        ui.onNodeWithTag("album.create").performClick()
        awaitState { model.room!!.rows("albums").any { it.optString("title") == "Autumn afternoon" } }
        val album = model.room!!.rows("albums").first { it.optString("title") == "Autumn afternoon" }
        runBlocking { withContext(Dispatchers.Main) {
            assertTrue(model.execute(JSONObject().put("action", "save_album")
                .put("room_id", model.roomId).put("id", album.getString("id"))
                .put("title", "Autumn afternoon").put("asset_ids", org.json.JSONArray().put(asset))))
        } }
        assertEquals(asset, model.room!!.rows("albums").first { it.optString("id") == album.optString("id") }
            .getJSONArray("asset_ids").getString(0))
        val offlineTag = "album.offline.${album.optString("id")}"
        ui.onNodeWithTag("collection.panel.album").performScrollToNode(hasTestTag(offlineTag))
        ui.onNodeWithTag(offlineTag).assertIsDisplayed().performClick()
        awaitState { model.room!!.rows("assets").first { it.optString("id") == asset }.optBoolean("pinned_offline") }
        ui.onNodeWithText("Unpin originals").assertExists()
        val arrangeTag = "album.edit.${album.optString("id")}"
        ui.onNodeWithTag("collection.panel.album").performScrollToNode(hasTestTag(arrangeTag))
        ui.onNodeWithTag(arrangeTag).assertIsDisplayed().performClick()
        awaitState { ui.onAllNodesWithTag("album.caption").fetchSemanticsNodes().isNotEmpty() }
        ui.onNodeWithTag("album.caption").performTextInput("A day worth keeping")
        ui.onNodeWithTag("album.cover.$asset").performScrollTo().performClick()
        ui.onNodeWithTag("collection.save").performScrollTo().performClick()
        awaitState {
            model.room!!.rows("albums").first { it.optString("id") == album.optString("id") }
                .optString("caption") == "A day worth keeping"
        }
        val updatedAlbum = model.room!!.rows("albums").first { it.optString("id") == album.optString("id") }
        assertEquals(asset, updatedAlbum.optString("cover_asset_id"))
        assertTrue(model.room!!.rows("assets").any { it.optString("id") == asset })

        ui.onNodeWithTag("album.share.${album.optString("id")}").performScrollTo().performClick()
        ui.onNodeWithText("Share only “Autumn afternoon”").assertExists()
        ui.onNodeWithTag("album.share.publish").assertIsEnabled()
        ui.onNodeWithTag("album.share.invite").assertIsNotEnabled()
    }

    @Test fun codecAndTrimKeepAutomationAndRefuseFutureFormats() {
        val clip = FilmClip(assetId = "test", duration = 8.0,
            opacityKeyframes = listOf(FilmKeyframe(0.0, 0.0), FilmKeyframe(8.0, 1.0)),
            volumeKeyframes = listOf(FilmKeyframe(2.0, 0.2), FilmKeyframe(6.0, 0.8)))
        val trimmed = clip.resized(4.0)
        assertEquals(0.5, trimmed.opacityKeyframes.last().value, 0.000001)
        assertEquals(0.5, trimmed.volumeKeyframes.last().value, 0.000001)
        assertTrue((trimmed.opacityKeyframes + trimmed.volumeKeyframes).all { it.time <= 4 })
        val project = FilmProject(id = "project", roomId = "room", title = "Codec", clips = listOf(trimmed), chapters = listOf(FilmChapter("Beginning", 0.0)))
        assertEquals(project, FilmProject.from(project.json()))
        val legacy = project.json().apply { remove("format_version") }
        assertTrue(FilmProject.from(legacy).supportsEditing)
        val future = project.json().put("format_version", 2)
        assertThrows(IllegalArgumentException::class.java) { FilmProject.from(future) }
        assertEquals(2, future.getInt("format_version"))
    }
}
