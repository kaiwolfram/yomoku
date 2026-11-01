package app.yomoku

import android.database.sqlite.SQLiteDatabase
import android.os.Handler
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.text.LinkAnnotation
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import java.io.File
import java.util.concurrent.Executors
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Actual CoreSession/JNI/FIFO/SQLite; injected provider calls never touch a real collection. */
@RunWith(AndroidJUnit4::class)
class AnkiExportSessionTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val directory =
        File(instrumentation.targetContext.noBackupFilesDir, "anki-export-${System.nanoTime()}")
    private val operations = LinkedBlockingQueue<AnkiLiveOperation>()
    private val results = LinkedBlockingQueue<AnkiLiveResult>()
    private val phases = mutableListOf<AnkiExportPhase?>()
    private var createModel = false
    private var preflightFailure: AnkiExportError? = null
    private var duplicates = emptyList<Long>()
    private var preflightReads = 0
    private val executor = Executors.newSingleThreadExecutor()
    private lateinit var core: CoreSession

    private fun await(predicate: (ViewModel) -> Boolean) {
        try {
            compose.waitUntil(30_000) {
                var matches = false
                instrumentation.runOnMainSync { matches = predicate(core.view) }
                matches
            }
        } catch (error: ComposeTimeoutException) {
            throw AssertionError(
                "Export phases: $phases; last view: ${core.view.ankiExport}",
                error,
            )
        }
    }

    private fun send(event: Event) = instrumentation.runOnMainSync { core.dispatch(event) }

    private fun open(useExecutor: Boolean = true) {
        directory.mkdirs()
        instrumentation.runOnMainSync {
            core =
                CoreSession(
                    File(directory, "db.sqlite3").path,
                    sudachiDictionaryProvider(instrumentation.targetContext),
                    ankiExecutor =
                        if (useExecutor) executor else Executors.newSingleThreadExecutor(),
                    readAnkiPreflight = {
                        assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                        preflightReads++
                        preflightFailure?.let { failure -> AnkiPreflightResult.Failed(failure) }
                            ?: AnkiPreflightResult.Complete(
                                AnkiPreflightObservation(
                                    listOf(AnkiPreflightDeck(Long.MAX_VALUE, false)),
                                    duplicates,
                                    if (createModel) emptyList()
                                    else
                                        listOf(
                                            AnkiPreflightModel(
                                                9007199254740993L,
                                                it.layout.name,
                                                true,
                                            )
                                        ),
                                )
                            )
                    },
                    readAnkiDecks = {
                        AnkiReadResult.Value(listOf(AnkiProviderDeck(Long.MAX_VALUE, "日本語", false)))
                    },
                    invokeAnkiLive = {
                        assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                        // An invocation must see its already committed pending journal.
                        val table =
                            if (it is AnkiLiveOperation.CreateModel) "anki_model_creations"
                            else "anki_submissions"
                        SQLiteDatabase.openDatabase(
                                File(directory, "db.sqlite3").path,
                                null,
                                SQLiteDatabase.OPEN_READONLY,
                            )
                            .use { sql ->
                                sql.rawQuery(
                                        "SELECT state FROM $table ORDER BY id DESC LIMIT 1",
                                        null,
                                    )
                                    .use { row ->
                                        assertTrue(row.moveToFirst())
                                        assertEquals("pending", row.getString(0))
                                    }
                            }
                        operations.add(it)
                        checkNotNull(results.poll(60, TimeUnit.SECONDS))
                    },
                )
            core.onViewChanged = { phases.add(core.view.ankiExport?.phase) }
        }
        await { !it.storageBusy }
    }

    private fun prepare(): AnkiExportView {
        send(Event.CreatePaste("猫と猫。"))
        await { it.documents.size == 1 && !it.storageBusy }
        send(Event.OpenReader(core.view.documents.single().documentId))
        await { it.reader != null && !it.storageBusy }
        val view = core.view
        val page = checkNotNull(view.reader)
        send(
            Event.LoadReaderTokens(
                ReaderTokenKey(page.documentId, page.anchor, checkNotNull(view.readerAnalysis))
            )
        )
        await { it.effectiveReaderTokens != null && !it.storageBusy }
        val tokens = checkNotNull(core.view.effectiveReaderTokens)
        val selected =
            EffectiveReaderSelection(
                tokens.identity,
                tokens.tokens.last { it.surface == "猫" }.segmentIndex,
            )
        send(Event.LoadEffectiveReaderWord(selected))
        await { it.effectiveReaderWord?.details != null && !it.storageBusy }
        val prepare = Event.PrepareReaderAnkiExport(AnkiSelection.Effective(selected))
        send(prepare)
        assertEquals(null, core.view.ankiExport)
        send(Event.OpenSettings)
        send(Event.LoadAnkiSettings)
        await { it.ankiSettings.phase == AnkiSettingsPhase.READY }
        assertEquals(null, core.view.ankiSettings.defaultDeckId)
        send(Event.CloseSettings)
        send(Event.LoadEffectiveReaderWord(selected))
        await { it.effectiveReaderWord?.details != null && !it.storageBusy }
        send(prepare)
        assertEquals(null, core.view.ankiExport)
        send(Event.OpenSettings)
        send(Event.SetAnkiDefaultDeck(Long.MAX_VALUE))
        await { it.ankiSettings.phase == AnkiSettingsPhase.READY }
        send(prepare)
        assertEquals(null, core.view.ankiExport)
        send(Event.CloseSettings)
        send(Event.LoadEffectiveReaderWord(selected))
        await { it.effectiveReaderWord?.details != null && !it.storageBusy }
        send(prepare)
        return checkNotNull(core.view.ankiExport)
    }

    private fun checkAndBegin() {
        send(Event.BeginAnkiExportPreflight)
        await {
            it.ankiExport?.phase ==
                if (createModel) AnkiExportPhase.CreateModel
                else AnkiExportPhase.ReuseModel(9007199254740993L)
        }
        val frozen = checkNotNull(core.view.ankiExport)
        send(Event.BeginPreparedAnkiExport(frozen.preparationId, frozen.phase))
    }

    private fun close() {
        results.offer(AnkiLiveResult.NoSubmission)
        instrumentation.runOnMainSync { core.close() }
        compose.waitUntil(30_000) { core.isClosed }
        assertTrue(executor.awaitTermination(30, TimeUnit.SECONDS))
        directory.deleteRecursively()
    }

    private fun tapCat(last: Boolean) {
        val cats = core.view.effectiveReaderTokens!!.tokens.filter { it.surface == "猫" }
        val token = if (last) cats.last() else cats.first()
        val text =
            compose
                .onNodeWithTag("reader_text")
                .fetchSemanticsNode()
                .config[SemanticsProperties.Text]
                .single()
        val link =
            text
                .getLinkAnnotations(0, text.length)
                .single {
                    (it.item as? LinkAnnotation.Clickable)?.tag ==
                        "reader_token_${token.segmentIndex}"
                }
                .item
        compose.runOnUiThread { link.linkInteractionListener!!.onClick(link) }
        await { it.effectiveReaderWord?.details?.token?.segmentIndex == token.segmentIndex }
        compose.waitForIdle()
    }

    private fun showDictionary() {
        val prepared = prepare()
        send(Event.ClearAnkiExport(prepared.preparationId))
        send(Event.OpenSettings)
        instrumentation.runOnMainSync { core.checkAnkiDecks() }
        compose.waitUntil(30_000) { core.ankiDecks.state is AnkiDeckDiscoveryState.Ready }
        send(Event.CloseSettings)
        compose.setContent {
            YomokuTheme {
                YomokuApp(
                    core.view,
                    core.sessionKey,
                    core::dispatch,
                    ankiDecks = core.ankiDecks.state,
                )
            }
        }
        tapCat(last = true)
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()

    @Test
    fun dictionaryAddFreezesTappedOccurrenceAcrossDismissalAndKeepsReaderState() {
        open()
        try {
            showDictionary()
            val original = core.view
            assertEquals(0, preflightReads)
            assertTrue(operations.isEmpty())
            click("word_anki_add")
            assertEquals(AnkiExportPhase.Prepared, core.view.ankiExport?.phase)
            compose.onNodeWithTag("word_anki_add").assertDoesNotExist()
            assertEquals(0, preflightReads)
            val prepared = core.view.ankiExport
            send(Event.DismissReaderWord)
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
            tapCat(last = false)
            assertEquals(prepared, core.view.ankiExport)
            assertEquals(0, preflightReads)
            click("anki_export_check")
            await { it.ankiExport?.phase is AnkiExportPhase.ReuseModel }
            click("anki_export_begin")
            val operation =
                checkNotNull(operations.poll(60, TimeUnit.SECONDS)) as AnkiLiveOperation.InsertNote
            assertEquals(Long.MAX_VALUE, operation.deckId)
            assertEquals("猫", operation.fields[0])
            assertEquals("猫と<strong class=\"yomoku-target\">猫</strong>。", operation.fields[3])
            results.add(AnkiLiveResult.NoteAdded(Long.MAX_VALUE))
            await { it.ankiExport?.phase == AnkiExportPhase.NoteAdded(Long.MAX_VALUE) }
            compose
                .onNodeWithTag("anki_export_status")
                .performScrollTo()
                .assertTextEquals("Added to AnkiDroid.")
            assertEquals(original.reader, core.view.reader)
            assertEquals(original.effectiveReaderTokens, core.view.effectiveReaderTokens)
            assertEquals(original.statistics, core.view.statistics)
            assertEquals(original.completionNotice, core.view.completionNotice)
            tapCatAfterDismissal()
            assertEquals(
                original.effectiveReaderWord?.details,
                core.view.effectiveReaderWord?.details,
            )
            assertTrue(operations.isEmpty())
        } finally {
            close()
        }
    }

    private fun tapCatAfterDismissal() {
        send(Event.DismissReaderWord)
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        tapCat(last = true)
    }

    @Test
    fun dictionaryPreflightRechecksAvailabilityAndShowsDuplicatesWithoutWrites() {
        preflightFailure = AnkiExportError.PROVIDERUNAVAILABLE
        open()
        try {
            showDictionary()
            click("word_anki_add")
            click("anki_export_check")
            await { it.ankiExport?.phase is AnkiExportPhase.ProviderFailed }
            compose
                .onNodeWithTag("anki_export_status")
                .performScrollTo()
                .assertTextEquals("AnkiDroid is unavailable.")
            compose.onNodeWithTag("anki_export_begin").assertDoesNotExist()
            compose.onNodeWithTag("word_anki_settings").assertExists()
            preflightFailure = null
            duplicates = listOf(Long.MAX_VALUE, 9007199254740993L)
            click("anki_export_check")
            await { it.ankiExport?.phase == AnkiExportPhase.Existing(duplicates) }
            compose
                .onNodeWithTag("anki_export_status")
                .performScrollTo()
                .assertTextEquals(
                    "Already in AnkiDroid. Note IDs: ${duplicates.joinToString(", ")}"
                )
            compose.onNodeWithTag("anki_export_begin").assertDoesNotExist()
            assertTrue(operations.isEmpty())
            assertEquals(2, preflightReads)
        } finally {
            close()
        }
    }

    @Test
    fun readyAutomaticallyInvokesOnceAndProjectsDurableOutcome() {
        open()
        try {
            prepare()
            val original = core.view
            instrumentation.runOnMainSync {
                core.dispatch(Event.OpenSettings)
                compose.activity.setContent {
                    YomokuTheme { SettingsScreen(core.view, core::dispatch) }
                }
            }
            compose.onNodeWithTag("anki_export_check").performScrollTo().performClick()
            await { it.ankiExport?.phase is AnkiExportPhase.ReuseModel }
            val checked = checkNotNull(core.view.ankiExport)
            compose.onNodeWithTag("anki_export_begin").performScrollTo().performClick()
            val operation =
                checkNotNull(operations.poll(60, TimeUnit.SECONDS)) as AnkiLiveOperation.InsertNote
            assertEquals(Long.MAX_VALUE, operation.deckId)
            assertEquals("猫", operation.fields[0])
            assertEquals("猫と<strong class=\"yomoku-target\">猫</strong>。", operation.fields[3])
            compose
                .onNodeWithTag("anki_export_status")
                .performScrollTo()
                .assertTextEquals("Sending to AnkiDroid…")
            compose.onNodeWithText("Added to AnkiDroid.").assertDoesNotExist()
            send(Event.BeginPreparedAnkiExport(checked.preparationId, checked.phase))
            send(Event.DispatchAnkiLive)
            send(Event.OpenSettings)
            assertTrue(operations.isEmpty())
            results.add(AnkiLiveResult.NoteAdded(Long.MAX_VALUE))
            await { it.ankiExport?.phase == AnkiExportPhase.NoteAdded(Long.MAX_VALUE) }
            compose
                .onNodeWithTag("anki_export_status")
                .performScrollTo()
                .assertTextEquals("Added to AnkiDroid.")
            SQLiteDatabase.openDatabase(
                    File(directory, "db.sqlite3").path,
                    null,
                    SQLiteDatabase.OPEN_READONLY,
                )
                .use { sql ->
                    sql.rawQuery("SELECT state, known_note_id FROM anki_submissions", null).use {
                        row ->
                        assertTrue(row.moveToFirst())
                        assertEquals("added", row.getString(0))
                        assertEquals(Long.MAX_VALUE, row.getLong(1))
                    }
                }
            assertEquals(original.reader, core.view.reader)
            assertEquals(original.effectiveReaderTokens, core.view.effectiveReaderTokens)
            send(Event.CloseSettings)
            send(
                Event.LoadEffectiveReaderWord(checkNotNull(original.effectiveReaderWord?.selection))
            )
            await { it.effectiveReaderWord?.details != null && !it.storageBusy }
            assertEquals(
                original.effectiveReaderWord.details,
                core.view.effectiveReaderWord?.details,
            )
            assertEquals(original.completionNotice, core.view.completionNotice)
            assertTrue(operations.isEmpty())
        } finally {
            close()
        }
    }

    @Test
    fun modelCreationStopsAfterAcknowledgementAndRestartCannotReplay() {
        createModel = true
        open()
        try {
            prepare()
            checkAndBegin()
            assertTrue(operations.poll(60, TimeUnit.SECONDS) is AnkiLiveOperation.CreateModel)
            results.add(AnkiLiveResult.ModelCreated(Long.MAX_VALUE))
            await { it.ankiExport?.phase == AnkiExportPhase.ModelCreated(Long.MAX_VALUE) }
            send(Event.RefreshLibrary)
            await { !it.storageBusy }
            assertTrue(operations.isEmpty())
            instrumentation.runOnMainSync { core.close() }
            open(useExecutor = false)
            assertNull(core.view.ankiExport)
            send(Event.DispatchAnkiLive)
            assertTrue(operations.isEmpty())
            send(Event.OpenSettings)
            assertNull(core.view.ankiRecovery)
        } finally {
            close()
        }
    }

    @Test
    fun closeAtReadyLeavesPendingJournalAndSuppressesQueuedInvocation() {
        open()
        try {
            prepare()
            instrumentation.runOnMainSync {
                core.onViewChanged = {
                    if (core.view.ankiExport?.phase == AnkiExportPhase.Ready) {
                        // Queued before CoreSession posts its Ready dispatch, but executed after
                        // this render finishes. Exercise the queued delivery close recheck.
                        Handler(Looper.getMainLooper()).post { core.close() }
                    }
                }
            }
            checkAndBegin()
            compose.waitUntil(30_000) { core.isClosed }
            assertTrue(executor.awaitTermination(30, TimeUnit.SECONDS))
            assertTrue(operations.isEmpty())
            open(useExecutor = false)
            assertNull(core.view.ankiExport)
            send(Event.OpenSettings)
            assertTrue(checkNotNull(core.view.ankiRecovery).notePending)
            send(Event.DispatchAnkiLive)
            assertTrue(operations.isEmpty())
        } finally {
            close()
        }
    }

    @Test
    fun inFlightCloseDropsLateResultAndRestartKeepsUncertainty() {
        open()
        try {
            prepare()
            checkAndBegin()
            assertTrue(operations.poll(60, TimeUnit.SECONDS) is AnkiLiveOperation.InsertNote)
            instrumentation.runOnMainSync { core.close() }
            assertTrue(core.isClosed)
            open(useExecutor = false)
            results.add(AnkiLiveResult.NoteAdded(Long.MAX_VALUE))
            assertTrue(executor.awaitTermination(30, TimeUnit.SECONDS))
            instrumentation.waitForIdleSync()
            assertNull(core.view.ankiExport)
            send(Event.OpenSettings)
            assertTrue(checkNotNull(core.view.ankiRecovery).notePending)
            send(Event.DispatchAnkiLive)
            assertTrue(operations.isEmpty())
        } finally {
            close()
        }
    }

    @Test
    fun failedAndDuplicatePreflightNeverBeginMutation() {
        preflightFailure = AnkiExportError.PROVIDERUNAVAILABLE
        open()
        try {
            prepare()
            send(Event.BeginAnkiExportPreflight)
            await {
                it.ankiExport?.phase ==
                    AnkiExportPhase.ProviderFailed(AnkiExportError.PROVIDERUNAVAILABLE)
            }
            preflightFailure = null
            duplicates = listOf(Long.MAX_VALUE, 9007199254740993L)
            send(Event.RetryAnkiExportPreflight)
            await { it.ankiExport?.phase == AnkiExportPhase.Existing(duplicates) }
            val export = checkNotNull(core.view.ankiExport)
            send(Event.BeginPreparedAnkiExport(export.preparationId, export.phase))
            send(Event.DispatchAnkiLive)
            assertTrue(operations.isEmpty())
        } finally {
            close()
        }
    }
}
