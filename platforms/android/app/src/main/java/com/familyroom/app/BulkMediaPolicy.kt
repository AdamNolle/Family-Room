package com.familyroom.app

internal const val MAX_SELECTED_MEMORIES = 500

internal fun updateBoundedSelection(
    selected: Set<String>,
    assetId: String,
    isSelected: Boolean,
    maximum: Int = MAX_SELECTED_MEMORIES,
): Set<String> = when {
    !isSelected -> selected - assetId
    assetId in selected || selected.size < maximum -> selected + assetId
    else -> selected
}

internal fun canChangeDeletionForSelection(
    role: String,
    currentDeviceId: String,
    contributors: Collection<String>,
): Boolean {
    if (contributors.isEmpty()) return false
    return role in setOf("owner", "admin") ||
        (role == "contributor" && contributors.all { it == currentDeviceId })
}
