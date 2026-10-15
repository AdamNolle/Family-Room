package com.familyroom.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.foundation.text.selection.SelectionContainer
import kotlinx.coroutines.launch
import org.json.JSONObject

internal data class AlbumPublicationItem(
    val id: String,
    val deleted: Boolean = false,
    val componentIds: List<String> = emptyList(),
)

internal fun canPublishAlbum(role: String, albumOnly: Boolean): Boolean =
    !albumOnly && role in listOf("owner", "admin")

internal fun canPublishAlbum(room: JSONObject): Boolean =
    canPublishAlbum(room.optString("role"), room.optBoolean("album_only"))

internal fun albumPublicationIds(
    albumIds: Iterable<String>,
    items: List<AlbumPublicationItem>,
): List<String> {
    val byId = items.associateBy(AlbumPublicationItem::id)
    val selected = linkedSetOf<String>()
    for (id in albumIds) {
        val original = byId[id]?.takeUnless(AlbumPublicationItem::deleted) ?: continue
        selected += id
        for (componentId in original.componentIds) {
            if (byId[componentId]?.deleted == false) selected += componentId
        }
    }
    return selected.toList()
}

internal fun albumPublicationAssets(room: JSONObject, album: JSONObject): List<JSONObject> {
    val assets = room.rows("assets")
    val albumIds = album.optJSONArray("asset_ids") ?: return emptyList()
    val selectedIds = albumPublicationIds(
        (0 until albumIds.length()).mapNotNull { albumIds.optString(it).takeIf(String::isNotBlank) },
        assets.map { asset ->
            val components = asset.optJSONArray("components")
            AlbumPublicationItem(
                id = asset.optString("id"),
                deleted = asset.optBoolean("deleted"),
                componentIds = (0 until (components?.length() ?: 0))
                    .mapNotNull { components?.optString(it)?.takeIf(String::isNotBlank) },
            )
        },
    ).toSet()
    return assets.filter { it.optString("id") in selectedIds }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun AlbumSharingSheet(
    model: RoomModel,
    room: JSONObject,
    album: JSONObject,
    onClose: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val roomId = room.optString("id")
    val albumId = album.optString("id")
    val assets = remember(roomId, albumId) { albumPublicationAssets(room, album) }
    var deviceCode by remember(roomId, albumId) { mutableStateOf("") }
    var invitation by remember(roomId, albumId) { mutableStateOf("") }
    var busy by remember(roomId, albumId) { mutableStateOf(false) }
    var progress by remember(roomId, albumId) { mutableStateOf("") }
    var error by remember(roomId, albumId) { mutableStateOf("") }

    suspend fun makeAvailable() {
        if (assets.isEmpty()) throw IllegalStateException("Add at least one available memory to this album first.")
        for ((index, asset) in assets.withIndex()) {
            if (!asset.optBoolean("local")) {
                progress = "Getting original ${index + 1} of ${assets.size}…"
                model.command(
                    JSONObject()
                        .put("action", "fetch")
                        .put("room_id", roomId)
                        .put("asset_id", asset.getString("id")),
                )
            }
        }
    }

    ModalBottomSheet(
        onDismissRequest = onClose,
        containerColor = MaterialTheme.colorScheme.surface,
        modifier = Modifier.testTag("album.share.sheet"),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(max = 760.dp)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Text(
                "Share only “${album.optString("title", "this album")}”",
                style = MaterialTheme.typography.headlineMedium.copy(fontFamily = RoomDisplay),
            )
            Text(
                "Viewers receive a separately encrypted album library, without access to the rest of this Room. Person names never grant access.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                "Publish again after changing the album to update current viewers. Copies already downloaded by a viewer cannot be recalled.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            ElevatedCard(modifier = Modifier.fillMaxWidth()) {
                Column(
                    modifier = Modifier.padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Text("Current viewers", style = MaterialTheme.typography.titleMedium)
                    Button(
                        onClick = {
                            scope.launch {
                                busy = true
                                error = ""
                                invitation = ""
                                try {
                                    makeAvailable()
                                    val result = model.command(
                                        JSONObject()
                                            .put("action", "publish_album")
                                            .put("room_id", roomId)
                                            .put("album_id", albumId),
                                    )
                                    model.sync(roomId)
                                    progress = "Updated ${result.optInt("count")} memories for current viewers."
                                } catch (failure: Exception) {
                                    error = failure.message ?: "This album could not be published."
                                } finally {
                                    busy = false
                                }
                            }
                        },
                        enabled = !busy && assets.isNotEmpty(),
                        modifier = Modifier.testTag("album.share.publish"),
                    ) { Text(if (busy) "Preparing album…" else "Publish or update album") }
                }
            }
            ElevatedCard(modifier = Modifier.fillMaxWidth()) {
                Column(
                    modifier = Modifier.padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Text("Invite a viewer", style = MaterialTheme.typography.titleMedium)
                    OutlinedTextField(
                        value = deviceCode,
                        onValueChange = { deviceCode = it },
                        label = { Text("Their device code") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth().testTag("album.share.device"),
                    )
                    Button(
                        onClick = {
                            scope.launch {
                                busy = true
                                error = ""
                                invitation = ""
                                try {
                                    makeAvailable()
                                    val result = model.command(
                                        JSONObject()
                                            .put("action", "invite_album")
                                            .put("room_id", roomId)
                                            .put("album_id", albumId)
                                            .put("device_id", deviceCode.trim()),
                                    )
                                    invitation = result.optString("invitation")
                                    model.sync(roomId)
                                    progress = "Album-only invitation created. Share it with the intended viewer."
                                    deviceCode = ""
                                } catch (failure: Exception) {
                                    error = failure.message ?: "The album invitation could not be created."
                                } finally {
                                    busy = false
                                }
                            }
                        },
                        enabled = !busy && assets.isNotEmpty() && deviceCode.isNotBlank(),
                        modifier = Modifier.testTag("album.share.invite"),
                    ) { Text("Create album-only invitation") }
                    if (invitation.isNotBlank()) {
                        SelectionContainer {
                            Text(
                                invitation,
                                style = MaterialTheme.typography.bodySmall,
                                fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace,
                            )
                        }
                        TextButton(
                            onClick = {
                                context.getSystemService(Context.CLIPBOARD_SERVICE)
                                    ?.let { it as ClipboardManager }
                                    ?.setPrimaryClip(ClipData.newPlainText("Album invitation", invitation))
                                progress = "Invitation copied. Send it only to the intended viewer."
                            },
                            modifier = Modifier.testTag("album.share.copy"),
                        ) { Text("Copy invitation") }
                    }
                }
            }
            if (progress.isNotBlank()) {
                Text(progress, style = MaterialTheme.typography.bodySmall)
            }
            if (error.isNotBlank()) {
                Text(error, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
            }
            TextButton(onClick = onClose, modifier = Modifier.testTag("album.share.done")) { Text("Done") }
        }
    }
}
