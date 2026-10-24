package app.yomoku

import android.database.sqlite.SQLiteDatabase
import android.os.Process
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.Event
import java.io.File
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

/** Default: real-clock close/reopen. The host runner also crosses process death/reboot. */
@RunWith(AndroidJUnit4::class)
class ReaderClockRecoveryTest {
    @Test
    fun realClockRecovery() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val arguments = InstrumentationRegistry.getArguments()
        val run = arguments.getString("readerClockRun")
        require(run == null || run.matches(Regex("[0-9a-f]{32}")))
        val phase = arguments.getString("readerClockPhase") ?: "roundtrip"
        require(phase in listOf("roundtrip", "before", "afterKill", "afterReboot"))
        val directory =
            File(
                instrumentation.targetContext.noBackupFilesDir,
                "reader-recovery-${run ?: System.nanoTime()}",
            )
        val database = File(directory, "reader.sqlite3")
        val baseline = File(directory, "milliseconds")
        var active: CoreSession? = null
        fun onMain(action: () -> Unit) = instrumentation.runOnMainSync(action)
        fun await(condition: () -> Boolean) {
            val deadline = System.nanoTime() + 30_000_000_000L
            while (true) {
                var done = false
                onMain { done = condition() }
                if (done) return
                check(System.nanoTime() < deadline) { "Real-clock recovery timed out" }
                Thread.sleep(10)
            }
        }
        fun open(): CoreSession {
            lateinit var core: CoreSession
            onMain {
                core =
                    CoreSession(
                        database.path,
                        sudachiDictionaryProvider(instrumentation.targetContext),
                    )
            }
            active = core
            await { !core.view.storageBusy }
            assertEquals(ContentRoute.MAIN, core.view.contentRoute)
            return core
        }
        fun close(core: CoreSession) {
            onMain { core.close() }
            await { core.isClosed }
            active = null
        }
        try {
            if (phase == "roundtrip" || phase == "before") {
                assertFalse(directory.exists())
                assertTrue(directory.mkdir())
                val core = open()
                onMain { core.dispatch(Event.CreatePaste("Reader recovery\n猫と犬。")) }
                await { !core.view.storageBusy }
                onMain { core.dispatch(Event.OpenReader(core.view.documents.single().documentId)) }
                await { !core.view.storageBusy && core.view.reader != null }
                onMain { core.setReaderActive(true) }
                await { !core.view.storageBusy && core.view.readerTime != null }
                Thread.sleep(250)
                close(core) // Automatic final Pause, no explicit duration/checkpoint event.
                val saved = core.view.readerTime!!.pending.value
                assertTrue(saved >= 250L)
                baseline.writeText(saved.toString())
            }
            var restored: Long? = null
            if (phase != "before") {
                val expected = baseline.readText().toLong()
                val core = open()
                onMain { core.dispatch(Event.OpenReader(core.view.documents.single().documentId)) }
                await { !core.view.storageBusy && core.view.reader != null }
                onMain { core.setReaderActive(true) }
                await { !core.view.storageBusy && core.view.readerTime != null }
                restored = core.view.readerTime!!.pending.value
                assertEquals(expected, restored)
                close(core)
                baseline.writeText(core.view.readerTime!!.pending.value.toString())
            }
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READONLY).use { db
                ->
                for (table in listOf("reading_completions", "reading_occurrences")) {
                    db.rawQuery("SELECT count(*) FROM $table", null).use {
                        assertTrue(it.moveToFirst())
                        assertEquals(0L, it.getLong(0))
                    }
                }
            }
            File(directory, "$phase.json")
                .writeText(
                    JSONObject()
                        .put("phase", phase)
                        .put("pid", Process.myPid())
                        .put("saved_ms", baseline.readText().toLong())
                        .put("restored_ms", restored)
                        .put("status", "PASS")
                        .toString(2)
                )
        } finally {
            active?.let { close(it) }
            if (run == null) assertTrue(directory.deleteRecursively())
        }
    }
}
