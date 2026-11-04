package app.yomoku

import android.database.sqlite.SQLiteDatabase
import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.*
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Production JNI/session/SQLite with provider observations and writes supplied by a fake
 * collection.
 */
@RunWith(AndroidJUnit4::class)
class AnkiEntrySessionTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private lateinit var core: CoreSession
    private val notes = ConcurrentHashMap<String, List<Long>>()
    private val writes = AtomicInteger()

    private fun send(event: Event) = instrumentation.runOnMainSync { core.dispatch(event) }

    private fun view(): ViewModel {
        lateinit var result: ViewModel
        instrumentation.runOnMainSync { result = core.view }
        return result
    }

    private fun await(predicate: (ViewModel) -> Boolean): ViewModel {
        val deadline = System.nanoTime() + 60_000_000_000L
        while (true) {
            val value = view()
            if (predicate(value)) return value
            check(System.nanoTime() < deadline) {
                "Anki entry timed out: ${value.ankiSetup}, ${value.ankiEntries}, ${value.ankiExport}; word=${value.effectiveReaderWord}; reader=${value.documentError}"
            }
            Thread.sleep(10)
        }
    }

    private fun open(database: File) {
        instrumentation.runOnMainSync {
            core =
                CoreSession(
                    database.path,
                    sudachiDictionaryProvider(instrumentation.targetContext),
                    readAnkiPreflight = { operation ->
                        assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                        assertEquals("Yomoku v2", operation.layout.name)
                        AnkiPreflightResult.Complete(
                            AnkiPreflightObservation(
                                listOf(AnkiPreflightDeck(7, false)),
                                notes[operation.surface].orEmpty(),
                                listOf(AnkiPreflightModel(9, operation.layout.name, true)),
                            )
                        )
                    },
                    invokeAnkiLive = { operation ->
                        assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                        assertTrue(operation is AnkiLiveOperation.InsertNote)
                        val note = operation as AnkiLiveOperation.InsertNote
                        SQLiteDatabase.openDatabase(
                                database.path,
                                null,
                                SQLiteDatabase.OPEN_READONLY,
                            )
                            .use { sql ->
                                sql.rawQuery(
                                        "SELECT state FROM anki_submissions ORDER BY id DESC LIMIT 1",
                                        null,
                                    )
                                    .use { row ->
                                        assertTrue(row.moveToFirst())
                                        assertEquals("pending", row.getString(0))
                                    }
                            }
                        assertEquals("猫", note.fields.first())
                        assertEquals(7L, note.deckId)
                        assertTrue(note.fields[2].contains("cat"))
                        assertEquals("<strong class=\"yomoku-target\">猫</strong>。", note.fields[3])
                        writes.incrementAndGet()
                        notes[note.surface] = listOf(88L)
                        AnkiLiveResult.NoteAdded(88)
                    },
                )
        }
        await { !it.storageBusy && it.ankiSettings.phase == AnkiSettingsPhase.READY }
    }

    private fun close() {
        instrumentation.runOnMainSync { core.close() }
        val deadline = System.nanoTime() + 30_000_000_000L
        while (true) {
            var closed = false
            instrumentation.runOnMainSync { closed = core.isClosed }
            if (closed) return
            check(System.nanoTime() < deadline)
            Thread.sleep(10)
        }
    }

    private fun selectCat(id: DocumentId): AnkiSelection {
        send(Event.OpenReader(id))
        var state = await { it.reader != null && !it.storageBusy }
        send(
            Event.LoadReaderTokens(
                ReaderTokenKey(id, state.reader!!.anchor, state.readerAnalysis!!)
            )
        )
        state = await { it.effectiveReaderTokens != null && !it.storageBusy }
        val snapshot = state.effectiveReaderTokens!!
        val selected =
            EffectiveReaderSelection(
                snapshot.identity,
                snapshot.tokens.first { it.surface == "猫" }.segmentIndex,
            )
        send(Event.LoadEffectiveReaderWord(selected))
        val submitted = view()
        check(submitted.effectiveReaderWord?.selection == selected) {
            "Selection rejected: selected=$selected; tokens=${submitted.effectiveReaderTokens?.identity}; current=${submitted.reader?.anchor}/${submitted.readerAnalysis}; route=${submitted.contentRoute}; settings=${submitted.settingsOpen}; busy=${submitted.storageBusy}/${submitted.readingTimeSavePending}/${submitted.completionSavePending}; word=${submitted.effectiveReaderWord}; status=${submitted.effectiveReaderStatus}"
        }
        await {
            check(it.effectiveReaderWord?.error == null) {
                "Word lookup failed: ${it.effectiveReaderWord}"
            }
            it.effectiveReaderWord?.details != null && !it.storageBusy
        }
        return AnkiSelection.Effective(selected)
    }

    @Test
    fun oneTapAddsOnlyOnceAndRestartRecognizesExistingEntryWithoutOpeningSettings() {
        val directory =
            File(instrumentation.targetContext.noBackupFilesDir, "anki-entry-${System.nanoTime()}")
                .apply { mkdirs() }
        val database = File(directory, "db.sqlite3")
        open(database)
        try {
            send(Event.OpenSettings)
            send(Event.SetAnkiDefaultDeck(7))
            await { it.ankiSetup == AnkiSetupPhase.Ready }
            send(Event.CloseSettings)
            send(Event.CreatePaste("猫。"))
            val id = await {
                it.documents.size == 1 && !it.storageBusy
            }
                .documents
                .single()
                .documentId
            val selection = selectCat(id)
            val before = view().effectiveReaderWord!!.details!!
            send(Event.LoadReaderAnkiEntries(selection))
            val entry = await {
                it.ankiEntries.any { entry -> entry.phase == AnkiEntryPhase.Add }
            }
                .ankiEntries
                .first { it.phase == AnkiEntryPhase.Add }
            send(Event.AddReaderAnkiEntry(selection, entry.key))
            await {
                it.ankiEntries.any { entry -> entry.phase == AnkiEntryPhase.Existing(listOf(88L)) }
            }
            send(Event.AddReaderAnkiEntry(selection, entry.key))
            assertEquals(1, writes.get())
            assertEquals(
                before.token.vocabulary,
                view().effectiveReaderWord!!.details!!.token.vocabulary,
            )
            assertEquals(before.encountered, view().effectiveReaderWord!!.details!!.encountered)
            close()
            open(database)
            await { it.ankiSetup == AnkiSetupPhase.Ready }
            assertFalse(view().settingsOpen)
            assertEquals(AnkiDeckDiscoveryState.Unchecked, core.ankiDecks.state)
            val reopened = selectCat(id)
            send(Event.LoadReaderAnkiEntries(reopened))
            await {
                it.ankiEntries.any { entry -> entry.phase == AnkiEntryPhase.Existing(listOf(88L)) }
            }
            assertEquals(1, writes.get())
        } finally {
            close()
            directory.deleteRecursively()
        }
    }
}
