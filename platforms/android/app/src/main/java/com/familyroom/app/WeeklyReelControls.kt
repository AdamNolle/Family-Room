package com.familyroom.app

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import kotlinx.coroutines.launch
import org.json.JSONObject

@Composable
internal fun WeeklyReelSkipAction(model: RoomModel, roomId: String, storyId: String) {
    val scope = rememberCoroutineScope()
    var confirming by remember(storyId) { mutableStateOf(false) }
    TextButton(
        onClick = { confirming = true },
        modifier = Modifier.testTag("theater.skipReel.$storyId"),
    ) { Text("Skip reel") }
    if (confirming) {
        AlertDialog(
            onDismissRequest = { confirming = false },
            title = { Text("Skip this private reel?") },
            text = { Text("Only the reel draft is removed. Photos and videos in your Room stay in place.") },
            confirmButton = {
                TextButton(onClick = {
                    confirming = false
                    scope.launch {
                        model.execute(JSONObject().put("action", "skip_auto_reel")
                            .put("room_id", roomId).put("id", storyId))
                    }
                }, modifier = Modifier.testTag("theater.skipReel.confirm.$storyId")) {
                    Text("Skip reel")
                }
            },
            dismissButton = {
                TextButton(onClick = { confirming = false }) { Text("Keep reel") }
            },
        )
    }
}
