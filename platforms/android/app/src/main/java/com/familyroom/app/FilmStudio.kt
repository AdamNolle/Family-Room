package com.familyroom.app

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import android.net.Uri
import android.graphics.Color as NativeColor
import android.view.Gravity
import android.widget.FrameLayout
import android.widget.MediaController
import android.widget.VideoView
import java.io.File
import org.json.JSONObject
import java.util.Locale

/** Phone task stack / tablet docked timeline, using the Room's actual originals. */
@Composable
fun FilmStudio(session: FilmSession, thumbnail: @Composable (JSONObject) -> Unit, openOriginal: (JSONObject) -> Unit, exportMovie: (File) -> Unit) {
    val project = session.project ?: return
    val wide = LocalConfiguration.current.screenWidthDp >= 840
    BackHandler { if (session.rendering) session.cancelRender() else session.close() }
    session.error?.let { message ->
        AlertDialog(onDismissRequest = session::clearError, title = { Text("Film studio") }, text = { Text(message) },
            confirmButton = { TextButton(onClick = session::clearError) { Text("OK") } })
    }
    Scaffold(
        topBar = {
            Row(Modifier.fillMaxWidth().statusBarsPadding().padding(horizontal = 16.dp, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                Text("Film studio", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                TextButton(onClick = session::close, enabled = !session.preparing && !session.saving && !session.rendering, modifier = Modifier.testTag("film.close")) { Text("Close") }
            }
        }
    ) { padding ->
        Column(Modifier.padding(padding).fillMaxSize().imePadding()) {
            if (session.preparing || session.saving || session.rendering) LinearProgressIndicator(Modifier.fillMaxWidth())
            if (wide) {
                Column(Modifier.fillMaxSize().padding(24.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    FilmHeading(session, compact = true)
                    Row(Modifier.weight(1f), horizontalArrangement = Arrangement.spacedBy(20.dp)) {
                        Column(Modifier.weight(1f).verticalScroll(rememberScrollState()).testTag("film.mediaPanel"), verticalArrangement = Arrangement.spacedBy(20.dp)) {
                            FilmOriginal(session, thumbnail, openOriginal)
                            FilmRenderControls(session, exportMovie)
                            FilmMediaBin(session, thumbnail)
                            FilmChapters(session)
                        }
                        Column(Modifier.width(320.dp).verticalScroll(rememberScrollState()).testTag("film.inspectorPanel")) { FilmInspector(session) }
                    }
                    FilmTimeline(session, Modifier.heightIn(max = 220.dp).verticalScroll(rememberScrollState()))
                }
            } else {
                Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) { FilmHistory(session) }
                Column(Modifier.weight(1f).fillMaxWidth().verticalScroll(rememberScrollState()).padding(20.dp).testTag("film.content"), verticalArrangement = Arrangement.spacedBy(24.dp)) {
                    FilmHeading(session, showHistory = false)
                    FilmOriginal(session, thumbnail, openOriginal)
                    FilmRenderControls(session, exportMovie)
                    FilmMediaBin(session, thumbnail)
                    FilmTimeline(session)
                    FilmInspector(session)
                    FilmChapters(session)
                }
            }
        }
    }
}

@Composable private fun FilmHeading(session: FilmSession, showHistory: Boolean = true, compact: Boolean = false) {
    val project = session.project ?: return
    if (compact) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(24.dp)) {
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("PRIVATE FILM STUDIO", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                OutlinedTextField(project.title, { title -> session.update { it.copy(title = title) } }, label = { Text("Film title") },
                    textStyle = MaterialTheme.typography.headlineSmall.copy(fontFamily = RoomDisplay), singleLine = true,
                    enabled = session.canEdit && !session.preparing, modifier = Modifier.fillMaxWidth().testTag("film.title"))
            }
            Column(Modifier.widthIn(max = 390.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) { FilmHistory(session) }
        }
    } else Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("PRIVATE FILM STUDIO", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        OutlinedTextField(project.title, { title -> session.update { it.copy(title = title) } }, label = { Text("Film title") },
            textStyle = MaterialTheme.typography.headlineSmall.copy(fontFamily = RoomDisplay), singleLine = true,
            enabled = session.canEdit && !session.preparing, modifier = Modifier.fillMaxWidth().testTag("film.title"))
        if (showHistory) FilmHistory(session)
    }
}

@Composable private fun FilmHistory(session: FilmSession) {
        val focus = LocalFocusManager.current
        Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = session::undo, enabled = session.canEdit && session.undoCount > 0 && !session.preparing, modifier = Modifier.testTag("film.undo")) { Text("Undo") }
            OutlinedButton(onClick = session::redo, enabled = session.canEdit && session.redoCount > 0 && !session.preparing, modifier = Modifier.testTag("film.redo")) { Text("Redo") }
            Button(onClick = { focus.clearFocus(); session.save() }, enabled = session.canEdit && !session.preparing && !session.saving, modifier = Modifier.testTag("film.save")) { Text("Save draft") }
        }
        Text(if (session.saving) "Saving privately…" else if (session.dirty) "Changes waiting to save" else "Draft saved on this device",
            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.testTag("film.saveStatus"))
        if (!session.hasContributionPermission) Text("Your Room role no longer allows film edits. Your saved draft is preserved.", color = MaterialTheme.colorScheme.error)
}

@Composable private fun FilmOriginal(session: FilmSession, thumbnail: @Composable (JSONObject) -> Unit, openOriginal: (JSONObject) -> Unit) {
    val selected = session.selected
    val asset = session.room?.rows("assets")?.find { it.optString("id") == selected?.assetId }
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text(if (session.renderedFile != null) "Rendered movie" else "Selected original", style = MaterialTheme.typography.titleMedium)
        Surface(color = MaterialTheme.colorScheme.surfaceVariant, shape = MaterialTheme.shapes.medium, modifier = Modifier.testTag("film.preview")) {
            Box(Modifier.fillMaxWidth().heightIn(min = 150.dp, max = 300.dp), contentAlignment = Alignment.Center) {
                if (session.renderedFile != null) NativeMovie(session.renderedFile!!)
                else if (asset != null) thumbnail(asset)
                else Text("Select a clip to inspect its original.", modifier = Modifier.padding(28.dp), color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        if (asset != null) TextButton(onClick = { openOriginal(asset) }) { Text("Open ${asset.optString("filename")}", maxLines = 1, overflow = TextOverflow.Ellipsis) }
        Text("Edits leave your originals unchanged. Drafts stay private until a movie is published.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable private fun NativeMovie(file: File) {
    key(file.absolutePath) {
        AndroidView(factory = { context ->
            FrameLayout(context).apply {
                setBackgroundColor(NativeColor.BLACK)
                val video = VideoView(context)
                addView(video, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT, Gravity.CENTER))
                val controls = MediaController(context).apply { setAnchorView(video) }
                video.setMediaController(controls)
                video.setVideoURI(Uri.fromFile(file))
                video.setOnPreparedListener { video.seekTo(1); controls.show() }
                tag = video
            }
        }, modifier = Modifier.fillMaxWidth().height(240.dp).testTag("film.movie"), onRelease = { (it.tag as VideoView).stopPlayback() })
    }
}

@Composable private fun FilmRenderControls(session: FilmSession, exportMovie: (File) -> Unit) {
    val project = session.project ?: return
    val focus = LocalFocusManager.current
    var sizes by remember { mutableStateOf(false) }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Box {
                OutlinedButton(onClick = { sizes = true }, enabled = session.canEdit && !session.preparing, modifier = Modifier.testTag("film.size")) { Text("${project.width} × ${project.height}") }
                DropdownMenu(expanded = sizes, onDismissRequest = { sizes = false }) {
                    for ((label, width, height) in listOf(Triple("Landscape · 1080p", 1920, 1080), Triple("Portrait · 1080p", 1080, 1920), Triple("Square · 1080p", 1080, 1080), Triple("Landscape · 480p", 854, 480))) {
                        DropdownMenuItem(text = { Text(label) }, onClick = { sizes = false; session.update { it.copy(width = width, height = height) } }, modifier = Modifier.testTag("film.size.$width.$height"))
                    }
                }
            }
            if (session.rendering) OutlinedButton(onClick = session::cancelRender) { Text("Cancel rendering") }
            else Button(onClick = { focus.clearFocus(); session.render() }, enabled = session.canEdit && !session.preparing && project.clips.isNotEmpty(), modifier = Modifier.testTag("film.render")) { Text("Render movie") }
        }
        if (session.renderStatus.isNotEmpty()) Text(session.renderStatus, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        session.renderedFile?.let { file ->
            Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedButton(onClick = { exportMovie(file) }, enabled = !session.preparing, modifier = Modifier.testTag("film.export")) { Text("Export movie") }
                Button(onClick = session::publishMovie, enabled = session.canEdit && !session.preparing && !session.saving, modifier = Modifier.testTag("film.publish")) { Text("Publish to ${session.room?.optString("name") ?: "Room"}") }
            }
        }
        if (project.autoGenerated) Text("Weekly draft · Save draft to keep it beyond two weeks.", style = MaterialTheme.typography.bodySmall)
    }
}

@Composable private fun FilmMediaBin(session: FilmSession, thumbnail: @Composable (JSONObject) -> Unit) {
    val assets = session.room?.rows("assets").orEmpty().filter { !it.optBoolean("deleted") && it.optString("kind") in listOf("photo", "video", "audio") }
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Media bin", style = MaterialTheme.typography.titleMedium)
        Text("Choose a memory to add to the timeline.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (assets.isEmpty()) Text("Add photos or videos to this Room first.")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            for (asset in assets) OutlinedCard(onClick = { session.add(asset) }, enabled = session.canEdit && !session.preparing,
                modifier = Modifier.width(152.dp).testTag("film.add.${asset.optString("id")}")) {
                Box(Modifier.fillMaxWidth().height(98.dp).background(MaterialTheme.colorScheme.surfaceVariant), contentAlignment = Alignment.Center) {
                    if (asset.optString("kind") == "audio") Text("Audio", style = MaterialTheme.typography.labelLarge) else thumbnail(asset)
                }
                Text(asset.optString("filename"), maxLines = 2, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(12.dp))
            }
        }
    }
}

@Composable private fun FilmTimeline(session: FilmSession, modifier: Modifier = Modifier) {
    val project = session.project ?: return
    val assets = session.room?.rows("assets").orEmpty().associateBy { it.optString("id") }
    val scale = if (project.duration > 180) 600.0 / project.duration else 28.0
    val step = maxOf(5.0, kotlin.math.ceil(project.duration / 500) * 5)
    Surface(modifier.fillMaxWidth(), shape = MaterialTheme.shapes.large, color = MaterialTheme.colorScheme.surface, border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("Timeline", style = MaterialTheme.typography.titleMedium)
            Text("${project.clips.size} clips · ${seconds(project.duration)} sec. Layers play together.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Column(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Row {
                    Text("Seconds", Modifier.width(76.dp), style = MaterialTheme.typography.labelSmall)
                    for (tick in 0..maxOf(1, kotlin.math.ceil(project.duration / step).toInt())) {
                        Text(seconds(tick * step), Modifier.width((step * scale).dp), style = MaterialTheme.typography.labelSmall, fontFamily = FontFamily.Monospace)
                    }
                }
                for (layer in 0..maxOf(1, project.clips.maxOfOrNull { it.track } ?: 0)) Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Layer ${layer + 1}", Modifier.width(76.dp), style = MaterialTheme.typography.labelSmall)
                    Box(Modifier.width(maxOf(500.0, project.duration * scale + 80).dp).height(56.dp).background(MaterialTheme.colorScheme.surfaceVariant, MaterialTheme.shapes.small)) {
                        for (clip in project.clips.filter { it.track == layer }) {
                            val selected = clip.id == session.selectedId
                            val name = assets[clip.assetId]?.optString("filename") ?: "Unavailable original"
                            FilledTonalButton(onClick = { session.select(clip.id) }, modifier = Modifier.offset(x = (clip.start * scale).dp)
                                .width(maxOf(64.0, clip.duration / clip.speed * scale).dp).height(56.dp).testTag("film.clip.${clip.id}"),
                                colors = ButtonDefaults.filledTonalButtonColors(containerColor = if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.secondaryContainer,
                                    contentColor = if (selected) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSecondaryContainer),
                                contentPadding = PaddingValues(8.dp), shape = MaterialTheme.shapes.small) {
                                Text(name, maxLines = 1, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.bodySmall)
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable private fun FilmInspector(session: FilmSession) {
    val clip = session.selected
    Surface(shape = MaterialTheme.shapes.large, color = MaterialTheme.colorScheme.surface, border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
            Text("CLIP INSPECTOR", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            if (clip == null) Text("Select a clip in the timeline.")
            else {
                val enabled = session.canEdit && !session.preparing
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    NumericField("Start", clip.start, 0.0..86400.0, Modifier.weight(1f), enabled) { session.modifyClip(clip.id) { c -> c.copy(start = it) } }
                    NumericField("Trim in", clip.trimIn, 0.0..86400.0, Modifier.weight(1f), enabled) { session.modifyClip(clip.id) { c -> c.copy(trimIn = it) } }
                }
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    NumericField("Duration", clip.duration, 0.001..86400.0, Modifier.weight(1f), enabled, "film.duration") { value ->
                        session.modifyClip(clip.id) { c -> c.resized(value) }
                    }
                    Column(Modifier.weight(1f)) {
                        Text("Layer ${clip.track + 1}", style = MaterialTheme.typography.labelSmall)
                        Row {
                            TextButton(onClick = { session.modifyClip(clip.id) { c -> c.copy(track = c.track - 1) } }, enabled = enabled && clip.track > 0, modifier = Modifier.semantics { contentDescription = "Move to lower layer" }) { Text("−") }
                            TextButton(onClick = { session.modifyClip(clip.id) { c -> c.copy(track = c.track + 1) } }, enabled = enabled && clip.track < 15, modifier = Modifier.semantics { contentDescription = "Move to higher layer" }) { Text("+") }
                        }
                    }
                }
                ClipSlider("Speed", clip.speed, 0.25f..4f, session, enabled) { session.modifyClip(clip.id, record = false) { c -> c.copy(speed = it) } }
                ClipSlider("Volume", clip.volume, 0f..2f, session, enabled) { session.modifyClip(clip.id, record = false) { c -> c.copy(volume = it) } }
                ClipSlider("Opacity", clip.opacity, 0f..1f, session, enabled) { session.modifyClip(clip.id, record = false) { c -> c.copy(opacity = it) } }
                ClipSlider("Exposure", clip.exposure, -3f..3f, session, enabled) { session.modifyClip(clip.id, record = false) { c -> c.copy(exposure = it) } }
                ClipSlider("Saturation", clip.saturation, 0f..2f, session, enabled) { session.modifyClip(clip.id, record = false) { c -> c.copy(saturation = it) } }
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    NumericField("Fade in", clip.fadeIn, 0.0..86400.0, Modifier.weight(1f), enabled) { session.modifyClip(clip.id) { c -> c.copy(fadeIn = it) } }
                    NumericField("Fade out", clip.fadeOut, 0.0..86400.0, Modifier.weight(1f), enabled) { session.modifyClip(clip.id) { c -> c.copy(fadeOut = it) } }
                }
                OutlinedTextField(clip.title, { session.modifyClip(clip.id) { c -> c.copy(title = it) } }, label = { Text("Title overlay") }, enabled = enabled, modifier = Modifier.fillMaxWidth())
                TextButton(onClick = session::splitSelected, enabled = enabled && clip.duration >= 0.002, modifier = Modifier.testTag("film.split")) { Text("Split in half") }
                TextButton(onClick = { session.update { p -> p.copy(clips = p.clips.filter { it.id != clip.id }) } }, enabled = enabled,
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error), modifier = Modifier.testTag("film.remove")) { Text("Remove clip") }
                FilmKeyframes(session, clip, enabled)
            }
        }
    }
}

@Composable private fun FilmKeyframes(session: FilmSession, clip: FilmClip, enabled: Boolean) {
    var expanded by remember(clip.id) { mutableStateOf(false) }
    TextButton(onClick = { expanded = !expanded }) { Text(if (expanded) "Hide keyframes" else "Keyframes") }
    if (!expanded) return
    for (opacity in listOf(true, false)) {
        val frames = if (opacity) clip.opacityKeyframes else clip.volumeKeyframes
        val label = if (opacity) "Opacity" else "Volume"
        Text("$label automation", style = MaterialTheme.typography.titleSmall)
        fun apply(next: List<FilmKeyframe>) {
            session.modifyClip(clip.id) { c -> if (opacity) c.copy(opacityKeyframes = next) else c.copy(volumeKeyframes = next) }
        }
        for ((index, frame) in frames.withIndex()) Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                NumericField("Time (source sec)", frame.time, 0.0..clip.duration, Modifier.weight(1f), enabled) { value -> apply(frames.mapIndexed { i, f -> if (i == index) f.copy(time = value) else f }) }
                NumericField("$label value", frame.value, 0.0..(if (opacity) 1.0 else 4.0), Modifier.weight(1f), enabled) { value -> apply(frames.mapIndexed { i, f -> if (i == index) f.copy(value = value) else f }) }
            }
            TextButton(onClick = { apply(frames.filterIndexed { i, _ -> i != index }) }, enabled = enabled) { Text("Remove keyframe") }
        }
        TextButton(onClick = { apply(frames.filter { it.time != clip.duration } + FilmKeyframe(clip.duration, if (opacity) clip.opacity else clip.volume)) }, enabled = enabled) { Text("Set $label keyframe at end") }
    }
}

@Composable private fun FilmChapters(session: FilmSession) {
    val project = session.project ?: return
    var expanded by remember { mutableStateOf(false) }
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        TextButton(onClick = { expanded = !expanded }) { Text(if (expanded) "Hide chapters" else "Chapters") }
        if (expanded) {
            for ((index, chapter) in project.chapters.withIndex()) Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(chapter.title, { value -> session.update { p -> p.copy(chapters = p.chapters.mapIndexed { i, c -> if (i == index) c.copy(title = value) else c }) } },
                    label = { Text("Chapter title") }, enabled = session.canEdit, modifier = Modifier.fillMaxWidth())
                NumericField("Seconds", chapter.time, 0.0..86400.0, enabled = session.canEdit) { value -> session.update { p -> p.copy(chapters = p.chapters.mapIndexed { i, c -> if (i == index) c.copy(time = value) else c }) } }
                TextButton(onClick = { session.update { p -> p.copy(chapters = p.chapters.filterIndexed { i, _ -> i != index }) } }, enabled = session.canEdit) { Text("Remove chapter") }
            }
            TextButton(onClick = { session.update { it.copy(chapters = it.chapters + FilmChapter("Chapter ${it.chapters.size + 1}", 0.0)) } }, enabled = session.canEdit) { Text("Add chapter") }
        }
    }
}

@Composable private fun NumericField(label: String, value: Double, range: ClosedFloatingPointRange<Double>, modifier: Modifier = Modifier,
    enabled: Boolean = true, tag: String = "", changed: (Double) -> Unit) {
    var text by remember { mutableStateOf(seconds(value)) }
    LaunchedEffect(value) { if (text.toDoubleOrNull() != value) text = seconds(value) }
    val parsed = text.toDoubleOrNull()
    val valid = parsed != null && parsed.isFinite() && parsed in range
    OutlinedTextField(text, { input -> text = input; input.toDoubleOrNull()?.takeIf { it.isFinite() && it in range }?.let(changed) },
        label = { Text(label) }, singleLine = true, isError = !valid, enabled = enabled,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal), modifier = modifier.testTag(tag),
        supportingText = if (valid) null else { { Text("${seconds(range.start)}–${seconds(range.endInclusive)}") } })
}

@Composable private fun ClipSlider(label: String, value: Double, range: ClosedFloatingPointRange<Float>, session: FilmSession, enabled: Boolean, changed: (Double) -> Unit) {
    var adjusting by remember(session.selectedId, label) { mutableStateOf(false) }
    Column {
        Row { Text(label, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium); Text(seconds(value), style = MaterialTheme.typography.labelMedium, fontFamily = FontFamily.Monospace) }
        Slider(value.toFloat().coerceIn(range), onValueChange = { if (!adjusting) { session.checkpoint(); adjusting = true }; changed(it.toDouble()) },
            onValueChangeFinished = { adjusting = false }, valueRange = range, enabled = enabled, modifier = Modifier.semantics { contentDescription = label })
    }
}

private fun seconds(value: Double) = String.format(Locale.ROOT, "%.3f", value).trimEnd('0').trimEnd('.')
