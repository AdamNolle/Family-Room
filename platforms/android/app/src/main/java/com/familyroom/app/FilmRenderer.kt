package com.familyroom.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Color
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.text.SpannableString
import android.text.Spanned
import android.text.style.AbsoluteSizeSpan
import android.text.style.ForegroundColorSpan
import androidx.annotation.OptIn
import androidx.media3.common.C
import androidx.media3.common.Effect
import androidx.media3.common.MediaItem
import androidx.media3.common.OverlaySettings
import androidx.media3.common.SpeedParameters
import androidx.media3.common.audio.AudioProcessor
import androidx.media3.common.audio.BaseAudioProcessor
import androidx.media3.common.util.Size
import androidx.media3.common.util.UnstableApi
import androidx.media3.common.VideoCompositorSettings
import androidx.media3.effect.OverlayEffect
import androidx.media3.effect.Presentation
import androidx.media3.effect.RgbMatrix
import androidx.media3.effect.StaticOverlaySettings
import androidx.media3.effect.TextOverlay
import androidx.media3.transformer.Composition
import androidx.media3.transformer.EditedMediaItem
import androidx.media3.transformer.EditedMediaItemSequence
import androidx.media3.transformer.Effects
import androidx.media3.transformer.ExportException
import androidx.media3.transformer.ExportResult
import androidx.media3.transformer.Transformer
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlin.math.ceil
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.roundToLong

/** Native Media3 rendering. Originals are supplied by the encrypted catalog, never modified. */
@OptIn(UnstableApi::class)
object FilmRenderer {
    suspend fun render(context: Context, project: FilmProject, assets: List<JSONObject>, originals: Map<String, File>, output: File) {
        require(project.supportsEditing) { "Update Family Room to render this project format." }
        require(project.clips.isNotEmpty() && project.duration > 0 && project.duration <= 86400) { "Add a clip before rendering." }
        require(project.width in 16..8192 && project.height in 16..8192) { "Choose supported movie dimensions." }
        require(!output.exists()) { "Choose a new movie destination." }
        val known = assets.associateBy { it.getString("id") }
        require(project.clips.all { originals[it.assetId]?.isFile == true && known[it.assetId]?.optString("kind") in listOf("photo", "video", "audio") }) { "Download every original before rendering." }
        val audibleAssets = withContext(Dispatchers.IO) {
            val audible = mutableSetOf<String>()
            for ((assetId, clips) in project.clips.groupBy { it.assetId }) {
                if (known.getValue(assetId).optString("kind") == "photo") continue
                val retriever = MediaMetadataRetriever()
                try {
                    retriever.setDataSource(originals.getValue(assetId).absolutePath)
                    val duration = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)?.toDoubleOrNull()?.div(1000)
                        ?: error("This original’s playable duration cannot be read. Keep it as a downloadable original.")
                    require(clips.all { it.trimIn >= 0 && it.trimIn + it.duration <= duration + 0.001 }) { "A clip extends beyond its original. Adjust its trim or duration before rendering." }
                    val hasAudio = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_HAS_AUDIO) == "yes"
                    require(hasAudio || known.getValue(assetId).optString("kind") != "audio") { "An audio source has no playable audio track." }
                    if (hasAudio) audible.add(assetId)
                } finally { retriever.release() }
            }
            audible
        }
        val background = withContext(Dispatchers.IO) {
            File.createTempFile("film-background-", ".png", context.cacheDir).also { file ->
                val bitmap = Bitmap.createBitmap(16, 16, Bitmap.Config.ARGB_8888)
                try { bitmap.eraseColor(Color.BLACK); file.outputStream().use { check(bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)) } }
                finally { bitmap.recycle() }
            }
        }
        try {
            withContext(Dispatchers.Main.immediate) {
                val ordered = project.clips.withIndex().sortedWith(compareByDescending<IndexedValue<FilmClip>> { it.value.track }.thenByDescending { it.index }).map { it.value }
                val visuals = ordered.filter { known.getValue(it.assetId).optString("kind") != "audio" }
                val sequences = mutableListOf<EditedMediaItemSequence>()
                for (clip in visuals) {
                    val kind = known.getValue(clip.assetId).optString("kind")
                    val videoEffects = mutableListOf<Effect>(Presentation.createForWidthAndHeight(project.width, project.height, Presentation.LAYOUT_SCALE_TO_FIT), grade(clip))
                    if (clip.title.isNotBlank()) {
                        val text = SpannableString(clip.title)
                        text.setSpan(ForegroundColorSpan(Color.WHITE), 0, text.length, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
                        text.setSpan(AbsoluteSizeSpan(maxOf(12, (project.width * 0.025).roundToInt())), 0, text.length, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
                        videoEffects.add(OverlayEffect(listOf(TextOverlay.createStaticTextOverlay(text, StaticOverlaySettings.Builder().setBackgroundFrameAnchor(0f, -0.75f).build()))))
                    }
                    val media = source(originals.getValue(clip.assetId), clip, kind)
                    val item = EditedMediaItem.Builder(media).setDurationUs(us(if (kind == "photo") clip.duration else clip.trimIn + clip.duration)).setFrameRate(30)
                        .setRemoveAudio(true).setSpeed(SpeedParameters(speed(clip), true)).setEffects(Effects(emptyList(), videoEffects)).build()
                    sequences.add(sequence(clip, project.duration, item, C.TRACK_TYPE_VIDEO))
                }
                // This final sequence is the background. The compositor draws inputs back to front.
                val base = EditedMediaItem.Builder(MediaItem.Builder().setUri(Uri.fromFile(background)).setImageDurationMs(ceil(project.duration * 1000).toLong()).build())
                    .setDurationUs(us(project.duration)).setFrameRate(30).setRemoveAudio(true)
                    .setEffects(Effects(emptyList(), listOf(Presentation.createForWidthAndHeight(project.width, project.height, Presentation.LAYOUT_STRETCH_TO_FIT)))).build()
                sequences.add(EditedMediaItemSequence.withVideoFrom(listOf(base)))
                for (clip in ordered.filter { it.assetId in audibleAssets }) {
                    val item = EditedMediaItem.Builder(source(originals.getValue(clip.assetId), clip, "audio"))
                        .setDurationUs(us(clip.trimIn + clip.duration)).setRemoveVideo(true).setSpeed(SpeedParameters(speed(clip), true))
                        .setEffects(Effects(listOf(GainProcessor(clip)), emptyList())).build()
                    sequences.add(sequence(clip, project.duration, item, C.TRACK_TYPE_AUDIO))
                }
                val composition = Composition.Builder(sequences).setHdrMode(Composition.HDR_MODE_TONE_MAP_HDR_TO_SDR_USING_OPEN_GL)
                    .setVideoCompositorSettings(object : VideoCompositorSettings {
                        override fun getOutputSize(inputSizes: List<Size>) = Size(project.width, project.height)
                        override fun getOverlaySettings(inputId: Int, presentationTimeUs: Long): OverlaySettings {
                            val clip = visuals.getOrNull(inputId)
                            val time = presentationTimeUs / 1_000_000.0
                            val alpha = if (clip == null) 1.0 else if (time >= clip.start && time < clip.end) clip.opacityAt(time - clip.start) else 0.0
                            return StaticOverlaySettings.Builder().setAlphaScale(alpha.toFloat()).build()
                        }
                    }).build()
                export(context, composition, output)
                coroutineContext.ensureActive()
            }
        } catch (e: Throwable) { output.delete(); throw e }
        finally { background.delete() }
    }

    private fun us(seconds: Double) = (seconds * 1_000_000).roundToLong()
    private fun speed(clip: FilmClip) = object : androidx.media3.common.audio.SpeedProvider {
        override fun getSpeed(timeUs: Long) = clip.speed.toFloat()
        override fun getNextSpeedChangeTimeUs(timeUs: Long) = C.TIME_UNSET
    }
    private fun source(file: File, clip: FilmClip, kind: String): MediaItem {
        val builder = MediaItem.Builder().setUri(Uri.fromFile(file))
        if (kind == "photo") builder.setImageDurationMs(ceil(clip.duration * 1000).toLong())
        else builder.setClippingConfiguration(MediaItem.ClippingConfiguration.Builder().setStartPositionUs(us(clip.trimIn)).setEndPositionUs(us(clip.trimIn + clip.duration)).build())
        return builder.build()
    }
    private fun sequence(clip: FilmClip, total: Double, item: EditedMediaItem, track: Int): EditedMediaItemSequence {
        val builder = EditedMediaItemSequence.Builder(setOf(track))
        if (clip.start > 0) builder.addGap(us(clip.start))
        builder.addItem(item)
        if (total - clip.end > 0.0000005) builder.addGap(us(total - clip.end))
        return builder.build()
    }
    private fun grade(clip: FilmClip) = object : RgbMatrix {
        override fun getMatrix(presentationTimeUs: Long, useHdr: Boolean): FloatArray {
            val exposure = 2.0.pow(clip.exposure).toFloat()
            val s = clip.saturation.toFloat(); val inverse = 1 - s
            val r = 0.2126f * inverse; val g = 0.7152f * inverse; val b = 0.0722f * inverse
            return floatArrayOf((r + s) * exposure, r * exposure, r * exposure, 0f,
                g * exposure, (g + s) * exposure, g * exposure, 0f,
                b * exposure, b * exposure, (b + s) * exposure, 0f, 0f, 0f, 0f, 1f)
        }
    }
    private suspend fun export(context: Context, composition: Composition, output: File) = suspendCancellableCoroutine<Unit> { continuation ->
        val transformer = Transformer.Builder(context).setVideoMimeType("video/avc").setAudioMimeType("audio/mp4a-latm")
            .addListener(object : Transformer.Listener {
                override fun onCompleted(composition: Composition, result: ExportResult) { if (continuation.isActive) continuation.resume(Unit) }
                override fun onError(composition: Composition, result: ExportResult, exception: ExportException) { if (continuation.isActive) continuation.resumeWithException(exception) }
            }).build()
        continuation.invokeOnCancellation {
            if (Looper.myLooper() == Looper.getMainLooper()) transformer.cancel()
            else Handler(Looper.getMainLooper()).post { transformer.cancel() }
        }
        try { transformer.start(composition, output.absolutePath) }
        catch (e: Exception) { if (continuation.isActive) continuation.resumeWithException(e) }
    }

    /** Media3 applies speed before user audio effects; sample the output-time envelope. */
    internal class GainProcessor(private val clip: FilmClip) : BaseAudioProcessor() {
        private var frames = 0L
        private val curve = clip.volumeKeyframes.sortedBy { it.time }
        private fun gain(time: Double): Double {
            val sourceTime = time * clip.speed
            val value = if (curve.isEmpty()) clip.volume else if (sourceTime <= curve.first().time) curve.first().value else {
                var low = 1; var high = curve.size
                while (low < high) { val middle = (low + high) / 2; if (curve[middle].time < sourceTime) low = middle + 1 else high = middle }
                if (low == curve.size) curve.last().value else {
                    val a = curve[low - 1]; val b = curve[low]
                    if (a.time == b.time) b.value else a.value + (b.value - a.value) * (sourceTime - a.time) / (b.time - a.time)
                }
            }
            return (value * clip.fadeAt(time)).coerceIn(0.0, 4.0)
        }
        override fun onConfigure(format: AudioProcessor.AudioFormat): AudioProcessor.AudioFormat {
            if (format.encoding != C.ENCODING_PCM_16BIT && format.encoding != C.ENCODING_PCM_FLOAT) throw AudioProcessor.UnhandledAudioFormatException(format)
            return format
        }
        override fun onFlush(metadata: AudioProcessor.StreamMetadata) { frames = 0 }
        override fun queueInput(input: ByteBuffer) {
            input.order(ByteOrder.nativeOrder())
            val output = replaceOutputBuffer(input.remaining())
            val format = inputAudioFormat
            while (input.remaining() >= format.bytesPerFrame) {
                val gain = gain(frames.toDouble() / format.sampleRate)
                repeat(format.channelCount) {
                    if (format.encoding == C.ENCODING_PCM_FLOAT) output.putFloat((input.float * gain).coerceIn(-1.0, 1.0).toFloat())
                    else output.putShort((input.short * gain).roundToInt().coerceIn(Short.MIN_VALUE.toInt(), Short.MAX_VALUE.toInt()).toShort())
                }
                frames++
            }
            check(!input.hasRemaining()) { "Incomplete PCM frame." }
            output.flip()
        }
    }
}
