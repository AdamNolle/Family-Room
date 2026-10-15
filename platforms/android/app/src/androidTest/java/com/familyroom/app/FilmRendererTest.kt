package com.familyroom.app

import android.graphics.Bitmap
import android.graphics.Color
import android.media.MediaCodec
import android.media.MediaExtractor
import android.media.MediaFormat
import android.media.MediaMetadataRetriever
import android.media.MediaMuxer
import android.os.SystemClock
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.UUID
import kotlin.math.PI
import kotlin.math.sin
import kotlin.math.sqrt

@RunWith(AndroidJUnit4::class)
class FilmRendererTest {
    private val context get() = InstrumentationRegistry.getInstrumentation().targetContext

    @Test fun layeredMovieContainsTimedPicturesAndAudibleSpeedAdjustedGain() = runBlocking {
        val root = File(context.cacheDir, "movie-test-${UUID.randomUUID()}").apply { mkdirs() }
        try {
            fun photo(name: String, color: Int): File = File(root, name).also { file ->
                val bitmap = Bitmap.createBitmap(128, 128, Bitmap.Config.ARGB_8888)
                try { bitmap.eraseColor(color); file.outputStream().use { assertTrue(bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)) } }
                finally { bitmap.recycle() }
            }
            val red = photo("red.png", Color.RED)
            val blue = photo("blue.png", Color.BLUE)
            val audio = File(root, "tone.wav")
            val samples = 48_000
            val wave = ByteBuffer.allocate(44 + samples * 2).order(ByteOrder.LITTLE_ENDIAN)
            wave.put("RIFF".toByteArray()).putInt(36 + samples * 2).put("WAVEfmt ".toByteArray())
                .putInt(16).putShort(1).putShort(1).putInt(48_000).putInt(96_000).putShort(2).putShort(16)
                .put("data".toByteArray()).putInt(samples * 2)
            for (i in 0 until samples) wave.putShort((sin(2 * PI * 440 * i / 48_000) * 0.2 * Short.MAX_VALUE).toInt().toShort())
            audio.writeBytes(wave.array())
            fun asset(id: String, file: File, kind: String) = JSONObject().put("id", id).put("filename", file.name).put("kind", kind)
            val assets = listOf(asset("red", red, "photo"), asset("blue", blue, "photo"), asset("audio", audio, "audio"))
            val project = FilmProject(id = "native-render", roomId = "test", title = "Layer test", width = 128, height = 128, clips = listOf(
                FilmClip(assetId = "red", duration = 1.2),
                FilmClip(assetId = "blue", track = 1, start = 0.4, duration = 0.4, opacity = 0.5),
                FilmClip(assetId = "audio", track = 2, start = 0.2, trimIn = 0.1, duration = 0.6, speed = 2.0,
                    fadeIn = 0.04, fadeOut = 0.04, volumeKeyframes = listOf(FilmKeyframe(0.0, 0.2), FilmKeyframe(0.6, 0.8)))
            ))
            val output = File(root, "movie.mp4")
            withTimeout(90_000) { FilmRenderer.render(context, project, assets, mapOf("red" to red, "blue" to blue, "audio" to audio), output) }
            assertTrue(output.length() > 1000)
            val retriever = MediaMetadataRetriever()
            try {
                retriever.setDataSource(output.absolutePath)
                assertEquals(1200.0, retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)!!.toDouble(), 100.0)
                fun pixel(time: Long): Int {
                    val frame = retriever.getFrameAtTime(time, MediaMetadataRetriever.OPTION_CLOSEST) ?: error("The exported movie has no decoded frame.")
                    try { assertEquals(128, frame.width); assertEquals(128, frame.height); return frame.getPixel(64, 64) }
                    finally { frame.recycle() }
                }
                val before = pixel(100_000); val during = pixel(600_000); val after = pixel(1_000_000)
                assertTrue(Color.red(before) > 200 && Color.blue(before) < 30)
                assertTrue("The overlapping layer must retain both colors.", Color.red(during) > 70 && Color.blue(during) > 70 && Color.green(during) < 30)
                assertTrue("The finished upper layer must disappear.", Color.red(after) > 200 && Color.blue(after) < 30)
            } finally { retriever.release() }
            val decoded = decodeAudio(output)
            fun rms(from: Double, to: Double): Double {
                val samplesInWindow = decoded.filter { it.first >= from && it.first < to }.map { it.second }
                assertTrue("Decoded audio must cover $from–$to.", samplesInWindow.isNotEmpty())
                return sqrt(samplesInWindow.sumOf { it * it } / samplesInWindow.size)
            }
            assertTrue("No audio should start before the clip.", rms(0.02, 0.14) < 0.01)
            val early = rms(0.28, 0.32); val late = rms(0.37, 0.41)
            assertTrue("The source must be audible with applied gain: $early.", early in 0.02..0.09)
            assertTrue("Source-time gain must advance at twice speed: early=$early, late=$late.", late > early * 1.15 && late < 0.13)
            assertTrue("The speed-adjusted audio must end at 0.5 seconds.", rms(0.6, 0.8) < 0.01)
        } finally { root.deleteRecursively() }
    }

    @Test fun trimmedRotatedVideoKeepsTimingOrientationAndTitle() = runBlocking {
        val root = File(context.cacheDir, "video-test-${UUID.randomUUID()}").apply { mkdirs() }
        try {
            fun photo(name: String, pixels: (Int, Int) -> Int): File = File(root, name).also { file ->
                val bitmap = Bitmap.createBitmap(256, 128, Bitmap.Config.ARGB_8888)
                try {
                    for (y in 0 until bitmap.height) for (x in 0 until bitmap.width) bitmap.setPixel(x, y, pixels(x, y))
                    file.outputStream().use { assertTrue(bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)) }
                } finally { bitmap.recycle() }
            }
            val green = photo("green.png") { _, _ -> Color.GREEN }
            val pattern = photo("pattern.png") { x, _ -> if (x < 128) Color.RED else Color.BLUE }
            val black = photo("black.png") { _, _ -> Color.BLACK }
            fun asset(id: String, file: File, kind: String) = JSONObject().put("id", id).put("filename", file.name).put("kind", kind)
            val source = File(root, "source.mp4")
            withTimeout(180_000) {
                FilmRenderer.render(context, FilmProject(id = "video-source", roomId = "test", title = "Video source", width = 256, height = 128,
                    clips = listOf(FilmClip(assetId = "green", duration = 0.3), FilmClip(assetId = "pattern", start = 0.3, duration = 0.7))),
                    listOf(asset("green", green, "photo"), asset("pattern", pattern, "photo")), mapOf("green" to green, "pattern" to pattern), source)
            }
            val rotated = File(root, "rotated.mp4")
            val extractor = MediaExtractor()
            val muxer = MediaMuxer(rotated.absolutePath, MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4)
            try {
                extractor.setDataSource(source.absolutePath)
                val tracks = (0 until extractor.trackCount).associateWith { track ->
                    extractor.selectTrack(track); muxer.addTrack(extractor.getTrackFormat(track))
                }
                muxer.setOrientationHint(90); muxer.start()
                val buffer = ByteBuffer.allocate(1_048_576)
                val info = MediaCodec.BufferInfo()
                while (extractor.sampleTrackIndex >= 0) {
                    buffer.clear()
                    val size = extractor.readSampleData(buffer, 0)
                    if (size < 0) break
                    info.set(0, size, extractor.sampleTime, extractor.sampleFlags)
                    muxer.writeSampleData(tracks.getValue(extractor.sampleTrackIndex), buffer, info)
                    extractor.advance()
                }
                muxer.stop()
            } finally { muxer.release(); extractor.release() }
            val metadata = MediaMetadataRetriever()
            try {
                metadata.setDataSource(rotated.absolutePath)
                assertEquals("90", metadata.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_ROTATION))
            } finally { metadata.release() }
            val project = FilmProject(id = "rotated-project", roomId = "test", title = "Rotated video", width = 128, height = 256,
                clips = listOf(FilmClip(assetId = "black", duration = 0.5),
                    FilmClip(assetId = "video", track = 1, start = 0.1, trimIn = 0.4, duration = 0.4, speed = 2.0, title = "QA")))
            val output = File(root, "movie.mp4")
            withTimeout(180_000) {
                FilmRenderer.render(context, project, listOf(asset("black", black, "photo"), asset("video", rotated, "video")),
                    mapOf("black" to black, "video" to rotated), output)
            }
            val retriever = MediaMetadataRetriever()
            try {
                retriever.setDataSource(output.absolutePath)
                assertEquals(500.0, retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)!!.toDouble(), 80.0)
                fun frame(time: Long) = retriever.getFrameAtTime(time, MediaMetadataRetriever.OPTION_CLOSEST) ?: error("No decoded video frame.")
                for (time in listOf(33_000L, 400_000L)) {
                    val image = frame(time)
                    try { assertTrue("The video must be absent outside its speed-adjusted interval.", Color.red(image.getPixel(64, 80)) < 30) }
                    finally { image.recycle() }
                }
                val image = frame(180_000L)
                try {
                    assertEquals(128, image.width); assertEquals(256, image.height)
                    val top = image.getPixel(64, 50); val bottom = image.getPixel(64, 200)
                    assertTrue("Clockwise rotation must place red above blue after trimming past green.", Color.red(top) > 180 && Color.blue(top) < 50)
                    assertTrue("Clockwise rotation must retain the blue lower half.", Color.blue(bottom) > 180 && Color.red(bottom) < 50)
                    var whitePixels = 0
                    for (y in 0 until image.height) for (x in 0 until image.width) {
                        val pixel = image.getPixel(x, y)
                        if (Color.red(pixel) > 180 && Color.green(pixel) > 180 && Color.blue(pixel) > 180) whitePixels++
                    }
                    assertTrue("The title must appear in the decoded movie, not only in the editor.", whitePixels > 3)
                } finally { image.recycle() }
            } finally { retriever.release() }
        } finally { root.deleteRecursively() }
    }

    private fun decodeAudio(file: File): List<Pair<Double, Double>> {
        val extractor = MediaExtractor()
        var decoder: MediaCodec? = null
        try {
            extractor.setDataSource(file.absolutePath)
            val track = (0 until extractor.trackCount).firstOrNull { extractor.getTrackFormat(it).getString(MediaFormat.KEY_MIME)?.startsWith("audio/") == true }
                ?: error("The exported movie has no audio track.")
            extractor.selectTrack(track)
            val format = extractor.getTrackFormat(track)
            val codec = MediaCodec.createDecoderByType(format.getString(MediaFormat.KEY_MIME)!!)
            decoder = codec; codec.configure(format, null, null, 0); codec.start()
            var rate = format.getInteger(MediaFormat.KEY_SAMPLE_RATE)
            var channels = format.getInteger(MediaFormat.KEY_CHANNEL_COUNT)
            var floatPcm = false; var sentEnd = false; var ended = false
            val result = mutableListOf<Pair<Double, Double>>()
            val info = MediaCodec.BufferInfo()
            val deadline = SystemClock.elapsedRealtime() + 15_000
            while (!ended && SystemClock.elapsedRealtime() < deadline) {
                if (!sentEnd) {
                    val index = codec.dequeueInputBuffer(10_000)
                    if (index >= 0) {
                        val input = codec.getInputBuffer(index)!!
                        val count = extractor.readSampleData(input, 0)
                        if (count < 0) { codec.queueInputBuffer(index, 0, 0, 0, MediaCodec.BUFFER_FLAG_END_OF_STREAM); sentEnd = true }
                        else { codec.queueInputBuffer(index, 0, count, extractor.sampleTime, 0); extractor.advance() }
                    }
                }
                val index = codec.dequeueOutputBuffer(info, 10_000)
                if (index == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                    val actual = codec.outputFormat; rate = actual.getInteger(MediaFormat.KEY_SAMPLE_RATE); channels = actual.getInteger(MediaFormat.KEY_CHANNEL_COUNT)
                    floatPcm = actual.containsKey(MediaFormat.KEY_PCM_ENCODING) && actual.getInteger(MediaFormat.KEY_PCM_ENCODING) == android.media.AudioFormat.ENCODING_PCM_FLOAT
                } else if (index >= 0) {
                    val output = codec.getOutputBuffer(index)!!.order(ByteOrder.LITTLE_ENDIAN)
                    output.position(info.offset); output.limit(info.offset + info.size)
                    var frame = 0
                    while (output.remaining() >= channels * if (floatPcm) 4 else 2) {
                        var sum = 0.0
                        repeat(channels) { sum += if (floatPcm) output.float.toDouble() else output.short / 32768.0 }
                        result.add((info.presentationTimeUs / 1_000_000.0 + frame.toDouble() / rate) to (sum / channels)); frame++
                    }
                    ended = info.flags and MediaCodec.BUFFER_FLAG_END_OF_STREAM != 0
                    codec.releaseOutputBuffer(index, false)
                }
            }
            assertTrue("Audio decoding must reach end-of-stream.", ended)
            return result
        } finally { try { decoder?.stop() } finally { decoder?.release(); extractor.release() } }
    }
}
