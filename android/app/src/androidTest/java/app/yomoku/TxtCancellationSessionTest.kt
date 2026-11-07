package app.yomoku

import android.database.sqlite.SQLiteDatabase
import android.net.Uri
import android.os.Looper
import android.util.Base64
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.shared.ImportOutcome
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class TxtCancellationSessionTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val context = instrumentation.targetContext

    private fun main(action: () -> Unit) = instrumentation.runOnMainSync(action)

    private fun await(condition: () -> Boolean) {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(90)
        while (true) {
            var done = false
            main { done = condition() }
            if (done) return
            check(System.nanoTime() < deadline) { "TXT cancellation callback timed out" }
            Thread.sleep(10)
        }
    }

    private fun fixture(filename: String, text: String): Uri =
        Uri.Builder()
            .scheme("content")
            .authority("app.yomoku.test.txt-fixtures")
            .appendPath(filename)
            .appendQueryParameter(
                "bytes",
                Base64.encodeToString("\uFEFF$text".toByteArray(), Base64.NO_WRAP),
            )
            .build()

    private fun gatedProvider(
        entered: CountDownLatch,
        release: CountDownLatch,
    ): DictionaryProvider {
        val actual = sudachiDictionaryProvider(context)
        return object : DictionaryProvider {
            override fun verifiedWordDictionaryPath(): String = ""

            override fun openVerifiedDictionary(): Long {
                check(Looper.myLooper() != Looper.getMainLooper())
                entered.countDown()
                return if (release.await(90, TimeUnit.SECONDS)) actual.openVerifiedDictionary()
                else 0L
            }
        }
    }

    @Test
    fun nativeCancellationReturnsDuringBlockedLoadButImportAwaitsOutcomeBeforeFreshRetry() {
        val directory = File(context.cacheDir, "txt-cancel-load-${System.nanoTime()}")
        assertTrue(directory.mkdir())
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val executor = Executors.newSingleThreadExecutor()
        lateinit var core: CoreSession
        lateinit var imports: FileImports
        main {
            core =
                CoreSession(
                    File(directory, "reader.sqlite3").path,
                    gatedProvider(entered, release),
                    cancellationExecutor = executor,
                )
            imports = FileImports(context.contentResolver, core)
        }
        val first = fixture("cancelled.srt", "猫と犬。")
        val remaining = fixture("remaining.txt", "残りの本。")
        try {
            await { !core.view.storageBusy }
            main { imports.start(first) }
            assertTrue(entered.await(30, TimeUnit.SECONDS))
            main {
                imports.cancel()
                imports.cancel() // Duplicate UI callbacks never enqueue another cancellation.
            }
            // Barrier proves the native call returns independently of the blocked FIFO loader.
            executor.submit {}.get(10, TimeUnit.SECONDS)
            main {
                assertTrue(core.view.storageBusy)
                assertTrue(imports.active)
                assertTrue(imports.cancelling)
                assertEquals("cancelled.srt", imports.filename)
                imports.start(remaining) // Cannot replace an unsettled attempt.
                assertEquals("cancelled.srt", imports.filename)
            }
            release.countDown()
            await { !imports.active && !core.view.storageBusy }
            main {
                assertNull(imports.result)
                assertTrue(core.view.documents.isEmpty())
                assertNull(core.view.documentError)
                assertNull(core.view.importSummary)
                imports.start(first) // Fresh cancellation handle for explicit retry.
            }
            await { !imports.active && !core.view.storageBusy }
            main {
                assertEquals(ImportOutcome.CREATED, imports.result?.outcome)
                assertEquals("cancelled.srt", core.view.documents.single().title)
                assertNull(core.view.reader)
            }
        } finally {
            release.countDown()
            main {
                imports.close()
                core.close()
            }
            await { core.isClosed }
            assertTrue(directory.deleteRecursively())
        }
    }

    @Test
    fun completedCommitIsRetainedWhileCancelReturnAndNativeCloseAreStillPending() {
        val directory = File(context.cacheDir, "txt-cancel-commit-${System.nanoTime()}")
        assertTrue(directory.mkdir())
        val database = File(directory, "reader.sqlite3")
        val entered = CountDownLatch(1)
        val loadRelease = CountDownLatch(1)
        val cancelRelease = CountDownLatch(1)
        val executor = Executors.newSingleThreadExecutor()
        executor.execute { check(cancelRelease.await(90, TimeUnit.SECONDS)) }
        lateinit var core: CoreSession
        lateinit var imports: FileImports
        main {
            core =
                CoreSession(
                    database.path,
                    gatedProvider(entered, loadRelease),
                    cancellationExecutor = executor,
                )
            imports = FileImports(context.contentResolver, core)
        }
        try {
            await { !core.view.storageBusy }
            main {
                imports.start(
                    FileImportFixtures.uri(
                        "completed.epub",
                        FileImportFixtures.epub("completed EPUB"),
                    )
                )
            }
            assertTrue(entered.await(30, TimeUnit.SECONDS))
            main { imports.cancel() }
            // The accepted file can finish before the separate cancellation executor runs.
            loadRelease.countDown()
            await { !core.view.storageBusy && imports.result != null }
            main {
                assertTrue(imports.active)
                assertTrue(imports.cancelling)
                assertEquals(ImportOutcome.CREATED, imports.result?.outcome)
                assertEquals("completed EPUB", core.view.documents.single().title)
                assertEquals("completed.epub", imports.filename)
                assertNull(core.view.importSummary)
                assertNull(core.view.documentError)
                // Owner teardown still cannot release a handle with a queued direct native call.
                imports.close()
                core.close()
                assertFalse(core.isClosed)
            }
            cancelRelease.countDown()
            await { core.isClosed }
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READONLY).use { db
                ->
                db.rawQuery("SELECT title FROM documents", null).use {
                    assertTrue(it.moveToFirst())
                    assertEquals("completed EPUB", it.getString(0))
                    assertFalse(it.moveToNext())
                }
            }
            main {
                core = CoreSession(database.path, sudachiDictionaryProvider(context))
                imports = FileImports(context.contentResolver, core)
            }
            await { !core.view.storageBusy }
            main {
                assertEquals("completed EPUB", core.view.documents.single().title)
                assertFalse(imports.active)
                assertNull(imports.result)
            }
        } finally {
            loadRelease.countDown()
            cancelRelease.countDown()
            main {
                imports.close()
                core.close()
            }
            await { core.isClosed }
            assertTrue(directory.deleteRecursively())
        }
    }
}
