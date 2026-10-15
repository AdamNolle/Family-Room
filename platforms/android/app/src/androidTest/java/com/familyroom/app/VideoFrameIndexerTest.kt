package com.familyroom.app

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Typeface
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.family_core.FamilyCore
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class VideoFrameIndexerTest {
    @Test fun evidenceTextBoundIsUtf8SafeForDenseOcr() {
        val source = "🎞️".repeat(300) + "birthday"
        val bounded = VideoFrameIndexer.boundedEvidenceText(source)
        assertTrue(bounded.toByteArray(Charsets.UTF_8).size <= 400)
        assertTrue(source.startsWith(bounded))
        assertTrue(bounded.isNotEmpty())
    }

    @Test fun renderedMovieYieldsBoundedLocalFrameSample() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val root = File(context.cacheDir, "video-finder-test-${UUID.randomUUID()}").apply { mkdirs() }
        try {
            val photo = File(root, "frame.png")
            val bitmap = Bitmap.createBitmap(640, 360, Bitmap.Config.ARGB_8888)
            try {
                bitmap.eraseColor(Color.WHITE)
                Canvas(bitmap).drawText("BIRTHDAY", 48f, 208f, Paint(Paint.ANTI_ALIAS_FLAG).apply {
                    color = Color.BLACK
                    textSize = 88f
                    typeface = Typeface.DEFAULT_BOLD
                })
                photo.outputStream().use { assertTrue(bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)) }
            } finally { bitmap.recycle() }
            val plain = File(root, "plain.png")
            val plainBitmap = Bitmap.createBitmap(640, 360, Bitmap.Config.ARGB_8888)
            try {
                plainBitmap.eraseColor(Color.WHITE)
                plain.outputStream().use { assertTrue(plainBitmap.compress(Bitmap.CompressFormat.PNG, 100, it)) }
            } finally { plainBitmap.recycle() }
            val movie = File(root, "frame.mp4")
            val project = FilmProject(id = "finder-test", roomId = "test", title = "Frame sample",
                width = 640, height = 360, clips = listOf(
                    FilmClip(assetId = "plain", duration = 2.0),
                    FilmClip(assetId = "photo", start = 2.0, duration = 3.0)))
            withTimeout(90_000) {
                FilmRenderer.render(context, project,
                    listOf(JSONObject().put("id", "plain").put("filename", plain.name).put("kind", "photo"),
                        JSONObject().put("id", "photo").put("filename", photo.name).put("kind", "photo")),
                    mapOf("plain" to plain, "photo" to photo), movie)
                val analysis = VideoFrameIndexer.analyze(movie) { _, _ -> }
                assertEquals(2_000L, analysis.intervalMs)
                assertTrue("Decoded ${analysis.sampledFrames} frames", analysis.sampledFrames >= 2)
                assertTrue(analysis.observations.length() <= 15)
                assertTrue((0 until analysis.observations.length()).any { index ->
                    val evidence = analysis.observations.getJSONObject(index)
                    evidence.optString("modality") == "screen_text" &&
                        evidence.optString("text").contains("BIRTHDAY", ignoreCase = true)
                })
                val core = FamilyCore.open(File(root, "vault").absolutePath, "07".repeat(32))
                try {
                    fun command(request: JSONObject) = JSONObject(core.command(request.toString()))
                    val roomId = command(JSONObject().put("action", "create_room")
                        .put("name", "Finder contract")).getString("id")
                    val assetId = command(JSONObject().put("action", "import")
                        .put("room_id", roomId).put("path", movie.absolutePath)).getString("id")
                    command(JSONObject().put("action", "index_video")
                        .put("room_id", roomId).put("asset_id", assetId)
                        .put("frames_indexed", true).put("frame_interval_ms", analysis.intervalMs)
                        .put("observations", analysis.observations))
                    val found = command(JSONObject().put("action", "search_video")
                        .put("room_id", roomId).put("query", "find birthday on screen")
                        .put("modality", "frame"))
                    assertEquals(assetId, found.getJSONArray("matches").getJSONObject(0).getString("asset_id"))
                    assertTrue("Match ${found.getJSONArray("matches").getJSONObject(0)}; evidence ${analysis.observations}",
                        found.getJSONArray("matches").getJSONObject(0).getLong("start_ms") >= 2_000L)
                    val broad = command(JSONObject().put("action", "search_video")
                        .put("room_id", roomId).put("query", "What does the sign say?")
                        .put("modality", "all"))
                    assertEquals(assetId, broad.getJSONArray("matches").getJSONObject(0).getString("asset_id"))
                    assertTrue(broad.getString("answer").contains("BIRTHDAY", ignoreCase = true))
                } finally { core.close() }
            }
        } finally {
            root.deleteRecursively()
        }
    }
}
