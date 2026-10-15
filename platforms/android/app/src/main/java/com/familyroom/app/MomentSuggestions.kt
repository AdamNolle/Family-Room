package com.familyroom.app

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import org.json.JSONArray
import org.json.JSONObject
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@Composable
internal fun MomentSuggestionsPanel(
    suggestions: List<JSONObject>,
    assets: List<JSONObject>,
    onSave: (title: String, assetIds: List<String>) -> Unit,
) {
    var activeSuggestion by remember { mutableStateOf<JSONObject?>(null) }

    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("Possible Moments", style = MaterialTheme.typography.titleLarge)
        Text(
            "Grouped by capture time. Review the selection before saving.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        suggestions.take(3).forEach { suggestion ->
            val date = remember(suggestion) {
                SimpleDateFormat("MMM d, yyyy", Locale.getDefault())
                    .format(Date(suggestion.optLong("start") * 1_000L))
            }
            Card(
                onClick = { activeSuggestion = suggestion },
                modifier = Modifier.fillMaxWidth().testTag("moment.suggestion.card"),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Text(date, style = MaterialTheme.typography.titleMedium)
                        Text(
                            "${suggestion.optJSONArray("asset_ids")?.length() ?: 0} photos and videos",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Text("Review →", color = MaterialTheme.colorScheme.primary)
                }
            }
        }
    }

    activeSuggestion?.let { suggestion ->
        MomentSuggestionReviewDialog(
            suggestion = suggestion,
            assets = assets,
            onDismiss = { activeSuggestion = null },
            onSave = { title, ids ->
                onSave(title, ids)
                activeSuggestion = null
            },
        )
    }
}

@Composable
private fun MomentSuggestionReviewDialog(
    suggestion: JSONObject,
    assets: List<JSONObject>,
    onDismiss: () -> Unit,
    onSave: (title: String, assetIds: List<String>) -> Unit,
) {
    val originalIds = remember(suggestion) {
        suggestion.optJSONArray("asset_ids").stringItems().toSet()
    }
    val date = remember(suggestion) {
        SimpleDateFormat("MMM d, yyyy", Locale.getDefault())
            .format(Date(suggestion.optLong("start") * 1_000L))
    }
    var title by remember(suggestion) { mutableStateOf("Moment · $date") }
    var selectedIds by remember(suggestion) { mutableStateOf(originalIds) }
    val selectedAssets = assets.filter { it.optString("id") in originalIds }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Review Moment") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Text(
                    "This draft was grouped by capture time. Change the title or selection before adding it to the Room.",
                    style = MaterialTheme.typography.bodySmall,
                )
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    label = { Text("Moment title") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("moment.suggestion.title"),
                )
                LazyColumn(modifier = Modifier.heightIn(max = 300.dp)) {
                    items(selectedAssets, key = { it.optString("id") }) { asset ->
                        val id = asset.optString("id")
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Checkbox(
                                checked = id in selectedIds,
                                onCheckedChange = { checked ->
                                    selectedIds = if (checked) selectedIds + id else selectedIds - id
                                },
                                modifier = Modifier.testTag("moment.suggestion.asset.$id"),
                            )
                            Text(
                                asset.optString("caption").ifBlank { asset.optString("filename") },
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                    }
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onSave(title.trim(), selectedIds.toList()) },
                enabled = title.isNotBlank() && selectedIds.isNotEmpty(),
                modifier = Modifier.testTag("moment.suggestion.save"),
            ) { Text("Save Moment") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

internal fun JSONArray?.stringItems(): List<String> {
    if (this == null) return emptyList()
    return (0 until length()).mapNotNull { index -> optString(index).takeIf(String::isNotBlank) }
}
