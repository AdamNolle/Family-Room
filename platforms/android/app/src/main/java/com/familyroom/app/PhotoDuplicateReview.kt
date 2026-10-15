package com.familyroom.app

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import org.json.JSONObject

@Composable
internal fun PhotoDuplicateReviewPanel(
    model: RoomModel,
    response: JSONObject?,
    scanning: Boolean,
    assets: List<JSONObject>,
    onScan: () -> Unit,
    onOpen: (JSONObject) -> Unit,
) {
    val matches = response?.rows("suggestions").orEmpty()
    ElevatedCard(
        modifier = Modifier.fillMaxWidth().testTag("library.similar_photos.panel"),
        colors = CardDefaults.elevatedCardColors(
            containerColor = MaterialTheme.colorScheme.surface.copy(alpha = 0.94f),
        ),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(18.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(
                "Possible look-alikes",
                style = MaterialTheme.typography.titleLarge.copy(fontFamily = RoomDisplay),
                color = MaterialTheme.colorScheme.onSurface,
            )
            Text(
                "A private visual check. Compare each suggestion before making changes.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Button(
                onClick = onScan,
                enabled = !scanning,
                modifier = Modifier.testTag("library.similar_photos.scan"),
            ) {
                Text(if (scanning) "Checking photos…" else "Find similar photos")
            }

            if (response != null) {
                Text(
                    "${response.optInt("indexed_photos")} checked on this device · " +
                        "${response.optInt("unavailable_locally")} originals unavailable · " +
                        "${response.optInt("unsupported_or_oversized")} skipped",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (matches.isEmpty()) {
                    Text(
                        "No close visual matches were found in the photos checked.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    matches.take(30).forEachIndexed { index, pair ->
                        val first = pair.optJSONObject("first")
                        val second = pair.optJSONObject("second")
                        val firstAsset = assets.firstOrNull {
                            it.optString("id") == first?.optString("asset_id")
                        }
                        val secondAsset = assets.firstOrNull {
                            it.optString("id") == second?.optString("asset_id")
                        }
                        ElevatedCard(
                            modifier = Modifier.fillMaxWidth()
                                .testTag("library.similar_photos.pair.$index"),
                            colors = CardDefaults.elevatedCardColors(
                                containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.65f),
                            ),
                        ) {
                            Row(
                                modifier = Modifier.fillMaxWidth().padding(10.dp),
                                horizontalArrangement = Arrangement.spacedBy(10.dp),
                            ) {
                                firstAsset?.let { DuplicatePhotoMember(model, it, onOpen, Modifier.weight(1f)) }
                                secondAsset?.let { DuplicatePhotoMember(model, it, onOpen, Modifier.weight(1f)) }
                            }
                        }
                    }
                    if (response.optBoolean("comparison_limit_reached") ||
                        response.optBoolean("result_limit_reached")
                    ) {
                        Text(
                            "Showing a bounded set of matches. Review these before checking again.",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun DuplicatePhotoMember(
    model: RoomModel,
    asset: JSONObject,
    onOpen: (JSONObject) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(6.dp)) {
        MemoryThumbnail(
            model,
            asset,
            Modifier.fillMaxWidth().height(104.dp)
                .clip(RoundedCornerShape(12.dp))
                .border(1.dp, MaterialTheme.colorScheme.onSurface.copy(alpha = 0.72f), RoundedCornerShape(12.dp)),
        )
        Text(
            asset.optString("caption").ifBlank { asset.optString("filename", "Photo") },
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurface,
            maxLines = 2,
        )
        TextButton(
            onClick = { onOpen(asset) },
            modifier = Modifier.testTag("library.similar_photos.open.${asset.optString("id")}"),
        ) { Text("Review photo") }
    }
}
