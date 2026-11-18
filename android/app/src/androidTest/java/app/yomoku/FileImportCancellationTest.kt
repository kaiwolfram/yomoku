package app.yomoku

import android.content.ContentResolver
import android.content.res.AssetFileDescriptor
import android.database.Cursor
import android.database.CursorWrapper
import android.database.sqlite.SQLiteDatabase
import android.net.Uri
import android.os.Looper
import android.os.ParcelFileDescriptor
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.Event
import app.yomoku.shared.ImportOutcome
import app.yomoku.shared.TextEncoding
import java.io.FileInputStream
import java.nio.file.Files
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Semaphore
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class FileImportCancellationTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private enum class Checkpoint {
        QUERY,
        METADATA,
        BEFORE_OPEN,
        OPEN,
        READ,
        EOF,
        CLOSE,
        ERROR,
    }

    @Test
    fun cancellingDuringEncodingDropsTheOldEventAndAllowsAFreshImport() {
        val core = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session
        compose.waitUntil(30_000) { !core.view.storageBusy }
        val gate = Gate()
        val encodes = AtomicInteger()
        var retired: Thread? = null
        val prefix = "encode-${System.nanoTime()}"
        val stale = FileImportFixtures.uri("$prefix-old.txt", "\uFEFF$prefix 猫。".toByteArray())
        val fresh = FileImportFixtures.uri("$prefix-new.txt", "\uFEFF$prefix 犬。".toByteArray())
        val previousObserver = core.onViewChanged
        val initialCount = core.view.documents.size
        lateinit var imports: FileImports
        compose.runOnIdle {
            imports =
                FileImports(
                    ContentResolver.wrap(TxtFixtureProvider()),
                    core,
                    encodeFileEvent = { event ->
                        assertNotSame(Looper.getMainLooper().thread, Thread.currentThread())
                        if (encodes.incrementAndGet() == 1) {
                            retired = Thread.currentThread()
                            gate.block()
                        }
                        event.bincodeSerialize()
                    },
                )
            imports.start(stale)
        }
        try {
            gate.awaitEntered()
            compose.runOnIdle {
                assertTrue(imports.active)
                assertFalse(core.view.storageBusy)
                imports.cancel()
                assertFalse(imports.active)
                imports.start(fresh)
            }
            compose.waitUntil(90_000) { !imports.active && !core.view.storageBusy }
            assertEquals(ImportOutcome.CREATED, imports.result?.outcome)
            gate.release()
            retired?.join(30_000)
            assertFalse(checkNotNull(retired).isAlive)
            compose.runOnIdle {
                assertEquals(initialCount + 1, core.view.documents.size)
                assertEquals("$prefix-new.txt", imports.result?.title)
                assertEquals(2, encodes.get())
            }
        } finally {
            gate.release()
            compose.runOnIdle {
                imports.close()
                core.onViewChanged = previousObserver
            }
        }
    }

    @Test
    fun encodedImportWaitsForTheStorageSlotAndSubmitsOnlyOnce() {
        val activity = compose.activity
        val directory =
            Files.createTempDirectory(activity.cacheDir.toPath(), "prepared-import-").toFile()
        val database = directory.resolve("storage.sqlite3")
        lateinit var core: CoreSession
        lateinit var imports: FileImports
        val gate = Gate()
        val encoded = CountDownLatch(1)
        val encodes = AtomicInteger()
        compose.runOnIdle {
            core = CoreSession(database.path, sudachiDictionaryProvider(activity))
            core.onClosed = { directory.deleteRecursively() }
            imports =
                FileImports(
                    ContentResolver.wrap(TxtFixtureProvider()),
                    core,
                    encodeFileEvent = { event ->
                        if (encodes.incrementAndGet() == 1) gate.block()
                        event.bincodeSerialize().also { encoded.countDown() }
                    },
                )
        }
        try {
            compose.waitUntil(30_000) { !core.view.storageBusy }
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use {
                lock ->
                lock.beginTransactionNonExclusive()
                try {
                    compose.runOnIdle {
                        core.dispatch(Event.CreatePaste("先行\n猫"))
                        assertTrue(core.view.storageBusy)
                        imports.start(
                            FileImportFixtures.uri("queued.txt", "\uFEFF犬。".toByteArray())
                        )
                    }
                    gate.awaitEntered()
                    gate.release()
                    assertTrue(encoded.await(30, TimeUnit.SECONDS))
                    compose.runOnIdle {
                        assertTrue(core.view.storageBusy)
                        assertTrue(imports.active)
                        assertNull(imports.result)
                    }
                    lock.setTransactionSuccessful()
                } finally {
                    lock.endTransaction()
                }
            }
            compose.waitUntil(90_000) { !imports.active && !core.view.storageBusy }
            assertEquals(1, encodes.get())
            assertEquals(ImportOutcome.CREATED, imports.result?.outcome)
            assertEquals(listOf("queued.txt", "先行"), core.view.documents.map { it.title })

            compose.runOnIdle { core.dispatch(Event.LoadLibraryWordDistributions) }
            compose.waitUntil(30_000) { !core.view.storageBusy }
            val distributions = checkNotNull(core.view.libraryWordDistributions)
            var callbacks = 0
            compose.runOnIdle {
                core.onCallbackMeasured = { callbacks++ }
                imports.start(FileImportFixtures.uri("unsupported.zip", byteArrayOf()))
            }
            compose.waitUntil(30_000) { !imports.active }
            compose.runOnIdle {
                assertEquals(0, callbacks)
                assertEquals(distributions, core.view.libraryWordDistributions)
                imports.start(FileImportFixtures.uri("queued.txt", "\uFEFF犬。".toByteArray()))
            }
            compose.waitUntil(30_000) { !imports.active && !core.view.storageBusy }
            compose.runOnIdle {
                assertEquals(ImportOutcome.EXISTING, imports.result?.outcome)
                assertEquals(1, callbacks)
                assertEquals(distributions, core.view.libraryWordDistributions)
            }
        } finally {
            gate.release()
            compose.runOnIdle {
                imports.close()
                core.close()
            }
        }
    }

    private class Gate {
        private val entered = CountDownLatch(1)
        private val permit = Semaphore(0)

        fun block() {
            entered.countDown()
            // Model a provider that ignores executor shutdown/interruption.
            permit.acquireUninterruptibly()
        }

        fun awaitEntered() =
            assertTrue("Provider did not reach checkpoint", entered.await(30, TimeUnit.SECONDS))

        fun release() = permit.release()
    }

    /**
     * Reuse the real pipe/payload fixture, adding local observation of provider calls/ownership.
     */
    private class Provider(val stale: Uri, val checkpoint: Checkpoint) : TxtFixtureProvider() {
        val gate = Gate()
        val reads = AtomicInteger()
        val opens = AtomicInteger()
        val metadataReads = AtomicInteger()
        @Volatile var worker: Thread? = null
        @Volatile var cursorClosed = false
        @Volatile var streamClosed = false

        override fun query(
            uri: Uri,
            projection: Array<out String>?,
            selection: String?,
            args: Array<out String>?,
            sort: String?,
        ): Cursor {
            if (uri != stale)
                return checkNotNull(super.query(uri, projection, selection, args, sort))
            worker = Thread.currentThread()
            val cursor =
                object :
                    CursorWrapper(
                        checkNotNull(super.query(uri, projection, selection, args, sort))
                    ) {
                    override fun moveToFirst(): Boolean {
                        if (checkpoint == Checkpoint.METADATA) gate.block()
                        return super.moveToFirst()
                    }

                    override fun getString(index: Int): String {
                        metadataReads.incrementAndGet()
                        return super.getString(index)
                    }

                    override fun close() {
                        assertSame(worker, Thread.currentThread())
                        super.close()
                        cursorClosed = true
                    }
                }
            if (checkpoint == Checkpoint.QUERY || checkpoint == Checkpoint.ERROR) gate.block()
            if (checkpoint == Checkpoint.ERROR) {
                cursor.close()
                error("Obsolete provider error")
            }
            return cursor
        }

        override fun openAssetFile(uri: Uri, mode: String): AssetFileDescriptor {
            val asset = checkNotNull(super.openAssetFile(uri, mode))
            if (uri != stale) return asset
            opens.incrementAndGet()
            if (checkpoint == Checkpoint.OPEN) gate.block()
            return object :
                AssetFileDescriptor(
                    asset.parcelFileDescriptor,
                    asset.startOffset,
                    asset.declaredLength,
                ) {
                override fun createInputStream(): FileInputStream =
                    object : ParcelFileDescriptor.AutoCloseInputStream(parcelFileDescriptor) {
                        override fun read(bytes: ByteArray, offset: Int, count: Int): Int {
                            val call = reads.incrementAndGet()
                            if (
                                (checkpoint == Checkpoint.READ && call == 1) ||
                                    (checkpoint == Checkpoint.EOF && call == 2)
                            )
                                gate.block()
                            return super.read(bytes, offset, count)
                        }

                        override fun close() {
                            assertSame(worker, Thread.currentThread())
                            if (checkpoint == Checkpoint.CLOSE) gate.block()
                            try {
                                super.close()
                            } finally {
                                streamClosed = true
                            }
                        }
                    }
            }
        }

        fun memory(): Long {
            if (Thread.currentThread() == worker && checkpoint == Checkpoint.BEFORE_OPEN)
                gate.block()
            return Long.MAX_VALUE
        }

        fun awaitRetiredWorker() {
            val thread = checkNotNull(worker)
            thread.join(30_000)
            assertFalse("Cancelled reader did not retire", thread.isAlive)
        }
    }

    @Test
    fun cancelledProvidersReleaseResourcesAndCannotOverwriteAFreshImport() {
        val core = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session
        compose.waitUntil(30_000) { !core.view.storageBusy }
        for (checkpoint in Checkpoint.entries) {
            val prefix = "cancel-$checkpoint-${System.nanoTime()}"
            val stale = FileImportFixtures.uri("$prefix-old.txt", "\uFEFF$prefix 猫。".toByteArray())
            val fresh =
                FileImportFixtures.uri(
                    "$prefix-new.txt",
                    "$prefix ".toByteArray() + byteArrayOf(0xc2.toByte(), 0xa9.toByte()),
                )
            val provider = Provider(stale, checkpoint)
            val updates = mutableListOf<FileImportProgress>()
            val results = mutableListOf<FileImportResult?>()
            val initialCount = core.view.documents.size
            lateinit var imports: FileImports
            val previousObserver = core.onViewChanged
            compose.runOnIdle {
                imports =
                    FileImports(
                        ContentResolver.wrap(provider),
                        core,
                        observer =
                            object : FileImportObserver {
                                override fun update(progress: FileImportProgress) {
                                    updates.add(progress)
                                }

                                override fun finished(result: FileImportResult?) {
                                    results.add(result)
                                }

                                override fun close() {}
                            },
                        availableImportMemory = provider::memory,
                    )
                imports.start(stale)
                if (checkpoint == Checkpoint.READ) {
                    // Keep the published filename queued on main until after cancellation.
                    provider.gate.awaitEntered()
                    imports.cancel()
                }
            }
            try {
                provider.gate.awaitEntered()
                compose.runOnIdle {
                    if (checkpoint != Checkpoint.READ) imports.cancel()
                    assertFalse(imports.active)
                    assertNull(imports.result)
                }
                // The first provider is still blocked; a new executor must accept the second file.
                compose.waitUntil(30_000) { !core.view.storageBusy }
                compose.runOnIdle { imports.start(fresh) }
                compose.waitUntil(30_000) { imports.choosingEncoding }
                val (currentUpdates, currentResults) =
                    compose.runOnIdle { updates.toList() to results.toList() }
                provider.gate.release()
                provider.awaitRetiredWorker()
                // Drain all late main-thread callbacks after the obsolete worker has exited.
                compose.runOnIdle {
                    assertEquals(currentUpdates, updates)
                    assertEquals(currentResults, results)
                    assertTrue(imports.choosingEncoding)
                    assertEquals("$prefix-new.txt", imports.filename)
                    assertNull(imports.result)
                }
                assertTrue(provider.cursorClosed)
                when (checkpoint) {
                    Checkpoint.QUERY,
                    Checkpoint.METADATA,
                    Checkpoint.BEFORE_OPEN,
                    Checkpoint.ERROR -> {
                        assertEquals(0, provider.opens.get())
                        if (checkpoint != Checkpoint.BEFORE_OPEN)
                            assertEquals(0, provider.metadataReads.get())
                    }
                    else -> {
                        assertTrue(provider.streamClosed)
                        assertEquals(
                            when (checkpoint) {
                                Checkpoint.OPEN -> 0
                                Checkpoint.READ -> 1
                                else ->
                                    2 // Data and EOF; closing is still private acquisition work.
                            },
                            provider.reads.get(),
                        )
                    }
                }
                assertEquals(initialCount, core.view.documents.size)
                compose.runOnIdle { imports.selectEncoding(TextEncoding.UTF8) }
                compose.waitUntil(30_000) { !imports.active && !core.view.storageBusy }
                assertEquals(ImportOutcome.CREATED, imports.result?.outcome)
                assertEquals(initialCount + 1, core.view.documents.size)
                assertEquals(listOf(null, imports.result), results)
            } finally {
                compose.runOnIdle {
                    imports.close()
                    core.onViewChanged = previousObserver
                }
                provider.gate.release()
                provider.awaitRetiredWorker()
            }
        }
    }

    @Test
    fun closingDuringAnUninterruptibleReadReturnsBeforeProviderAndDiscardsItsResult() {
        val core = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session
        compose.waitUntil(30_000) { !core.view.storageBusy }
        val stale =
            FileImportFixtures.uri("closed-${System.nanoTime()}.txt", "\uFEFF猫。".toByteArray())
        val provider = Provider(stale, Checkpoint.READ)
        var finishes = 0
        var closes = 0
        val initialCount = core.view.documents.size
        val previousObserver = core.onViewChanged
        lateinit var imports: FileImports
        var importsClosed = false
        compose.runOnIdle {
            imports =
                FileImports(
                    ContentResolver.wrap(provider),
                    core,
                    observer =
                        object : FileImportObserver {
                            override fun update(progress: FileImportProgress) {}

                            override fun finished(result: FileImportResult?) {
                                finishes++
                            }

                            override fun close() {
                                closes++
                            }
                        },
                )
            imports.start(stale)
        }
        try {
            provider.gate.awaitEntered()
            compose.runOnIdle {
                imports.close()
                importsClosed = true
                assertFalse(imports.active)
                assertEquals(1, closes)
            }
            provider.gate.release()
            provider.awaitRetiredWorker()
            compose.runOnIdle {
                assertEquals(0, finishes)
                assertNull(imports.result)
                assertEquals(initialCount, core.view.documents.size)
            }
            assertEquals(1, provider.reads.get())
            assertTrue(provider.cursorClosed)
            assertTrue(provider.streamClosed)
        } finally {
            compose.runOnIdle {
                if (!importsClosed) imports.close()
                core.onViewChanged = previousObserver
            }
            provider.gate.release()
            provider.awaitRetiredWorker()
        }
    }
}
