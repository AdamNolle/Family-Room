package com.familyroom.app

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject

/** Shared Room catalog collections; album membership never owns or deletes an original. */
@Composable
fun RoomCollectionPanel(model: RoomModel, room: JSONObject, kind: String, query: String = "",
    onPlayPlaylist: (JSONObject) -> Unit = {}) {
    val scope = rememberCoroutineScope()
    val keyboardController = LocalSoftwareKeyboardController.current
    val canEdit = room.optString("role") in listOf("owner", "admin", "contributor")
    val canPublish = kind == "album" && canPublishAlbum(room)
    val collectionName = if (kind == "playlist") "playlist" else "album"
    val field = if (kind == "playlist") "story_ids" else "asset_ids"
    var newTitle by remember(room.optString("id"), kind) { mutableStateOf("") }
    var editing by remember(room.optString("id"), kind) { mutableStateOf<JSONObject?>(null) }
    var sharing by remember(room.optString("id"), kind) { mutableStateOf<JSONObject?>(null) }
    Column(
        Modifier.fillMaxWidth().heightIn(max = 640.dp)
            .verticalScroll(rememberScrollState())
            .testTag("collection.panel.$kind"),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(if (kind == "playlist") "Playlists" else "Albums", style = MaterialTheme.typography.headlineSmall)
        if (canEdit) Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(newTitle, { newTitle = it }, label = { Text("$collectionName name") },
                singleLine = true, modifier = Modifier.weight(1f).testTag("$collectionName.title"))
            Button(onClick = {
                val name = newTitle.trim()
                newTitle = ""
                keyboardController?.hide()
                scope.launch {
                    model.execute(JSONObject().put("action", "save_$collectionName")
                        .put("room_id", room.getString("id")).put("title", name).put(field, JSONArray()))
                }
            }, enabled = newTitle.isNotBlank(), modifier = Modifier.testTag("$collectionName.create")) { Text("Create") }
        }
        if (kind == "playlist") {
            val playlists = room.rows("playlists").filter { it.optString("title").contains(query, true) }
            if (playlists.isEmpty()) Text("Collect published films in the order you want to watch them.", color = MaterialTheme.colorScheme.onSurfaceVariant)
            for (playlist in playlists) {
                val count = playlist.optJSONArray("story_ids")?.length() ?: 0
                Card(Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(7.dp)) {
                        Text(playlist.optString("title"), style = MaterialTheme.typography.titleMedium)
                        Text("$count published ${if (count == 1) "film" else "films"}", color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Row {
                            TextButton(onClick = { onPlayPlaylist(playlist) }, enabled = playlistFilms(room, playlist).isNotEmpty(),
                                modifier = Modifier.testTag("playlist.play.${playlist.optString("id")}")) { Text("Play all") }
                            if (canEdit) TextButton(onClick = { editing = playlist },
                                modifier = Modifier.testTag("playlist.edit.${playlist.optString("id")}")) { Text("Arrange") }
                        }
                    }
                }
            }
        } else {
            val albums = room.rows("albums").filter { it.optString("title").contains(query, true) }
            if (albums.isEmpty()) Text("Make an album to collect memories across Moments.", color = MaterialTheme.colorScheme.onSurfaceVariant)
            for (album in albums) {
                val count = album.optJSONArray("asset_ids")?.length() ?: 0
                val albumIds = (0 until count).map { album.getJSONArray("asset_ids").getString(it) }
                val chosenId = album.optString("cover_asset_id").takeIf { it in albumIds }
                val cover = (listOfNotNull(chosenId) + albumIds).firstNotNullOfOrNull { id ->
                    room.rows("assets").find { it.optString("id") == id && !it.optBoolean("deleted") }
                }
                Card(Modifier.fillMaxWidth()) {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        if (canEdit) TextButton(
                            onClick = { editing = album },
                            modifier = Modifier.testTag("album.edit.${album.optString("id")}"),
                        ) { Text("Arrange") }
                        if (canPublish) TextButton(
                            onClick = { sharing = album },
                            modifier = Modifier.testTag("album.share.${album.optString("id")}"),
                        ) { Text("Share privately") }
                        if (cover != null) MemoryThumbnail(model, cover, Modifier.fillMaxWidth().height(190.dp))
                        Row(Modifier.padding(16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            Column(Modifier.weight(1f)) {
                            Text(album.optString("title"), style = MaterialTheme.typography.titleMedium)
                            album.optString("caption").takeIf { it.isNotBlank() }?.let {
                                Text(it, maxLines = 2, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                            Text("$count ${if (count == 1) "memory" else "memories"}", color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                    AlbumOfflineControls(model, room, album, albumIds)
                        }
                    }
                }
            }
        }
    }
    editing?.let { collection ->
        RoomCollectionEditor(model, room, collection, kind, onClose = { editing = null })
    }
    sharing?.let { album ->
        AlbumSharingSheet(model, room, album, onClose = { sharing = null })
    }
}

@Composable
private fun AlbumOfflineControls(model: RoomModel, room: JSONObject, album: JSONObject, albumIds: List<String>) {
    val scope = rememberCoroutineScope()
    val roomAssets = room.rows("assets").filter { !it.optBoolean("deleted") }
    val linkedIds = albumIds.flatMap { id ->
        roomAssets.firstOrNull { it.optString("id") == id }
            ?.optJSONArray("components")
            ?.let { components -> (0 until components.length()).map { components.getString(it) } }
            ?: emptyList()
    }
    val assets = (albumIds + linkedIds).distinct().mapNotNull { id ->
        roomAssets.firstOrNull { it.optString("id") == id }
    }
    val pinnedCount = assets.count { it.optBoolean("pinned_offline") }
    var saving by remember(album.optString("id")) { mutableStateOf(false) }
    var completed by remember(album.optString("id")) { mutableIntStateOf(0) }
    var error by remember(album.optString("id")) { mutableStateOf("") }
    val allPinned = assets.isNotEmpty() && pinnedCount == assets.size
    val evictable = assets.filter { it.optBoolean("local") && !it.optBoolean("pinned_offline") && it.optInt("verified_copies") > 1 }
    var confirmEviction by remember(album.optString("id")) { mutableStateOf(false) }

    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        TextButton(
            onClick = {
                scope.launch {
                    saving = true
                    completed = 0
                    error = ""
                    val action = if (allPinned) "unpin_original" else "pin_original"
                    for (asset in assets) {
                        val isPinned = asset.optBoolean("pinned_offline")
                        if ((action == "pin_original" && isPinned) || (action == "unpin_original" && !isPinned)) {
                            completed++
                            continue
                        }
                        val ok = model.execute(JSONObject()
                            .put("action", action)
                            .put("room_id", room.getString("id"))
                            .put("asset_id", asset.getString("id")))
                        if (!ok) {
                            error = model.status
                            model.reload()
                            break
                        }
                        completed++
                    }
                    saving = false
                }
            },
            enabled = !saving && assets.isNotEmpty(),
            modifier = Modifier.testTag("album.offline.${album.optString("id")}")
        ) {
            Text(when {
                saving -> "Saving originals…"
                allPinned -> "Unpin originals"
                pinnedCount > 0 -> "Save originals offline ($pinnedCount/${assets.size})"
                else -> "Save originals offline"
            })
        }
        if (evictable.isNotEmpty()) {
            TextButton(
                onClick = { confirmEviction = true },
                enabled = !saving,
                modifier = Modifier.testTag("album.evict.${album.optString("id")}")
            ) { Text("Free device storage (${evictable.size})") }
        }
        if (confirmEviction) {
            AlertDialog(
                onDismissRequest = { confirmEviction = false },
                title = { Text("Free device storage?") },
                text = { Text("Only unpinned originals with a second verified copy can be removed. They can be downloaded again later.") },
                confirmButton = {
                    TextButton(onClick = {
                        confirmEviction = false
                        scope.launch {
                            saving = true
                            completed = 0
                            error = ""
                            for (asset in evictable) {
                                val ok = model.execute(JSONObject()
                                    .put("action", "evict_original")
                                    .put("room_id", room.getString("id"))
                                    .put("asset_id", asset.getString("id")))
                                if (!ok) {
                                    error = model.status
                                    model.reload()
                                    break
                                }
                                completed++
                            }
                            saving = false
                        }
                    }) { Text("Free space") }
                },
                dismissButton = { TextButton(onClick = { confirmEviction = false }) { Text("Cancel") } }
            )
        }
        if (saving) Text("$completed of ${assets.size} originals processed", style = MaterialTheme.typography.bodySmall)
        if (error.isNotBlank()) Text(error, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
    }
}

fun playlistFilms(room: JSONObject, playlist: JSONObject): List<Pair<JSONObject, JSONObject>> {
    val storyRows = room.rows("stories")
    val movieRows = room.rows("assets")
    val ids = playlist.optJSONArray("story_ids") ?: return emptyList()
    val orderedIds = (0 until ids.length()).map(ids::optString)
    val resolved = playablePlaylistOrder(
        storyIds = orderedIds,
        stories = storyRows.map { story ->
            PlaylistStoryRef(
                id = story.optString("id"),
                publishedAssetId = story.optString("published_asset"),
                privateDraft = story.optBoolean("private_draft"),
            )
        },
        movies = movieRows.map { movie ->
            PlaylistMovieRef(
                id = movie.optString("id"),
                kind = movie.optString("kind"),
                deleted = movie.optBoolean("deleted"),
            )
        },
    )
    val storiesById = storyRows.associateBy { it.optString("id") }
    val moviesById = movieRows.associateBy { it.optString("id") }
    return resolved.mapNotNull { (storyId, movieId) ->
        val story = storiesById[storyId] ?: return@mapNotNull null
        val movie = moviesById[movieId] ?: return@mapNotNull null
        story to movie
    }
}

@Composable
fun RoomCollectionEditor(model: RoomModel, room: JSONObject, collection: JSONObject, kind: String, onClose: () -> Unit) {
    val scope = rememberCoroutineScope()
    val isPlaylist = kind == "playlist"
    val field = if (isPlaylist) "story_ids" else "asset_ids"
    var title by remember(collection.optString("id")) { mutableStateOf(collection.optString("title")) }
    var caption by remember(collection.optString("id")) { mutableStateOf(collection.optString("caption")) }
    var coverId by remember(collection.optString("id")) {
        mutableStateOf(collection.optString("cover_asset_id").takeUnless { it.isEmpty() || it == "null" })
    }
    var confirmDelete by remember(collection.optString("id")) { mutableStateOf(false) }
    val ids = remember(collection.optString("id")) {
        mutableStateListOf<String>().also { list ->
            collection.optJSONArray(field)?.let { array -> for (i in 0 until array.length()) list.add(array.getString(i)) }
        }
    }
    val candidates = if (isPlaylist) {
        val published = linkedMapOf<String, JSONObject>()
        for (story in room.rows("stories")) if (!story.optBoolean("private_draft") && story.optString("published_asset").isNotEmpty()) published[story.optString("id")] = story
        published.values.toList()
    } else room.rows("assets").filter { !it.optBoolean("deleted") }
    val byId = candidates.associateBy { it.optString("id") }
    Dialog(onDismissRequest = onClose, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
            Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
                Text(if (isPlaylist) "Arrange your films" else "Arrange your album", style = MaterialTheme.typography.headlineMedium)
                OutlinedTextField(title, { title = it }, label = { Text("Name") }, modifier = Modifier.fillMaxWidth().testTag("collection.name"))
                if (!isPlaylist) {
                    OutlinedTextField(caption, { caption = it }, label = { Text("Album note") },
                        modifier = Modifier.fillMaxWidth().testTag("album.caption"), minLines = 2, maxLines = 4)
                    if (coverId != null) TextButton(onClick = { coverId = null }) { Text("Use first memory as cover") }
                }
                Text("Watch order".takeIf { isPlaylist } ?: "Album order", style = MaterialTheme.typography.titleMedium)
                ids.toList().forEachIndexed { index, id ->
                    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(3.dp)) {
                        Text("${index + 1}. ${byId[id]?.optString("title")?.takeIf { isPlaylist } ?: byId[id]?.optString("filename") ?: "Unavailable item"}${if (!isPlaylist && coverId == id) " · Cover" else ""}")
                        Row(horizontalArrangement = Arrangement.spacedBy(7.dp)) {
                        if (!isPlaylist) TextButton(onClick = { coverId = id }, enabled = coverId != id,
                            modifier = Modifier.testTag("album.cover.$id")) { Text("Make cover") }
                        TextButton(onClick = { ids.add(index - 1, ids.removeAt(index)) }, enabled = index > 0,
                            modifier = Modifier.semantics { contentDescription = "Move earlier" }) { Text("↑") }
                        TextButton(onClick = { ids.add(index + 1, ids.removeAt(index)) }, enabled = index + 1 < ids.size,
                            modifier = Modifier.semantics { contentDescription = "Move later" }) { Text("↓") }
                        TextButton(onClick = { ids.removeAt(index) }) { Text("Remove") }
                        }
                    }
                }
                Text(if (isPlaylist) "Add published films" else "Add memories", style = MaterialTheme.typography.titleMedium)
                for (candidate in candidates.filter { it.optString("id") !in ids }) {
            TextButton(
                onClick = { ids.add(candidate.getString("id")) },
                modifier = Modifier.fillMaxWidth().testTag(
                    "${if (isPlaylist) "playlist" else "album"}.add.${candidate.getString("id")}",
                ),
            ) {
                        Text("+ ${if (isPlaylist) candidate.optString("title") else candidate.optString("caption").ifEmpty { candidate.optString("filename") }}")
                    }
                }
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Button(onClick = { scope.launch {
                        val array = JSONArray(); ids.forEach { array.put(it) }
                        val request = JSONObject().put("action", if (isPlaylist) "save_playlist" else "save_album")
                            .put("room_id", room.getString("id")).put("id", collection.getString("id"))
                            .put("title", title).put(field, array).put("pinned", collection.optBoolean("pinned"))
                        if (!isPlaylist) request.put("caption", caption)
                            .put("cover_asset_id", coverId?.takeIf { it in ids } ?: JSONObject.NULL)
                        if (model.execute(request)) onClose()
                    } }, enabled = title.isNotBlank() && (isPlaylist || caption.toByteArray(Charsets.UTF_8).size <= 2_000),
                        modifier = Modifier.testTag("collection.save")) { Text("Save") }
                    if (isPlaylist) TextButton(onClick = { confirmDelete = true }) { Text("Delete playlist") }
                    TextButton(onClick = onClose) { Text("Close") }
                }
            }
        }
    }
    if (confirmDelete) AlertDialog(onDismissRequest = { confirmDelete = false },
        title = { Text("Delete this playlist?") },
        text = { Text("The films and their originals stay in the Room.") },
        confirmButton = { TextButton(onClick = { scope.launch {
            if (model.execute(JSONObject().put("action", "delete_playlist")
                .put("room_id", room.getString("id")).put("id", collection.getString("id")))) onClose()
        } }) { Text("Delete playlist") } },
        dismissButton = { TextButton(onClick = { confirmDelete = false }) { Text("Cancel") } })
}
