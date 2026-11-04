package app.yomoku

import android.database.sqlite.SQLiteDatabase
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.shared.*
import java.io.File
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Real Android JNI/SQLite and retained shell worker; provider reads are injected. */
@RunWith(AndroidJUnit4::class)
class AnkiRecoverySessionTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val dictionary =
        object : DictionaryProvider {
            override fun verifiedWordDictionaryPath(): String = error("unexpected dictionary load")

            override fun openVerifiedDictionary(): Long = error("unexpected analyzer load")
        }
    private val layout =
        AnkiReadModelLayout(
            "Frozen model 𠮷",
            listOf("Surface", "Reading", "Meanings", "Sentence"),
            ".card { color: black; } /* old */",
            "Word",
            "{{Surface}}",
            "{{Sentence}}",
        )

    private fun await(condition: () -> Boolean) {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(30)
        while (!condition()) {
            check(System.nanoTime() < deadline) { "Anki recovery did not complete" }
            Thread.sleep(10)
        }
    }

    private fun awaitPhase(core: CoreSession, phase: AnkiRecoveryPhase) {
        await {
            var matches = false
            instrumentation.runOnMainSync { matches = core.view.ankiRecovery?.phase == phase }
            matches
        }
    }

    private fun open(
        database: File,
        executor: ExecutorService = Executors.newSingleThreadExecutor(),
        read: (AnkiRecoveryReadOperation) -> AnkiRecoveryReadResult,
    ): CoreSession {
        lateinit var core: CoreSession
        instrumentation.runOnMainSync {
            core =
                CoreSession(
                    database.path,
                    dictionary,
                    readAnkiRecovery = read,
                    ankiExecutor = executor,
                )
        }
        await {
            var idle = false
            instrumentation.runOnMainSync { idle = !core.view.storageBusy }
            idle
        }
        return core
    }

    private fun seed(database: File) {
        val initial = open(database) { error("startup must not read provider") }
        instrumentation.runOnMainSync { initial.close() }
        val fields =
            JSONObject()
                .put("name", layout.name)
                .put("field_names", JSONArray(layout.fieldNames))
                .put("css", layout.css)
                .put("card_name", layout.cardName)
                .put("question_format", layout.questionFormat)
                .put("answer_format", layout.answerFormat)
        val payload = JSONObject().put("version", 1).put("layout", fields).toString()
        SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use {
            it.execSQL(
                "INSERT INTO anki_model_creations(payload, state) VALUES(?, 'pending')",
                arrayOf(payload),
            )
        }
    }

    private fun state(database: File): Pair<String, Long?> =
        SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READONLY).use { sql ->
            sql.rawQuery("SELECT state, existing_model_id FROM anki_model_creations", null).use {
                assertTrue(it.moveToFirst())
                it.getString(0) to if (it.isNull(1)) null else it.getLong(1)
            }
        }

    private fun begin(core: CoreSession) = instrumentation.runOnMainSync {
        core.dispatch(Event.BeginAnkiRecoveryRead(AnkiRecoveryReadTarget.MODEL))
    }

    private fun directory() =
        File(instrumentation.targetContext.noBackupFilesDir, "anki-${System.nanoTime()}").also {
            assertTrue(it.mkdir())
        }

    @Test
    fun readRunsOffMainWhileNavigationWorksAndFailureCannotReleaseJournal() {
        val directory = directory()
        val database = File(directory, "db.sqlite3")
        val operations = LinkedBlockingQueue<AnkiRecoveryReadOperation>()
        val responses = LinkedBlockingQueue<AnkiRecoveryReadResult>()
        seed(database)
        val core =
            open(database) {
                assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                operations.add(it)
                checkNotNull(responses.poll(30, TimeUnit.SECONDS))
            }
        try {
            assertTrue(operations.isEmpty())
            instrumentation.runOnMainSync { core.dispatch(Event.OpenSettings) }
            compose.setContent {
                app.yomoku.theme.YomokuTheme { SettingsScreen(core.view, core::dispatch) }
            }
            compose
                .onNodeWithTag("anki_recovery_status")
                .assertTextEquals("An earlier AnkiDroid operation needs checking.")
            compose.onNodeWithTag("anki_recovery_model").performClick()
            assertEquals(
                AnkiRecoveryReadOperation.Model(null, layout),
                operations.poll(30, TimeUnit.SECONDS),
            )
            begin(core)
            compose.onNodeWithTag("anki_recovery_status").assertTextEquals("Checking AnkiDroid…")
            compose.onNodeWithTag("anki_recovery_model").assertIsNotEnabled()
            instrumentation.runOnMainSync {
                core.dispatch(Event.SelectDestination(Destination.STATISTICS))
                assertEquals(Destination.STATISTICS, core.view.destination)
                assertNull(core.view.documentError)
                core.dispatch(Event.OpenSettings)
            }
            assertTrue(operations.isEmpty())
            assertEquals("pending" to null, state(database))
            responses.add(AnkiRecoveryReadResult.Failed(AnkiExportError.PERMISSIONDENIED))
            // The projected acknowledgement is the commit barrier. A concurrent
            // Android read-only connection can require rollback-journal recovery.
            awaitPhase(core, AnkiRecoveryPhase.FAILED)
            assertEquals("uncertain" to null, state(database))
            compose.onNodeWithText("AnkiDroid access permission is required.").assertIsDisplayed()
            compose.onNodeWithTag("anki_recovery_retry").performClick()
            awaitPhase(core, AnkiRecoveryPhase.UNRESOLVED)
            assertTrue(operations.isEmpty()) // Retry restores SQLite only.
            compose.onNodeWithTag("anki_recovery_model").performClick()
            assertEquals(
                AnkiRecoveryReadOperation.Model(null, layout),
                operations.poll(30, TimeUnit.SECONDS),
            )
            responses.add(AnkiRecoveryReadResult.Model(Long.MAX_VALUE))
            awaitPhase(core, AnkiRecoveryPhase.READY)
            assertEquals("existing" to Long.MAX_VALUE, state(database))
            compose
                .onNodeWithTag("anki_recovery_status")
                .assertTextEquals("No pending AnkiDroid recovery.")
            compose.onNodeWithTag("anki_recovery_model").assertDoesNotExist()
            begin(core)
            assertTrue(operations.isEmpty())
        } finally {
            responses.add(AnkiRecoveryReadResult.Model(null))
            instrumentation.runOnMainSync { core.close() }
            directory.deleteRecursively()
        }
    }

    @Test
    fun closeDuringReadRetainsJournalAndLateCallbackCannotReachRestartedSession() {
        val directory = directory()
        val database = File(directory, "db.sqlite3")
        val oldRead = LinkedBlockingQueue<Unit>()
        val oldResponse = LinkedBlockingQueue<AnkiRecoveryReadResult>()
        seed(database)
        val oldExecutor = Executors.newSingleThreadExecutor()
        val old =
            open(database, oldExecutor) {
                oldRead.add(Unit)
                checkNotNull(oldResponse.poll(30, TimeUnit.SECONDS))
            }
        begin(old)
        checkNotNull(oldRead.poll(30, TimeUnit.SECONDS))
        instrumentation.runOnMainSync {
            old.close()
            assertTrue(old.isClosed)
        }
        val reads = LinkedBlockingQueue<Unit>()
        val responses = LinkedBlockingQueue<AnkiRecoveryReadResult>()
        val fresh =
            open(database) {
                reads.add(Unit)
                checkNotNull(responses.poll(30, TimeUnit.SECONDS))
            }
        try {
            assertTrue(reads.isEmpty())
            instrumentation.runOnMainSync { fresh.dispatch(Event.OpenSettings) }
            begin(fresh)
            checkNotNull(reads.poll(30, TimeUnit.SECONDS))
            oldResponse.add(AnkiRecoveryReadResult.Model(Long.MAX_VALUE))
            assertTrue(oldExecutor.awaitTermination(30, TimeUnit.SECONDS))
            instrumentation.waitForIdleSync()
            assertEquals("pending" to null, state(database))
            responses.add(AnkiRecoveryReadResult.Model(9007199254740993L))
            awaitPhase(fresh, AnkiRecoveryPhase.READY)
            assertEquals("existing" to 9007199254740993L, state(database))
        } finally {
            oldResponse.offer(AnkiRecoveryReadResult.Model(null))
            responses.offer(AnkiRecoveryReadResult.Model(null))
            instrumentation.runOnMainSync { fresh.close() }
            directory.deleteRecursively()
        }
    }
}
