package com.familyroom.app

import android.content.Context
import android.net.Uri
import androidx.documentfile.provider.DocumentFile
import kotlinx.coroutines.ensureActive
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.ArrayDeque
import kotlin.coroutines.coroutineContext

internal object AndroidFolderImport {
    private const val MAX_INITIAL_ITEMS = 20_000
    private const val MAX_IMPORTS_PER_RULE = 50
    private val mediaExtensions = setOf(
        "jpg", "jpeg", "png", "heic", "heif", "mov", "mp4", "m4v",
        "webm", "dng", "gif", "tiff", "webp", "avif",
    )

    fun initialSnapshot(context: Context, treeUri: Uri): List<String> {
        val identifiers = mutableListOf<String>()
        for (document in walk(context, treeUri)) {
            if (!isMedia(document)) continue
            identifiers += fingerprint(document)
            check(identifiers.size <= MAX_INITIAL_ITEMS) {
                "This folder has more than $MAX_INITIAL_ITEMS existing photos and videos. Turn on “Import existing media” or choose a smaller folder."
            }
        }
        return identifiers
    }

    suspend fun scan(context: Context, vault: Vault): ScanResult {
        val snapshot = vault.command(JSONObject().put("action", "snapshot"))
        val rooms = snapshot.optJSONArray("rooms") ?: JSONArray()
        val rules = snapshot.optJSONArray("watches") ?: JSONArray()
        val errors = mutableListOf<String>()
        var imported = 0

        for (index in 0 until rules.length()) {
            coroutineContext.ensureActive()
            val rule = rules.optJSONObject(index) ?: continue
            val treeText = rule.optString("path")
            if (rule.optBoolean("paused") || !treeText.startsWith("content://")) continue

            val roomId = rule.optString("room_id")
            val room = (0 until rooms.length())
                .mapNotNull(rooms::optJSONObject)
                .firstOrNull { it.optString("id") == roomId }
                ?: continue
            if (room.optString("role") !in setOf("owner", "admin", "contributor")) continue

            val treeUri = runCatching { Uri.parse(treeText) }.getOrNull()
            if (treeUri == null || !hasPersistedReadAccess(context, treeUri)) {
                errors += rule.optString("id") + ": folder access needs attention"
                continue
            }

            val seenArray = rule.optJSONArray("seen") ?: JSONArray()
            val seen = (0 until seenArray.length()).mapNotNull(seenArray::optString).toHashSet()
            val excludedArray = rule.optJSONArray("excluded_extensions") ?: JSONArray()
            val excluded = (0 until excludedArray.length())
                .mapNotNull(excludedArray::optString)
                .map { it.trim().removePrefix(".").lowercase() }
                .toSet()
            var handled = 0

            try {
                for (document in walk(context, treeUri)) {
                    coroutineContext.ensureActive()
                    if (!isMedia(document)) continue
                    val extension = extension(document)
                    if (extension in excluded) continue
                    val identifier = fingerprint(document)
                    if (identifier in seen) continue
                    val modifiedAt = document.lastModified()
                    if (modifiedAt > 0L && System.currentTimeMillis() - modifiedAt < 2_000L) continue
                    if (handled >= MAX_IMPORTS_PER_RULE) break
                    handled += 1

                    var temporary: File? = null
                    try {
                        temporary = File.createTempFile("room-import-", ".$extension", context.cacheDir)
                        context.contentResolver.openInputStream(document.uri)?.use { input ->
                            temporary.outputStream().use(input::copyTo)
                        } ?: error("Android could not open this media item.")
                        vault.command(
                            JSONObject()
                                .put("action", "import")
                                .put("room_id", roomId)
                                .put("path", temporary.absolutePath),
                        )
                        vault.command(
                            JSONObject()
                                .put("action", "acknowledge_watch_item")
                                .put("id", rule.optString("id"))
                                .put("identifier", identifier),
                        )
                        imported += 1
                        seen += identifier
                    } catch (failure: Exception) {
                        errors += (document.name ?: "Media item") + ": " + (failure.message ?: "import failed")
                    } finally {
                        temporary?.delete()
                    }
                }
            } catch (failure: SecurityException) {
                errors += rule.optString("id") + ": " + (failure.message ?: "folder access was revoked")
            } catch (failure: Exception) {
                errors += rule.optString("id") + ": " + (failure.message ?: "folder scan failed")
            }
        }
        return ScanResult(imported, errors)
    }

    private fun hasPersistedReadAccess(context: Context, uri: Uri): Boolean =
        context.contentResolver.persistedUriPermissions.any { permission ->
            permission.uri == uri && permission.isReadPermission
        }

    private fun walk(context: Context, treeUri: Uri): Sequence<DocumentFile> = sequence {
        val root = DocumentFile.fromTreeUri(context, treeUri)
            ?: error("Android could not open the selected folder.")
        val pending = ArrayDeque<DocumentFile>()
        val visited = mutableSetOf<String>()
        pending.addLast(root)
        while (pending.isNotEmpty()) {
            val directory = pending.removeLast()
            if (!visited.add(directory.uri.toString())) continue
            if (directory.isDirectory) {
                directory.listFiles().forEach(pending::addLast)
            } else if (directory.isFile && isMedia(directory)) {
                yield(directory)
            }
        }
    }

    private fun isMedia(document: DocumentFile): Boolean = extension(document) in mediaExtensions

    private fun extension(document: DocumentFile): String =
        document.name
            ?.substringAfterLast('.', "")
            ?.lowercase()
            .orEmpty()

    private fun fingerprint(document: DocumentFile): String =
        "content:" + document.uri + ":" + document.length() + ":" + document.lastModified()

    data class ScanResult(val imported: Int, val errors: List<String>)
}
