package com.familyroom.app

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BackgroundSyncPolicyTest {
    @Test
    fun retriesWhenTheNetworkIsOffline() {
        assertTrue(shouldRetryBackgroundSync("offline", emptyList()))
    }

    @Test
    fun retriesWhenAnyRoomReportsARecoverableSyncError() {
        assertTrue(shouldRetryBackgroundSync("unmetered", listOf(emptyList(), listOf("gateway unavailable"))))
    }

    @Test
    fun doesNotRetryWhenEveryRoomSyncsCleanly() {
        assertFalse(shouldRetryBackgroundSync("metered", listOf(emptyList(), emptyList())))
    }

    @Test
    fun quotaExhaustionWaitsForThePeriodicRetryInsteadOfRapidBackoff() {
        assertFalse(shouldRetryBackgroundSync("unmetered", listOf(listOf("Upload paused: storage full"))))
    }
}
