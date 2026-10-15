package com.familyroom.app

import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.spec.GCMParameterSpec
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class RecoveryKeyAccessTest {
    @Test
    fun unwrapsTheDeviceKeyInTheCoreRecoveryFormat() {
        val key = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()
        val recoveryKey = ByteArray(32).also(SecureRandom()::nextBytes)
        val nonce = ByteArray(12).also(SecureRandom()::nextBytes)
        val ciphertext = Cipher.getInstance("AES/GCM/NoPadding").run {
            init(Cipher.ENCRYPT_MODE, key, GCMParameterSpec(128, nonce))
            doFinal(recoveryKey)
        }

        val result = RecoveryKeyAccess.asHex(nonce + ciphertext, key)

        assertEquals(recoveryKey.joinToString("") { "%02x".format(it.toInt() and 0xff) }, result)
        recoveryKey.fill(0)
    }

    @Test
    fun rejectsTruncatedOrTamperedWrappedKeys() {
        val key = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()
        assertThrows(IllegalArgumentException::class.java) {
            RecoveryKeyAccess.decrypt(ByteArray(27), key)
        }

        val nonce = ByteArray(12)
        val ciphertext = ByteArray(48)
        ciphertext[0] = 1
        assertThrows(Exception::class.java) {
            RecoveryKeyAccess.decrypt(nonce + ciphertext, key)
        }
    }
}
