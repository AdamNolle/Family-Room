package com.familyroom.app

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
@Composable
internal fun LibraryScopeFilterBar(
    selectedKind: String,
    onKindChange: (String) -> Unit,
    selectedYear: Int,
    years: List<Int>,
    onYearChange: (Int) -> Unit,
    selectedMomentId: String,
    moments: List<Pair<String, String>>,
    onMomentChange: (String) -> Unit,
) {
    val yearMenuExpanded = remember { mutableStateOf(false) }
    val momentMenuExpanded = remember { mutableStateOf(false) }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .horizontalScroll(rememberScrollState())
            .testTag("library.scope_filters"),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        listOf("all" to "All media", "photo" to "Photos", "video" to "Videos").forEach { (kind, label) ->
            FilterChip(
                selected = selectedKind == kind,
                onClick = { onKindChange(kind) },
                label = { Text(label) },
                modifier = Modifier.testTag("library.filter.media.$kind"),
            )
        }
        Box {
            FilterChip(
                selected = selectedYear != 0,
                onClick = { yearMenuExpanded.value = true },
                label = { Text(if (selectedYear == 0) "All years" else selectedYear.toString()) },
                modifier = Modifier.testTag("library.filter.year"),
            )
            DropdownMenu(
                expanded = yearMenuExpanded.value,
                onDismissRequest = { yearMenuExpanded.value = false },
            ) {
                DropdownMenuItem(
                    text = { Text("All years") },
                    onClick = {
                        yearMenuExpanded.value = false
                        onYearChange(0)
                    },
                    modifier = Modifier.testTag("library.filter.year.all"),
                )
                years.forEach { year ->
                    DropdownMenuItem(
                        text = { Text(year.toString()) },
                        onClick = {
                            yearMenuExpanded.value = false
                            onYearChange(year)
                        },
                        modifier = Modifier.testTag("library.filter.year.$year"),
                    )
                }
            }
        }
        Box {
            FilterChip(
                selected = selectedMomentId.isNotBlank(),
                onClick = { momentMenuExpanded.value = true },
                label = {
                    Text(moments.firstOrNull { it.first == selectedMomentId }?.second ?: "All Moments")
                },
                modifier = Modifier.testTag("library.filter.moment"),
            )
            DropdownMenu(
                expanded = momentMenuExpanded.value,
                onDismissRequest = { momentMenuExpanded.value = false },
            ) {
                DropdownMenuItem(
                    text = { Text("All Moments") },
                    onClick = {
                        onMomentChange("")
                        momentMenuExpanded.value = false
                    },
                    modifier = Modifier.testTag("library.filter.moment.all"),
                )
                moments.forEach { (id, title) ->
                    DropdownMenuItem(
                        text = { Text(title) },
                        onClick = {
                            onMomentChange(id)
                            momentMenuExpanded.value = false
                        },
                        modifier = Modifier.testTag("library.filter.moment.$id"),
                    )
                }
            }
        }
    }
}
