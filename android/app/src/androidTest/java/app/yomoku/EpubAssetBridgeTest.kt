package app.yomoku

import android.database.sqlite.SQLiteDatabase
import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.EpubAssetError
import app.yomoku.nativebridge.EpubAssetListener
import app.yomoku.nativebridge.EpubCoverListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.EpubImport
import app.yomoku.shared.Event
import app.yomoku.shared.ViewModel
import java.io.ByteArrayOutputStream
import java.io.File
import java.security.MessageDigest
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import java.util.zip.ZipEntry
import java.util.zip.ZipInputStream
import java.util.zip.ZipOutputStream
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class EpubAssetBridgeTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val path = "images/挿絵.png"
    private val payload = ByteArray(256) { it.toByte() }

    private class Effects : EffectsListener {
        val ready = LinkedBlockingQueue<Any>()

        override fun onEffects(requests: ByteArray) {
            ready.add(requests)
        }

        override fun onError(error: FfiError) {
            ready.add(error)
        }
    }

    private class Response : EpubAssetListener {
        val ready = LinkedBlockingQueue<Any>()

        override fun onAsset(mediaType: String, bytes: ByteArray) {
            check(Looper.myLooper() != Looper.getMainLooper())
            check(mediaType == "image/png")
            ready.add(bytes)
        }

        override fun onError(error: EpubAssetError) {
            check(Looper.myLooper() != Looper.getMainLooper())
            ready.add(error)
        }

        fun await(): Any = checkNotNull(ready.poll(60, TimeUnit.SECONDS))
    }

    private class CoverResponse : EpubCoverListener {
        val ready = LinkedBlockingQueue<Any>()

        override fun onCover(path: String, mediaType: String, sha256: ByteArray) {
            check(Looper.myLooper() != Looper.getMainLooper())
            ready.add(Triple(path, mediaType, sha256))
        }

        override fun onNoCover() {
            check(Looper.myLooper() != Looper.getMainLooper())
            ready.add(Unit)
        }

        override fun onError(error: EpubAssetError) {
            check(Looper.myLooper() != Looper.getMainLooper())
            ready.add(error)
        }

        fun await(): Any = checkNotNull(ready.poll(60, TimeUnit.SECONDS))
    }

    private fun cover(bridge: NativeBridge, id: Long): Any {
        val response = CoverResponse()
        instrumentation.runOnMainSync { bridge.loadEpubCoverReference(id, response) }
        return response.await()
    }

    private fun epub(): ByteArray {
        val original = FileImportFixtures.epub("assets", "<p>前<img src='$path' alt='挿絵'/>後</p>")
        val output = ByteArrayOutputStream()
        ZipInputStream(original.inputStream()).use { input ->
            ZipOutputStream(output).use { zip ->
                while (true) {
                    val entry = input.nextEntry ?: break
                    var bytes = input.readBytes()
                    if (entry.name == "book.opf") {
                        bytes =
                            bytes
                                .toString(Charsets.UTF_8)
                                .replace(
                                    "</manifest>",
                                    "<item id='image' href='$path' media-type='image/png' properties='cover-image'/></manifest>",
                                )
                                .toByteArray()
                    }
                    zip.putNextEntry(ZipEntry(entry.name))
                    zip.write(bytes)
                    zip.closeEntry()
                }
                zip.putNextEntry(ZipEntry(path))
                zip.write(payload)
                zip.closeEntry()
            }
        }
        return output.toByteArray()
    }

    private fun read(bridge: NativeBridge, id: Long, media: String = "image/png"): Any {
        val response = Response()
        // Enqueue from Android main; the callback and SQLite read belong to Rust's FIFO.
        instrumentation.runOnMainSync { bridge.loadEpubImageAsset(id, path, media, response) }
        return response.await()
    }

    @Test
    fun binaryAssetsReopenReadOnlyWithoutDictionariesAndRejectCorruptionThroughJni() {
        val context = instrumentation.targetContext
        val directory = File(context.noBackupFilesDir, "epub-assets-${System.nanoTime()}")
        assertTrue(directory.mkdir())
        val database = File(directory, "reader.sqlite3")
        val effects = Effects()
        try {
            val id =
                NativeBridge(database.path, effects, sudachiDictionaryProvider(context)).use {
                    bridge ->
                    val bytes = epub()
                    bridge.update(
                        Event.ImportEpub(EpubImport(bytes.map { it.toUByte() }, "assets.epub"))
                            .bincodeSerialize()
                    )
                    assertTrue(effects.ready.poll(90, TimeUnit.SECONDS) is ByteArray)
                    val view = ViewModel.bincodeDeserialize(bridge.view())
                    assertNull(view.documentError)
                    checkNotNull(view.importSummary).documentId.value
                }
            val noDictionary =
                object : DictionaryProvider {
                    override fun openVerifiedDictionary(): Long = error("asset read loaded Full")

                    override fun verifiedWordDictionaryPath(): String =
                        error("asset read loaded word index")
                }
            NativeBridge(database.path, effects, noDictionary).use { bridge ->
                val before = bridge.view()
                repeat(2) {
                    val reference = cover(bridge, id) as Triple<*, *, *>
                    assertEquals(path, reference.first)
                    assertEquals("image/png", reference.second)
                    assertArrayEquals(
                        MessageDigest.getInstance("SHA-256").digest(payload),
                        reference.third as ByteArray,
                    )
                }
                assertSame(Unit, cover(bridge, Long.MAX_VALUE))
                assertSame(
                    EpubAssetError.InvalidRequest,
                    runCatching { bridge.loadEpubCoverReference(0, CoverResponse()) }
                        .exceptionOrNull(),
                )
                repeat(2) { assertArrayEquals(payload, read(bridge, id) as ByteArray) }
                assertSame(EpubAssetError.InvalidDocument, read(bridge, id, "image/jpeg"))
                assertSame(EpubAssetError.NotFound, read(bridge, Long.MAX_VALUE))
                assertArrayEquals(before, bridge.view())
                SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE)
                    .use { sql ->
                        sql.execSQL("UPDATE epub_assets SET bytes = X'00'")
                    }
                assertSame(EpubAssetError.InvalidDocument, read(bridge, id))
                // Corrupt reads do not repair the BLOB or change reader state.
                assertSame(EpubAssetError.InvalidDocument, read(bridge, id))
                assertArrayEquals(before, bridge.view())
                SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE)
                    .use { sql ->
                        sql.execSQL("UPDATE epub_documents SET cover_path = 'missing.png'")
                    }
                assertSame(EpubAssetError.InvalidDocument, cover(bridge, id))
                assertSame(EpubAssetError.InvalidDocument, cover(bridge, id))
                assertArrayEquals(before, bridge.view())
                assertTrue(effects.ready.isEmpty())
            }
        } finally {
            directory.deleteRecursively()
        }
    }
}
