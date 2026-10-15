package com.familyroom.app

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.google.mlkit.vision.face.FaceDetector
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.UUID

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun FaceReviewSheet(model: RoomModel, roomId: String, onClose: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val room = model.rooms.find { it.optString("id") == roomId }
    val photos = room?.rows("assets").orEmpty().filter {
        it.optString("kind") == "photo" && !it.optBoolean("deleted")
    }
    val localPhotos = photos.filter { it.optBoolean("local") }
    val people = room?.rows("people").orEmpty()
    val personKeys = people.map { it.optString("id") }.toSet()
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    var scanRequested by remember(roomId) { mutableStateOf(false) }
    var scanning by remember(roomId) { mutableStateOf(false) }
    var progress by remember(roomId) { mutableStateOf("") }
    var error by remember(roomId) { mutableStateOf("") }
    var crops by remember(roomId) { mutableStateOf(emptyList<FaceCrop>()) }
    var taggedCropIds by remember(roomId) { mutableStateOf(emptySet<String>()) }
    var selectedPersonId by remember(roomId) { mutableStateOf(people.firstOrNull()?.optString("id").orEmpty()) }
    var newPersonName by remember(roomId) { mutableStateOf("") }
    var personMenuExpanded by remember(roomId) { mutableStateOf(false) }

    LaunchedEffect(personKeys) {
        if (selectedPersonId !in personKeys) selectedPersonId = people.firstOrNull()?.optString("id").orEmpty()
    }

    LaunchedEffect(scanRequested, roomId) {
        if (!scanRequested) return@LaunchedEffect
        scanning = true
        progress = "Preparing a local face review…"
        error = ""
        crops = emptyList()
        taggedCropIds = emptySet()
        var completed = 0
        var failures = 0
        var stoppedAtLimit = false
        var detector: FaceDetector? = null
        try {
            val activeDetector = FaceReviewAnalyzer.createDetector()
            detector = activeDetector
            for ((index, asset) in localPhotos.withIndex()) {
                if (crops.size >= FaceReviewAnalyzer.MAX_REVIEW_CROPS) {
                    stoppedAtLimit = true
                    break
                }
                val assetId = asset.optString("id")
                val temporary = File(context.cacheDir, "face-review-${UUID.randomUUID()}")
                progress = "Checking saved photo ${index + 1} of ${localPhotos.size}…"
                try {
                    withContext(Dispatchers.IO) {
                        model.command(
                            JSONObject()
                                .put("action", "materialize")
                                .put("room_id", roomId)
                                .put("asset_id", assetId)
                                .put("destination", temporary.absolutePath),
                        )
                    }
                    val found = FaceReviewAnalyzer.review(temporary, assetId, activeDetector)
                    val remaining = FaceReviewAnalyzer.MAX_REVIEW_CROPS - crops.size
                    crops = crops + found.take(remaining)
                    if (found.size > remaining) stoppedAtLimit = true
                    completed += 1
                    if (stoppedAtLimit) break
                } catch (cancelled: CancellationException) {
                    throw cancelled
                } catch (failure: Exception) {
                    failures += 1
                } finally {
                    temporary.delete()
                }
            }
            progress = buildString {
                append("Found ${crops.size} face crops in $completed readable local photos. Review each crop before adding an ordinary Person tag.")
                if (stoppedAtLimit) append(" Showing the first ${FaceReviewAnalyzer.MAX_REVIEW_CROPS}; run another pass after reviewing them.")
            }
            if (failures > 0) error = "$failures local photo${if (failures == 1) "" else "s"} could not be analyzed."
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (failure: Exception) {
            error = failure.message ?: "The local face review could not be completed."
        } finally {
            detector?.close()
            scanning = false
            scanRequested = false
        }
    }

    ModalBottomSheet(
        onDismissRequest = onClose,
        sheetState = sheetState,
        containerColor = MaterialTheme.colorScheme.surface,
        modifier = Modifier.testTag("people.face_review.sheet"),
    ) {
        LazyColumn(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = 380.dp, max = 780.dp)
                .testTag("people.face_review.content"),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 20.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            item {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        "A careful second look",
                        style = MaterialTheme.typography.headlineMedium.copy(fontFamily = RoomDisplay),
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Text(
                        "Review face crops from photos already saved on this device. You choose who each photo belongs to; names and identities are never guessed.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        "Face crops stay temporary. There is no cross-photo grouping or stored face template. Confirming a crop adds a Person tag to its whole photo.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            item {
                ElevatedCard(modifier = Modifier.fillMaxWidth()) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        Text("On this device", style = MaterialTheme.typography.titleSmall)
                        Text(
                            "${localPhotos.size} of ${photos.size} Room photos are saved here. Cloud-only originals are skipped.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Button(
                            onClick = { scanRequested = true },
                            enabled = !scanning && localPhotos.isNotEmpty(),
                            modifier = Modifier.testTag("people.face_review.scan"),
                        ) {
                            Text(if (scanning) "Scanning…" else "Scan saved photos")
                        }
                        if (progress.isNotEmpty()) {
                            Text(progress, style = MaterialTheme.typography.bodySmall)
                        }
                        if (error.isNotEmpty()) {
                            Text(error, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                        }
                    }
                }
            }
            if (localPhotos.isEmpty()) {
                item {
                    Text(
                        "No photos are saved on this device yet. Add or download a photo to review faces here.",
                        modifier = Modifier.testTag("people.face_review.no_local_photos"),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            if (crops.isNotEmpty()) {
                item {
                    FaceReviewPersonPicker(
                        people = people,
                        selectedPersonId = selectedPersonId,
                        newPersonName = newPersonName,
                        personMenuExpanded = personMenuExpanded,
                        onSelectedPerson = { selectedPersonId = it },
                        onNewPersonName = { newPersonName = it },
                        onPersonMenuExpanded = { personMenuExpanded = it },
                        onCreatePerson = {
                            scope.launch {
                                try {
                                    val created = model.command(
                                        JSONObject()
                                            .put("action", "save_person")
                                            .put("room_id", roomId)
                                            .put("name", newPersonName.trim()),
                                    )
                                    model.reload()
                                    selectedPersonId = created.optString("id")
                                    newPersonName = ""
                                } catch (failure: Exception) {
                                    error = failure.message ?: "Person could not be added."
                                }
                            }
                        },
                    )
                }
                items(crops.filterNot { it.id in taggedCropIds }, key = { it.id }) { crop ->
                    val asset = photos.firstOrNull { it.optString("id") == crop.assetId }
                    FaceCropCard(
                        crop = crop,
                        title = asset?.optString("caption")?.takeIf(String::isNotBlank)
                            ?: asset?.optString("filename")?.takeIf(String::isNotBlank)
                            ?: "Room photo",
                        canTag = selectedPersonId.isNotBlank(),
                        onTag = {
                            if (asset == null || selectedPersonId.isBlank()) return@FaceCropCard
                            scope.launch {
                                val peopleIds = asset.peopleIds() + selectedPersonId
                                val payload = JSONArray().apply { peopleIds.sorted().forEach(::put) }
                                val saved = model.execute(
                                    JSONObject()
                                        .put("action", "patch_asset")
                                        .put("room_id", roomId)
                                        .put("asset_id", crop.assetId)
                                        .put("fields", JSONObject().put("people", payload)),
                                )
                                if (saved) taggedCropIds = taggedCropIds + crop.id
                            }
                        },
                    )
                }
            }
            item {
                TextButton(onClick = onClose, modifier = Modifier.testTag("people.face_review.done")) {
                    Text("Done")
                }
            }
        }
    }
}

@Composable
private fun FaceReviewPersonPicker(
    people: List<JSONObject>,
    selectedPersonId: String,
    newPersonName: String,
    personMenuExpanded: Boolean,
    onSelectedPerson: (String) -> Unit,
    onNewPersonName: (String) -> Unit,
    onPersonMenuExpanded: (Boolean) -> Unit,
    onCreatePerson: () -> Unit,
) {
    val selectedName = people.firstOrNull { it.optString("id") == selectedPersonId }
        ?.optString("name")
        ?.ifBlank { "Unnamed person" }
    ElevatedCard(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text("Choose a Person", style = MaterialTheme.typography.titleSmall)
            if (selectedName != null) {
                androidx.compose.foundation.layout.Box {
                    OutlinedButton(onClick = { onPersonMenuExpanded(true) }) {
                        Text(selectedName)
                    }
                    DropdownMenu(
                        expanded = personMenuExpanded,
                        onDismissRequest = { onPersonMenuExpanded(false) },
                    ) {
                        for (person in people) {
                            DropdownMenuItem(
                                text = { Text(person.optString("name").ifBlank { "Unnamed person" }) },
                                onClick = {
                                    onSelectedPerson(person.optString("id"))
                                    onPersonMenuExpanded(false)
                                },
                            )
                        }
                    }
                }
            } else {
                Text(
                    "Create a Person first. A Person is someone shown in a photo, not a Room member.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                OutlinedTextField(
                    value = newPersonName,
                    onValueChange = onNewPersonName,
                    label = { Text("New Person’s name") },
                    singleLine = true,
                    modifier = Modifier.weight(1f).testTag("people.face_review.new_person_name"),
                )
                Button(
                    onClick = onCreatePerson,
                    enabled = newPersonName.isNotBlank(),
                    modifier = Modifier.testTag("people.face_review.create_person"),
                ) {
                    Text("Add")
                }
            }
        }
    }
}

@Composable
private fun FaceCropCard(crop: FaceCrop, title: String, canTag: Boolean, onTag: () -> Unit) {
    val bitmap by produceState<Bitmap?>(initialValue = null, key1 = crop.id) {
        value = withContext(Dispatchers.IO) {
            BitmapFactory.decodeByteArray(crop.jpeg, 0, crop.jpeg.size)
        }
    }
    DisposableEffect(bitmap) {
        onDispose { bitmap?.recycle() }
    }
    Card(
        modifier = Modifier.fillMaxWidth().testTag("people.face_review.crop.${crop.id}"),
        shape = RoundedCornerShape(18.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(12.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (bitmap != null) {
                Image(
                    bitmap = bitmap!!.asImageBitmap(),
                    contentDescription = "Temporary face crop from $title",
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.size(82.dp, 96.dp),
                )
            }
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(title, style = MaterialTheme.typography.titleSmall, maxLines = 2)
                Text(
                    "Review this crop, then tag the whole photo if the Person is present.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Button(onClick = onTag, enabled = canTag, modifier = Modifier.testTag("people.face_review.tag.${crop.id}")) {
                    Text("Tag photo")
                }
            }
        }
    }
}
