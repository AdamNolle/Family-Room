package com.familyroom.app

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileInputStream
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.UUID
import java.util.zip.ZipInputStream

/** Optional Apache-licensed Vosk model. No microphone or hosted transcription is used. */
internal object OfflineSpeechModel {
    private const val NAME = "vosk-model-small-en-us-0.15"
    private const val MODEL_URL = "https://alphacephei.com/vosk/models/$NAME.zip"
    private const val ARCHIVE_SHA256 = "30f26242c4eb449f948e42cb302dd7a686cb29a3423a8367f99ff41780942498"
    private const val MAX_ARCHIVE_BYTES = 50_000_000L
    private const val MAX_EXPANDED_BYTES = 100_000_000L
    private val installLock = Mutex()

    private fun root(context: Context) = File(context.noBackupFilesDir, "offline-speech")
    private fun destination(context: Context) = File(root(context), NAME)

    fun installed(context: Context): File? = destination(context).takeIf { model ->
        runCatching { File(model, ".family-room-sha256").readText() }.getOrNull() == ARCHIVE_SHA256 &&
            File(model, "am/final.mdl").isFile &&
            File(model, "conf/model.conf").isFile &&
            File(model, "graph/HCLr.fst").isFile
    }

    suspend fun install(context: Context, onProgress: suspend (Int) -> Unit): File {
        installLock.lock()
        try {
            installed(context)?.let { return it }
            return withContext(Dispatchers.IO) {
                val connectivity = context.getSystemService(ConnectivityManager::class.java)
                val active = connectivity.activeNetwork
                val capabilities = active?.let(connectivity::getNetworkCapabilities)
                check(capabilities?.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED) == true) {
                    "Connect to an unmetered network to download the 40 MB offline speech model."
                }
                val parent = root(context).apply { mkdirs() }
                val archive = File(parent, ".download-${UUID.randomUUID()}.zip")
                val staging = File(parent, ".install-${UUID.randomUUID()}")
                try {
                    download(archive, onProgress)
                    extract(archive, staging)
                    File(staging, ".family-room-sha256").writeText(ARCHIVE_SHA256)
                    val target = destination(context)
                    if (target.exists()) check(target.deleteRecursively()) { "Cannot replace incomplete speech model." }
                    check(staging.renameTo(target)) { "Cannot finish installing speech model." }
                    checkNotNull(installed(context)) { "Speech model did not pass installation checks." }
                } finally {
                    archive.delete()
                    staging.deleteRecursively()
                }
            }
        } finally {
            installLock.unlock()
        }
    }

    private suspend fun download(archive: File, onProgress: suspend (Int) -> Unit) {
        val connection = (URL(MODEL_URL).openConnection() as HttpURLConnection).apply {
            connectTimeout = 15_000
            readTimeout = 60_000
            instanceFollowRedirects = false
        }
        try {
            check(connection.responseCode == HttpURLConnection.HTTP_OK) { "Speech model download failed." }
            val expected = connection.contentLengthLong
            check(expected in 1..MAX_ARCHIVE_BYTES) { "Unexpected speech model download size." }
            val digest = MessageDigest.getInstance("SHA-256")
            var received = 0L
            var reported = -1
            connection.inputStream.use { input ->
                archive.outputStream().buffered().use { output ->
                    val buffer = ByteArray(64 * 1024)
                    while (true) {
                        val count = input.read(buffer)
                        if (count < 0) break
                        received += count
                        check(received <= MAX_ARCHIVE_BYTES) { "Speech model download exceeded expected size." }
                        digest.update(buffer, 0, count)
                        output.write(buffer, 0, count)
                        val percent = (received * 100 / expected).toInt()
                        if (percent >= reported + 5) {
                            reported = percent
                            withContext(Dispatchers.Main) { onProgress(percent) }
                        }
                    }
                }
            }
            check(received == expected) { "Speech model download was interrupted." }
            check(digest.digest().joinToString("") { "%02x".format(it) } == ARCHIVE_SHA256) {
                "Speech model integrity check failed."
            }
        } finally {
            connection.disconnect()
        }
    }

    internal fun extract(archive: File, staging: File) {
        check(staging.mkdirs()) { "Cannot prepare speech model storage." }
        var files = 0
        var expanded = 0L
        ZipInputStream(FileInputStream(archive).buffered()).use { zip ->
            while (true) {
                val entry = zip.nextEntry ?: break
                check(entry.name.startsWith("$NAME/")) { "Unexpected speech model archive layout." }
                val relative = entry.name.removePrefix("$NAME/").let {
                    if (entry.isDirectory) it.removeSuffix("/") else it
                }
                if (relative.isEmpty()) { zip.closeEntry(); continue }
                check(relative.split('/').none { it.isEmpty() || it == "." || it == ".." }) {
                    "Unsafe speech model archive path."
                }
                val target = File(staging, relative)
                check(target.canonicalPath.startsWith(staging.canonicalPath + File.separator)) {
                    "Unsafe speech model archive path."
                }
                if (entry.isDirectory) {
                    check(target.mkdirs() || target.isDirectory) { "Cannot create speech model folder." }
                } else {
                    check(++files <= 50) { "Speech model archive has too many files." }
                    check(target.parentFile?.mkdirs() == true || target.parentFile?.isDirectory == true) {
                        "Cannot create speech model folder."
                    }
                    target.outputStream().buffered().use { output ->
                        val buffer = ByteArray(64 * 1024)
                        while (true) {
                            val count = zip.read(buffer)
                            if (count < 0) break
                            expanded += count
                            check(expanded <= MAX_EXPANDED_BYTES) { "Speech model archive is too large." }
                            output.write(buffer, 0, count)
                        }
                    }
                }
                zip.closeEntry()
            }
        }
        check(files >= 5 && File(staging, "am/final.mdl").isFile &&
            File(staging, "conf/model.conf").isFile && File(staging, "graph/HCLr.fst").isFile) {
            "Speech model archive is incomplete."
        }
    }
}
