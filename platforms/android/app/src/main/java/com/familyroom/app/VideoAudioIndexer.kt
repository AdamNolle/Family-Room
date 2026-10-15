package com.familyroom.app

import android.media.AudioFormat
import android.media.MediaCodec
import android.media.MediaExtractor
import android.media.MediaFormat
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import org.vosk.Model
import org.vosk.Recognizer
import java.io.Closeable
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.math.max

internal data class VideoAudioAnalysis(
    val observations: JSONArray,
    val audioIndexed: Boolean,
    val message: String,
)

/** Decodes a video's audio stream into local PCM and sends it only to Vosk. */
internal class VideoAudioIndexer(modelDirectory: File) : Closeable {
    private val model = Model(modelDirectory.absolutePath)
    private class SpeechBudget(var bytes: Int = 0)

    override fun close() = model.close()

    suspend fun analyze(file: File, onProgress: suspend (Int) -> Unit): VideoAudioAnalysis =
        withContext(Dispatchers.IO) {
            val extractor = MediaExtractor()
            var decoder: MediaCodec? = null
            var recognizer: Recognizer? = null
            val observations = JSONArray()
            val budget = SpeechBudget()
            try {
                extractor.setDataSource(file.absolutePath)
                val track = (0 until extractor.trackCount).firstOrNull { trackIndex ->
                    extractor.getTrackFormat(trackIndex).getString(MediaFormat.KEY_MIME)
                        ?.startsWith("audio/") == true
                } ?: return@withContext VideoAudioAnalysis(observations, false, "No audio track")
                extractor.selectTrack(track)
                val inputFormat = extractor.getTrackFormat(track)
                val mime = checkNotNull(inputFormat.getString(MediaFormat.KEY_MIME))
                val durationUs = if (inputFormat.containsKey(MediaFormat.KEY_DURATION))
                    inputFormat.getLong(MediaFormat.KEY_DURATION) else 0L
                inputFormat.setInteger(MediaFormat.KEY_PCM_ENCODING, AudioFormat.ENCODING_PCM_16BIT)
                decoder = MediaCodec.createDecoderByType(mime)
                decoder.configure(inputFormat, null, null, 0)
                decoder.start()
                val info = MediaCodec.BufferInfo()
                var inputEnded = false
                var outputEnded = false
                var sampleRate = 0
                var channels = 0
                var baseUs = 0L
                var processedFrames = 0L
                var lastProgress = -1
                while (!outputEnded) {
                    currentCoroutineContext().ensureActive()
                    if (!inputEnded) {
                        val inputIndex = decoder.dequeueInputBuffer(10_000)
                        if (inputIndex >= 0) {
                            val input = checkNotNull(decoder.getInputBuffer(inputIndex))
                            val size = extractor.readSampleData(input, 0)
                            if (size < 0) {
                                decoder.queueInputBuffer(inputIndex, 0, 0, 0L,
                                    MediaCodec.BUFFER_FLAG_END_OF_STREAM)
                                inputEnded = true
                            } else {
                                decoder.queueInputBuffer(inputIndex, 0, size,
                                    extractor.sampleTime.coerceAtLeast(0L), 0)
                                extractor.advance()
                            }
                        }
                    }
                    val outputIndex = decoder.dequeueOutputBuffer(info, 10_000)
                    if (outputIndex >= 0) {
                        try {
                            if (info.size > 0) {
                                val format = decoder.outputFormat
                                val encoding = if (format.containsKey(MediaFormat.KEY_PCM_ENCODING))
                                    format.getInteger(MediaFormat.KEY_PCM_ENCODING)
                                else AudioFormat.ENCODING_PCM_16BIT
                                check(encoding == AudioFormat.ENCODING_PCM_16BIT) {
                                    "This video's audio decoder did not produce 16-bit PCM."
                                }
                                val rate = format.getInteger(MediaFormat.KEY_SAMPLE_RATE)
                                val channelCount = format.getInteger(MediaFormat.KEY_CHANNEL_COUNT)
                                check(rate in 8_000..192_000 && channelCount in 1..8) {
                                    "This video's audio format is not supported for local transcription."
                                }
                                if (recognizer == null) {
                                    sampleRate = rate
                                    channels = channelCount
                                    baseUs = info.presentationTimeUs.coerceAtLeast(0L)
                                    recognizer = Recognizer(model, sampleRate.toFloat()).apply { setWords(true) }
                                }
                                check(sampleRate == rate && channels == channelCount) {
                                    "This video changes audio format midstream."
                                }
                                val output = checkNotNull(decoder.getOutputBuffer(outputIndex))
                                output.position(info.offset)
                                output.limit(info.offset + info.size)
                                val pcm = monoPcm(output.slice(), channels)
                                if (pcm.isNotEmpty()) {
                                    val expectedUs = baseUs + processedFrames * 1_000_000L / sampleRate
                                    if (info.presentationTimeUs > expectedUs + 1_000_000L) {
                                        collectWords(checkNotNull(recognizer).finalResult, baseUs, observations, budget)
                                        recognizer?.close()
                                        recognizer = Recognizer(model, sampleRate.toFloat()).apply { setWords(true) }
                                        baseUs = info.presentationTimeUs
                                        processedFrames = 0
                                    } else if (info.presentationTimeUs > expectedUs + 100_000L) {
                                        val gapFrames = ((info.presentationTimeUs - expectedUs) * sampleRate /
                                            1_000_000L).toInt()
                                        val silence = ByteArray(4_096)
                                        var remaining = gapFrames * 2
                                        while (remaining > 0) {
                                            val count = minOf(remaining, silence.size)
                                            accept(checkNotNull(recognizer), silence, count, baseUs, observations, budget)
                                            remaining -= count
                                        }
                                        processedFrames += gapFrames
                                    }
                                    accept(checkNotNull(recognizer), pcm, pcm.size, baseUs, observations, budget)
                                    processedFrames += pcm.size / 2
                                }
                                if (durationUs > 0) {
                                    val percent = (info.presentationTimeUs * 100 / durationUs)
                                        .toInt().coerceIn(0, 100)
                                    if (percent >= lastProgress + 10) {
                                        lastProgress = percent
                                        withContext(Dispatchers.Main) { onProgress(percent) }
                                    }
                                }
                            }
                        } finally {
                            decoder.releaseOutputBuffer(outputIndex, false)
                        }
                        outputEnded = info.flags and MediaCodec.BUFFER_FLAG_END_OF_STREAM != 0
                    }
                }
                recognizer?.let { collectWords(it.finalResult, baseUs, observations, budget) }
                VideoAudioAnalysis(observations, true, "On-device speech indexed")
            } finally {
                recognizer?.close()
                decoder?.runCatching { stop() }
                decoder?.release()
                extractor.release()
            }
        }

    private fun accept(recognizer: Recognizer, pcm: ByteArray, length: Int,
                       baseUs: Long, observations: JSONArray, budget: SpeechBudget) {
        if (recognizer.acceptWaveForm(pcm, length)) {
            collectWords(recognizer.result, baseUs, observations, budget)
        }
    }

    private fun monoPcm(source: ByteBuffer, channels: Int): ByteArray {
        val frameBytes = channels * 2
        val frames = source.remaining() / frameBytes
        if (frames == 0) return byteArrayOf()
        source.order(ByteOrder.LITTLE_ENDIAN)
        if (channels == 1) return ByteArray(frames * 2).also(source::get)
        val result = ByteBuffer.allocate(frames * 2).order(ByteOrder.LITTLE_ENDIAN)
        repeat(frames) {
            var sum = 0
            repeat(channels) { sum += source.short.toInt() }
            result.putShort((sum / channels).toShort())
        }
        return result.array()
    }

    private fun collectWords(result: String, baseUs: Long, observations: JSONArray, budget: SpeechBudget) {
        val words = JSONObject(result).optJSONArray("result") ?: return
        var first = -1L
        var last = 0L
        var confidence = 0.0
        val phrase = mutableListOf<String>()
        fun flush() {
            if (phrase.isEmpty()) return
            val text = phrase.joinToString(" ").take(500)
            check(observations.length() < 7_000 && budget.bytes + text.toByteArray(Charsets.UTF_8).size <= 250_000) {
                "This film exceeds the local speech-index limit; frame search is still available."
            }
            observations.put(JSONObject().put("modality", "speech")
                .put("start_ms", first).put("end_ms", max(first, last))
                .put("text", text)
                .put("confidence_percent", (confidence * 100 / phrase.size).toInt().coerceIn(0, 100)))
            budget.bytes += text.toByteArray(Charsets.UTF_8).size
            phrase.clear()
            confidence = 0.0
        }
        for (position in 0 until words.length()) {
            val word = words.optJSONObject(position) ?: continue
            val text = word.optString("word").trim()
            if (text.isEmpty()) continue
            val start = baseUs / 1_000L + (word.optDouble("start") * 1_000).toLong()
            val end = baseUs / 1_000L + (word.optDouble("end") * 1_000).toLong()
            if (phrase.isNotEmpty() && (start > last + 1_000 || end > first + 4_000 || phrase.size >= 12)) flush()
            if (phrase.isEmpty()) first = start
            phrase.add(text)
            last = end
            confidence += word.optDouble("conf", 0.75)
        }
        flush()
    }
}
