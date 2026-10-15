package com.familyroom.app

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import uniffi.family_core.FamilyCore
import java.io.File
import java.util.UUID

/** Opt-in because it downloads the optional 40 MB model on first execution. */
@RunWith(AndroidJUnit4::class)
class VideoAudioIndexerTest {
    @Test fun spokenLineIsIndexedLocallyWithItsMovieTime() = runBlocking {
        assumeTrue(InstrumentationRegistry.getArguments().getString("familyRoomSpeechModel") == "true")
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext
        val root = File(context.cacheDir, "speech-index-test-${UUID.randomUUID()}").apply { mkdirs() }
        try {
            val movie = File(root, "spoken-birthday.mp4")
            instrumentation.context.assets.open("spoken-birthday.mp4").use { input ->
                movie.outputStream().use(input::copyTo)
            }
            val model = withTimeout(180_000) { OfflineSpeechModel.install(context) { } }
            val analysis = withTimeout(180_000) {
                VideoAudioIndexer(model).use { indexer -> indexer.analyze(movie) { } }
            }
            assertTrue(analysis.audioIndexed)
            assertTrue("Speech observations: ${analysis.observations}",
                (0 until analysis.observations.length()).any { position ->
                    analysis.observations.getJSONObject(position).optString("text")
                        .contains("birthday", ignoreCase = true)
                })
            val core = FamilyCore.open(File(root, "vault").absolutePath, "08".repeat(32))
            try {
                fun command(request: JSONObject) = JSONObject(core.command(request.toString()))
                val roomId = command(JSONObject().put("action", "create_room")
                    .put("name", "Spoken finder QA")).getString("id")
                val assetId = command(JSONObject().put("action", "import")
                    .put("room_id", roomId).put("path", movie.absolutePath)).getString("id")
                command(JSONObject().put("action", "index_video")
                    .put("room_id", roomId).put("asset_id", assetId)
                    .put("audio_indexed", true).put("frames_indexed", false)
                    .put("observations", analysis.observations))
                val found = command(JSONObject().put("action", "search_video")
                    .put("room_id", roomId).put("query", "birthday")
                    .put("modality", "audio"))
                assertEquals(assetId, found.getJSONArray("matches").getJSONObject(0).getString("asset_id"))
                assertTrue(found.getJSONArray("matches").getJSONObject(0).getLong("start_ms") in 0..3_100)
                command(JSONObject().put("action", "index_video")
                    .put("room_id", roomId).put("asset_id", assetId)
                    .put("audio_indexed", false).put("frames_indexed", true)
                    .put("preserve_existing", true).put("frame_interval_ms", 2_000)
                    .put("observations", org.json.JSONArray().put(JSONObject()
                        .put("modality", "screen_text").put("start_ms", 2_000)
                        .put("end_ms", 2_000).put("text", "YELLOW BALLOON")
                        .put("confidence_percent", 95))))
                assertEquals(assetId, command(JSONObject().put("action", "search_video")
                    .put("room_id", roomId).put("query", "birthday")
                    .put("modality", "audio"))
                    .getJSONArray("matches").getJSONObject(0).getString("asset_id"))
                assertEquals(assetId, command(JSONObject().put("action", "search_video")
                    .put("room_id", roomId).put("query", "yellow balloon")
                    .put("modality", "frame"))
                    .getJSONArray("matches").getJSONObject(0).getString("asset_id"))
            } finally { core.close() }
        } finally { root.deleteRecursively() }
    }
}
