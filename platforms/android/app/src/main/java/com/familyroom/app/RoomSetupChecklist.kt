package com.familyroom.app

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp

@Composable
internal fun RoomSetupChecklist(model: RoomModel, onAddFirstMedia: () -> Unit) {
    val room = model.room ?: return
    val context = LocalContext.current
    val role = room.optString("role")
    val canManageStorage = role == "owner" || role == "admin"

    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(18.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("Get your Room ready", style = MaterialTheme.typography.titleMedium)
            if (!canManageStorage) {
                Text("The Room owner manages shared storage and upload destinations. You can join without an account on their cloud provider.")
                return@Column
            }

            val sources = model.snapshot.rows("sources").filter { it.optString("room_id") == model.roomId }
        val hasPreferredStorage = sources.any { it.optBoolean("preferred") && !it.optBoolean("paused") }
        val folderWatches = model.snapshot.rows("watches").filter {
            it.optString("room_id") == model.roomId && it.optString("path").startsWith("content://")
        }
        val hasAutomaticUploads = folderWatches.isNotEmpty()
            val recoverySaved = context.getSharedPreferences("family-room-onboarding", 0)
                .getBoolean("recoveryAcknowledged", false)
            val hasFirstContribution = room.rows("assets").any {
                !it.optBoolean("deleted") && it.optString("kind") in setOf("photo", "video")
            }
        val availableSteps = listOf(hasPreferredStorage, hasAutomaticUploads, recoverySaved, hasFirstContribution).count { it }

            Text("$availableSteps of 4 steps. Choose a destination below, then set up recovery and add your first memories.", style = MaterialTheme.typography.bodySmall)
            SetupChecklistRow(
                title = "Choose preferred storage",
                done = hasPreferredStorage,
                detail = if (hasPreferredStorage) "A Room destination is ready." else "Continue to Upload destinations below.",
            )
            SetupChecklistRow(
                title = "Automatic uploads",
            done = hasAutomaticUploads,
            detail = when {
                folderWatches.any { !it.optBoolean("paused") } ->
                    "A selected folder is watching this Room."
                hasAutomaticUploads -> "Folder uploads are paused. Resume them in settings."
                else -> "Choose a device folder in Automatic uploads below."
            },
            )
            SetupChecklistRow(
                title = "Save the recovery key",
                done = recoverySaved,
                detail = if (recoverySaved) "You confirmed that you saved it separately." else "Reveal the key below, then confirm that you saved it separately.",
            )
            SetupChecklistRow(
                title = "Add the first photos or videos",
                done = hasFirstContribution,
                detail = if (hasFirstContribution) "This Room already has media." else "Manual imports are available from the Add button.",
            )
            if (!hasFirstContribution) {
                TextButton(onClick = onAddFirstMedia) { Text("Open the Room") }
            }
        }
    }
}

@Composable
private fun SetupChecklistRow(title: String, done: Boolean, detail: String) {
    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Text(if (done) "✓" else "○", color = MaterialTheme.colorScheme.primary)
        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(title, style = MaterialTheme.typography.labelLarge)
            Text(detail, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
