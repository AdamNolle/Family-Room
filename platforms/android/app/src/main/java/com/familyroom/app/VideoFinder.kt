package com.familyroom.app

import android.content.Context
import android.net.Uri
import android.widget.MediaController
import android.widget.VideoView
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.window.Dialog
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.util.UUID

private suspend fun indexVideo(
    context: Context,
    model: RoomModel,
    roomId: String,
    asset: JSONObject,
    alreadyLocal: Boolean,
    speechModelReady: Boolean,
    includeAudio: Boolean,
    includeFrames: Boolean,
    onProgress: suspend (String) -> Unit,
    onError: suspend (String) -> Unit,
): Boolean {
    val assetId = asset.optString("id")
    val suffix = "." + File(asset.optString("filename")).extension.ifBlank { "mp4" }
    val temporary = File(context.cacheDir, "video-index-${UUID.randomUUID()}$suffix")
    return try {
        if (!alreadyLocal) {
            model.command(JSONObject().put("action", "fetch")
                .put("room_id", roomId).put("asset_id", assetId))
        }
        model.command(JSONObject().put("action", "materialize")
            .put("room_id", roomId).put("asset_id", assetId)
            .put("destination", temporary.absolutePath))
        val frameAnalysis = if (includeFrames) {
            VideoFrameIndexer.analyze(temporary) { done, count ->
                if (done == count || done % 20 == 0) withContext(Dispatchers.Main) {
                    onProgress("${asset.optString("filename")} · frame $done of $count")
                }
            }
        } else null
        var speech: VideoAudioAnalysis? = null
        if (speechModelReady && includeAudio) {
            try {
                val modelPath = checkNotNull(OfflineSpeechModel.installed(context))
                val indexer = withContext(Dispatchers.IO) { VideoAudioIndexer(modelPath) }
                try {
                    speech = indexer.analyze(temporary) { percent ->
                        withContext(Dispatchers.Main) {
                            onProgress("${asset.optString("filename")} · listening $percent%")
                        }
                    }
                } finally {
                    withContext(Dispatchers.IO) { indexer.close() }
                }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                onError("${asset.optString("filename")}: speech analysis unavailable (${failure.message})")
            }
        }
        val observations = org.json.JSONArray()
        frameAnalysis?.let { analysis ->
            for (entry in 0 until analysis.observations.length())
                observations.put(analysis.observations.getJSONObject(entry))
        }
        speech?.let { result ->
            for (entry in 0 until result.observations.length())
                observations.put(result.observations.getJSONObject(entry))
        }
        check(frameAnalysis != null || speech?.audioIndexed == true) {
            speech?.message ?: "No local video analysis was available for this search."
        }
        model.command(JSONObject().put("action", "index_video")
            .put("room_id", roomId).put("asset_id", assetId)
            .put("audio_indexed", speech?.audioIndexed == true).put("frames_indexed", frameAnalysis != null)
            .put("preserve_existing", true).put("frame_interval_ms", frameAnalysis?.intervalMs ?: 0L)
            .put("observations", observations))
        true
    } catch (cancelled: CancellationException) {
        throw cancelled
    } catch (failure: Exception) {
        onError("${asset.optString("filename")}: ${failure.message ?: "Could not index"}")
        false
    } finally {
        withContext(Dispatchers.IO) { temporary.delete() }
    }
}

private data class VideoAssistantTurn(
    val prompt: String,
    val modality: String,
    val response: JSONObject,
    val speechUnavailable: Boolean,
    val id: String = UUID.randomUUID().toString(),
)

@OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
@Composable
internal fun VideoFinderSheet(model: RoomModel, roomId: String, onClose: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val scroll = rememberScrollState()
    val resultRequester = remember { BringIntoViewRequester() }
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
var question by remember(roomId) { mutableStateOf("") }
var modality by remember(roomId) { mutableStateOf("all") }
var includeRemoteVideos by remember(roomId) { mutableStateOf(false) }
    var searching by remember(roomId) { mutableStateOf(false) }
    val conversation = remember(roomId) { mutableStateListOf<VideoAssistantTurn>() }
    var indexStatus by remember(roomId) { mutableStateOf<JSONObject?>(null) }
    var progress by remember(roomId) { mutableStateOf("") }
    var error by remember(roomId) { mutableStateOf("") }
    var indexJob by remember(roomId) { mutableStateOf<Job?>(null) }
    var modelReady by remember(roomId) { mutableStateOf(OfflineSpeechModel.installed(context) != null) }
    var downloadJob by remember(roomId) { mutableStateOf<Job?>(null) }
    var selected by remember { mutableStateOf<Pair<JSONObject, Long>?>(null) }
    val assets = model.room?.rows("assets").orEmpty().associateBy { it.optString("id") }

    suspend fun refreshStatus(): JSONObject {
        val status = model.command(JSONObject().put("action", "video_index_status").put("room_id", roomId))
        indexStatus = status
        return status
    }
    suspend fun indexMissingVideos(
        missing: List<JSONObject>,
        includeAudio: Boolean = modelReady,
        includeFrames: Boolean = true,
    ) {
        var finished = 0
        var unavailable = 0
        for (video in missing) {
            val asset = assets[video.optString("asset_id")] ?: continue
        val alreadyLocal = video.optBoolean("local")
        progress = if (alreadyLocal) {
            "Analyzing ${asset.optString("filename")} (${finished + 1} of ${missing.size})"
        } else {
            "Downloading and indexing ${asset.optString("filename")} (${finished + 1} of ${missing.size})"
        }
        val indexed = indexVideo(context, model, roomId, asset, alreadyLocal, modelReady, includeAudio, includeFrames,
                onProgress = { message -> progress = message },
                onError = { message -> error = message })
            if (indexed) finished++ else unavailable++
            refreshStatus()
        }
        progress = "Indexed $finished selected video${if (finished == 1) "" else "s"}; $unavailable unavailable."
    }
    LaunchedEffect(roomId) {
        try { refreshStatus() } catch (failure: Exception) { error = failure.message ?: "Cannot read video index." }
    }
    DisposableEffect(roomId) {
        onDispose { indexJob?.cancel(); downloadJob?.cancel() }
    }

    ModalBottomSheet(onDismissRequest = onClose, modifier = Modifier.fillMaxWidth(),
        sheetState = sheetState, containerColor = MaterialTheme.colorScheme.surface) {
        val focusManager = LocalFocusManager.current
        val keyboard = LocalSoftwareKeyboardController.current
    fun submitSearch(prompt: String = question, modalityOverride: String? = null) {
        val request = prompt.trim()
        if (request.isBlank() || searching) return
        val requestedModality = modalityOverride ?: modality
        val indexPlan = videoSearchIndexPlan(requestedModality, speechModelAvailable = modelReady)
        modality = requestedModality
        focusManager.clearFocus(force = true)
        keyboard?.hide()
        searching = true
        scope.launch {
            try {
                error = ""
                val status = refreshStatus()
                val speechUnavailable = shouldExplainMissingSpeech(
                    indexPlan, videoIndexEntries(status.rows("videos")), includeRemoteVideos,
                )
            val pending = if (indexPlan.shouldIndex) pendingVideoIndexes(
                status,
                includeMissingAudio = indexPlan.includeMissingAudio,
                includeRemoteVideos = includeRemoteVideos,
            ) else emptyList()
            if (pending.isNotEmpty()) {
                progress = if (includeRemoteVideos) {
                    "Preparing your first search · downloading and indexing ${pending.size} Room video${if (pending.size == 1) "" else "s"}."
                } else {
                    "Preparing your first search · indexing ${pending.size} local video${if (pending.size == 1) "" else "s"}."
                }
                    indexJob = currentCoroutineContext()[Job]
                    try {
                        indexMissingVideos(
                            pending,
                            includeAudio = indexPlan.includeAudio,
                            includeFrames = indexPlan.includeFrames,
                        )
                    } finally {
                        indexJob = null
                    }
                }
                val search = JSONObject().put("action", "search_video")
                    .put("room_id", roomId).put("query", request).put("modality", requestedModality)
                conversation.lastOrNull()?.let { search.put("context_query", it.prompt) }
                val result = model.command(search)
                conversation.add(VideoAssistantTurn(request, requestedModality, result, speechUnavailable))
                question = ""
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) { error = failure.message ?: "Search failed." }
            finally { searching = false }
        }
    }

    LaunchedEffect(conversation.size) {
        if (conversation.isNotEmpty()) {
            withFrameNanos { }
            resultRequester.bringIntoView()
        }
    }

        Column(Modifier.fillMaxWidth().heightIn(max = 760.dp).verticalScroll(scroll)
            .padding(horizontal = 24.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
            Text("ASK YOUR VIDEOS", style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.primary)
            Text("Find the exact moment.", style = MaterialTheme.typography.headlineMedium.copy(fontFamily = RoomDisplay))
            Text("Ask about words spoken, text on screen, or visual labels. First search indexes this device’s videos. Opt in to remote-only originals when you want to include them.",
                style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        OutlinedTextField(
            value = question,
            onValueChange = { question = it.take(200) },
            enabled = !searching,
            label = { Text("Ask about your videos") },
            placeholder = { Text("What does birthday sign say?") },
            modifier = Modifier.fillMaxWidth().testTag("video.finder.query"),
            singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = { submitSearch() }),
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf("all" to "All", "audio" to "Spoken", "frame" to "In frame").forEach { (key, label) ->
                FilterChip(selected = modality == key, onClick = { modality = key }, label = { Text(label) })
            }
        }
        Button(
            onClick = { submitSearch() },
            enabled = question.isNotBlank() && !searching,
            modifier = Modifier.fillMaxWidth().testTag("video.finder.search"),
        ) {
            Text(if (searching) "Searching…" else "Find moment")
        }
        if (conversation.isEmpty()) {
                ElevatedCard(modifier = Modifier.testTag("video.finder.speech-model")) {
                    Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        Text("PRIVATE, EVIDENCE-BASED SEARCH", style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.primary)
                        Text("Try a spoken line, a sign or caption, or a visible subject.",
                            style = MaterialTheme.typography.bodyMedium)
                        AssistChip(onClick = {
                            question = "What does the birthday sign say?"
                            submitSearch("What does the birthday sign say?", "frame")
                        }, label = { Text("Find words on a sign") })
                        AssistChip(onClick = {
                            submitSearch("Find someone saying ‘I love you’", "audio")
                        }, label = { Text("Find a spoken line") })
                        AssistChip(onClick = {
                            submitSearch("Find ‘Happy birthday’ while blowing candles", "all")
                        }, label = { Text("Match a line to a scene") })
                    }
                }
            }
            if (!modelReady) {
                ElevatedCard {
                    Column(Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("OPTIONAL OFFLINE SPEECH INDEX", style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.primary)
                        Text("Install the English model to find exact spoken lines. Recognition stays on this device.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                        OutlinedButton(enabled = downloadJob == null && !searching, onClick = {
                            downloadJob = scope.launch {
                                try {
                                    error = ""
                                    OfflineSpeechModel.install(context) { percent ->
                                        progress = "Downloading offline speech model · $percent%"
                                    }
                                    modelReady = true
                                    progress = "Offline speech model ready. Ask again to index spoken lines."
                                } catch (cancelled: CancellationException) {
                                    progress = "Speech model download paused."
                                    throw cancelled
                                } catch (failure: Exception) {
                                    error = failure.message ?: "Could not install offline speech model."
                                } finally {
                                    downloadJob = null
                                }
                            }
                        }, modifier = Modifier.testTag("video.finder.speech-model.install")) {
                            Text(if (downloadJob == null) "Install speech model" else "Downloading model…")
                        }
                    }
                }
            }
            conversation.forEachIndexed { turnIndex, turn ->
                Surface(
                    modifier = Modifier.align(Alignment.End).fillMaxWidth(0.88f),
                    shape = MaterialTheme.shapes.large,
                    color = MaterialTheme.colorScheme.secondaryContainer,
                ) {
                    Text(turn.prompt, Modifier.padding(horizontal = 16.dp, vertical = 12.dp)
                        .testTag("video.finder.user-message"), style = MaterialTheme.typography.bodyMedium)
                }
                val scrollTarget = if (turnIndex == conversation.lastIndex)
                    Modifier.bringIntoViewRequester(resultRequester) else Modifier
                ElevatedCard(modifier = Modifier.fillMaxWidth().then(scrollTarget)
                    .testTag("video.finder.answer")) {
                    Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("FAMILY ROOM · LOCAL SEARCH", style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.primary)
                        Text("Evidence scope · ${when (turn.modality) {
                            "audio" -> "spoken words"
                            "frame" -> "video frames"
                            else -> "audio + frames"
                        }}", style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text(turn.response.optString("answer"), style = MaterialTheme.typography.titleMedium)
                        if (turn.speechUnavailable) {
                            Text("Some videos lack speech indexing because the optional offline model is not installed. Results may include transcripts already indexed on this device; frame evidence remains available in All mode.",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.tertiary)
                        }
                        val coverage = turn.response.optJSONObject("coverage")
                        Text("${coverage?.optInt("indexed") ?: 0} videos indexed on this device",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                turn.response.rows("matches").forEach { match ->
          val asset = assets[match.optString("asset_id")] ?: return@forEach
          val startMs = match.optLong("start_ms")
          val endMs = match.optLong("end_ms", startMs)
          val isExact = match.optString("match_kind") == "exact"
          val contextClues = match.optJSONArray("context_evidence")?.let { entries ->
              (0 until entries.length()).mapNotNull { index ->
                  entries.optJSONObject(index)?.let { entry ->
                      val label = when (entry.optString("modality")) {
                          "speech" -> "Spoken"
                          "screen_text" -> "On screen"
                          "sound_event" -> "Possible sound"
                          else -> "Visual suggestion"
                      }
                      "$label: ${entry.optString("text")}"
                  }
              }.distinct().take(2)
          }.orEmpty()
          val matchKindLabel = when (match.optString("match_kind")) {
              "exact" -> "Exact phrase"
              "evidence" -> "Indexed evidence"
              else -> "Suggested match"
          } + if (contextClues.isNotEmpty()) " · nearby clue" else ""
          ElevatedCard(onClick = { selected = asset to startMs }) {
                        Row(Modifier.fillMaxWidth().padding(12.dp), horizontalArrangement = Arrangement.spacedBy(12.dp),
                            verticalAlignment = Alignment.CenterVertically) {
              MemoryThumbnail(model, asset, Modifier.size(84.dp, 72.dp))
              Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(asset.optString("filename"), style = MaterialTheme.typography.titleSmall)
                Surface(
                  shape = MaterialTheme.shapes.extraLarge,
                  color = if (isExact) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.surfaceVariant,
                ) {
                  Text(
                    matchKindLabel,
                    Modifier.padding(horizontal = 9.dp, vertical = 4.dp),
                    style = MaterialTheme.typography.labelSmall,
                    color = if (isExact) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.colorScheme.onSurfaceVariant,
                  )
                }
                val startLabel = formatVideoTime(startMs / 1_000L)
                val endLabel = formatVideoTime(endMs / 1_000L)
                Text(if (endMs > startMs) "$startLabel–$endLabel" else "At $startLabel",
                  color = MaterialTheme.colorScheme.primary, style = MaterialTheme.typography.labelMedium)
                val evidence = match.optJSONArray("evidence")?.let { entries ->
                  (0 until entries.length()).mapNotNull { index ->
                    entries.optJSONObject(index)?.let { entry ->
                      val label = when (entry.optString("modality")) {
                        "speech" -> "Spoken"
                        "screen_text" -> "On screen"
                        else -> "Visual suggestion"
                      }
                      "$label: ${entry.optString("text")}"
                    }
                  }.distinct().filterNot { it in contextClues }.take(2)
                }.orEmpty()
              Text(evidence.joinToString(" · "), maxLines = 2,
                  style = MaterialTheme.typography.bodySmall,
                  color = MaterialTheme.colorScheme.onSurfaceVariant)
              if (contextClues.isNotEmpty()) {
                  Text("Nearby clue · ${contextClues.joinToString(" · ")}",
                      style = MaterialTheme.typography.bodySmall,
                      color = MaterialTheme.colorScheme.tertiary)
              }
                            }
                        }
                    }
                }
            }
            if (conversation.isNotEmpty()) {
                TextButton(onClick = { conversation.clear() }, modifier = Modifier.testTag("video.finder.clear-chat")) {
                    Text("Clear conversation")
                }
            }
            val status = indexStatus ?: JSONObject()
            val indexed = status.rows("videos")
            val pending = pendingVideoIndexes(status, includeMissingAudio = modelReady,
                includeRemoteVideos = includeRemoteVideos).size
            val total = indexed.size
            val frames = indexed.count { it.optBoolean("frames_indexed") }
            val audio = indexed.count { it.optBoolean("audio_indexed") }
            Text("$frames of $total videos indexed by frame · $audio indexed for speech",
                style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            if (indexed.any { !it.optBoolean("local") }) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Column(Modifier.weight(1f)) {
                        Text("Include remote-only videos", style = MaterialTheme.typography.bodyMedium)
                        if (includeRemoteVideos) Text(
                            "Selected originals download to this device and remain available offline. Storage and source network rules apply.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Switch(checked = includeRemoteVideos, onCheckedChange = { includeRemoteVideos = it },
                        enabled = !searching && indexJob == null,
                        modifier = Modifier.testTag("video.finder.include-remote"))
                }
            }
            if (pending > 0 || indexJob != null) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedButton(onClick = {
                        if (indexJob != null) {
                            indexJob?.cancel()
                            indexJob = null
                            progress = "Indexing paused. Completed videos remain searchable."
                        } else {
                            indexJob = scope.launch {
                                try {
                                    val status = refreshStatus()
                                    val missing = pendingVideoIndexes(status, includeMissingAudio = modelReady,
                                        includeRemoteVideos = includeRemoteVideos)
                                    indexMissingVideos(missing)
                                } finally {
                                    indexJob = null
                                }
                            }
                        }
                    }, modifier = Modifier.testTag("video.finder.index")) {
                    Text(if (indexJob == null) "Index $pending ${if (includeRemoteVideos) "Room" else "local"} video${if (pending == 1) "" else "s"}"
                            else "Stop indexing")
                    }
                }
            }
            if (progress.isNotBlank()) Text(progress, style = MaterialTheme.typography.bodySmall)
        if (error.isNotBlank()) {
            Text(error, color = MaterialTheme.colorScheme.error, modifier = Modifier.testTag("video.finder.error"))
        }
            Text("Frames and spoken words are analyzed on this device. Speech needs the optional English model. Sampled frames and recognition errors can miss brief or unclear details.",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)

            if (indexed.any { it.optBoolean("indexed") }) {
                TextButton(onClick = {
                    scope.launch {
                        try {
                            model.command(JSONObject().put("action", "clear_video_index").put("room_id", roomId))
                            conversation.clear()
                            refreshStatus()
                            progress = "Local video index cleared."
                        } catch (failure: Exception) { error = failure.message ?: "Could not clear index." }
                    }
                }) { Text("Clear local video index") }
            }
            Spacer(Modifier.height(24.dp))
        }
    }
        selected?.let { (asset, startMs) ->
            VideoEvidencePlayer(model, asset, startMs, onClose = { selected = null })
    }
}

@Composable
private fun VideoEvidencePlayer(model: RoomModel, asset: JSONObject, startMs: Long, onClose: () -> Unit) {
    val context = LocalContext.current
    var file by remember(asset.optString("id")) { mutableStateOf<File?>(null) }
    var error by remember { mutableStateOf("") }
    LaunchedEffect(asset.optString("id"), startMs) {
        val output = File(context.cacheDir, "video-play-${UUID.randomUUID()}.${File(asset.optString("filename")).extension.ifBlank { "mp4" }}")
        try {
            if (!asset.optBoolean("local")) model.command(JSONObject().put("action", "fetch")
                .put("room_id", asset.optString("room_id")).put("asset_id", asset.optString("id")))
            model.command(JSONObject().put("action", "materialize")
                .put("room_id", asset.optString("room_id")).put("asset_id", asset.optString("id"))
                .put("destination", output.absolutePath))
            file = output
        } catch (failure: Exception) {
            output.delete()
            error = failure.message ?: "Could not open this video."
        }
    }
    DisposableEffect(file) { onDispose { file?.delete() } }
    Dialog(onDismissRequest = onClose) {
        Surface(shape = MaterialTheme.shapes.large, color = MaterialTheme.colorScheme.surface) {
            Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(asset.optString("filename"), style = MaterialTheme.typography.titleMedium)
                val playable = file
                when {
                    playable != null -> AndroidView(factory = { viewContext ->
                        VideoView(viewContext).apply {
                            val controller = MediaController(viewContext)
                            controller.setAnchorView(this)
                            setMediaController(controller)
                            setVideoURI(Uri.fromFile(playable))
                                setOnPreparedListener { seekTo(startMs.coerceIn(0L, Int.MAX_VALUE.toLong()).toInt()); start() }
                        }
                    }, onRelease = { it.stopPlayback() }, modifier = Modifier.fillMaxWidth().height(280.dp))
                    error.isNotBlank() -> Text(error, color = MaterialTheme.colorScheme.error)
                    else -> CircularProgressIndicator()
                }
                    Text("Starting at ${formatVideoTime(startMs / 1_000L)}", style = MaterialTheme.typography.bodySmall)
                TextButton(onClick = onClose) { Text("Close") }
            }
        }
    }
}

private fun formatVideoTime(seconds: Long): String =
    "%02d:%02d:%02d".format(seconds / 3_600, (seconds / 60) % 60, seconds % 60)
