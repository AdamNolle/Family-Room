package com.familyroom.app

import android.graphics.Color as NativeColor
import android.graphics.Bitmap
import android.media.MediaMetadataRetriever
import android.widget.MediaController
import android.net.Uri
import android.provider.Settings
import android.view.Gravity
import android.widget.FrameLayout
import android.widget.VideoView
import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.util.UUID

/** A film opens as a disc inserted into a player; the content is the verified Room movie. */
@Composable
fun TheaterDiscSheet(model: RoomModel, initialStory: JSONObject, initialMovie: JSONObject,
    onClose: () -> Unit, queue: List<Pair<JSONObject, JSONObject>> = listOf(initialStory to initialMovie),
    playlistTitle: String? = null) {
    var index by remember(queue.map { it.first.optString("id") }) { mutableIntStateOf(0) }
    val (story, movie) = queue.getOrElse(index) { initialStory to initialMovie }
    val context = LocalContext.current
    val colors = MaterialTheme.colorScheme
    val reduceMotion = remember(context) {
        Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f
    }
    var phase by remember(movie.optString("id")) { mutableIntStateOf(0) }
    var attempt by remember(movie.optString("id")) { mutableIntStateOf(0) }
    var file by remember(movie.optString("id")) { mutableStateOf<File?>(null) }
    var artwork by remember(movie.optString("id")) { mutableStateOf<Bitmap?>(null) }
    var error by remember(movie.optString("id")) { mutableStateOf<String?>(null) }
    var playbackErrorText by remember(movie.optString("id")) { mutableStateOf<String?>(null) }
    var speechCues by remember(movie.optString("id")) { mutableStateOf(emptyList<SpokenCaptionCue>()) }
    var showSpokenCaptions by remember(movie.optString("id")) { mutableStateOf(true) }
    var playbackPositionMs by remember(movie.optString("id")) { mutableLongStateOf(0L) }
    val videoView = remember(movie.optString("id")) { mutableStateOf<VideoView?>(null) }
    val title = story.optString("title", "Family film")

    BackHandler(onBack = onClose)
    LaunchedEffect(movie.optString("id"), movie.optString("sha256")) {
        speechCues = if (movie.optString("kind") == "video") {
            try {
                val response = model.command(
                    JSONObject()
                        .put("action", "export_video_transcript")
                        .put("room_id", movie.getString("room_id"))
                        .put("asset_id", movie.getString("id")),
                )
                parseSpokenCaptionCues(response)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                emptyList()
            }
        } else {
            emptyList()
        }
    }
    LaunchedEffect(file, phase) {
        if (phase == 3 && file != null) {
            while (true) {
                playbackPositionMs = videoView.value?.currentPosition?.toLong() ?: 0L
                delay(200)
            }
        } else {
            playbackPositionMs = 0L
        }
    }
    LaunchedEffect(movie.optString("id"), attempt) {
        var output: File? = null
        try {
            if (movie.optString("kind") != "video") error("The published item is not a playable movie.")
            val localFile = File(context.cacheDir, "screening-${UUID.randomUUID()}.mp4")
            output = localFile
            if (!movie.optBoolean("local")) {
                model.command(JSONObject().put("action", "fetch")
                    .put("room_id", movie.getString("room_id")).put("asset_id", movie.getString("id")))
            }
            model.command(JSONObject().put("action", "materialize")
                .put("room_id", movie.getString("room_id")).put("asset_id", movie.getString("id"))
                .put("destination", localFile.absolutePath))
            artwork = withContext(Dispatchers.IO) { screeningFrame(localFile) }
            file = localFile
            if (reduceMotion) {
                phase = 3
            } else {
                phase = 1
                delay(430)
                phase = 2
                delay(610)
                phase = 3
            }
        } catch (cancelled: CancellationException) {
            output?.delete()
            throw cancelled
        } catch (failure: Exception) {
            output?.delete()
            error = failure.message ?: "The movie is not available from storage."
        }
    }
    DisposableEffect(file) { onDispose { file?.delete() } }

    Dialog(onDismissRequest = onClose, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Surface(Modifier.fillMaxSize(), color = colors.background) {
            Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(18.dp)) {
                Row(verticalAlignment = Alignment.Top) {
                    Column(Modifier.weight(1f)) {
                        Text(playlistTitle?.let { "$it · ${index + 1} of ${queue.size}" } ?: "NOW SHOWING",
                            style = MaterialTheme.typography.labelSmall,
                            color = colors.onSurfaceVariant, letterSpacing = 1.6.sp)
                        Text(title, fontFamily = RoomDisplay, fontSize = 38.sp,
                            color = colors.onSurface, maxLines = 2, overflow = TextOverflow.Ellipsis)
                    }
                    TextButton(onClick = onClose, modifier = Modifier.testTag("theater.close")) { Text("Close") }
                }
                if (queue.size > 1) Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedButton(onClick = { index-- }, enabled = index > 0,
                        modifier = Modifier.testTag("theater.previous")) { Text("Previous") }
                    OutlinedButton(onClick = { index++ }, enabled = index + 1 < queue.size,
                        modifier = Modifier.testTag("theater.next")) { Text("Next film") }
                }
            Spacer(Modifier.height(72.dp))
            if (phase == 3 && file != null && error == null) {
                    val movieFile = file!!
                    key(movieFile.absolutePath) {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .aspectRatio(
                                    story.optInt("width", 1920).coerceAtLeast(1).toFloat() /
                                        story.optInt("height", 1080).coerceAtLeast(1).toFloat(),
                                )
                                .clip(RoundedCornerShape(14.dp))
                                .testTag("theater.movie"),
                        ) {
                            AndroidView(
                                factory = { appContext ->
                                    FrameLayout(appContext).apply {
                                        setBackgroundColor(NativeColor.BLACK)
                                        val video = VideoView(appContext)
                                        addView(
                                            video,
                                            FrameLayout.LayoutParams(
                                                FrameLayout.LayoutParams.MATCH_PARENT,
                                                FrameLayout.LayoutParams.MATCH_PARENT,
                                                Gravity.CENTER,
                                            ),
                                        )
                                        val controls = MediaController(appContext).apply {
                                            setAnchorView(video)
                                        }
                                        video.setMediaController(controls)
                                        video.setOnErrorListener { _, _, _ ->
                                            playbackErrorText = "This film can’t be played on this device."
                                            true
                                        }
                                        video.setVideoURI(Uri.fromFile(movieFile))
                                        video.setOnPreparedListener {
                                            video.start()
                                            controls.show()
                                        }
                                        video.setOnCompletionListener {
                                            if (index + 1 < queue.size) index++
                                        }
                                        tag = video
                                        videoView.value = video
                                    }
                                },
                                modifier = Modifier.matchParentSize(),
                                onRelease = { container ->
                                    val releasedVideo = container.tag as? VideoView
                                    if (videoView.value === releasedVideo) videoView.value = null
                                    releasedVideo?.stopPlayback()
                                },
                            )
                            if (showSpokenCaptions) {
                                speechCues.firstOrNull { it.isActiveAt(playbackPositionMs) }?.let { cue ->
                                    Text(
                                        text = cue.text,
                                        style = MaterialTheme.typography.titleMedium,
                                        color = Color.White,
                                        modifier = Modifier
                                            .align(Alignment.BottomCenter)
                                            .padding(18.dp)
                                            .background(Color.Black.copy(alpha = 0.78f), RoundedCornerShape(12.dp))
                                            .padding(horizontal = 16.dp, vertical = 10.dp)
                                            .testTag("theater.caption.active"),
                                    )
                                }
                            }
                            playbackErrorText?.let { message ->
                                Surface(
                                    modifier = Modifier.align(Alignment.Center).testTag("theater.playback.error"),
                                    shape = RoundedCornerShape(16.dp),
                                    color = colors.surface,
                                    tonalElevation = 4.dp,
                                ) {
                                    Text(
                                        message,
                                        modifier = Modifier.padding(horizontal = 20.dp, vertical = 14.dp),
                                        color = colors.onSurface,
                                        style = MaterialTheme.typography.bodyMedium,
                                    )
                                }
                            }
                        }
                    }
                    if (speechCues.isNotEmpty()) {
                        TextButton(
                            onClick = { showSpokenCaptions = !showSpokenCaptions },
                            modifier = Modifier.testTag("theater.captions.toggle"),
                        ) {
                            Text(if (showSpokenCaptions) "Hide spoken captions" else "Show spoken captions")
                        }
                    }
                Text("Your original is preserved. Playback uses a temporary local copy.",
                        style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceVariant)
                } else {
                    val density = LocalDensity.current
                    val lift by animateFloatAsState(if (phase == 1) -35f else if (phase == 2) 84f else -16f,
                        animationSpec = spring(dampingRatio = 0.7f), label = "disc lift")
                    val tilt by animateFloatAsState(if (phase == 1) -15f else if (phase == 2) 68f else 0f,
                        animationSpec = spring(dampingRatio = 0.75f), label = "disc tilt")
                        val size by animateFloatAsState(if (phase == 1) 1.045f else if (phase == 2) 0.7f else 1f,
                            animationSpec = spring(dampingRatio = 0.74f), label = "disc size")
                        val spin by animateFloatAsState(if (phase == 2) 570f else 0f,
                            animationSpec = spring(dampingRatio = 0.82f), label = "disc spin")
                        val slotGlow by animateFloatAsState(if (phase == 2) 0.8f else 0f,
                            animationSpec = spring(dampingRatio = 0.78f), label = "player slot light")
                        Box(Modifier.fillMaxWidth().height(260.dp), contentAlignment = Alignment.Center) {
                        Box(Modifier.fillMaxWidth().height(104.dp).graphicsLayer { translationY = with(density) { 72.dp.toPx() } }
                            .background(Brush.linearGradient(listOf(colors.surface, colors.surfaceVariant, colors.surface)), RoundedCornerShape(23.dp))
                            .border(1.dp, colors.outlineVariant, RoundedCornerShape(23.dp)))
                        Box(Modifier.width(228.dp).height(if (phase == 2) 7.dp else 6.dp)
                            .graphicsLayer { translationY = with(density) { 53.dp.toPx() } }
                            .background(
                                Brush.horizontalGradient(
                                    listOf(colors.onSurface, colors.tertiary, colors.onSurface),
                                ),
                                CircleShape,
                            )
                            .border(0.6.dp, colors.outlineVariant, CircleShape))
                        Box(Modifier.width(64.dp).height(3.dp)
                            .graphicsLayer {
                                translationY = with(density) { 53.dp.toPx() }
                                alpha = slotGlow
                            }
                            .shadow(8.dp, CircleShape)
                            .background(colors.tertiary, CircleShape))
                    Box(Modifier.size(180.dp).graphicsLayer {
                            translationY = with(density) { lift.dp.toPx() }
                            rotationX = tilt
                            rotationZ = spin
                            scaleX = size; scaleY = size
                            alpha = if (phase == 2) 0.08f else 1f
                        cameraDistance = 16f * density.density
                    }.shadow(16.dp, CircleShape).clip(CircleShape).background(Color.White, CircleShape)
                            .border(7.dp, Color.White, CircleShape)
                            .border(1.dp, Color.Black, CircleShape)
                            .testTag("theater.disc"), contentAlignment = Alignment.Center) {
                        ScreeningDiscArtwork(artwork, Modifier.fillMaxSize().clip(CircleShape))
                        Box(Modifier.size(132.dp).border(0.8.dp, Color.White.copy(alpha = 0.43f), CircleShape))
                        Box(Modifier.size(96.dp).border(0.7.dp, Color.Black.copy(alpha = 0.23f), CircleShape))
                        Box(Modifier.size(35.dp).background(colors.background, CircleShape)
                                .border(3.dp, Color.White, CircleShape)
                                .border(0.8.dp, Color.Black, CircleShape))
                        }
                    }
                    if (error != null) {
                        Text("This film is waiting for its original.", fontFamily = RoomDisplay, fontSize = 28.sp)
                        Text(error!!, color = colors.onSurfaceVariant)
                TextButton(onClick = { error = null; phase = 0; file = null; artwork = null; attempt++ }) { Text("Try again") }
                    } else {
                        Row(horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
                            CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp)
                            Text("Preparing your film…", color = colors.onSurfaceVariant)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ScreeningDiscArtwork(frame: Bitmap?, modifier: Modifier = Modifier) {
    Box(modifier.background(Brush.linearGradient(listOf(Color(0xFF584235), Color(0xFFC28D6A), Color(0xFFE3CFBB)))), contentAlignment = Alignment.Center) {
        if (frame != null) {
            Image(frame!!.asImageBitmap(), contentDescription = "Film cover", modifier = Modifier.fillMaxSize().testTag("theater.artwork"), contentScale = ContentScale.Crop)
        } else {
            Text("FAMILY ROOM", color = Color.White, style = MaterialTheme.typography.labelMedium)
        }
    }
}

private fun screeningFrame(file: File): Bitmap? {
    val retriever = MediaMetadataRetriever()
    return try {
        retriever.setDataSource(file.absolutePath)
        retriever.getFrameAtTime(0)?.let { bitmap ->
            val scale = minOf(1f, 480f / maxOf(bitmap.width, bitmap.height))
            Bitmap.createScaledBitmap(
                bitmap,
                maxOf(1, (bitmap.width * scale).toInt()),
                maxOf(1, (bitmap.height * scale).toInt()),
                true
            )
        }
    } catch (_: Exception) {
        null
    } finally {
        retriever.release()
    }
}
