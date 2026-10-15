package com.familyroom.app

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID
import java.util.zip.ZipEntry
import java.util.zip.ZipException
import java.util.zip.ZipOutputStream

@RunWith(AndroidJUnit4::class)
class OfflineSpeechModelTest {
    @Test fun rejectsArchiveTraversalWithoutWritingOutsideInstall() {
        val root = temporaryRoot()
        try {
            val archive = zip(root, "vosk-model-small-en-us-0.15/../escape" to "bad")
            val failure = assertThrows(Exception::class.java) {
                OfflineSpeechModel.extract(archive, File(root, "stage"))
            }
            assertTrue(failure is IllegalStateException || failure is ZipException)
            assertFalse(File(root, "escape").exists())
        } finally {
            root.deleteRecursively()
        }
    }

    @Test fun rejectsUnexpectedArchiveRootAndTooManyFiles() {
        val root = temporaryRoot()
        try {
            val foreign = zip(root, "other-model/am/final.mdl" to "bad")
            assertThrows(IllegalStateException::class.java) {
                OfflineSpeechModel.extract(foreign, File(root, "foreign-stage"))
            }
            val entries = (0..50).map { "vosk-model-small-en-us-0.15/data/$it" to "x" }.toTypedArray()
            val oversized = zip(root, *entries)
            assertThrows(IllegalStateException::class.java) {
                OfflineSpeechModel.extract(oversized, File(root, "too-many-stage"))
            }
        } finally {
            root.deleteRecursively()
        }
    }

    private fun temporaryRoot(): File = File(
        InstrumentationRegistry.getInstrumentation().targetContext.cacheDir,
        "speech-archive-test-${UUID.randomUUID()}",
    ).apply { mkdirs() }

    private fun zip(root: File, vararg entries: Pair<String, String>): File {
        val archive = File(root, "archive-${UUID.randomUUID()}.zip")
        ZipOutputStream(archive.outputStream()).use { zip ->
            entries.forEach { (name, content) ->
                zip.putNextEntry(ZipEntry(name))
                zip.write(content.toByteArray())
                zip.closeEntry()
            }
        }
        return archive
    }
}
