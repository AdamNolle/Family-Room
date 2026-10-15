package com.familyroom.app

import java.security.Key
import javax.crypto.Cipher
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

internal object RecoveryKeyAccess {
    private const val NONCE_BYTES = 12
    private const val TAG_BITS = 128
    private const val KEY_BYTES = 32

    fun decrypt(wrappedKey: ByteArray, wrappingKey: Key): ByteArray {
        require(wrappedKey.size >= NONCE_BYTES + KEY_BYTES + TAG_BITS / 8) {
            "The saved recovery key is incomplete."
        }
        val secretKey = wrappingKey as? SecretKey
            ?: error("The device recovery-key wrapper is unavailable.")
        val nonce = wrappedKey.copyOfRange(0, NONCE_BYTES)
        val ciphertext = wrappedKey.copyOfRange(NONCE_BYTES, wrappedKey.size)
        val clear = Cipher.getInstance("AES/GCM/NoPadding").run {
            init(Cipher.DECRYPT_MODE, secretKey, GCMParameterSpec(TAG_BITS, nonce))
            doFinal(ciphertext)
        }
        require(clear.size == KEY_BYTES) {
            clear.fill(0)
            "The saved recovery key has an invalid length."
        }
        return clear
    }

    fun asHex(wrappedKey: ByteArray, wrappingKey: Key): String {
        val clear = decrypt(wrappedKey, wrappingKey)
        return try {
            clear.joinToString("") { "%02x".format(it.toInt() and 0xff) }
        } finally {
            clear.fill(0)
        }
    }
}
