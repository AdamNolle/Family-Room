package com.familyroom.app

import android.content.Intent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.core.content.FileProvider
import kotlinx.coroutines.launch
import org.json.JSONObject
import java.io.File
import java.util.UUID

@Composable
internal fun VideoTranscriptExportButton(model: RoomModel, roomId: String, assetId: String) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var busy by remember(assetId) { mutableStateOf(false) }
    var status by remember(assetId) { mutableStateOf("") }

    Column {
        OutlinedButton(
            onClick = {
                scope.launch {
                    busy = true
                    try {
                        val result = model.command(
                            JSONObject()
                                .put("action", "export_video_transcript")
                                .put("room_id", roomId)
                                .put("asset_id", assetId),
                        )
                        val cueCount = result.optInt("cue_count")
                        val webvtt = result.optString("webvtt")
                        if (cueCount == 0 || webvtt.isBlank()) {
                            status = "No speech captions indexed yet. Use Ask your videos first."
                            return@launch
                        }
                        val directory = File(context.cacheDir, "originals/subtitle-exports").apply { mkdirs() }
                        val file = File(directory, "captions-$assetId-${UUID.randomUUID()}.vtt")
                        file.writeText(webvtt)
                        val uri = FileProvider.getUriForFile(
                            context,
                            "${context.packageName}.originals",
                            file,
                        )
                        val share = Intent(Intent.ACTION_SEND).apply {
                            type = "text/vtt"
                            putExtra(Intent.EXTRA_STREAM, uri)
                            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                        }
                        context.startActivity(Intent.createChooser(share, "Save or share captions"))
                        status = "$cueCount recognized speech captions. Review wording against the original."
                    } catch (error: Exception) {
                        status = error.message ?: "Could not prepare captions."
                    } finally {
                        busy = false
                    }
                }
            },
            enabled = !busy,
            modifier = Modifier.testTag("video.captions.export.$assetId"),
        ) {
            Text(if (busy) "Preparing captions…" else "Prepare spoken captions")
        }
        if (status.isNotBlank()) {
            Text(status, modifier = Modifier.padding(start = 8.dp, top = 4.dp))
        }
    }
}
