package com.familyroom.app

import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.async
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.coroutineScope
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class BackgroundSyncTest {
    @get:Rule
    val ui = createAndroidComposeRule<MainActivity>()

    @Test
    fun applicationSharesOneVaultAndSerializesNativeSnapshotCommands() = runBlocking {
        val application = ui.activity.application as FamilyRoomApplication
        val vault = application.openVault()
        val snapshots = coroutineScope {
            listOf(
                async { vault.command(JSONObject().put("action", "snapshot")) },
                async { vault.command(JSONObject().put("action", "snapshot")) },
            ).map { it.await() }
        }

        assertSame(vault, application.openVault())
        assertEquals(snapshots[0].optString("device_id"), snapshots[1].optString("device_id"))
    }
}
