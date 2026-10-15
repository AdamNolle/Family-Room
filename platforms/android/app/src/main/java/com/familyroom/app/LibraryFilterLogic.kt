package com.familyroom.app

import java.time.Instant
import java.time.ZoneId

internal fun matchesPeopleSelection(
    assetPeople: Set<String>,
    selectedPeople: Set<String>,
    requireAll: Boolean,
): Boolean = selectedPeople.isEmpty() ||
    if (requireAll) assetPeople.containsAll(selectedPeople) else assetPeople.any(selectedPeople::contains)

internal fun togglePersonTag(assetPeople: Set<String>, personId: String): Set<String> =
    if (personId in assetPeople) assetPeople - personId else assetPeople + personId

internal fun capturedYear(capturedAtEpochSeconds: Long, zone: ZoneId = ZoneId.systemDefault()): Int? =
    runCatching { Instant.ofEpochSecond(capturedAtEpochSeconds).atZone(zone).year }.getOrNull()

internal fun matchesLibraryScope(
    kind: String,
    capturedAtEpochSeconds: Long,
    selectedKind: String,
    selectedYear: Int,
    zone: ZoneId = ZoneId.systemDefault(),
): Boolean {
    if (selectedKind != "all" && selectedKind != kind) return false
    return selectedYear == 0 || capturedYear(capturedAtEpochSeconds, zone) == selectedYear
}

internal fun matchesMomentSelection(
    assetId: String,
    selectedMomentId: String,
    momentAssetIds: Set<String>,
): Boolean = selectedMomentId.isBlank() || assetId in momentAssetIds

internal fun availableCaptureYears(
    captureTimesEpochSeconds: Iterable<Long>,
    zone: ZoneId = ZoneId.systemDefault(),
): List<Int> = captureTimesEpochSeconds.mapNotNull { capturedYear(it, zone) }.distinct().sortedDescending()
