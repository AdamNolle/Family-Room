package com.familyroom.app

import android.app.Application
import android.content.Intent
import android.net.Uri
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.os.Bundle
import android.provider.OpenableColumns
import android.provider.Settings
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontFamily
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.media.MediaMetadataRetriever
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyGridScope
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.FileProvider
import androidx.core.view.WindowCompat
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.UUID

class RoomModel(application: Application) : AndroidViewModel(application) {
    private val app = application as FamilyRoomApplication
    var snapshot by mutableStateOf(JSONObject()); private set
    var roomId by mutableStateOf("")
    var status by mutableStateOf("Opening your private library…")
    var openingFailed by mutableStateOf(false); private set
    private var vault: Vault? = null
    private var initialized by mutableStateOf(false)
    private var appInForeground = true
    private var foregroundSyncJob: Job? = null
    private var networkCallbackRegistered = false
    private var observedNetworkState: String? = null
    private val connectivity = application.getSystemService(ConnectivityManager::class.java)
    private val networkCallback = object : ConnectivityManager.NetworkCallback() {
        override fun onAvailable(network: Network) = networkChanged()
        override fun onCapabilitiesChanged(network: Network, capabilities: NetworkCapabilities) = networkChanged()
        override fun onLost(network: Network) = networkChanged()
    }
    private fun networkChanged() { viewModelScope.launch {
        val state = networkState()
        if (state != observedNetworkState) {
            observedNetworkState = state
            if (appInForeground && ready && roomId.isNotEmpty()) sync()
        }
    } }
    private fun networkState(): String {
        val active = connectivity.activeNetwork ?: return "offline"
        val capabilities = connectivity.getNetworkCapabilities(active) ?: return "unknown"
        return if (capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED)) "unmetered" else "metered"
    }
    val ready get() = initialized
    val rooms get() = snapshot.rows("rooms")
    val room get() = rooms.find { it.optString("id") == roomId }
    val studio = FilmSession(this, viewModelScope, application)
    init { viewModelScope.launch {
        try {
            vault = app.openVault()
            connectivity.registerDefaultNetworkCallback(networkCallback)
            networkCallbackRegistered = true
            reload(); status = "Your private library is ready."
            initialized = true
            if (roomId.isNotEmpty()) sync()
            startForegroundSync()
        } catch (e: Exception) { status = e.message ?: "Cannot open library."; openingFailed = true }
    } }
    fun recoveryKeyForBackup(): String =
        vault?.recoveryKeyForBackup() ?: error("Open the library before creating a backup.")

    suspend fun command(request: JSONObject): JSONObject {
        val opened = vault ?: error("Library is locked.")
        if (request.optString("action") in setOf("sync_all", "replicate", "replicate_step", "retry_transfer", "fetch")) {
            opened.command(JSONObject().put("action", "set_network_state").put("state", networkState()))
        }
        return opened.command(request)
    }
    suspend fun reload() {
        snapshot = command(JSONObject().put("action", "snapshot"))
        if (rooms.none { it.optString("id") == roomId }) roomId = rooms.firstOrNull()?.optString("id") ?: ""
    }
    suspend fun execute(request: JSONObject): Boolean {
        try {
            val result = command(request)
            if (request.optString("action") in listOf("create_room", "join", "accept_invitation")) roomId = result.optString("id", roomId)
            reload(); status = "Saved privately."; return true
        } catch (e: Exception) { status = e.message ?: "The operation failed."; return false }
    }
    suspend fun sync(destination: String = roomId) {
        try {
            val result = command(JSONObject().put("action", "sync_all").put("room_id", destination))
            reload()
 val errors = result.optJSONArray("errors")
 val cacheCleanup = result.optJSONObject("cache_cleanup")
            status = if (rooms.find { it.optString("id") == destination }?.optString("role") == "removed") "This device has been removed from the Room."
 else if (errors != null && errors.length() > 0) errors.toString()
 else if (cacheCleanup?.optBoolean("enabled") == true && !cacheCleanup.optBoolean("complete")) "Room synchronized. Automatic cache cleanup is waiting for another verified copy; originals remain saved here."
                else "Room synchronized."
        } catch (e: Exception) { status = e.message ?: "Waiting for a connection." }
    }
    fun onBackground() {
        if (!appInForeground) return
        appInForeground = false
        foregroundSyncJob?.cancel()
        foregroundSyncJob = null
        BackgroundSyncScheduler.enqueueNow(app)
    }
    fun onForeground() {
        if (appInForeground) return
        appInForeground = true
        viewModelScope.launch {
            if (!ready) return@launch
            try { reload(); sync() } catch (e: Exception) { status = e.message ?: "Waiting for a connection." }
            startForegroundSync()
        }
    }
    private fun startForegroundSync() {
        if (!appInForeground || foregroundSyncJob?.isActive == true) return
        foregroundSyncJob = viewModelScope.launch {
            while (isActive) {
                if (roomId.isNotEmpty()) sync()
                delay(30_000)
            }
        }
    }
    override fun onCleared() {
        foregroundSyncJob?.cancel()
        if (networkCallbackRegistered) connectivity.unregisterNetworkCallback(networkCallback)
        studio.dispose()
    }
}
private fun LazyGridScope.fullWidthItem(content: @Composable ColumnScope.() -> Unit) {
    item(span = { GridItemSpan(maxLineSpan) }) {
        Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp), content = content)
    }
}

internal fun JSONObject.rows(field: String): List<JSONObject> {
    val items = optJSONArray(field) ?: return emptyList()
    return (0 until items.length()).map { items.getJSONObject(it) }
}

class MainActivity : ComponentActivity() {
    private var roomModel: RoomModel? = null
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        cacheDir.listFiles()?.filter { file -> file.isFile &&
                    (file.name.startsWith("video-index-") || file.name.startsWith("video-play-") || file.name.startsWith("face-review-") || file.name.startsWith("family-room-backup-") || file.name.startsWith("family-room-restore-"))
        }?.forEach { it.delete() }
        val model = ViewModelProvider(this)[RoomModel::class.java]
        roomModel = model
        setContent { FamilyRoom(model) }
    }
    override fun onStart() {
        super.onStart()
        roomModel?.onForeground()
    }
    override fun onStop() {
        if (!isChangingConfigurations) roomModel?.onBackground()
        super.onStop()
    }
    @OptIn(ExperimentalMaterial3Api::class)
    @Composable private fun FamilyRoom(model: RoomModel) {
        val scope = rememberCoroutineScope()
        val context = LocalContext.current
        val wide = LocalConfiguration.current.screenWidthDp >= 840
        val sections = listOf("Home", "Library", "Albums", "People", "Theater")
        var section by rememberSaveable { mutableStateOf("Home") }
        var settings by rememberSaveable { mutableStateOf(false) }
        var roomMenu by remember { mutableStateOf(false) }
    var searching by rememberSaveable { mutableStateOf(false) }
var videoFinder by rememberSaveable { mutableStateOf(false) }
var faceReview by rememberSaveable { mutableStateOf(false) }
var mergingPeople by rememberSaveable { mutableStateOf(false) }
    var selectingMedia by remember(model.roomId) { mutableStateOf(false) }
        var selectedMediaIds by remember(model.roomId) { mutableStateOf(emptySet<String>()) }
        var createSelectedAlbum by remember(model.roomId) { mutableStateOf(false) }
        var selectedAlbumTitle by remember(model.roomId) { mutableStateOf("") }
        var correctCaptureDate by remember(model.roomId) { mutableStateOf(false) }
        var showingRecentlyDeleted by remember(model.roomId) { mutableStateOf(false) }
        var confirmBulkDelete by remember(model.roomId) { mutableStateOf(false) }
    BackHandler(enabled = settings || searching || videoFinder || selectingMedia) {
        if (videoFinder) videoFinder = false
        else if (settings) settings = false
        else if (selectingMedia) { selectingMedia = false; selectedMediaIds = emptySet() }
        else searching = false
        }
        var name by remember { mutableStateOf("") }
        var personName by remember { mutableStateOf("") }
var invitation by remember { mutableStateOf("") }
var invitationPreview by remember { mutableStateOf<JSONObject?>(null) }
 var invitationReviewedToken by remember { mutableStateOf("") }
        var search by remember { mutableStateOf("") }
        var selectedPeople by remember(model.roomId) { mutableStateOf(emptySet<String>()) }
        var requireAllPeople by remember(model.roomId) { mutableStateOf(false) }
        var mediaKindFilter by remember(model.roomId) { mutableStateOf("all") }
 var selectedYearFilter by remember(model.roomId) { mutableStateOf(0) }
  var selectedMomentFilter by remember(model.roomId) { mutableStateOf("") }
  var photoDuplicateResponse by remember(model.roomId) { mutableStateOf<JSONObject?>(null) }
  var scanningPhotoDuplicates by remember(model.roomId) { mutableStateOf(false) }
        var gateway by remember { mutableStateOf("") }
        var device by remember { mutableStateOf("") }
        var inviteRole by remember { mutableStateOf("contributor") }
        var generated by remember { mutableStateOf("") }
        var ownershipTarget by remember(model.roomId) { mutableStateOf("") }
        var generatedOwnershipOffer by remember(model.roomId) { mutableStateOf("") }
        var ownershipOfferInput by remember(model.roomId) { mutableStateOf("") }
        var confirmOwnershipAcceptance by remember { mutableStateOf(false) }
        var pendingRemoval by remember { mutableStateOf<JSONObject?>(null) }
        var importRoom by rememberSaveable { mutableStateOf("") }
    var pendingMovieExport by rememberSaveable { mutableStateOf("") }
    var screening by remember { mutableStateOf<Pair<JSONObject, JSONObject>?>(null) }
 var screeningQueue by remember { mutableStateOf<List<Pair<JSONObject, JSONObject>>?>(null) }
 var screeningPlaylistTitle by remember { mutableStateOf<String?>(null) }
 var momentSuggestions by remember(model.roomId) { mutableStateOf(emptyList<JSONObject>()) }
        val movieExporter = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("video/mp4")) { uri ->
            val source = pendingMovieExport.takeIf(String::isNotEmpty)?.let(::File)
            pendingMovieExport = ""
            if (uri != null && source != null) scope.launch {
                try {
                    withContext(Dispatchers.IO) {
                        contentResolver.openOutputStream(uri)?.use { output -> source.inputStream().use { it.copyTo(output) } }
                            ?: error("This destination cannot accept the movie.")
                    }
                    model.status = "Movie exported. Your originals are preserved."
                } catch (e: kotlinx.coroutines.CancellationException) { throw e }
                catch (e: Exception) {
                    val message = e.message ?: "The movie could not be exported. Render it again."
                    model.status = message; model.studio.reportError(message)
                }
            }
        }
        var preservedPath by rememberSaveable { mutableStateOf("") }
        var preservedSummary by rememberSaveable { mutableStateOf("") }
        var exportingPreserved by remember { mutableStateOf(false) }
        val savePreserved = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/zip")) { uri ->
            val preparedPath = preservedPath
            val summary = preservedSummary
            preservedPath = ""
            scope.launch {
                val file = File(preparedPath)
                try {
                    if (uri != null && preparedPath.isNotEmpty()) {
                        withContext(Dispatchers.IO) {
                            contentResolver.openOutputStream(uri)?.use { output ->
                                file.inputStream().use { it.copyTo(output) }
                            } ?: error("Cannot write to the selected destination.")
                        }
                        model.status = summary
                    }
                } catch (e: Exception) {
                    model.status = e.message ?: "Cannot save preserved work."
                } finally {
                    if (preparedPath.isNotEmpty()) withContext(Dispatchers.IO) { file.delete() }
                    exportingPreserved = false
                }
            }
        }
        val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
            val destination = importRoom
            scope.launch {
                for (uri in uris) {
                    var original: File? = null
                    try {
                        original = withContext(Dispatchers.IO) { copyImport(uri) }
                        model.command(JSONObject().put("action", "import").put("room_id", destination).put("path", original.absolutePath))
                    } catch (e: Exception) { model.status = e.message ?: "Import failed." }
                    finally { original?.parentFile?.deleteRecursively() }
                }
                model.reload(); model.sync(destination)
            }
        }
 LaunchedEffect(model.roomId) { search = ""; generated = ""; device = ""; personName = ""; mediaKindFilter = "all"; selectedYearFilter = 0; selectedMomentFilter = ""; videoFinder = false; showingRecentlyDeleted = false; confirmBulkDelete = false; correctCaptureDate = false; selectingMedia = false; selectedMediaIds = emptySet() }
        val room = model.room
        val field = when(section) { "Albums" -> "albums"; "People" -> "people"; "Theater" -> "stories"; else -> "assets" }
        val people = room?.rows("people").orEmpty()
val allAssets = room?.rows("assets").orEmpty().filter { it.optBoolean("deleted") == showingRecentlyDeleted }
val onThisDayIds = room?.optJSONArray("on_this_day_asset_ids")?.stringItems()?.toSet().orEmpty()
val onThisDayAssets = allAssets.filter { it.optString("id") in onThisDayIds }
val captureYears = availableCaptureYears(allAssets.map { it.optLong("captured_at") })
 val momentChoices = room?.rows("moments").orEmpty().map { it.optString("id") to it.optString("title", "Moment") }
 LaunchedEffect(model.roomId, momentChoices.map { it.first }) {
 if (selectedMomentFilter.isNotBlank() && momentChoices.none { it.first == selectedMomentFilter }) {
 selectedMomentFilter = ""
 }
 }
 val selectedMomentAssetIds = room?.rows("moments").orEmpty()
  .firstOrNull { it.optString("id") == selectedMomentFilter }
  ?.optJSONArray("asset_ids").stringItems().toSet().orEmpty()
 LaunchedEffect(model.roomId, room?.rows("moments")?.size, room?.optString("role")) {
  if (room?.optString("role") in listOf("owner", "admin", "contributor")) {
   momentSuggestions = try {
    model.command(JSONObject().put("action", "suggest_moments").put("room_id", model.roomId))
     .rows("suggestions")
   } catch (_: Exception) {
    emptyList()
   }
  } else {
   momentSuggestions = emptyList()
  }
 }
        val rows = room?.rows(field).orEmpty()
        val latest = if (field == "stories") linkedMapOf<String, JSONObject>().apply {
            for (story in rows) put(story.optString("id") + ":" + story.optBoolean("private_draft"), story)
        }.values.toList() else rows
        val personNames = people.associate { it.optString("id") to it.optString("name") }
        val items = latest.filter { item ->
            val taggedNames = item.peopleIds().mapNotNull(personNames::get)
            val matchesSearch = search.isBlank() ||
                listOf("caption", "filename", "title", "name").any { key -> item.optString(key).contains(search, true) } ||
                taggedNames.any { it.contains(search, true) }
            val matchesPeople = showingRecentlyDeleted || section != "Library" ||
                matchesPeopleSelection(item.peopleIds(), selectedPeople, requireAllPeople)
 val matchesScope = showingRecentlyDeleted || section != "Library" ||
 (matchesLibraryScope(item.optString("kind"), item.optLong("captured_at"), mediaKindFilter, selectedYearFilter) &&
  matchesMomentSelection(item.optString("id"), selectedMomentFilter, selectedMomentAssetIds))
            item.optBoolean("deleted") == showingRecentlyDeleted && matchesSearch && matchesPeople && matchesScope
        }
        val hasActiveLibraryFilters = section == "Library" &&
 (mediaKindFilter != "all" || selectedYearFilter != 0 || selectedMomentFilter.isNotBlank() || selectedPeople.isNotEmpty())
        val hasActiveFilters = search.isNotBlank() || hasActiveLibraryFilters
        val dark = isSystemInDarkTheme()
        DisposableEffect(dark) {
            val controller = WindowCompat.getInsetsController(window, window.decorView)
            controller.isAppearanceLightStatusBars = !dark
            controller.isAppearanceLightNavigationBars = !dark
            onDispose { }
        }
    val colors = roomColorScheme(dark)
        MaterialTheme(colorScheme = colors) {
            if (!model.ready) {
                Box(Modifier.fillMaxSize().background(colors.background),
                    contentAlignment = androidx.compose.ui.Alignment.Center) {
                    Column(horizontalAlignment = androidx.compose.ui.Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        if (model.openingFailed) Text("Library unavailable", style = MaterialTheme.typography.titleMedium)
                        else CircularProgressIndicator()
                        Text(model.status, color = colors.onBackground)
                    }
                }
            } else if (model.studio.project != null) {
            FilmStudio(model.studio, thumbnail = { MemoryThumbnail(model, it) }, openOriginal = { asset -> scope.launch { openOriginal(model, asset) } }, exportMovie = { file ->
                pendingMovieExport = file.absolutePath; movieExporter.launch("Family Room movie.mp4")
            })
        } else {
            screening?.let { (story, movie) ->
 TheaterDiscSheet(model, story, movie, onClose = { screening = null; screeningQueue = null; screeningPlaylistTitle = null },
 queue = screeningQueue ?: listOf(story to movie), playlistTitle = screeningPlaylistTitle)
 }
if (videoFinder && room != null) VideoFinderSheet(model, room.getString("id"), onClose = { videoFinder = false })
if (faceReview && room != null) FaceReviewSheet(model, room.getString("id"), onClose = { faceReview = false })
if (mergingPeople && room != null) {
    PersonMergeDialog(
        people = room.rows("people"),
        onDismiss = { mergingPeople = false },
        onMerge = { sourceId, targetId ->
            mergingPeople = false
            scope.launch {
                try {
                    model.execute(
                        JSONObject()
                            .put("action", "merge_person")
                            .put("room_id", room.getString("id"))
                            .put("source_person_id", sourceId)
                            .put("target_person_id", targetId),
                    )
                    model.reload()
                    model.sync(room.getString("id"))
                    model.status = "Duplicate Person merged. Original photos are unchanged."
                } catch (failure: Exception) {
                    model.status = failure.message ?: "Could not merge these People."
                }
            }
        },
    )
}
        if (confirmOwnershipAcceptance && room != null) {
            AlertDialog(
                onDismissRequest = { confirmOwnershipAcceptance = false },
                title = { Text("Accept Room ownership?") },
                text = { Text("This device becomes the Room owner. The current owner becomes an admin (or viewer for an album-only Room), and future contributions use a rotated key. Previously downloaded copies remain with their Members.") },
                confirmButton = {
                    TextButton(onClick = {
                        confirmOwnershipAcceptance = false
                        val targetRoom = room.getString("id")
                        val offer = ownershipOfferInput.trim()
                        scope.launch {
                            if (model.execute(JSONObject().put("action", "accept_owner_transfer")
                                    .put("room_id", targetRoom).put("offer", offer))) {
                                ownershipOfferInput = ""
                                var synchronized = false
                                for (source in model.snapshot.rows("sources").filter {
                                    it.optString("room_id") == targetRoom && !it.optBoolean("paused")
                                }) {
                                    try {
                                        model.command(JSONObject().put("action", "sync_source")
                                            .put("id", source.getString("id")))
                                        synchronized = true
                                    } catch (_: Exception) { }
                                }
                                model.reload()
                                model.status = if (synchronized) "Ownership transferred and the signed update was sent to storage."
                                    else "Ownership transferred on this device. The signed update will publish when storage reconnects."
                            }
                        }
                    }) { Text("Accept ownership") }
                },
                dismissButton = { TextButton(onClick = { confirmOwnershipAcceptance = false }) { Text("Cancel") } },
            )
        }
            pendingRemoval?.let { member ->
            val targetRoom = member.getString("room_id")
            val targetDevice = member.getString("device_id")
            AlertDialog(
                onDismissRequest = { pendingRemoval = null },
                title = { Text("Remove this device?") },
                text = { Text("Future contributions use a new Room key. Sync sends the removal to connected storage; offline devices update when they reconnect. Downloaded memories remain with the recipient.") },
                confirmButton = { TextButton(onClick = {
                    pendingRemoval = null
                    scope.launch {
                        model.execute(JSONObject().put("action","revoke_device").put("room_id",targetRoom).put("device_id",targetDevice))
                        for (source in model.snapshot.rows("sources").filter { it.optString("room_id") == targetRoom && !it.optBoolean("paused") }) {
                            model.execute(JSONObject().put("action","refresh_access").put("id",source.getString("id")))
                        }
                    }
                }) { Text("Remove device") } },
                dismissButton = { TextButton(onClick = { pendingRemoval = null }) { Text("Cancel") } }
                )
            }
        if (createSelectedAlbum && room != null) {
                AlertDialog(
                    onDismissRequest = { createSelectedAlbum = false },
                    title = { Text("Create album") },
                    text = {
                        Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                            Text("The selected memories stay in the Room library and are added to this album.")
                            OutlinedTextField(
                                value = selectedAlbumTitle,
                                onValueChange = { selectedAlbumTitle = it.take(120) },
                                label = { Text("Album name") },
                                singleLine = true,
                                modifier = Modifier.fillMaxWidth().testTag("library.batch.album.name"),
                            )
                        }
                    },
                    confirmButton = {
                        TextButton(
                            enabled = selectedAlbumTitle.isNotBlank() && selectedMediaIds.isNotEmpty(),
                            onClick = {
                                val ids = JSONArray().apply { selectedMediaIds.sorted().forEach { put(it) } }
                                scope.launch {
                                    val saved = model.execute(
                                        JSONObject()
                                            .put("action", "save_album")
                                            .put("room_id", room.getString("id"))
                                            .put("title", selectedAlbumTitle.trim())
                                            .put("asset_ids", ids),
                                    )
                                    if (saved) {
                                        createSelectedAlbum = false
                                        selectingMedia = false
                                        selectedMediaIds = emptySet()
                                    }
                                }
                            },
                        ) { Text("Create") }
                    },
                    dismissButton = {
                        TextButton(onClick = { createSelectedAlbum = false }) { Text("Cancel") }
                    },
            )
        }
        if (correctCaptureDate && room != null) {
            val selectedAssetsForDate = allAssets.filter { it.optString("id") in selectedMediaIds }
            val initialCaptureDate = selectedAssetsForDate.firstOrNull()?.optLong("captured_at") ?: 0L
            val pickerState = rememberDatePickerState(
                initialSelectedDateMillis = datePickerMillisForCapture(initialCaptureDate),
            )
            DatePickerDialog(
                onDismissRequest = { correctCaptureDate = false },
                confirmButton = {
                    TextButton(onClick = {
                        val selectedDateMillis = pickerState.selectedDateMillis
                        if (selectedDateMillis != null && selectedAssetsForDate.isNotEmpty()) {
                            val dates = JSONArray().apply {
                                selectedAssetsForDate.forEach { asset ->
                                    put(JSONObject()
                                        .put("asset_id", asset.optString("id"))
                                        .put("captured_at", correctedCaptureTimestamp(
                                            asset.optLong("captured_at"), selectedDateMillis,
                                        )))
                                }
                            }
                            scope.launch {
                                val saved = model.execute(
                                    JSONObject().put("action", "patch_asset_dates")
                                        .put("room_id", room.getString("id"))
                                        .put("dates", dates),
                                )
                                if (saved) { selectedMediaIds = emptySet(); selectingMedia = false }
                            }
                        }
                        correctCaptureDate = false
                    }) { Text("Apply") }
                },
                dismissButton = {
                    TextButton(onClick = { correctCaptureDate = false }) { Text("Cancel") }
                },
            ) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Times of day and the order between memories are preserved.")
                    DatePicker(state = pickerState)
                }
            }
        }
        if (confirmBulkDelete && room != null) {
            AlertDialog(
                onDismissRequest = { confirmBulkDelete = false },
                title = { Text("Move to Recently Deleted?") },
                text = { Text("${selectedMediaIds.size} memories will leave the Library. Their original files stay saved and can be restored later.") },
                confirmButton = {
                    TextButton(
                        onClick = {
                            confirmBulkDelete = false
                            val ids = JSONArray().apply { selectedMediaIds.sorted().forEach { put(it) } }
                            scope.launch {
                                val moved = model.execute(
                                    JSONObject().put("action", "patch_assets")
                                        .put("room_id", room.getString("id"))
                                        .put("asset_ids", ids)
                                        .put("fields", JSONObject().put("deleted", true)),
                                )
                                if (moved) { selectedMediaIds = emptySet(); selectingMedia = false }
                            }
                        },
                        colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                    ) { Text("Move to Recently Deleted") }
                },
                dismissButton = { TextButton(onClick = { confirmBulkDelete = false }) { Text("Cancel") } },
            )
        }
        Scaffold(
                topBar = { if (room != null) {
                    Row(Modifier.fillMaxWidth().statusBarsPadding().padding(horizontal = 16.dp, vertical = 8.dp), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                        Box(Modifier.weight(1f)) {
                            TextButton(onClick = { roomMenu = true }, modifier = Modifier.testTag("room.switcher")) {
                                Text(room.optString("name"), maxLines = 1, overflow = TextOverflow.Ellipsis)
                                Text(" ▾", color = colors.onSurfaceVariant)
                            }
                            DropdownMenu(expanded = roomMenu, onDismissRequest = { roomMenu = false }) {
                                for (r in model.rooms) DropdownMenuItem(text = { Text(r.getString("name")) }, onClick = {
 model.roomId = r.getString("id"); roomMenu = false; settings = false; videoFinder = false
                                })
                                HorizontalDivider()
                                DropdownMenuItem(text = { Text("Create or join a Room") }, onClick = { roomMenu = false; settings = true })
                            }
                        }
                        if (settings) TextButton(onClick = { settings = false }, modifier = Modifier.testTag("settings.done")) { Text("Done") }
                        else {
                            IconButton(onClick = { searching = !searching }, modifier = Modifier.testTag("memory.search")) {
                                Icon(painterResource(R.drawable.ic_search), contentDescription = "Search memories")
                            }
                            IconButton(onClick = { settings = true }, modifier = Modifier.testTag("room.settings")) {
                                Icon(painterResource(R.drawable.ic_settings), contentDescription = "Room settings")
                            }
                        }
                    }
                } },
                floatingActionButton = { if (room != null && !settings && room.optString("role") in listOf("owner", "admin", "contributor")) {
                    ExtendedFloatingActionButton(onClick = { importRoom = room.getString("id"); picker.launch(arrayOf("image/*", "video/*", "audio/*")) },
                        modifier = Modifier.testTag("memory.add"), containerColor = colors.primary, contentColor = colors.onPrimary) {
                        Text("＋ Add memories")
                    }
                } },
                bottomBar = { if (!wide && room != null && !settings) NavigationBar(containerColor = colors.surfaceVariant) {
                for (label in sections) {
                    NavigationBarItem(selected = section == label, onClick = { section = label; search = ""; if (label != "Library") { showingRecentlyDeleted = false; selectingMedia = false; selectedMediaIds = emptySet() } }, modifier = Modifier.testTag("section.$label"), icon = { SectionIcon(label, selected = section == label) }, label = { Text(label) })
                }
            } }) { padding ->
                val onGallery = !settings && (room == null || section in listOf("Home", "Library", "Theater"))
                val gallery = roomGalleryBackground()
                Row(Modifier.padding(padding).fillMaxSize().then(if (onGallery) Modifier.background(gallery) else Modifier.background(colors.background))) {
                    if (wide && room != null && !settings) NavigationRail {
                        Spacer(Modifier.height(24.dp))
                        for (label in sections) NavigationRailItem(
                            selected = section == label, onClick = { section = label; search = ""; if (label != "Library") { showingRecentlyDeleted = false; selectingMedia = false; selectedMediaIds = emptySet() } }, modifier = Modifier.testTag("section.$label"),
                            icon = { SectionIcon(label, selected = section == label) }, label = { Text(label) })
                    }
                    LazyVerticalGrid(columns = if (section == "Library" && !settings && room != null) GridCells.Adaptive(160.dp) else GridCells.Fixed(1),
                        modifier = Modifier.weight(1f).padding(24.dp).testTag("room.content"), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalArrangement = Arrangement.spacedBy(16.dp),
                        contentPadding = PaddingValues(bottom = if (room != null && !settings) 88.dp else 24.dp)) {
                    fullWidthItem {
                        Column(Modifier.fillMaxWidth().background(colors.surface.copy(alpha = 0.88f), RoundedCornerShape(16.dp)).border(1.dp, colors.outlineVariant, RoundedCornerShape(16.dp)).padding(22.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                            val titleInk = colors.onSurface
                            val metadataInk = colors.onSurfaceVariant
                            Box(Modifier.size(16.dp, 3.dp).background(if (dark) Color(0xFF846149) else Color(0xFFE8C1A4), RoundedCornerShape(2.dp)))
                            Text(if (room == null) "A PRIVATE HOME FOR SHARED MEMORIES" else if (settings) "YOUR ROOM" else room.optString("name").uppercase(), style = MaterialTheme.typography.labelSmall, fontFamily = FontFamily.Monospace, color = metadataInk)
                            Text(if (room == null) "A home for your memories." else if (settings) "Room settings" else when (section) { "Home" -> "Collected, together."; "Library" -> "Every memory, together."; "Theater" -> "Your Room, on film."; else -> section }, style = MaterialTheme.typography.headlineLarge.copy(fontFamily = RoomDisplay, fontSize = 40.sp), color = titleInk)
                            if (room == null) Text("Keep photos and films together, with the people you choose.", color = metadataInk)
                            else if (onGallery) {
                                val count = room.rows("assets").count { !it.optBoolean("deleted") }
                                Text("$count ${if (count == 1) "memory" else "memories"} · Private Room", style = MaterialTheme.typography.bodySmall, color = metadataInk)
                            }
                            if (model.status.isNotEmpty()) Text(model.status, style = MaterialTheme.typography.bodySmall, color = metadataInk)
                        }
                    }
                    if (room == null) fullWidthItem {
                        Column(Modifier.fillMaxWidth().background(colors.surface, RoundedCornerShape(12.dp)).padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Text("Create a Room", style = MaterialTheme.typography.titleMedium)
                        OutlinedTextField(value = name, onValueChange = { name = it }, label = { Text("Room name") }, modifier = Modifier.fillMaxWidth().testTag("room.name"))
                        Button(onClick = { scope.launch {
                            if (model.execute(JSONObject().put("action", "create_room").put("name", name))) { name = ""; settings = true }
                        } }, enabled = model.ready && name.isNotBlank(), modifier = Modifier.testTag("room.create")) { Text("Create Room") }
                        }
                    }
                    if (!settings && room != null) {
 if (searching) fullWidthItem {
 OutlinedTextField(value = search, onValueChange = { search = it }, label = { Text("Search $section") },
 modifier = Modifier.fillMaxWidth().testTag("memory.query"), singleLine = true,
 trailingIcon = { if (search.isNotEmpty()) TextButton(onClick = { search = "" }) { Text("Clear") } })
 }
if (section == "People" && room.optString("role") in listOf("owner", "admin", "contributor")) fullWidthItem {
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        OutlinedTextField(
                            value = personName,
                            onValueChange = { personName = it },
                            label = { Text("Name someone in your memories") },
                            singleLine = true,
                            modifier = Modifier.weight(1f).testTag("people.create.name"),
                        )
                        Button(
                            onClick = {
                                scope.launch {
                                    if (model.execute(
                                        JSONObject().put("action", "save_person")
                                            .put("room_id", room.getString("id"))
                                            .put("name", personName.trim()),
                                    )) personName = ""
                                }
                            },
                            enabled = personName.isNotBlank(),
                            modifier = Modifier.testTag("people.create"),
                        ) {
                            Text("Add")
                        }
                    }
            if (room.rows("people").size > 1) {
                OutlinedButton(
                    onClick = { mergingPeople = true },
                    modifier = Modifier.fillMaxWidth().testTag("people.merge.open"),
                ) {
                    Text("Merge duplicate People")
                }
            }
                    ElevatedCard(
                        onClick = { faceReview = true },
                        modifier = Modifier.fillMaxWidth().testTag("people.face_review.open"),
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth().padding(16.dp),
                            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            Icon(painterResource(R.drawable.ic_people), contentDescription = null, tint = colors.primary)
                            Column(modifier = Modifier.weight(1f)) {
                                Text("Review face crops", style = MaterialTheme.typography.titleSmall)
                                Text(
                                    "On-device only · names are never guessed",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = colors.onSurfaceVariant,
                                )
                            }
                            Text("↗", color = colors.primary)
                        }
                    }
                }
            }
                if (section == "Library") fullWidthItem {
                    val deletedCount = room.rows("assets").count { it.optBoolean("deleted") }
                    OutlinedButton(
                        onClick = {
                            showingRecentlyDeleted = !showingRecentlyDeleted
                            selectingMedia = false
                            selectedMediaIds = emptySet()
                            selectedPeople = emptySet()
                            requireAllPeople = false
   mediaKindFilter = "all"
   selectedYearFilter = 0
   selectedMomentFilter = ""
                        },
                        modifier = Modifier.fillMaxWidth().testTag("library.recently_deleted"),
                    ) {
                        Text(if (showingRecentlyDeleted) "Back to Library" else "Recently Deleted · $deletedCount")
                    }
                    if (showingRecentlyDeleted) {
                        Text(
                            if (deletedCount == 0) "Recently Deleted is empty. Your originals are safe."
                            else "Deleted memories stay here until restored. Their original files remain saved.",
                            style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceVariant,
                        )
                    }
                }
      if (section == "Library" && !showingRecentlyDeleted) fullWidthItem {
        LibraryScopeFilterBar(
                        selectedKind = mediaKindFilter,
                    onKindChange = { mediaKindFilter = it },
                    selectedYear = selectedYearFilter,
                        years = captureYears,
                        onYearChange = { selectedYearFilter = it },
                        selectedMomentId = selectedMomentFilter,
                        moments = momentChoices,
                        onMomentChange = { selectedMomentFilter = it },
        )
      }
      if (section == "Library" && !showingRecentlyDeleted) fullWidthItem {
        PhotoDuplicateReviewPanel(
          model = model,
          response = photoDuplicateResponse,
          scanning = scanningPhotoDuplicates,
          assets = allAssets,
          onScan = {
            scope.launch {
              scanningPhotoDuplicates = true
              try {
                photoDuplicateResponse = model.command(
                  JSONObject()
                    .put("action", "suggest_photo_duplicates")
                    .put("room_id", model.roomId),
                )
              } catch (error: Exception) {
                model.status = error.message ?: "Could not compare photos."
              } finally {
                scanningPhotoDuplicates = false
              }
            }
          },
          onOpen = { asset -> scope.launch { openOriginal(model, asset) } },
        )
      }
                if (section == "Library" && room.optString("role") in listOf("owner", "admin", "contributor")) {
                    fullWidthItem {
                        if (!selectingMedia) {
                            TextButton(
                                onClick = { selectingMedia = true },
                                modifier = Modifier.testTag("library.select"),
                            ) {
                                Text("Select memories")
                            }
                        } else {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                            ) {
                                Text(
                                    if (selectedMediaIds.size == MAX_SELECTED_MEMORIES) "$MAX_SELECTED_MEMORIES selected · maximum per batch" else "${selectedMediaIds.size} selected",
                                    modifier = Modifier.weight(1f), style = MaterialTheme.typography.titleSmall,
                                )
                                TextButton(
                                    onClick = { selectedMediaIds = items.mapNotNull { it.optString("id").takeIf(String::isNotEmpty) }.take(MAX_SELECTED_MEMORIES).toSet() },
                                    enabled = selectedMediaIds.size < minOf(items.size, MAX_SELECTED_MEMORIES),
                                    modifier = Modifier.testTag("library.select.all"),
                                ) { Text("Select up to 500") }
                                TextButton(
                                    onClick = { selectingMedia = false; selectedMediaIds = emptySet() },
                                    modifier = Modifier.testTag("library.select.done"),
                                ) { Text("Done") }
                            }
                        }
                    }
                }
                if (section == "Library" && selectingMedia && selectedMediaIds.isNotEmpty()) {
                    fullWidthItem {
                        val selectedAssets = allAssets.filter { it.optString("id") in selectedMediaIds }
                        val allFavorite = selectedAssets.isNotEmpty() && selectedAssets.all { it.optBoolean("favorite") }
                        val mayChangeDeletion = canChangeDeletionForSelection(
                            room.optString("role"),
                            model.snapshot.optString("device_id"),
                            selectedAssets.map { it.optString("contributor") },
                        )
                        Column(
                            modifier = Modifier.fillMaxWidth(),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            OutlinedButton(
                                onClick = {
                                    val ids = JSONArray().apply { selectedMediaIds.sorted().forEach { put(it) } }
                                    scope.launch {
                                        val saved = model.execute(
                                            JSONObject()
                                                .put("action", "patch_assets")
                                                .put("room_id", room.getString("id"))
                                                .put("asset_ids", ids)
                                                .put("fields", JSONObject().put("favorite", !allFavorite)),
                                        )
                                        if (saved) { selectedMediaIds = emptySet(); selectingMedia = false }
                                    }
                                },
                                enabled = !showingRecentlyDeleted,
                                modifier = Modifier.testTag("library.batch.favorite"),
                            ) { Text(if (allFavorite) "Unfavorite" else "Favorite") }

                            var tagMenu by remember(model.roomId) { mutableStateOf(false) }
                            Box {
                                OutlinedButton(onClick = { tagMenu = true }, enabled = !showingRecentlyDeleted && people.isNotEmpty(), modifier = Modifier.testTag("library.batch.tag")) {
                                    Text("Tag Person")
                                }
                                DropdownMenu(expanded = tagMenu, onDismissRequest = { tagMenu = false }) {
                                    for (person in people) {
                                        val anyTagged = selectedAssets.any { person.optString("id") in it.peopleIds() }
                                        val allTagged = selectedAssets.isNotEmpty() && selectedAssets.all { person.optString("id") in it.peopleIds() }
                                        if (!allTagged) DropdownMenuItem(
                                            text = { Text("Add ${person.optString("name")}") },
                                            onClick = {
                                                tagMenu = false
                                                val ids = JSONArray().apply { selectedMediaIds.sorted().forEach { put(it) } }
                                                scope.launch {
                                                    val saved = model.execute(
                                                        JSONObject().put("action", "tag_assets")
                                                            .put("room_id", room.getString("id"))
                                                            .put("asset_ids", ids)
                                                            .put("person_id", person.getString("id")),
                                                    )
                                                    if (saved) { selectedMediaIds = emptySet(); selectingMedia = false }
                                                }
                                            },
                                        )
                                        if (anyTagged) DropdownMenuItem(
                                            text = { Text("Remove ${person.optString("name")}") },
                                            onClick = {
                                                tagMenu = false
                                                val ids = JSONArray().apply { selectedMediaIds.sorted().forEach { put(it) } }
                                                scope.launch {
                                                    val saved = model.execute(
                                                        JSONObject().put("action", "tag_assets")
                                                            .put("room_id", room.getString("id"))
                                                            .put("asset_ids", ids)
                                                            .put("person_id", person.getString("id"))
                                                            .put("remove", true),
                                                    )
                                                    if (saved) { selectedMediaIds = emptySet(); selectingMedia = false }
                                                }
                                            },
                                        )
                                    }
                                }
                            }

                            }
                            Button(
                                onClick = { selectedAlbumTitle = ""; createSelectedAlbum = true },
                                enabled = !showingRecentlyDeleted,
                                modifier = Modifier.fillMaxWidth().testTag("library.batch.album"),
                            ) { Text("Create album") }
                            OutlinedButton(
                                onClick = { correctCaptureDate = true },
                                enabled = !showingRecentlyDeleted,
                                modifier = Modifier.fillMaxWidth().testTag("library.batch.date"),
                            ) { Text("Correct capture date") }
                            if (showingRecentlyDeleted) {
                                Button(
                                    onClick = {
                                        val ids = JSONArray().apply { selectedMediaIds.sorted().forEach { put(it) } }
                                        scope.launch {
                                            val restored = model.execute(
                                                JSONObject().put("action", "patch_assets")
                                                    .put("room_id", room.getString("id"))
                                                    .put("asset_ids", ids)
                                                    .put("fields", JSONObject().put("deleted", false)),
                                            )
                                            if (restored) { selectedMediaIds = emptySet(); selectingMedia = false }
                                        }
                                    },
                                    enabled = mayChangeDeletion,
                                    modifier = Modifier.fillMaxWidth().testTag("library.batch.restore"),
                                ) { Text("Restore selected") }
                            } else {
                                OutlinedButton(
                                    onClick = { confirmBulkDelete = true },
                                    enabled = mayChangeDeletion,
                                    modifier = Modifier.fillMaxWidth().testTag("library.batch.delete"),
                                    colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                ) { Text("Move to Recently Deleted") }
                                if (!mayChangeDeletion) {
                                    Text("Select memories you contributed, or ask a Room owner or admin.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                                }
                            }
                        }
                    }
                }
                if (section == "Library") fullWidthItem {
                ElevatedCard(onClick = { videoFinder = true }, modifier = Modifier.fillMaxWidth().testTag("video.finder.open")) {
 Row(Modifier.fillMaxWidth().padding(16.dp), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
 horizontalArrangement = Arrangement.spacedBy(12.dp)) {
 Icon(painterResource(R.drawable.ic_search), contentDescription = null, tint = colors.primary)
 Column(Modifier.weight(1f)) {
 Text("Ask your videos", style = MaterialTheme.typography.titleMedium)
 Text("Find a spoken line or something in a frame", style = MaterialTheme.typography.bodySmall,
 color = colors.onSurfaceVariant)
 }
 Text("↗", color = colors.primary)
                    }
                }
            }
                if (section == "Library" && !showingRecentlyDeleted && people.isNotEmpty()) fullWidthItem {
                PeopleFilterBar(
                    people = people,
                    selectedPeople = selectedPeople,
                    requireAll = requireAllPeople,
                    onTogglePerson = { id ->
                        selectedPeople = if (id in selectedPeople) selectedPeople - id else selectedPeople + id
                    },
                    onRequireAllChange = { requireAllPeople = it },
                    onClear = {
                        selectedPeople = emptySet()
                        requireAllPeople = false
                    },
                )
            }
 if (section == "Home" && !searching && momentSuggestions.isNotEmpty()) fullWidthItem {
  MomentSuggestionsPanel(
   suggestions = momentSuggestions,
   assets = allAssets,
   onSave = { title, assetIds ->
    scope.launch {
     val ids = JSONArray().apply { assetIds.forEach(::put) }
     if (model.execute(JSONObject()
       .put("action", "save_moment")
       .put("room_id", room.getString("id"))
       .put("title", title)
       .put("featured", false)
       .put("asset_ids", ids))) {
      momentSuggestions = momentSuggestions.filterNot { candidate ->
       candidate.optJSONArray("asset_ids").stringItems().toSet() == assetIds.toSet()
      }
     }
    }
   },
  )
 }
        if (section == "Home" && !searching && items.isNotEmpty()) fullWidthItem {
            RoomHighlights(items.filter { it.optString("kind") in listOf("photo", "video") }.take(5),
                thumbnail = { asset, modifier -> MemoryThumbnail(model, asset, modifier) },
                onOpen = { asset -> scope.launch { openOriginal(model, asset) } })
        }
        }
        if (section == "Home" && !searching && room?.rows("moments").orEmpty().isNotEmpty()) fullWidthItem {
            MomentRiverPanel(
                model = model,
                roomId = model.roomId,
                moments = room?.rows("moments").orEmpty(),
                assets = allAssets,
                canEdit = room?.optString("role") in listOf("owner", "admin", "contributor"),
                onOpen = { momentId -> selectedMomentFilter = momentId; section = "Library" },
            )
        }
        if (section == "Home" && !searching && onThisDayAssets.isNotEmpty()) fullWidthItem {
            RoomHighlights(
                assets = onThisDayAssets.take(5),
                thumbnail = { asset, modifier -> MemoryThumbnail(model, asset, modifier) },
                onOpen = { asset -> scope.launch { openOriginal(model, asset) } },
                title = "On this day",
                subtitle = "Moments from earlier years"
            )
        }
        if (section == "Home" && items.isNotEmpty()) fullWidthItem {
                            Text("Recent contributions", style = MaterialTheme.typography.titleLarge.copy(fontFamily = RoomDisplay), color = colors.onSurface,
                                modifier = Modifier.fillMaxWidth().padding(top = 8.dp, bottom = 4.dp))
                        }
                        if (section == "Theater" && room != null && room.optString("role") in listOf("owner", "admin", "contributor")) fullWidthItem {
                            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                                Button(onClick = model.studio::create, enabled = !model.studio.preparing,
                                    modifier = Modifier.weight(1f).testTag("theater.createFilm")) {
                                    Text(if (model.studio.preparing) "Opening studio…" else "Create a film")
                                }
                                OutlinedButton(onClick = { openOrCreateWeeklyReel(model, room, scope) },
                                    enabled = !model.studio.preparing,
                                    modifier = Modifier.weight(1f).testTag("theater.makeWeeklyReel")) {
                                    Text("Make this week’s reel")
                                }
                            }
                        }
                        if (section == "Theater") room?.let { activeRoom -> fullWidthItem {
                            RoomCollectionPanel(model, activeRoom, "playlist", query = search, onPlayPlaylist = { playlist ->
                                playlistFilms(activeRoom, playlist).firstOrNull()?.let { first ->
                                    screening = first
                                    screeningQueue = playlistFilms(activeRoom, playlist)
                                    screeningPlaylistTitle = playlist.optString("title")
                                }
                            })
                        } }
                        if (section == "Albums") room?.let { activeRoom -> fullWidthItem { RoomCollectionPanel(model, activeRoom, "album", query = search) } }
                if (items.isEmpty() && section != "Albums" && !showingRecentlyDeleted) fullWidthItem {
                            Column(Modifier.fillMaxWidth().background(colors.surface, RoundedCornerShape(12.dp)).padding(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                                SectionIcon(section)
                                Text(if (hasActiveFilters) "No matching memories" else when (section) {
                                    "Albums" -> "Collections worth keeping"
                                    "People" -> "The people in your memories"
                                    "Theater" -> "A place for your family films"
                                    else -> "Your first memory starts here"
                                }, style = MaterialTheme.typography.titleLarge.copy(fontFamily = RoomDisplay))
                                Text(if (hasActiveFilters) "Change or clear a filter to see more memories." else when (section) {
                                    "Albums" -> "Curated albums will appear here. Removing a photo from an album keeps its original."
                                    "People" -> "People named in photos will appear here. Naming someone never invites them to this Room."
                                    "Theater" -> "Make a film from your Room’s photos and clips. Drafts stay private until a movie is published."
                                    else -> "Add a photo or video to share it with this Room."
                            }, color = colors.onSurfaceVariant)
                        }
                    }
                    if (!settings && room != null && section != "Albums") items(items, key = { it.optString("id") + it.optString("version_id") }) { asset -> Card(Modifier.fillMaxWidth().testTag("memory.card.${asset.optString("id")}"), colors = CardDefaults.cardColors(containerColor = colors.surface)) { Column {
                            if (field == "assets") {
                                MemoryThumbnail(model, asset)
                                if (selectingMedia) {
                                    val assetId = asset.optString("id")
                                    val selected = assetId in selectedMediaIds
                                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                                        Checkbox(
                                            checked = selected,
                                            onCheckedChange = { checked ->
                                                selectedMediaIds = updateBoundedSelection(selectedMediaIds, assetId, checked)
                                            },
                                            enabled = selected || selectedMediaIds.size < MAX_SELECTED_MEMORIES,
                                            modifier = Modifier
                                                .testTag("library.select.asset.$assetId")
                                                .semantics {
                                                    contentDescription = if (selected) "Deselect ${asset.optString("filename")}" else "Select ${asset.optString("filename")}"
                                                },
                                        )
                                    }
                                }
                            }
                        if (field == "stories") {
                            val coverId = asset.optString("published_asset").takeUnless { it.isEmpty() || it == "null" }
                                ?: asset.rows("clips").firstOrNull()?.optString("asset_id")
                            room.rows("assets").find { it.optString("id") == coverId }?.let { MemoryThumbnail(model, it) }
                        }
                        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(asset.optString("caption").ifEmpty { asset.optString("filename").ifEmpty { asset.optString("title").ifEmpty { asset.optString("name") } } }, style = MaterialTheme.typography.titleMedium, maxLines = 2, overflow = TextOverflow.Ellipsis)
            if (field == "assets" && people.isNotEmpty() && room.optString("role") in listOf("owner", "admin", "contributor")) {
                AssetPeopleMenu(
                    assetPeople = asset.peopleIds(),
                    people = people,
                    enabled = true,
                    tag = "memory.people." + asset.optString("id"),
                    onTogglePerson = { personId ->
                        val updated = togglePersonTag(asset.peopleIds(), personId)
                        val tags = JSONArray().apply { updated.sorted().forEach { put(it) } }
                        scope.launch {
                            model.execute(
                                JSONObject().put("action", "patch_asset")
                                    .put("room_id", asset.getString("room_id"))
                                    .put("asset_id", asset.getString("id"))
                                    .put("fields", JSONObject().put("people", tags)),
                            )
                        }
                    },
                )
            }
                        if (field == "stories") {
                            Text(if (asset.optBoolean("private_draft")) "Private draft" else "Published film", style = MaterialTheme.typography.labelSmall, color = colors.onSurfaceVariant)
                            Text("${asset.rows("clips").size} clips", style = MaterialTheme.typography.bodySmall)
                            if (asset.optBoolean("auto_generated") && asset.optBoolean("private_draft")) {
                                WeeklyReelSkipAction(model, room.getString("id"), asset.getString("id"))
                            }
                            if (asset.optInt("format_version", 1) != 1) Text("Update Family Room to edit this project.", color = colors.onSurfaceVariant)
                            else if (room.optString("role") in listOf("owner", "admin", "contributor")) {
                                TextButton(onClick = { model.studio.open(asset) }, modifier = Modifier.testTag("theater.open.${asset.optString("id")}")) { Text("Open studio") }
                            }
                            room.rows("assets").find { it.optString("id") == asset.optString("published_asset") }?.let { movie ->
                    TextButton(onClick = { screeningQueue = null; screeningPlaylistTitle = null; screening = asset to movie }, modifier = Modifier.testTag("theater.watch.${asset.optString("id")}")) { Text("Watch") }
                    if (room.optString("role") in listOf("owner", "admin", "contributor")) {
                        var playlistMenu by remember(asset.optString("id")) { mutableStateOf(false) }
                        Box {
                            TextButton(onClick = { playlistMenu = true }) { Text("Add to playlist") }
                            DropdownMenu(expanded = playlistMenu, onDismissRequest = { playlistMenu = false }) {
                                for (playlist in room.rows("playlists")) {
                                    val existing = playlist.optJSONArray("story_ids") ?: JSONArray()
                                    val hasFilm = (0 until existing.length()).any { existing.optString(it) == asset.optString("id") }
                                    DropdownMenuItem(text = { Text(playlist.optString("title")) }, enabled = !hasFilm, onClick = {
                                        playlistMenu = false
                                        val next = JSONArray(); for (i in 0 until existing.length()) next.put(existing.getString(i))
                                        next.put(asset.getString("id"))
                                        scope.launch { model.execute(JSONObject().put("action", "save_playlist")
                                            .put("room_id", room.getString("id")).put("id", playlist.getString("id"))
                                            .put("title", playlist.getString("title")).put("story_ids", next)) }
                                    })
                                }
                            }
                        }
                    }
                            }
                        }
                        if (field == "assets") {
                            var diagnostics by remember(asset.optString("id")) { mutableStateOf(false) }
                            TextButton(onClick = { diagnostics = !diagnostics }) { Text(if (diagnostics) "Hide availability" else "Availability") }
                            if (diagnostics) {
                                Text(asset.optString("availability"))
                                Text("${asset.optInt("verified_copies")} verified copies", style = MaterialTheme.typography.bodySmall)
                            }
                            TextButton(onClick = { scope.launch { openOriginal(model, asset) } }) { Text("Open original") }
                        if (asset.optString("kind") == "video" && !asset.optBoolean("deleted")) {
                            VideoTranscriptExportButton(model, room.getString("id"), asset.getString("id"))
                        }
                            val noteComments = room?.rows("comments").orEmpty().filter { it.optString("asset_id") == asset.optString("id") }
                            for (note in noteComments) Column(Modifier.fillMaxWidth().background(colors.surfaceVariant.copy(alpha = 0.42f), RoundedCornerShape(12.dp)).padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                Text(note.optString("body"), style = MaterialTheme.typography.bodyMedium)
                                val noteAuthor = note.optString("author")
                                val attribution = if (noteAuthor == model.snapshot.optString("device_id")) "You" else "Member ${noteAuthor.take(8)}"
                                Text(attribution, style = MaterialTheme.typography.labelSmall, color = colors.onSurfaceVariant)
                            }
                            var commentDraft by remember(asset.optString("id")) { mutableStateOf("") }
                            OutlinedTextField(value = commentDraft, onValueChange = { value -> if (value.toByteArray(Charsets.UTF_8).size <= 10_000) commentDraft = value }, modifier = Modifier.fillMaxWidth().testTag("memory.comment.input.${asset.optString("id")}"), label = { Text("Leave a family note") }, minLines = 1, maxLines = 3)
                            TextButton(onClick = { scope.launch {
                                model.execute(JSONObject().put("action", "comment").put("room_id", asset.getString("room_id")).put("asset_id", asset.getString("id")).put("body", commentDraft))
                                commentDraft = ""
                            } }, enabled = room?.optString("role") != "removed" && commentDraft.isNotBlank(), modifier = Modifier.testTag("memory.comment.post.${asset.optString("id")}")) { Text("Post note") }

                            TextButton(onClick = { scope.launch { model.execute(JSONObject().put("action", "patch_asset").put("room_id", asset.getString("room_id")).put("asset_id", asset.getString("id")).put("fields", JSONObject().put("favorite", !asset.optBoolean("favorite")))) } }, enabled = room?.optString("role") in listOf("owner", "admin", "contributor")) { Text(if (asset.optBoolean("favorite")) "Unfavorite" else "Favorite") }
                            var captionHistory by remember(asset.optString("id")) { mutableStateOf(emptyList<JSONObject>()) }
                            var showingCaptionHistory by remember(asset.optString("id")) { mutableStateOf(false) }
                            var captionHistoryError by remember(asset.optString("id")) { mutableStateOf("") }
                            TextButton(onClick = {
                                showingCaptionHistory = !showingCaptionHistory
                                if (showingCaptionHistory) scope.launch {
                                    captionHistoryError = ""
                                    try {
                                        val response = model.command(JSONObject().put("action", "asset_history").put("room_id", asset.getString("room_id")).put("asset_id", asset.getString("id")))
                                        val revisions = response.optJSONArray("history") ?: JSONArray()
                                        captionHistory = (0 until revisions.length()).mapNotNull { revisions.optJSONObject(it) }
                                    } catch (error: Exception) {
                                        captionHistoryError = error.message ?: "Caption history is unavailable."
                                    }
                                }
                            }, modifier = Modifier.testTag("memory.caption.history.${asset.optString("id")}")) { Text("Caption history") }
                            if (captionHistoryError.isNotBlank()) Text(captionHistoryError, color = colors.error)
                            if (showingCaptionHistory) for ((index, revision) in captionHistory.withIndex()) {
                                val captionValue = revision.optString("caption")
                                val revisionAuthor = revision.optString("author")
                                val attribution = if (revisionAuthor == model.snapshot.optString("device_id")) "You" else "Member ${revisionAuthor.take(8)}"
                                Column(Modifier.fillMaxWidth().background(colors.surfaceVariant.copy(alpha = 0.28f), RoundedCornerShape(10.dp)).padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                    Text(captionValue.ifEmpty { "(No caption)" }, style = MaterialTheme.typography.bodyMedium)
                                    Text("$attribution · revision ${revision.optLong("clock")}", style = MaterialTheme.typography.labelSmall, color = colors.onSurfaceVariant)
                                    if (captionValue != asset.optString("caption") && room?.optString("role") in listOf("owner", "admin", "contributor")) TextButton(onClick = { scope.launch { model.execute(JSONObject().put("action", "patch_asset").put("room_id", asset.getString("room_id")).put("asset_id", asset.getString("id")).put("fields", JSONObject().put("caption", captionValue))) } }, modifier = Modifier.testTag("memory.caption.history.use.${asset.optString("id")}.$index")) { Text("Use this caption") }
                                }
                            }
                            val likes = room?.rows("reactions").orEmpty().filter { it.optString("asset_id") == asset.optString("id") && it.optBoolean("liked") }
                            val likedByMe = likes.any { it.optString("author") == model.snapshot.optString("device_id") }
                            TextButton(onClick = {
                                scope.launch {
                                    model.execute(JSONObject().put("action", "set_like")
                                        .put("room_id", asset.getString("room_id"))
                                        .put("asset_id", asset.getString("id"))
                                        .put("liked", !likedByMe))
                                }
                            }, enabled = room?.optString("role") != "removed", modifier = Modifier.testTag("memory.like.${asset.optString("id")}")) {
                                Text("${if (likedByMe) "Unlike" else "Like"} · ${likes.size}")
                            }
                        }
                    } } } }
        if (room == null || settings) fullWidthItem {
            if (settings && room != null) {
                RoomSetupChecklist(model) { settings = false }
                AndroidAutomaticUploads(model)
            }
            RecoveryBackupSettings(
                model = model,
                canRestore = canRestoreEncryptedBackup(
                    libraryReady = model.ready,
                    roomCount = model.rooms.size,
                    pendingInvitationCount = model.snapshot.rows("pending_invitations").size,
                ),
            )
            if (settings && room != null) LeaveRoomAction(model, room.getString("id"), room.optString("role"))
                        if (room != null) {
                    if (room.optString("role") == "owner" || room.optString("role") == "admin") {
                    Text("Storage and sync", style = MaterialTheme.typography.titleMedium)
                    RoomStorageSources(model, model.roomId)
                            OutlinedTextField(value = gateway, onValueChange = { gateway = it }, label = { Text("Room gateway HTTPS address") })
                            Button(onClick = { scope.launch { model.execute(JSONObject().put("action", "add_source").put("room_id", model.roomId).put("name", "Room gateway").put("kind", "gateway").put("endpoint", gateway)); model.sync() } }, enabled = gateway.isNotBlank()) { Text("Connect gateway") }
                    } else {
                        Text("The Room owner manages shared storage and gateway access. You can join without an account on their cloud provider.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                            Button(onClick = { scope.launch { model.sync() } }) { Text("Sync Room") }
                            if (room.optString("role") == "removed") {
                                Text("This device was removed. Downloaded memories remain available; new shared changes are blocked.")
                            }
                            if (room.optInt("rejected_edits") > 0) {
                                Text("${room.optInt("rejected_edits")} offline changes were preserved locally and have not been shared.")
                                Text("Export changes, private film drafts and available originals as an unencrypted ZIP. Keep it private. Missing originals are listed in its manifest.", style = MaterialTheme.typography.bodySmall)
                                Button(onClick = {
                                    val targetRoom = room.getString("id")
                                    exportingPreserved = true
                                    scope.launch {
                                        val file = File(cacheDir, "Preserved-work-${UUID.randomUUID()}.zip")
                                        try {
                                            val result = model.command(JSONObject().put("action","export_preserved").put("room_id",targetRoom).put("path",file.absolutePath))
                                            preservedPath = file.absolutePath
                                            preservedSummary = "Preserved work saved. ${result.optInt("exported")} originals exported; ${result.optInt("missing")} originals were unavailable on this device."
                                            savePreserved.launch("Family-Room-preserved-work.zip")
                                        } catch (e: Exception) {
                                            withContext(Dispatchers.IO) { file.delete() }
                                            preservedPath = ""
                                            exportingPreserved = false
                                            model.status = e.message ?: "Cannot export preserved work."
                                        }
                                    }
                                }, enabled = !exportingPreserved) { Text(if (exportingPreserved) "Preparing preserved work…" else "Export preserved work") }
                            }
                            if (room.optString("role") == "owner") {
                                Text("Members", style = MaterialTheme.typography.titleMedium)
                                for (member in room.rows("members")) {
                                    val memberId = member.getString("device_id")
                                    if (memberId != room.getString("owner_id")) {
                                        Column {
                                            Text("Device ${memberId.take(8)} · ${member.optString("role")}")
                                            Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                                if (!room.optBoolean("album_only")) {
                                                    for (newRole in listOf("viewer", "contributor", "admin")) {
                                                        TextButton(onClick = { scope.launch {
                                                            model.execute(JSONObject().put("action","set_role").put("room_id",room.getString("id")).put("device_id",memberId).put("role",newRole))
                                                            model.sync(room.getString("id"))
                                                        } }, enabled = member.optString("role") != newRole) { Text(newRole) }
                                                    }
                                                }
                                                TextButton(onClick = { pendingRemoval = member }) { Text("Remove") }
                                        }
                                    }
                                }

                                val successors = room.rows("members").filter {
                                    it.optString("device_id") != room.optString("owner_id")
                                }
                                if (successors.isNotEmpty()) {
                                    Text("Room ownership", style = MaterialTheme.typography.titleMedium)
                                    Text("Choose an existing Member. They must accept the signed offer on their device.", style = MaterialTheme.typography.bodySmall)
                                    for (member in successors) {
                                        val memberId = member.optString("device_id")
                                        FilterChip(
                                            selected = ownershipTarget == memberId,
                                            onClick = { ownershipTarget = memberId },
                                            label = { Text("Device ${memberId.take(8)} · ${member.optString("role")}") },
                                        )
                                    }
                                    Button(onClick = { scope.launch {
                                        try {
                                            generatedOwnershipOffer = model.command(JSONObject()
                                                .put("action", "prepare_owner_transfer")
                                                .put("room_id", room.getString("id"))
                                                .put("device_id", ownershipTarget))
                                                .optString("offer")
                                        } catch (e: Exception) { model.status = e.message ?: "Cannot prepare the ownership offer." }
                                    } }, enabled = ownershipTarget.isNotBlank()) { Text("Prepare transfer offer") }
                                    Text(if (room.optBoolean("album_only"))
                                        "After acceptance, your device becomes a viewer. The Room history stays in place and future contributions use a rotated key."
                                    else "After acceptance, your device becomes an admin. The Room history stays in place and future contributions use a rotated key.",
                                        style = MaterialTheme.typography.bodySmall)
                                    if (generatedOwnershipOffer.isNotBlank()) {
                                        Text("Send this signed offer privately to the selected device. It is bound to that Member.", style = MaterialTheme.typography.bodySmall)
                                        SelectionContainer { Text(generatedOwnershipOffer, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace) }
                                        TextButton(onClick = {
                                            val send = Intent(Intent.ACTION_SEND).apply {
                                                type = "text/plain"
                                                putExtra(Intent.EXTRA_TEXT, generatedOwnershipOffer)
                                            }
                                            context.startActivity(Intent.createChooser(send, "Share ownership offer"))
                                        }) { Text("Share ownership offer") }
                                    }
                                }
                            }
                            if (room.optString("role") != "owner" && room.optString("role") != "removed") {
                                Text("Room ownership", style = MaterialTheme.typography.titleMedium)
                                Text("Paste a signed ownership offer shared with this device. Accepting makes this device the owner.", style = MaterialTheme.typography.bodySmall)
                                OutlinedTextField(
                                    value = ownershipOfferInput,
                                    onValueChange = { ownershipOfferInput = it },
                                    label = { Text("Signed ownership offer") },
                                    minLines = 3,
                                    modifier = Modifier.fillMaxWidth(),
                                )
                                Button(onClick = { confirmOwnershipAcceptance = true }, enabled = ownershipOfferInput.isNotBlank()) {
                                    Text("Review ownership offer")
                                }
                            }

                                OutlinedTextField(value = device, onValueChange = { device = it }, label = { Text("Member’s device code") })
                                Row { for (role in if (room.optBoolean("album_only")) listOf("viewer") else listOf("contributor", "viewer", "admin")) TextButton(onClick = { inviteRole = role }) { Text(if (role == inviteRole) "✓ $role" else role) } }
                            Button(onClick = { scope.launch {
                                try {
                                    generated = model.command(JSONObject().put("action", "invite")
                                        .put("room_id", model.roomId).put("device_id", device)
                                        .put("role", if (room.optBoolean("album_only")) "viewer" else inviteRole))
                                        .getString("invitation")
                                    var published = false
                                    for (source in model.snapshot.rows("sources").filter {
                                        it.optString("room_id") == model.roomId && !it.optBoolean("paused")
                                    }) {
                                        try {
                                            model.command(JSONObject().put("action", "refresh_access")
                                                .put("id", source.getString("id")))
                                            published = true
                                        } catch (_: Exception) { }
                                    }
                                    model.reload()
                                    model.status = if (published) "Invitation ready. Connected storage is prepared for this Member."
                                        else "Invitation ready. Sync the Room before the new Member connects."
                                } catch (e: Exception) { model.status = e.message ?: "Cannot create invitation." }
                            } }, enabled = device.isNotBlank()) { Text("Create invitation") }
                                if (generated.isNotEmpty()) SelectionContainer { Text(generated) }
                            }
                        }
                        if (settings) {
                            Text("Create another Room", style = MaterialTheme.typography.titleMedium)
                            OutlinedTextField(value = name, onValueChange = { name = it }, label = { Text("Room name") }, modifier = Modifier.fillMaxWidth().testTag("room.name"))
                            Button(onClick = { scope.launch {
                                if (model.execute(JSONObject().put("action", "create_room").put("name", name))) { name = ""; settings = true }
                            } }, enabled = model.ready && name.isNotBlank(), modifier = Modifier.testTag("room.create")) { Text("Create Room") }
                        }
                        if (model.snapshot.rows("pending_invitations").isNotEmpty()) {
 Text("Invitations to review", style = MaterialTheme.typography.titleMedium)
 model.snapshot.rows("pending_invitations").forEach { pending ->
  ElevatedCard(Modifier.fillMaxWidth()) {
   Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
    Text(pending.optString("name"), style = MaterialTheme.typography.titleMedium)
    Text("Device ${pending.optString("owner_id").take(8)} invited this device as a ${pending.optString("role")}.", style = MaterialTheme.typography.bodySmall)
    if (pending.optBoolean("album_only")) Text("This invitation only grants access to its shared album.", style = MaterialTheme.typography.bodySmall)
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
     Button(onClick = { scope.launch { if (model.execute(JSONObject().put("action", "accept_invitation").put("id", pending.optString("id")))) settings = false } }) { Text("Accept") }
     TextButton(onClick = { scope.launch { model.execute(JSONObject().put("action", "decline_invitation").put("id", pending.optString("id"))) } }) { Text("Decline") }
    }
   }
  }
 }
 Text("Decline removes the invitation from this device only; the sender is not notified.", style = MaterialTheme.typography.bodySmall)
}
Text("Your device code")
 SelectionContainer { Text(model.snapshot.optString("device_id"), style = MaterialTheme.typography.bodySmall) }
 OutlinedTextField(value = invitation, onValueChange = { invitation = it; invitationPreview = null; invitationReviewedToken = "" }, label = { Text("Invitation") }, modifier = Modifier.fillMaxWidth())
 Button(onClick = { scope.launch {
  try {
   val token = invitation.trim()
   invitationPreview = model.command(JSONObject().put("action", "preview_invitation").put("invitation", token))
   invitationReviewedToken = token
  } catch (failure: Exception) { model.status = failure.message ?: "Cannot review invitation." }
 } }, enabled = model.ready && invitation.isNotBlank()) { Text("Review invitation") }
 invitationPreview?.let { preview ->
  ElevatedCard(Modifier.fillMaxWidth()) {
   Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
    Text("You’ve been invited to ${preview.optString("name")}", style = MaterialTheme.typography.titleMedium)
    Text("Owner device ${preview.optString("owner_id").take(8)} · ${preview.optString("role")} access", style = MaterialTheme.typography.bodySmall)
    if (preview.optBoolean("album_only")) Text("This invitation grants access only to a shared album.", style = MaterialTheme.typography.bodySmall)
    Button(onClick = { scope.launch {
     if (model.execute(JSONObject().put("action", "stage_invitation").put("invitation", invitationReviewedToken))) {
      invitation = ""; invitationPreview = null; invitationReviewedToken = ""
     }
    } }) { Text("Save for review") }
   }
  }
 }
                    }
                }
            }
        }
        }
        }
    }
@Composable
private fun SectionIcon(section: String, selected: Boolean = false) {
    val resource = when (section) {
        "Home" -> R.drawable.ic_home
        "Library" -> R.drawable.ic_library
        "Albums" -> R.drawable.ic_albums
        "People" -> R.drawable.ic_people
        else -> R.drawable.ic_theater
    }
    val colors = MaterialTheme.colorScheme
    val context = LocalContext.current
    val reduced = remember(context) {
        Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f
    }
    val scale by animateFloatAsState(
        if (selected && !reduced) 1.055f else 1f,
        animationSpec = spring(dampingRatio = 0.72f),
        label = "selected icon scale"
    )
    val tilt by animateFloatAsState(
        if (selected && !reduced) 4f else 0f,
        animationSpec = spring(dampingRatio = 0.72f),
        label = "selected icon tilt"
    )
    val shape = RoundedCornerShape(8.dp)

    Box(
        Modifier.size(30.dp)
            .graphicsLayer {
                scaleX = scale
                scaleY = scale
                rotationX = tilt
                rotationY = -tilt
                shadowElevation = if (selected) 7.dp.toPx() else 3.dp.toPx()
                this.shape = shape
                clip = true
            }
            .background(
                brush = Brush.linearGradient(
                    colors = if (selected) listOf(colors.tertiary, colors.primary)
                    else listOf(colors.surface, colors.secondaryContainer)
                ),
                shape = shape
            )
            .border(0.8.dp, colors.outlineVariant, shape),
        contentAlignment = androidx.compose.ui.Alignment.Center
    ) {
        Box(
            Modifier.align(Alignment.TopStart)
                .fillMaxWidth(0.72f)
                .height(8.dp)
                .background(Color.White.copy(alpha = if (selected) 0.28f else 0.50f), shape)
        )
        Icon(
            painterResource(resource),
            contentDescription = null,
            modifier = Modifier.size(18.dp),
            tint = if (selected) colors.onPrimary else colors.primary
        )
    }
}

private suspend fun openOriginal(model: RoomModel, asset: JSONObject) {
        try {
            val folder = File(cacheDir, "originals").apply { mkdirs() }
            val output = File(folder, asset.getString("id") + "-" + File(asset.getString("filename")).name)
            if (!asset.optBoolean("local")) model.command(JSONObject().put("action", "fetch").put("room_id", asset.getString("room_id")).put("asset_id", asset.getString("id")))
            model.command(JSONObject().put("action", "materialize").put("room_id", asset.getString("room_id")).put("asset_id", asset.getString("id")).put("destination", output.absolutePath))
            val uri = FileProvider.getUriForFile(this, "$packageName.originals", output)
            startActivity(Intent(Intent.ACTION_VIEW).setDataAndType(uri, contentResolver.getType(uri) ?: "application/octet-stream").addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION))
        } catch (e: Exception) { model.status = e.message ?: "Cannot open original." }
    }
    private fun copyImport(uri: Uri): File {
        var name = "original"
        contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { if (it.moveToFirst()) name = it.getString(0) }
        val directory = File(cacheDir, "import-${java.util.UUID.randomUUID()}").apply { mkdirs() }
        val target = File(directory, File(name).name)
        contentResolver.openInputStream(uri)?.use { input -> target.outputStream().use { input.copyTo(it) } } ?: error("Source unavailable.")
        return target
    }
}
