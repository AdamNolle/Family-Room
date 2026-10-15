package com.familyroom.app

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import java.io.File
import java.util.UUID
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

@Composable
internal fun RecoveryBackupSettings(model: RoomModel, canRestore: Boolean) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var recoveryKey by remember { mutableStateOf("") }
    var recoveryAcknowledged by remember {
        mutableStateOf(context.getSharedPreferences("family-room-onboarding", 0).getBoolean("recoveryAcknowledged", false))
    }
    var enteredRecoveryKey by remember { mutableStateOf("") }
    var preparedBackupPath by rememberSaveable { mutableStateOf("") }
    var status by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }

    val backupExporter = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/zip"),
    ) { destination ->
        val prepared = preparedBackupPath
        preparedBackupPath = ""
        scope.launch {
            val file = File(prepared)
            try {
                if (destination == null || prepared.isBlank()) {
                    status = "Backup export canceled."
                    return@launch
                }
                busy = true
                withContext(Dispatchers.IO) {
                    val output = context.contentResolver.openOutputStream(destination)
                        ?: error("The selected location cannot save this backup.")
                    output.use { target -> file.inputStream().use { it.copyTo(target) } }
                }
                status = "Encrypted backup saved. Keep the recovery key separately."
            } catch (failure: Exception) {
                status = failure.message ?: "The encrypted backup could not be saved."
            } finally {
                withContext(Dispatchers.IO) { file.delete() }
                busy = false
            }
        }
    }

    val backupImporter = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { source ->
        if (source == null) {
            status = "Backup restore canceled."
        } else {
            scope.launch {
                busy = true
                val file = File(context.cacheDir, "family-room-restore-${UUID.randomUUID()}.frbackup")
                try {
                    val key = enteredRecoveryKey.trim()
                    require(key.isNotEmpty()) { "Enter the recovery key for this backup first." }
                    withContext(Dispatchers.IO) {
                        val input = context.contentResolver.openInputStream(source)
                            ?: error("The selected backup cannot be opened.")
                        input.use { stream -> file.outputStream().use { stream.copyTo(it) } }
                    }
                    model.command(
                        JSONObject()
                            .put("action", "restore")
                            .put("path", file.absolutePath)
                            .put("recovery_key", key),
                    )
                    model.reload()
                    enteredRecoveryKey = ""
                    recoveryKey = ""
                    status = "Backup restored. Review storage sources before syncing."
                } catch (failure: Exception) {
                    status = failure.message ?: "The encrypted backup could not be restored."
                } finally {
                    withContext(Dispatchers.IO) { file.delete() }
                    busy = false
                }
            }
        }
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(
            modifier = Modifier.padding(18.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text("Recovery and backup", style = MaterialTheme.typography.titleMedium)
            Text(
                "The backup contains your encrypted catalog and available originals. Save its recovery key somewhere separate from the backup.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            if (recoveryKey.isEmpty()) {
                TextButton(
                    enabled = model.ready && !busy,
                    onClick = {
                        scope.launch {
                            try {
                                recoveryKey = withContext(Dispatchers.IO) {
                                    model.recoveryKeyForBackup()
                                }
                                status = "Copy this key to a separate secure place."
                            } catch (failure: Exception) {
                                status = failure.message ?: "The recovery key is unavailable."
                            }
                        }
                    },
                ) { Text("Show recovery key") }
            } else {
                SelectionContainer {
                    Text(
                        recoveryKey,
                        style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    )
                }
                TextButton(onClick = { recoveryKey = "" }, enabled = !busy) {
                    Text("Hide recovery key")
                }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Checkbox(
                        checked = recoveryAcknowledged,
                        onCheckedChange = { saved ->
                            recoveryAcknowledged = saved
                            context.getSharedPreferences("family-room-onboarding", 0)
                                .edit().putBoolean("recoveryAcknowledged", saved).apply()
                        },
                    )
                    Text("I saved this recovery key separately", style = MaterialTheme.typography.bodySmall)
                }
            }
            Button(
                enabled = model.ready && !busy,
                onClick = {
                    scope.launch {
                        busy = true
                        val file = File(context.cacheDir, "family-room-backup-${UUID.randomUUID()}.frbackup")
                        try {
                            model.command(
                                JSONObject()
                                    .put("action", "backup")
                                    .put("path", file.absolutePath),
                            )
                            preparedBackupPath = file.absolutePath
                            backupExporter.launch("Family-Room.frbackup")
                        } catch (failure: Exception) {
                            withContext(Dispatchers.IO) { file.delete() }
                            status = failure.message ?: "The encrypted backup could not be prepared."
                        } finally {
                            busy = false
                        }
                    }
                },
            ) { Text(if (busy) "Preparing…" else "Create encrypted backup") }

            if (canRestore) {
                OutlinedTextField(
                    value = enteredRecoveryKey,
                    onValueChange = { enteredRecoveryKey = it },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text("Backup recovery key") },
                    visualTransformation = PasswordVisualTransformation(),
                    singleLine = true,
                    enabled = !busy,
                )
                Button(
                    enabled = model.ready && !busy && enteredRecoveryKey.isNotBlank(),
                    onClick = {
                        backupImporter.launch(arrayOf("application/zip", "application/octet-stream", "*/*"))
                    },
                ) { Text("Restore into this empty library") }
            } else {
                Text(
                    "Restore is available only before a Room is created or joined. It never replaces an existing library.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            if (status.isNotBlank()) {
                Text(status, style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}
