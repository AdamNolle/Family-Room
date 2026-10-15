package com.familyroom.app

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.FilterChip
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import org.json.JSONObject

internal fun JSONObject.peopleIds(): Set<String> = buildSet {
    val people = optJSONArray("people") ?: return@buildSet
    for (index in 0 until people.length()) {
        people.optString(index).takeIf(String::isNotBlank)?.let(::add)
    }
}

@Composable
internal fun AssetPeopleMenu(
    assetPeople: Set<String>,
    people: List<JSONObject>,
    enabled: Boolean,
    onTogglePerson: (String) -> Unit,
    tag: String,
) {
    val expanded = androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(false) }
    Box {
        TextButton(
            onClick = { expanded.value = true },
            enabled = enabled,
            modifier = Modifier.testTag(tag),
        ) {
            Text("People · \${assetPeople.size}")
        }
        DropdownMenu(expanded = expanded.value, onDismissRequest = { expanded.value = false }) {
            for (person in people) {
                val id = person.optString("id")
                val selected = id in assetPeople
                DropdownMenuItem(
                    text = { Text(person.optString("name").ifBlank { "Unnamed person" }) },
                    onClick = {
                        expanded.value = false
                        onTogglePerson(id)
                    },
                    trailingIcon = { if (selected) Text("✓", color = MaterialTheme.colorScheme.primary) },
                    modifier = Modifier.testTag("$tag.$id"),
                )
            }
        }
    }
}

@Composable
internal fun PeopleFilterBar(
    people: List<JSONObject>,
    selectedPeople: Set<String>,
    requireAll: Boolean,
    onTogglePerson: (String) -> Unit,
    onRequireAllChange: (Boolean) -> Unit,
    onClear: () -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .background(colors.surface.copy(alpha = 0.94f), RoundedCornerShape(14.dp))
            .border(1.dp, colors.outlineVariant, RoundedCornerShape(14.dp))
            .padding(horizontal = 16.dp, vertical = 14.dp)
            .testTag("library.people_filter"),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Text("People", style = MaterialTheme.typography.titleSmall)
                Text(
                    "Show memories with any or all selected people.",
                    style = MaterialTheme.typography.bodySmall,
                    color = colors.onSurfaceVariant,
                )
            }
            if (selectedPeople.isNotEmpty()) {
                TextButton(
                    onClick = onClear,
                    modifier = Modifier.testTag("people.filter.clear"),
                ) {
                    Text("Clear")
                }
            }
        }
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            people.forEach { person ->
                val id = person.optString("id")
                val name = person.optString("name").ifBlank { "Unnamed person" }
                FilterChip(
                    selected = id in selectedPeople,
                    onClick = { onTogglePerson(id) },
                    label = { Text(name) },
                    modifier = Modifier.testTag("people.filter.$id"),
                )
            }
        }
        if (selectedPeople.isNotEmpty()) {
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Match", style = MaterialTheme.typography.labelMedium, color = colors.onSurfaceVariant)
                FilterChip(
                    selected = !requireAll,
                    onClick = { onRequireAllChange(false) },
                    label = { Text("Any selected") },
                    modifier = Modifier.testTag("people.filter.any"),
                )
                FilterChip(
                    selected = requireAll,
                    onClick = { onRequireAllChange(true) },
                    label = { Text("All selected") },
                    modifier = Modifier.testTag("people.filter.all"),
                )
            }
        }
    }
}
