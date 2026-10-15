package com.familyroom.app

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class RecoveryBackupPolicyTest {
    @Test
    fun restoreIsAvailableOnlyInAnOpenPristineLibrary() {
        assertTrue(canRestoreEncryptedBackup(libraryReady = true, roomCount = 0, pendingInvitationCount = 0))
        assertFalse(canRestoreEncryptedBackup(libraryReady = false, roomCount = 0, pendingInvitationCount = 0))
        assertFalse(canRestoreEncryptedBackup(libraryReady = true, roomCount = 1, pendingInvitationCount = 0))
        assertFalse(canRestoreEncryptedBackup(libraryReady = true, roomCount = 0, pendingInvitationCount = 1))
    }
}
