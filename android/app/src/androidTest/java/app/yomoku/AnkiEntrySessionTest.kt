package app.yomoku

import android.content.Intent
import android.database.sqlite.SQLiteDatabase
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Production JNI/session/SQLite with provider observations and writes supplied by a fake
 * collection.
 */
@RunWith(AndroidJUnit4::class)
class AnkiEntrySessionTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val entryQueries = AtomicInteger()
    private var providerEntered: CountDownLatch? = null
    private var providerRelease: CountDownLatch? = null
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private lateinit var core: CoreSession
    private val notes = ConcurrentHashMap<Pair<String, String>, List<Long>>()
    private val writes = AtomicInteger()
    private var expectedSentence = "<strong class=\"yomoku-target\">猫</strong>。"
    @Volatile private var readFailure: AnkiExportError? = null

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
                "Anki entry timed out: ${value.ankiSetup}, ${value.ankiEntries}, ${value.ankiExport}; word=${value.effectiveReaderWord}; reader=${value.documentError}; vocabulary=${value.vocabulary.word?.selection}/${value.vocabulary.word?.error}; candidates=${value.vocabulary.word?.candidates != null}; settings=${value.settingsOpen}; busy=${value.storageBusy}"
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
                        assertEquals("Yomoku", operation.layout.name)
                        if (operation.surface.isNotEmpty()) {
                            entryQueries.incrementAndGet()
                            providerEntered?.countDown()
                            check(providerRelease?.await(30, TimeUnit.SECONDS) != false)
                        }
                        if (operation.surface.isNotEmpty() && readFailure != null)
                            AnkiPreflightResult.Failed(readFailure!!)
                        else
                            AnkiPreflightResult.Complete(
                                AnkiPreflightObservation(
                                    listOf(AnkiPreflightDeck(7, false)),
                                    notes[operation.surface to operation.reading].orEmpty(),
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
                                        "SELECT name FROM sqlite_schema WHERE type = 'table' AND name LIKE 'anki_%'",
                                        null,
                                    )
                                    .use { row ->
                                        assertTrue(row.moveToFirst())
                                        assertEquals("anki_settings", row.getString(0))
                                        assertFalse(row.moveToNext())
                                    }
                            }
                        assertEquals("猫", note.fields.first())
                        assertEquals(7L, note.deckId)
                        assertTrue(note.fields[2].contains("cat"))
                        assertEquals(expectedSentence, note.fields[3])
                        writes.incrementAndGet()
                        notes[note.surface to note.reading] = listOf(88L)
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
            "Selection rejected: selected=$selected; tokens=${submitted.effectiveReaderTokens?.identity}; current=${submitted.reader?.anchor}/${submitted.readerAnalysis}; route=${submitted.contentRoute}; settings=${submitted.settingsOpen}; busy=${submitted.storageBusy}/${submitted.completionSavePending}; word=${submitted.effectiveReaderWord}; status=${submitted.effectiveReaderStatus}"
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

    @Test
    fun vocabularyActionsShareProviderFlowAndKeepStatusObservations() {
        val directory =
            File(
                    instrumentation.targetContext.noBackupFilesDir,
                    "anki-vocabulary-${System.nanoTime()}",
                )
                .apply { mkdirs() }
        val database = File(directory, "db.sqlite3")
        val executor = Executors.newSingleThreadExecutor()
        val launches = AtomicInteger()
        var browser: AnkiNoteBrowser? = null
        expectedSentence = ""
        open(database)
        try {
            send(Event.CreatePaste("猫。"))
            await { it.documents.size == 1 && !it.storageBusy }
            send(Event.SelectDestination(Destination.VOCABULARY))
            send(Event.LoadVocabulary)
            val row = await {
                it.vocabulary.rows.isNotEmpty() && !it.storageBusy
            }
                .vocabulary
                .rows
                .first { it.surface == "猫" }
            send(Event.OpenVocabularyWord(row.id))
            await { it.vocabulary.word?.candidates != null && !it.storageBusy }
            assertTrue(view().ankiEntries.isEmpty())
            send(Event.OpenSettings)
            send(Event.SetAnkiDefaultDeck(7))
            await { it.ankiSetup == AnkiSetupPhase.Ready }
            send(Event.CloseSettings)
            readFailure = AnkiExportError.PERMISSIONDENIED
            instrumentation.runOnMainSync {
                browser =
                    AnkiNoteBrowser(
                        executor,
                        { AnkiReadResult.Value(null) },
                        currentEntries = { core.view.ankiEntries },
                        resolveEntries = { ids ->
                            assertEquals(listOf(88L), ids.map { it.value })
                            AnkiReadResult.Value(Intent("test.vocabulary.anki"))
                        },
                    ) {
                        core.view.ankiExport
                    }
                browser.attach { launches.incrementAndGet() }
            }
            compose.setContent {
                YomokuTheme {
                    YomokuApp(core.view, core.sessionKey, core::dispatch, ankiBrowser = browser)
                }
            }
            compose.waitForIdle()
            await { !it.storageBusy && it.vocabulary.word == null }
            compose.onNodeWithTag("vocabulary_row_${row.id.value}").performClick()
            await { it.vocabulary.word?.candidates != null && !it.storageBusy }
            // Advance Compose frames so the sheet can observe the completed native lookup.
            compose.waitUntil(60_000) {
                val state = view()
                state.ankiEntries.isNotEmpty() &&
                    state.ankiEntries.all { it.phase is AnkiEntryPhase.Failed }
            }
            val failed = view().ankiEntries.first { it.headword == "猫" && it.reading == "ねこ" }
            readFailure = null
            compose
                .onNodeWithTag("anki_entry_${ankiEntryTag(failed.key)}")
                .performScrollTo()
                .performClick()
            val entry = await {
                it.ankiEntries.any { it.key == failed.key && it.phase == AnkiEntryPhase.Add }
            }
                .ankiEntries
                .first { it.key == failed.key }
            compose
                .onNodeWithTag("anki_entry_${ankiEntryTag(entry.key)}")
                .performScrollTo()
                .performClick()
            send(Event.AddReaderAnkiEntry(entry.selection, entry.key))
            await {
                it.ankiEntries.any {
                    it.key == entry.key && it.phase == AnkiEntryPhase.Existing(listOf(88L))
                }
            }
            assertEquals(1, writes.get())
            compose
                .onNodeWithTag("anki_entry_${ankiEntryTag(entry.key)}")
                .performScrollTo()
                .performClick()
            compose.waitUntil(30_000) { launches.get() == 1 }
            val settled = view().ankiEntries
            val queries = entryQueries.get()
            val identity = view().vocabulary.word!!.selection
            fun assertSettled() {
                assertEquals(settled, view().ankiEntries)
                assertEquals(queries, entryQueries.get())
                assertEquals(identity, view().vocabulary.word!!.selection)
            }
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use {
                sql ->
                sql.execSQL(
                    "CREATE TRIGGER fail_vocabulary_anki BEFORE INSERT ON vocabulary_status_events BEGIN SELECT RAISE(ABORT, 'test status'); END"
                )
                try {
                    compose.onNodeWithTag("word_status_LEARNING").performScrollTo().performClick()
                    await { !it.storageBusy && it.vocabulary.wordStatusError != null }
                    assertSettled()
                    sql.execSQL("DROP TRIGGER fail_vocabulary_anki")
                    compose
                        .onNodeWithTag("vocabulary_word_status_retry")
                        .performScrollTo()
                        .performClick()
                    await { !it.storageBusy && it.vocabulary.wordStatus == null }
                    assertSettled()
                } finally {
                    sql.execSQL("DROP TRIGGER IF EXISTS fail_vocabulary_anki")
                }
            }
            notes.remove("猫" to "ねこ")
            send(Event.CheckAnkiSetup(AnkiSetupRequest.REFRESH))
            await { it.ankiEntries.any { it.key == entry.key && it.phase == AnkiEntryPhase.Add } }
            assertTrue(entryQueries.get() > queries)
            val oldSelection = entry.selection
            send(Event.DismissVocabularyWord)
            send(Event.OpenVocabularyWord(row.id))
            await { it.vocabulary.word?.candidates != null && !it.storageBusy }
            assertNotEquals(
                oldSelection,
                AnkiSelection.Vocabulary(view().vocabulary.word!!.selection),
            )
            send(Event.AddReaderAnkiEntry(oldSelection, entry.key))
            assertEquals(1, writes.get())
        } finally {
            compose.runOnUiThread {
                compose.activity.setContent {}
                browser?.close()
            }
            executor.shutdown()
            assertTrue(executor.awaitTermination(30, TimeUnit.SECONDS))
            close()
            directory.deleteRecursively()
        }
    }

    @Test
    fun statusWritesKeepSettledIconsAndDoNotRepeatDelayedProviderQueries() {
        val directory =
            File(instrumentation.targetContext.noBackupFilesDir, "anki-status-${System.nanoTime()}")
                .apply { mkdirs() }
        val database = File(directory, "db.sqlite3")
        open(database)
        try {
            send(Event.OpenSettings)
            send(Event.SetAnkiDefaultDeck(7))
            await { it.ankiSetup == AnkiSetupPhase.Ready }
            send(Event.CloseSettings)
            send(Event.CreatePaste("猫。"))
            val book = await {
                it.documents.size == 1 && !it.storageBusy
            }
                .documents
                .single()
                .documentId
            notes["猫" to "ねこ"] = listOf(88L)
            providerEntered = CountDownLatch(1)
            providerRelease = CountDownLatch(1)
            send(Event.OpenReader(book))
            compose.setContent {
                YomokuTheme { YomokuApp(core.view, core.sessionKey, core::dispatch) }
            }
            val ready = await { it.effectiveReaderTokens != null && !it.storageBusy }
            val span =
                effectiveDisplayTokens(
                        ready.reader!!,
                        ready.effectiveReaderTokens!!.identity,
                        ready.effectiveReaderTokens,
                    )!!
                    .first { it.token.surface == "猫" }
            val text =
                compose
                    .onNodeWithTag("reader_text")
                    .fetchSemanticsNode()
                    .config[SemanticsProperties.Text]
                    .single()
            val link = text.getLinkAnnotations(span.start, span.end).single().item
            compose.runOnUiThread { link.linkInteractionListener!!.onClick(link) }
            compose.waitUntil(30_000) { providerEntered!!.count == 0L }
            assertTrue(view().ankiEntries.any { it.phase == AnkiEntryPhase.Checking })
            providerRelease!!.countDown()
            await {
                it.ankiEntries.isNotEmpty() &&
                    it.ankiEntries.all { it.phase != AnkiEntryPhase.Checking }
            }
            compose.waitForIdle()
            captureUiTrial(compose, "anki-word-sheet")
            val settled = view().ankiEntries
            val queries = entryQueries.get()
            fun assertSettled() {
                settled.forEach { entry ->
                    compose.onNodeWithTag("anki_entry_${ankiEntryTag(entry.key)}").assertExists()
                }
                compose
                    .onAllNodesWithContentDescription("Open 猫【ねこ】 in Anki")
                    .assertCountEquals(
                        settled.count {
                            it.headword == "猫" &&
                                it.reading == "ねこ" &&
                                it.phase is AnkiEntryPhase.Existing
                        }
                    )
                compose
                    .onAllNodes(
                        hasProgressBarRangeInfo(ProgressBarRangeInfo.Indeterminate) and
                            hasAnyAncestor(hasTestTag("reader_word_panel")),
                        useUnmergedTree = true,
                    )
                    .assertCountEquals(0)
                assertEquals(settled, view().ankiEntries)
                assertEquals(queries, entryQueries.get())
            }
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use {
                sql ->
                // Framework SQLite and bundled Rust SQLite do not share in-process lock
                // bookkeeping.
                // A separate process holds the writer until the pending UI has been checked.
                val lock =
                    ProcessBuilder(
                            "/system/bin/app_process",
                            "/system/bin",
                            "app.yomoku.SqliteWriteLock",
                            database.path,
                        )
                        .apply {
                            environment()["CLASSPATH"] =
                                instrumentation.context.applicationInfo.sourceDir
                            redirectErrorStream(true)
                        }
                        .start()
                try {
                    val output = lock.inputStream.bufferedReader()
                    val ready =
                        java.util.concurrent.CompletableFuture.supplyAsync {
                            val lines = StringBuilder()
                            while (true) {
                                val line = output.readLine() ?: error("Writer exited: $lines")
                                if (line == "locked") break
                                lines.appendLine(line)
                            }
                        }
                    ready.get(20, TimeUnit.SECONDS)
                    compose.onNodeWithTag("word_status_LEARNING").performClick()
                    assertTrue(view().effectiveReaderStatus!!.pending)
                    compose.onNodeWithTag("word_status_KNOWN").assertIsNotEnabled()
                    assertSettled()
                } finally {
                    lock.outputStream.close()
                    if (!lock.waitFor(10, TimeUnit.SECONDS)) lock.destroyForcibly()
                }
                await { !it.storageBusy && it.effectiveReaderStatus?.pending == false }
                compose.onNodeWithTag("word_status_LEARNING").assertIsSelected()
                assertSettled()
                sql.execSQL(
                    "CREATE TRIGGER fail_anki_status BEFORE INSERT ON vocabulary_status_events BEGIN SELECT RAISE(ABORT, 'test status'); END"
                )
                try {
                    compose.onNodeWithTag("word_status_KNOWN").performClick()
                    await { !it.storageBusy && it.effectiveReaderStatus?.error != null }
                    assertSettled()
                    compose
                        .onNodeWithTag("effective_status_retry")
                        .performScrollTo()
                        .assertIsEnabled()
                    sql.execSQL("DROP TRIGGER fail_anki_status")
                    compose.onNodeWithTag("effective_status_retry").performClick()
                    await { !it.storageBusy && it.effectiveReaderStatus?.pending == false }
                    compose.onNodeWithTag("word_status_KNOWN").performScrollTo().assertIsSelected()
                    assertSettled()
                } finally {
                    sql.execSQL("DROP TRIGGER IF EXISTS fail_anki_status")
                }
            }
        } finally {
            providerRelease?.countDown()
            compose.runOnUiThread { compose.activity.setContent {} }
            close()
            directory.deleteRecursively()
        }
    }
}
