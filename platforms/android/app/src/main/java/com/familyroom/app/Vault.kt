package com.familyroom.app

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.spec.GCMParameterSpec
import java.io.File
import org.json.JSONObject
import uniffi.family_core.FamilyCore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

class Vault(private val context: Context) {
    private val recoveryKeyFile = File(context.filesDir, "key.gcm")
    private val core: FamilyCore
    private val commandMutex = Mutex()
    init {
        val root = File(context.filesDir, "Vault").apply { mkdirs() }
        val keys = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        val alias = "family-room-vault"
        if (!keys.containsAlias(alias)) {
            if (File(root,"catalog.fr").exists()) error("Recover the vault key before opening this library")
            KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES,"AndroidKeyStore").apply {
                init(KeyGenParameterSpec.Builder(alias,KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build()); generateKey()
            }
        }
        val wrappingKey = keys.getKey(alias,null)
        val file = recoveryKeyFile
        val seed = if (file.exists()) {
            RecoveryKeyAccess.decrypt(file.readBytes(), wrappingKey)
        } else {
            if (File(root,"catalog.fr").exists()) error("Recover the vault key before opening this library")
            val random = ByteArray(32).also { SecureRandom().nextBytes(it) }
            val cipher=Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE,wrappingKey) }
            val pending=File(context.filesDir,"key.gcm.pending"); pending.writeBytes(cipher.iv+cipher.doFinal(random)); check(pending.renameTo(file)); random
        }
        try { core = FamilyCore.open(root.absolutePath,seed.joinToString("") { "%02x".format(it.toInt() and 255) }) }
        finally { seed.fill(0) }
    }
    fun recoveryKeyForBackup(): String {
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        val wrappingKey = keyStore.getKey("family-room-vault", null)
            ?: error("The device recovery-key wrapper is unavailable.")
        return RecoveryKeyAccess.asHex(recoveryKeyFile.readBytes(), wrappingKey)
    }

    suspend fun command(value: JSONObject): JSONObject = commandMutex.withLock {
        withContext(Dispatchers.IO) { JSONObject(core.command(value.toString())) }
    }
}
