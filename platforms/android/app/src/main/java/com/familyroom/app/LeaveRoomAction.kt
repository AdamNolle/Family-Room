package com.familyroom.app

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.json.JSONObject

@Composable
internal fun LeaveRoomAction(model: RoomModel, roomId: String, role: String) {
    var confirmationOpen by remember(roomId) { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Leave Room", style = MaterialTheme.typography.titleMedium)
        if (role == "owner") {
            Text(
                "Transfer ownership to another Member before leaving. This keeps an owner in the Room.",
                style = MaterialTheme.typography.bodySmall,
            )
        } else {
            Text(
                "This removes this device’s Room keys and storage connections. Ask an owner to remove this device from the shared roster to revoke its grant for the Room.",
                style = MaterialTheme.typography.bodySmall,
            )
            Button(
                onClick = { confirmationOpen = true },
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
            ) {
                Text(if (role == "removed") "Forget this Room" else "Leave Room")
            }
        }
    }
    if (confirmationOpen) {
        AlertDialog(
            onDismissRequest = { confirmationOpen = false },
            title = { Text(if (role == "removed") "Forget this Room?" else "Leave this Room on this device?") },
            text = {
                Text(
                    "This device will forget its Room keys and stop syncing this Room. Ask an owner to remove this device from the shared roster to revoke its grant. Other Members keep their copies; a saved device backup may restore this Room.",
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmationOpen = false
                        scope.launch {
                            model.execute(
                                JSONObject().put("action", "leave_room").put("room_id", roomId),
                            )
                        }
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                ) {
                    Text(if (role == "removed") "Forget Room" else "Leave Room")
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmationOpen = false }) { Text("Cancel") }
            },
        )
    }
}
