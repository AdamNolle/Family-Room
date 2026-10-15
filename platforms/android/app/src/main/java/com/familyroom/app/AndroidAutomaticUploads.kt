package com.familyroom.app

import android.content.Intent
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.documentfile.provider.DocumentFile
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject

@Composable
internal fun AndroidAutomaticUploads(model: RoomModel) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val room = model.room
    val roomId = model.roomId
    val mayContribute = room != null && room.optString("role") in setOf("owner", "admin", "contributor")
    var includeExisting by rememberSaveable(roomId) { mutableStateOf(false) }
    var busy by rememberSaveable(roomId) { mutableStateOf(false) }
    var message by rememberSaveable(roomId) { mutableStateOf("") }
    val rules = model.snapshot.rows("watches").filter {
        it.optString("room_id") == roomId && it.optString("path").startsWith("content://")
    }

    val folderPicker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            busy = true
            message = ""
            try {
                if (context.contentResolver.persistedUriPermissions.none { it.uri == uri && it.isReadPermission }) {
                    context.contentResolver.takePersistableUriPermission(
                        uri,
                        Intent.FLAG_GRANT_READ_URI_PERMISSION,
                    )
                }
                val initialSeen = if (includeExisting) {
                    emptyList()
                } else {
                    withContext(Dispatchers.IO) { AndroidFolderImport.initialSnapshot(context, uri) }
                }
                val request = JSONObject()
                    .put("action", "watch_content_tree")
                    .put("room_id", roomId)
                    .put("tree_uri", uri.toString())
                    .put("include_existing", includeExisting)
                    .put("initial_seen", JSONArray(initialSeen))
                val result = model.command(request)
                model.reload()
                message = if (includeExisting) {
                    "Folder connected. Existing photos and videos will be added to this Room."
                } else {
                    "Folder connected. New photos and videos will be added; existing items were left in place."
                }
                BackgroundSyncScheduler.enqueueFolderImportNow(context)
                if (result.optBoolean("updated")) message += " This folder’s existing upload rule was updated."
            } catch (failure: Exception) {
                message = failure.message ?: "Could not connect this folder."
            } finally {
                busy = false
            }
        }
    }

    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(18.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text("Automatic uploads", style = MaterialTheme.typography.titleMedium)
            Text(
                "Choose a folder on this device and its destination Room. Android checks it periodically in the background; upload timing depends on Android’s background scheduling.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text("Import existing media", style = MaterialTheme.typography.labelLarge)
                    Text(
                        "When off, only items added after setup are included.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Switch(
                    checked = includeExisting,
                    onCheckedChange = { includeExisting = it },
                    enabled = !busy,
                )
            }
            Button(
                onClick = { folderPicker.launch(null) },
                enabled = mayContribute && !busy && roomId.isNotBlank(),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(if (busy) "Connecting folder…" else "Choose upload folder")
            }
            if (!mayContribute) {
                Text(
                    "A Room owner, admin, or contributor can set up automatic uploads.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            rules.forEach { rule ->
                AutomaticUploadRule(model, context, rule, mayContribute) { text ->
                    message = text
                    BackgroundSyncScheduler.enqueueFolderImportNow(context)
                }
            }
            if (message.isNotBlank()) {
                Text(message, style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}

@Composable
private fun AutomaticUploadRule(
    model: RoomModel,
    context: android.content.Context,
    rule: JSONObject,
    enabled: Boolean,
    onChanged: (String) -> Unit,
) {
    val scope = rememberCoroutineScope()
    val id = rule.optString("id")
    val uri = Uri.parse(rule.optString("path"))
    val name = remember(uri) {
        DocumentFile.fromTreeUri(context, uri)?.name ?: "Selected folder"
    }
    val hasAccess = context.contentResolver.persistedUriPermissions.any {
        it.uri == uri && it.isReadPermission
    }
    val exclusions = rule.optJSONArray("excluded_extensions") ?: JSONArray()
    val initialExclusions = (0 until exclusions.length())
        .mapNotNull(exclusions::optString)
        .joinToString(", ")
    var exclusionDraft by rememberSaveable(id) { mutableStateOf(initialExclusions) }
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = 8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(name, style = MaterialTheme.typography.titleSmall)
        Text(
            "Destination: " + (model.rooms.firstOrNull { it.optString("id") == rule.optString("room_id") }
                ?.optString("name") ?: "Room"),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            when {
                !hasAccess -> "Folder access needs attention. Choose the folder again to reconnect it."
                rule.optBoolean("paused") -> "Paused"
                else -> "Watching this folder"
            },
            style = MaterialTheme.typography.labelMedium,
            color = if (hasAccess) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
        )
        OutlinedTextField(
            value = exclusionDraft,
            onValueChange = { exclusionDraft = it },
            label = { Text("Excluded extensions") },
            supportingText = { Text("Comma separated, for example gif, webp") },
            enabled = enabled,
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(
                onClick = {
                    scope.launch {
                        val extensions = exclusionDraft.split(",")
                            .map { it.trim().removePrefix(".").lowercase() }
                            .filter(String::isNotBlank)
                        val saved = model.execute(
                            JSONObject()
                                .put("action", "set_watch_exclusions")
                                .put("id", id)
                                .put("excluded_extensions", JSONArray(extensions)),
                        )
                        onChanged(if (saved) "Upload exclusions saved." else model.status)
                    }
                },
                enabled = enabled,
            ) { Text("Save exclusions") }
            OutlinedButton(
                onClick = {
                    scope.launch {
                        val paused = !rule.optBoolean("paused")
                        val saved = model.execute(
                            JSONObject().put("action", "pause_watch").put("id", id).put("paused", paused),
                        )
                        onChanged(if (saved) if (paused) "Automatic uploads paused." else "Automatic uploads resumed." else model.status)
                    }
                },
                enabled = enabled,
            ) { Text(if (rule.optBoolean("paused")) "Resume" else "Pause") }
            TextButton(
                onClick = {
                    scope.launch {
                        val removed = model.execute(JSONObject().put("action", "remove_watch").put("id", id))
                        if (removed) {
                            val stillUsed = model.snapshot.rows("watches").any {
                                it.optString("path") == uri.toString()
                            }
                            val permission = context.contentResolver.persistedUriPermissions
                                .firstOrNull { it.uri == uri && it.isReadPermission }
                            if (!stillUsed && permission != null) {
                                runCatching {
                                    context.contentResolver.releasePersistableUriPermission(
                                        uri,
                                        Intent.FLAG_GRANT_READ_URI_PERMISSION,
                                    )
                                }
                            }
                        }
                        onChanged(if (removed) "Folder disconnected." else model.status)
                    }
                },
                enabled = enabled,
            ) { Text("Disconnect") }
        }
    }
}
