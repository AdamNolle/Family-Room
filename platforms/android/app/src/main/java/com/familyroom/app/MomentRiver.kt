package com.familyroom.app

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale

@Composable
internal fun MomentRiverPanel(
    model: RoomModel,
    roomId: String,
    moments: List<JSONObject>,
    assets: List<JSONObject>,
    canEdit: Boolean,
    onOpen: (String) -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    val ordered = moments.sortedWith(
        compareBy<JSONObject> { it.optLong("start") }.thenBy { it.optString("id") },
    )
    val years = ordered.map { momentYear(it.optLong("start")) }.distinct().sortedDescending()
    var selectedYear by remember(roomId) { mutableIntStateOf(0) }
    val visible = if (selectedYear == 0) ordered else ordered.filter {
        momentYear(it.optLong("start")) == selectedYear
    }
    val scope = rememberCoroutineScope()

    Column(
        Modifier
            .fillMaxWidth()
            .testTag("moment.river.panel")
            .background(colors.surface.copy(alpha = 0.82f), RoundedCornerShape(24.dp))
            .border(1.dp, colors.outlineVariant.copy(alpha = 0.58f), RoundedCornerShape(24.dp))
            .padding(vertical = 18.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 18.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text("Memory river", style = MaterialTheme.typography.titleLarge)
                Text("A timeline shaped by the moments you keep", style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceVariant)
            }
            Text("${visible.size}", style = MaterialTheme.typography.labelLarge, color = colors.onSurfaceVariant,
                modifier = Modifier.testTag("moment.river.count"))
        }

        Row(
            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 14.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            FilterChip(
                selected = selectedYear == 0,
                onClick = { selectedYear = 0 },
                label = { Text("All years") },
                modifier = Modifier.testTag("moment.river.year.all"),
            )
            years.forEach { year ->
                FilterChip(
                    selected = selectedYear == year,
                    onClick = { selectedYear = year },
                    label = { Text(year.toString()) },
                    modifier = Modifier.testTag("moment.river.year.$year"),
                )
            }
        }

        LazyRow(
            Modifier.fillMaxWidth().height(208.dp),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 16.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            items(visible, key = { it.optString("id") }) { moment ->
                val id = moment.optString("id")
                val ids = moment.optJSONArray("asset_ids") ?: JSONArray()
                val count = ids.length()
                val featured = moment.optBoolean("featured")
                val cover = (0 until count)
                    .asSequence()
                    .map { ids.optString(it) }
                    .mapNotNull { assetId -> assets.firstOrNull { it.optString("id") == assetId } }
                    .firstOrNull()
                val date = SimpleDateFormat("MMM d, yyyy", Locale.getDefault())
                    .format(Date(moment.optLong("start") * 1000L))
                val width = (236 + count * 7).coerceIn(252, 340).dp

                Card(
                    Modifier
                        .width(width)
                        .height(196.dp)
                        .clickable { onOpen(id) }
                        .testTag("moment.river.card.$id"),
                    shape = RoundedCornerShape(20.dp),
                    colors = CardDefaults.cardColors(
                        containerColor = if (featured) colors.tertiaryContainer.copy(alpha = 0.72f) else colors.surfaceVariant.copy(alpha = 0.7f),
                    ),
                ) {
                    Row(Modifier.fillMaxWidth().padding(start = 14.dp, top = 12.dp, end = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(date, style = MaterialTheme.typography.labelMedium, color = colors.onSurfaceVariant)
                            Text(if (featured) "In the spotlight" else "Moment", style = MaterialTheme.typography.labelSmall, color = colors.primary)
                        }
                        if (canEdit) IconButton(
                            onClick = {
                                val assetIds = JSONArray().apply { for (index in 0 until ids.length()) put(ids.getString(index)) }
                                scope.launch {
                                    model.execute(
                                        JSONObject()
                                            .put("action", "save_moment")
                                            .put("room_id", roomId)
                                            .put("id", id)
                                            .put("title", moment.optString("title"))
                                            .put("asset_ids", assetIds)
                                            .put("featured", !featured),
                                    )
                                }
                            },
                            modifier = Modifier
                                .testTag("moment.river.feature.$id")
                                .semantics { contentDescription = if (featured) "Remove ${moment.optString("title")} from spotlight" else "Feature ${moment.optString("title")} on Home" },
                        ) {
                            Text(if (featured) "★" else "☆", color = if (featured) colors.primary else colors.onSurfaceVariant, style = MaterialTheme.typography.titleLarge)
                        } else if (featured) Text("★", color = colors.primary, style = MaterialTheme.typography.titleLarge)
                    }
                    Row(
                        Modifier.fillMaxWidth().weight(1f).padding(horizontal = 14.dp, vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        if (cover != null) {
                            MemoryThumbnail(model, cover, Modifier.size(82.dp, 78.dp))
                        } else {
                            Box(Modifier.size(82.dp, 78.dp).background(colors.surface, RoundedCornerShape(12.dp)))
                        }
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(5.dp)) {
                            Text(moment.optString("title", "Moment"), style = MaterialTheme.typography.titleMedium, maxLines = 2)
                            Text("$count ${if (count == 1) "memory" else "memories"}", style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceVariant)
                            Text("Open moment  ›", style = MaterialTheme.typography.labelMedium, color = colors.primary)
                        }
                    }
                }
            }
        }
    }
}

private fun momentYear(timestamp: Long): Int = Calendar.getInstance().run {
    timeInMillis = timestamp * 1000L
    get(Calendar.YEAR)
}
