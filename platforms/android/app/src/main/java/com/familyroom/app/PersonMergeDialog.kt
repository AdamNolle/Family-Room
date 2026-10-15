package com.familyroom.app

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import org.json.JSONObject

@Composable
fun PersonMergeDialog(
    people: List<JSONObject>,
    onDismiss: () -> Unit,
    onMerge: (String, String) -> Unit,
) {
    val personIds = people.map { it.optString("id") }
    var sourceId by remember(personIds) { mutableStateOf(personIds.firstOrNull().orEmpty()) }
    var targetId by remember(personIds) {
        mutableStateOf(personIds.drop(1).firstOrNull().orEmpty())
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Merge duplicate People") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("Choose the duplicate profile and the one to keep. This changes People labels and tags only; it never invites anyone or grants Room access.")
                PersonMergePicker(
                    label = "Merge this profile",
                    people = people,
                    selectedId = sourceId,
                    excludedId = "",
                    testTag = "people.merge.source",
                    onSelected = { selected ->
                        sourceId = selected
                        if (targetId == selected) {
                            targetId = personIds.firstOrNull { it != selected }.orEmpty()
                        }
                    },
                )
                PersonMergePicker(
                    label = "Keep this profile",
                    people = people,
                    selectedId = targetId,
                    excludedId = sourceId,
                    testTag = "people.merge.target",
                    onSelected = { targetId = it },
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onMerge(sourceId, targetId) },
                enabled = sourceId.isNotBlank() && targetId.isNotBlank() && sourceId != targetId,
                modifier = Modifier.testTag("people.merge.confirm"),
            ) {
                Text("Merge")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel") }
        },
    )
}

@Composable
private fun PersonMergePicker(
    label: String,
    people: List<JSONObject>,
    selectedId: String,
    excludedId: String,
    testTag: String,
    onSelected: (String) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }
    val selectedName = people.firstOrNull { it.optString("id") == selectedId }
        ?.optString("name") ?: "Choose a Person"
    Box {
        OutlinedButton(
            onClick = { expanded = true },
            modifier = Modifier.fillMaxWidth().testTag(testTag),
        ) {
            Column(horizontalAlignment = androidx.compose.ui.Alignment.Start) {
                Text(label, style = MaterialTheme.typography.labelSmall)
                Text(selectedName, style = MaterialTheme.typography.bodyMedium)
            }
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            people.filter { it.optString("id") != excludedId }.forEach { person ->
                DropdownMenuItem(
                    text = { Text(person.optString("name", "Person")) },
                    onClick = {
                        onSelected(person.optString("id"))
                        expanded = false
                    },
                    modifier = Modifier.testTag("people.merge.person.${person.optString("id")}"),
                )
            }
        }
    }
}
