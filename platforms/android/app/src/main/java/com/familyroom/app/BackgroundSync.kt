package com.familyroom.app

import android.app.Application
import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.util.concurrent.TimeUnit

/** Holds one encrypted catalog engine shared by the UI and persistent workers. */
class FamilyRoomApplication : Application() {
    private val vaultMutex = Mutex()
    private var cachedVault: Vault? = null

    override fun onCreate() {
        super.onCreate()
        BackgroundSyncScheduler.ensurePeriodic(this)
    }

    suspend fun openVault(): Vault = vaultMutex.withLock {
        cachedVault ?: withContext(Dispatchers.IO) { Vault(this@FamilyRoomApplication) }
            .also { cachedVault = it }
    }
}

internal object BackgroundSyncScheduler {
    private const val PERIODIC_NAME = "family-room-periodic-sync"
    private const val IMMEDIATE_NAME = "family-room-background-sync"
    private const val WATCH_PERIODIC_NAME = "family-room-folder-import"
    private const val WATCH_IMMEDIATE_NAME = "family-room-folder-import-now"

    private fun constraints() = Constraints.Builder()
        .setRequiredNetworkType(NetworkType.CONNECTED)
        .build()

    fun ensurePeriodic(context: Context) {
        val request = PeriodicWorkRequestBuilder<RoomSyncWorker>(15, TimeUnit.MINUTES)
            .setConstraints(constraints())
            .build()
        WorkManager.getInstance(context).enqueueUniquePeriodicWork(
            PERIODIC_NAME,
            ExistingPeriodicWorkPolicy.KEEP,
            request,
        )
        val folderImport = PeriodicWorkRequestBuilder<RoomFolderImportWorker>(15, TimeUnit.MINUTES)
            .build()
        WorkManager.getInstance(context).enqueueUniquePeriodicWork(
            WATCH_PERIODIC_NAME,
            ExistingPeriodicWorkPolicy.KEEP,
            folderImport,
        )
    }

    fun enqueueNow(context: Context) {
        val request = OneTimeWorkRequestBuilder<RoomSyncWorker>()
            .setConstraints(constraints())
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 30, TimeUnit.SECONDS)
            .build()
        WorkManager.getInstance(context).enqueueUniqueWork(
            IMMEDIATE_NAME,
            ExistingWorkPolicy.KEEP,
            request,
        )
    }

    fun enqueueFolderImportNow(context: Context) {
        val request = OneTimeWorkRequestBuilder<RoomFolderImportWorker>()
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 30, TimeUnit.SECONDS)
            .build()
        WorkManager.getInstance(context).enqueueUniqueWork(
            WATCH_IMMEDIATE_NAME,
            ExistingWorkPolicy.KEEP,
            request,
        )
    }
}

internal class RoomFolderImportWorker(
    context: Context,
    parameters: WorkerParameters,
) : CoroutineWorker(context, parameters) {
    override suspend fun doWork(): Result {
        if (!File(applicationContext.filesDir, "Vault/catalog.fr").isFile) return Result.success()
        return try {
            val vault = (applicationContext as FamilyRoomApplication).openVault()
            val result = AndroidFolderImport.scan(applicationContext, vault)
            if (result.imported > 0) BackgroundSyncScheduler.enqueueNow(applicationContext)
            if (result.errors.isNotEmpty()) Result.retry() else Result.success()
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (_: Exception) {
            Result.retry()
        }
    }
}

internal fun shouldRetryBackgroundSync(networkState: String, roomErrors: List<List<String>>): Boolean =
    networkState == "offline" || roomErrors.flatten().any { it != "Upload paused: storage full" }

class RoomSyncWorker(context: Context, parameters: WorkerParameters) : CoroutineWorker(context, parameters) {
    override suspend fun doWork(): Result {
        if (!File(applicationContext.filesDir, "Vault/catalog.fr").isFile) return Result.success()
        val networkState = currentNetworkState(applicationContext)
        if (networkState == "offline") return Result.retry()

        return try {
            val vault = (applicationContext as FamilyRoomApplication).openVault()
            vault.command(JSONObject().put("action", "set_network_state").put("state", networkState))
            val rooms = vault.command(JSONObject().put("action", "snapshot")).optJSONArray("rooms")
                ?: return Result.success()
            val roomErrors = mutableListOf<List<String>>()
            for (index in 0 until rooms.length()) {
                currentCoroutineContext().ensureActive()
                val room = rooms.optJSONObject(index) ?: continue
                if (room.optString("role") == "removed") continue
                try {
                    val result = vault.command(
                        JSONObject()
                            .put("action", "sync_all")
                            .put("room_id", room.optString("id")),
                    )
                    val errors = result.optJSONArray("errors")
                    roomErrors += (0 until (errors?.length() ?: 0)).mapNotNull { errors?.optString(it) }
                } catch (cancelled: CancellationException) {
                    throw cancelled
                } catch (failure: Exception) {
                    roomErrors += listOf(failure.message ?: "Room sync failed")
                }
            }
            if (shouldRetryBackgroundSync(networkState, roomErrors)) Result.retry() else Result.success()
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (_: Exception) {
            Result.retry()
        }
    }
}

internal fun currentNetworkState(context: Context): String {
    val connectivity = context.getSystemService(ConnectivityManager::class.java)
    val network = connectivity.activeNetwork ?: return "offline"
    val capabilities = connectivity.getNetworkCapabilities(network) ?: return "unknown"
    if (!capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)) return "offline"
    return if (capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED)) "unmetered" else "metered"
}
