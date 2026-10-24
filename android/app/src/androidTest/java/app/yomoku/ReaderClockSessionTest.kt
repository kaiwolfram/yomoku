package app.yomoku

import android.database.sqlite.SQLiteDatabase
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.DocumentError
import app.yomoku.shared.Event
import app.yomoku.shared.PageLength
import app.yomoku.shared.TextOffset
import app.yomoku.shared.ViewModel
import java.io.File
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

/** Fake monotonic samples, real Android scheduling/JNI/SQLite. */
@RunWith(AndroidJUnit4::class)
class ReaderClockSessionTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    private fun onMain(action: () -> Unit) = instrumentation.runOnMainSync(action)

    private fun await(
        core: CoreSession,
        condition: (ViewModel) -> Boolean = { !it.storageBusy },
    ): ViewModel {
        val deadline = System.nanoTime() + 30_000_000_000L
        while (true) {
            lateinit var view: ViewModel
            onMain { view = core.view }
            if (condition(view)) return view
            check(System.nanoTime() < deadline) {
                "Reader clock callback timed out: busy=${view.storageBusy}, time=${view.readerTime}, error=${view.documentError}"
            }
            Thread.sleep(10)
        }
    }

    private fun awaitClosed(core: CoreSession) {
        val deadline = System.nanoTime() + 30_000_000_000L
        while (true) {
            var closed = false
            onMain { closed = core.isClosed }
            if (closed) return
            check(System.nanoTime() < deadline) { "Final reader save did not drain" }
            Thread.sleep(10)
        }
    }

    private fun dispatch(core: CoreSession, event: Event): ViewModel {
        onMain { core.dispatch(event) }
        return await(core)
    }

    private inner class Fixture : AutoCloseable {
        val directory =
            File(instrumentation.targetContext.filesDir, "reader-clock-${System.nanoTime()}")
        val database = File(directory, "reader.sqlite3")
        var now = 0L // Read and changed on Android's main thread only.
        lateinit var core: CoreSession
        val db: SQLiteDatabase

        init {
            assertTrue(directory.mkdir())
            open()
            await(core)
            val id =
                dispatch(core, Event.CreatePaste("猫と犬。\n".repeat(150)))
                    .documents
                    .single()
                    .documentId
            dispatch(core, Event.OpenReader(id))
            db = SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE)
        }

        fun open() {
            onMain {
                core =
                    CoreSession(
                        database.path,
                        sudachiDictionaryProvider(instrumentation.targetContext),
                        { now },
                        50L,
                    )
            }
        }

        fun start() {
            onMain { core.setReaderActive(true) }
            val ready = await(core) { !it.storageBusy && it.readerTime != null }
            assertNull(ready.documentError)
        }

        fun saved(milliseconds: Long): ViewModel =
            await(core) {
                !it.storageBusy &&
                    !it.readingTimeSavePending &&
                    it.readerTime?.pending?.value == milliseconds
            }

        fun assertNoHistory() {
            for (table in listOf("reading_completions", "reading_occurrences")) {
                db.rawQuery("SELECT count(*) FROM $table", null).use {
                    assertTrue(it.moveToFirst())
                    assertEquals(table, 0L, it.getLong(0))
                }
            }
            db.rawQuery("SELECT count(*) FROM vocabulary WHERE status != 'new'", null).use {
                assertTrue(it.moveToFirst())
                assertEquals(0L, it.getLong(0))
            }
        }

        override fun close() {
            db.execSQL("DROP TRIGGER IF EXISTS fail_save")
            db.execSQL("DROP TRIGGER IF EXISTS fail_restore")
            onMain { core.close() }
            awaitClosed(core)
            db.close()
            assertTrue(directory.deleteRecursively())
        }
    }

    @Test
    fun periodicIdleInteractionSettingsMovementAndFreshEpochPersistOnlyDurations() {
        Fixture().use { f ->
            f.start()
            onMain {
                f.now = 1_000L
                f.core.dispatch(Event.SaveReadingPosition(f.core.view.reader!!.position))
            }
            await(f.core)
            onMain { f.now = 301_000L }
            f.saved(300_000L) // Same-offset writes preserve the original idle deadline.
            onMain {
                f.core.onReaderInteraction()
                f.now = 302_000L
                f.core.dispatch(Event.OpenSettings)
            }
            f.saved(301_000L)
            onMain {
                f.now = 900_000L
                f.core.onReaderInteraction() // Settings cannot resume hidden accounting.
            }
            dispatch(f.core, Event.SetPageLength(PageLength.CHARACTERS250))
            assertEquals(301_000L, f.core.view.readerTime!!.pending.value)
            dispatch(f.core, Event.CloseSettings)
            onMain {
                f.now = 901_000L
                f.core.setReaderActive(false)
                f.core.setReaderActive(false)
            }
            f.saved(302_000L)
            onMain {
                f.now = 2_000_000L
                f.core.setReaderActive(true)
                f.now = 2_001_000L
                val position = f.core.view.reader!!.position.copy(offset = TextOffset(3))
                f.core.dispatch(Event.SaveReadingPosition(position))
            }
            val moved =
                await(f.core) {
                    !it.storageBusy && it.readerTime?.position?.offset == TextOffset(3)
                }
            assertEquals(0L, moved.readerTime!!.pending.value)
            onMain {
                f.now = 2_002_000L
                f.core.close()
            }
            awaitClosed(f.core)
            f.now = 5L // A different process/reboot has a fresh monotonic origin.
            f.open()
            val initial = await(f.core)
            assertEquals(ContentRoute.MAIN, initial.contentRoute)
            dispatch(f.core, Event.OpenReader(initial.documents.single().documentId))
            f.start()
            assertEquals(1_000L, f.core.view.readerTime!!.pending.value)
            onMain {
                f.now = 105L
                f.core.dispatch(Event.CloseDocument)
            }
            await(f.core) { !it.storageBusy && it.contentRoute == ContentRoute.MAIN }
            f.db
                .rawQuery(
                    "SELECT byte_offset, milliseconds FROM pending_reading_time ORDER BY byte_offset",
                    null,
                )
                .use {
                    assertTrue(it.moveToFirst())
                    assertEquals(0L, it.getLong(0))
                    assertEquals(303_000L, it.getLong(1))
                    assertTrue(it.moveToNext())
                    assertEquals(3L, it.getLong(0))
                    assertEquals(1_100L, it.getLong(1))
                    assertFalse(it.moveToNext())
                }
            f.assertNoHistory()
        }
    }

    @Test
    fun restoreWaitsForBusyWorkAndFailureNeedsExplicitRetry() {
        Fixture().use { f ->
            f.db.execSQL(
                "CREATE TRIGGER fail_restore BEFORE INSERT ON pending_reading_time BEGIN SELECT RAISE(ABORT, 'restore failure'); END"
            )
            onMain {
                f.core.dispatch(Event.RefreshLibrary)
                f.core.setReaderActive(true)
            }
            val failed = await(f.core) { !it.storageBusy && it.documentError != null }
            assertEquals(DocumentError.UNAVAILABLE, failed.documentError)
            assertNull(failed.readerTime)
            onMain { f.now = 500_000L }
            f.db.execSQL("DROP TRIGGER fail_restore")
            dispatch(f.core, Event.RestoreReadingTime)
            onMain {
                f.now = 501_000L
                f.core.setReaderActive(false)
            }
            f.saved(1_000L) // Time spent waiting for restore was never active.
            f.assertNoHistory()
        }
    }

    @Test
    fun closeWaitsForFinalSampleBeforeNativeRelease() {
        Fixture().use { f ->
            f.start()
            onMain {
                f.now = 2_000L
                f.core.close()
                f.core.close()
                assertThrows(IllegalStateException::class.java) {
                    f.core.dispatch(Event.RefreshLibrary)
                }
            }
            awaitClosed(f.core)
            f.db.rawQuery("SELECT milliseconds, revision FROM pending_reading_time", null).use {
                assertTrue(it.moveToFirst())
                assertEquals(2_000L, it.getLong(0))
                assertEquals(2L, it.getLong(1)) // Restore + final checkpoint.
            }
            f.assertNoHistory()
        }
    }

    @Test
    fun failedCheckpointRetainsFrozenRetryAndFinalSlotDuringClose() {
        Fixture().use { f ->
            f.start()
            f.db.execSQL(
                "CREATE TRIGGER fail_save BEFORE UPDATE ON pending_reading_time BEGIN SELECT RAISE(ABORT, 'save failure'); END"
            )
            onMain { f.now = 1_000L }
            await(f.core) {
                !it.storageBusy && it.readingTimeSavePending && it.documentError != null
            }
            onMain {
                f.now = 2_000L
                f.core.close()
                assertFalse(f.core.isClosed)
            }
            // Let at least one onCleared-style retry fail, then allow the same payload to drain.
            Thread.sleep(1_100)
            onMain { assertFalse(f.core.isClosed) }
            f.db.execSQL("DROP TRIGGER fail_save")
            awaitClosed(f.core)
            f.db.rawQuery("SELECT milliseconds, revision FROM pending_reading_time", null).use {
                assertTrue(it.moveToFirst())
                assertEquals(2_000L, it.getLong(0))
                assertEquals(3L, it.getLong(1))
            }
            f.assertNoHistory()
        }
    }

    @Test
    fun completionCommitsTheFinalSampleBeforeClosingWithoutStartingAnotherClock() {
        Fixture().use { f ->
            f.start()
            onMain {
                val page = f.core.view.reader!!
                val complete =
                    Event.CompleteReaderPage(page.documentId, page.anchor, page.position, 123L)
                f.now = 1_000L
                f.core.dispatch(complete)
                f.core.dispatch(complete)
                f.now = 2_000L
                f.core.close()
                assertFalse(f.core.isClosed)
            }
            awaitClosed(f.core)
            f.db.rawQuery("SELECT active_time_ms FROM reading_completions", null).use {
                assertTrue(it.moveToFirst())
                assertEquals(1_000L, it.getLong(0))
                assertFalse(it.moveToNext())
            }
            f.db.rawQuery("SELECT sum(milliseconds) FROM pending_reading_time", null).use {
                assertTrue(it.moveToFirst())
                assertEquals(0L, it.getLong(0))
            }
        }
    }
}
