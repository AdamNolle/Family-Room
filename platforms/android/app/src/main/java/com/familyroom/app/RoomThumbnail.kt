package com.familyroom.app

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.media.MediaMetadataRetriever
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File

@Composable
fun MemoryThumbnail(model: RoomModel, asset: JSONObject, modifier: Modifier = Modifier.fillMaxWidth().heightIn(max = 320.dp).aspectRatio(3f / 2f)) {
    val cacheDir = LocalContext.current.cacheDir
    var bitmap by remember(asset.optString("id")) { mutableStateOf<Bitmap?>(null) }
    val local = asset.optBoolean("local")
    val kind = asset.optString("kind")
    var loading by remember(asset.optString("id"), local) { mutableStateOf(local && kind in listOf("photo", "video")) }
    LaunchedEffect(asset.optString("id"), local) {
        if (local && (kind == "photo" || kind == "video")) {
            loading = true
            bitmap = withContext(Dispatchers.IO) {
                val temporary = File.createTempFile("memory-preview-", ".tmp", cacheDir)
                try {
                    model.command(JSONObject().put("action", "materialize")
                        .put("room_id", asset.getString("room_id"))
                        .put("asset_id", asset.getString("id"))
                        .put("destination", temporary.absolutePath))
                    if (kind == "video") {
                        val retriever = MediaMetadataRetriever()
                        try {
                            retriever.setDataSource(temporary.absolutePath)
                            retriever.getFrameAtTime(0)?.let { frame ->
                                val scale = minOf(1f, 800f / maxOf(frame.width, frame.height))
                                Bitmap.createScaledBitmap(frame, maxOf(1, (frame.width * scale).toInt()),
                                    maxOf(1, (frame.height * scale).toInt()), true)
                            }
                        } finally { retriever.release() }
                    } else {
                        val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                        BitmapFactory.decodeFile(temporary.absolutePath, options)
                        options.inJustDecodeBounds = false
                        options.inSampleSize = maxOf(1, maxOf(options.outWidth, options.outHeight) / 800)
                        BitmapFactory.decodeFile(temporary.absolutePath, options)
                    }
                } catch (_: Exception) { null }
                finally { temporary.delete() }
            }
            loading = false
        }
    }
    RoomPhotoFrame(modifier) {
        bitmap?.let {
            Image(it.asImageBitmap(), contentDescription = asset.optString("filename"),
                modifier = Modifier.fillMaxSize(), contentScale = ContentScale.Crop)
        }
        if (bitmap == null && loading) Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surfaceVariant), contentAlignment = androidx.compose.ui.Alignment.Center) {
            CircularProgressIndicator(Modifier.size(24.dp).semantics { contentDescription = "Loading preview" }, strokeWidth = 2.dp)
        }
        if (bitmap == null && !loading) Text(if (!local) "Original available from storage" else "Preview not available",
            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surfaceVariant).padding(16.dp))
    }
}

