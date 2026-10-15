package com.familyroom.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BulkMediaPolicyTest {
    @Test
    fun selectionNeverExceedsTheBatchLimitAndStillAllowsDeselection() {
        val selected = (0 until MAX_SELECTED_MEMORIES).map { "asset-$it" }.toSet()

        assertEquals(selected, updateBoundedSelection(selected, "extra", true))
        assertEquals(MAX_SELECTED_MEMORIES - 1, updateBoundedSelection(selected, "asset-0", false).size)
        assertEquals(selected, updateBoundedSelection(selected, "asset-0", true))
    }

    @Test
    fun contributorsCanOnlyTrashTheirOwnSelection() {
        assertTrue(canChangeDeletionForSelection("contributor", "device-a", listOf("device-a")))
        assertTrue(canChangeDeletionForSelection("contributor", "device-a", listOf("device-a", "device-a")))
        assertFalse(canChangeDeletionForSelection("contributor", "device-a", listOf("device-a", "device-b")))
        assertFalse(canChangeDeletionForSelection("viewer", "device-a", listOf("device-a")))
        assertTrue(canChangeDeletionForSelection("admin", "device-a", listOf("device-b")))
        assertFalse(canChangeDeletionForSelection("owner", "device-a", emptyList()))
    }
}
