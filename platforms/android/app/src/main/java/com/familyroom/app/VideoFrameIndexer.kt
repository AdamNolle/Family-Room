package com.familyroom.app

import android.graphics.Bitmap
import android.media.MediaMetadataRetriever
import com.google.android.gms.tasks.Task
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.label.ImageLabeling
import com.google.mlkit.vision.label.defaults.ImageLabelerOptions
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlin.math.ceil
import kotlin.math.max
import kotlin.math.min

internal data class VideoFrameAnalysis(
    val observations: JSONArray,
    val intervalMs: Long,
    val sampledFrames: Int,
)

/** Temporary decoded frames never leave this device. The encrypted core stores only observations. */
internal object VideoFrameIndexer {
    suspend fun analyze(file: File, onFrame: suspend (Int, Int) -> Unit): VideoFrameAnalysis = withContext(Dispatchers.IO) {
        val retriever = MediaMetadataRetriever()
        val recognizer = TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)
        val labeler = ImageLabeling.getClient(ImageLabelerOptions.DEFAULT_OPTIONS)
        try {
            retriever.setDataSource(file.absolutePath)
            val durationMs = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)
                ?.toLongOrNull()?.takeIf { it > 0 } ?: error("This video has no readable duration.")
            val intervalMs = max(2_000L, ceil(durationMs / 1_500.0).toLong())
            val frameCount = min(1_500, ceil(durationMs / intervalMs.toDouble()).toInt())
            val observations = JSONArray()
            var sampled = 0
            for (frameNumber in 0 until frameCount) {
                currentCoroutineContext().ensureActive()
                val requestedMs = frameNumber * intervalMs
                val frame = retriever.getFrameAtTime(requestedMs * 1_000L, MediaMetadataRetriever.OPTION_CLOSEST)
                    ?: continue
                val scaled = scaleFrame(frame)
                try {
                    val image = InputImage.fromBitmap(scaled, 0)
                    val text = recognizer.process(image).awaitResult()
                    val labels = labeler.process(image).awaitResult()
                    val endMs = min(durationMs, requestedMs + intervalMs)
                    text.textBlocks.asSequence().map { it.text.trim() }
                        .filter { it.isNotBlank() }.take(3).forEach { words ->
                            observations.put(observation("screen_text", requestedMs, endMs, words, 90))
                        }
                    labels.asSequence().filter { it.confidence >= 0.75f }.take(2).forEach { label ->
                        observations.put(observation("visual_label", requestedMs, endMs,
                            label.text, (label.confidence * 100).toInt().coerceIn(0, 100)))
                    }
                    sampled++
                    onFrame(frameNumber + 1, frameCount)
                } finally {
                    if (scaled !== frame) scaled.recycle()
                    frame.recycle()
                }
            }
            check(sampled > 0) { "No video frames could be decoded." }
            VideoFrameAnalysis(observations, intervalMs, sampled)
        } finally {
            labeler.close()
            recognizer.close()
            retriever.release()
        }
    }

    private fun scaleFrame(frame: Bitmap): Bitmap {
        val longEdge = max(frame.width, frame.height)
        if (longEdge <= 960) return frame
        val factor = 960.0 / longEdge
        return Bitmap.createScaledBitmap(frame,
            max(1, (frame.width * factor).toInt()), max(1, (frame.height * factor).toInt()), true)
    }

    private fun observation(kind: String, startMs: Long, endMs: Long, words: String, confidence: Int) =
        JSONObject().put("modality", kind).put("start_ms", startMs).put("end_ms", endMs)
            .put("text", boundedEvidenceText(words)).put("confidence_percent", confidence)

    // 1,500 sampled frames can yield 7,500 observations. Keep even dense OCR
    // within the encrypted catalog's per-observation and total-text limits.
    internal fun boundedEvidenceText(words: String): String {
        val text = words.trim()
        var end = 0
        var bytes = 0
        while (end < text.length) {
            val codePoint = Character.codePointAt(text, end)
            val step = Character.charCount(codePoint)
            val size = String(Character.toChars(codePoint)).toByteArray(Charsets.UTF_8).size
            if (bytes + size > 400) break
            bytes += size
            end += step
        }
        return text.substring(0, end).trimEnd()
    }

    private suspend fun <T> Task<T>.awaitResult(): T = suspendCancellableCoroutine { continuation ->
        addOnSuccessListener { value -> if (continuation.isActive) continuation.resume(value) }
        addOnFailureListener { error -> if (continuation.isActive) continuation.resumeWithException(error) }
        addOnCanceledListener { if (continuation.isActive) continuation.cancel() }
    }
}
