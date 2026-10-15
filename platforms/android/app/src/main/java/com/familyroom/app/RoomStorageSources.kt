package com.familyroom.app

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.json.JSONObject

@Composable
fun RoomStorageSources(model: RoomModel, roomId: String) {
    val sources = model.snapshot.rows("sources").filter { it.optString("room_id") == roomId }
    val scope = rememberCoroutineScope()
    var cleanupMessage by remember(roomId) { mutableStateOf("") }
    val cacheLimit = model.rooms.firstOrNull { it.optString("id") == roomId }
        ?.optLong("device_cache_limit_bytes", -1L) ?: -1L
    Column(verticalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.fillMaxWidth()) {
        Text("Upload destinations", style = MaterialTheme.typography.titleSmall)
        Text("Free device space", style = MaterialTheme.typography.titleMedium)
        Text("Only originals with another verified copy are eligible. Offline pins stay on this device; older unpinned memories are considered first.",
            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf("500 MB" to 500L * 1_024 * 1_024, "1 GB" to 1_024L * 1_024 * 1_024, "5 GB" to 5L * 1_024 * 1_024 * 1_024).forEach { (label, bytes) ->
                TextButton(onClick = { scope.launch {
                    try {
                        val result = model.command(JSONObject().put("action", "free_device_storage")
                            .put("room_id", roomId).put("bytes_to_free", bytes))
                        model.reload()
                        val freed = result.optLong("bytes_freed", 0L)
                        val complete = result.optBoolean("complete")
                        val pinned = result.optInt("pinned_originals_preserved", 0)
                        cleanupMessage = "Freed ${formatStorageBytes(freed)}. ${if (complete) "Request met." else "Not enough verified copies to meet the full request."} $pinned pinned originals stayed here."
                    } catch (error: Exception) {
                        cleanupMessage = error.message ?: "Could not free device space."
                    }
                } }) { Text(label) }
            }
        }
        if (cleanupMessage.isNotEmpty()) Text(cleanupMessage, style = MaterialTheme.typography.bodySmall)
        Text("Automatic device cache · ${if (cacheLimit < 0) "No limit" else formatStorageBytes(cacheLimit)}",
            style = MaterialTheme.typography.titleMedium)
        Text("After Room sync, Family Room removes only originals with another verified copy. Offline pins and sole copies stay here.",
            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            listOf<Pair<String, Long?>>("No limit" to null, "2 GB" to 2_147_483_648L,
                "5 GB" to 5_368_709_120L, "10 GB" to 10_737_418_240L).forEach { (label, limit) ->
                TextButton(onClick = { scope.launch {
                    try {
                        val result = model.command(JSONObject().put("action", "set_device_cache_limit")
                            .put("room_id", roomId).put("max_bytes", limit ?: JSONObject.NULL))
                        model.reload()
                        val cleanup = result.optJSONObject("cleanup")
                        val freed = cleanup?.optLong("bytes_freed", 0L) ?: 0L
                        val complete = cleanup?.optBoolean("complete", true) ?: true
                        cleanupMessage = if (limit == null) "Automatic cache cleanup is off."
                        else "Cache limit set to $label. Freed ${formatStorageBytes(freed)} now. ${if (complete) "Limit met." else "Cleanup will retry after another verified copy is available."}"
                    } catch (error: Exception) {
                        cleanupMessage = error.message ?: "Could not update the cache limit."
                    }
                } }) { Text(if (cacheLimit == (limit ?: -1L)) "✓ $label" else label) }
            }
        }
        for (source in sources) {
            val id = source.optString("id")
            Column(verticalArrangement = Arrangement.spacedBy(3.dp), modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                Text(source.optString("name"), style = MaterialTheme.typography.titleMedium)
                Text(source.optString("kind").replace('_', ' ') + if (source.optBoolean("paused")) " · Paused" else "",
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                if (source.optBoolean("preferred")) Text("Preferred for new uploads")
                if (source.optBoolean("overflow")) Text("Used when preferred storage is full")
                if (source.optString("kind") != "local") {
                    val unmeteredOnly = source.optString("network_policy", "any") == "unmetered_only"
                    TextButton(onClick = { scope.launch {
                        val policy = if (unmeteredOnly) "any" else "unmetered_only"
                        if (model.execute(JSONObject().put("action", "set_source_network_policy")
                                .put("id", id).put("policy", policy)) && policy == "any") model.sync(roomId)
                    } }) { Text(if (unmeteredOnly) "Media: unmetered only" else "Media: any network") }
                    Text("Catalog updates can use mobile data; original media transfers follow this setting.",
                        style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    if (!source.optBoolean("preferred")) {
                        TextButton(onClick = { scope.launch {
                            if (model.execute(JSONObject().put("action", "prefer_source").put("id", id))) model.sync(roomId)
                        } }) { Text("Make preferred") }
                        TextButton(onClick = { scope.launch {
                            if (model.execute(JSONObject().put("action", "set_overflow")
                                    .put("room_id", roomId)
                                    .put("id", if (source.optBoolean("overflow")) JSONObject.NULL else id))) model.sync(roomId)
                        } }) { Text(if (source.optBoolean("overflow")) "Clear overflow" else "Use for overflow") }
                    }
                }
            }
        }
    }
}

private fun formatStorageBytes(bytes: Long): String = when {
    bytes >= 1_000_000_000L -> "%.1f GB".format(bytes / 1_000_000_000.0)
    bytes >= 1_000_000L -> "%.0f MB".format(bytes / 1_000_000.0)
    else -> "$bytes bytes"
}
