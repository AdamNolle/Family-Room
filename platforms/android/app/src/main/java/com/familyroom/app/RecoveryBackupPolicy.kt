package com.familyroom.app

internal fun canRestoreEncryptedBackup(
    libraryReady: Boolean,
    roomCount: Int,
    pendingInvitationCount: Int,
): Boolean = libraryReady && roomCount == 0 && pendingInvitationCount == 0
