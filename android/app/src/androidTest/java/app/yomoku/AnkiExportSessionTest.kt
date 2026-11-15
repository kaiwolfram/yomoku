package app.yomoku

import android.content.Intent
import android.database.sqlite.SQLiteDatabase
import android.os.Handler
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
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
                        if (it.surface.isNotEmpty()) preflightReads++
                        preflightFailure
                            ?.takeIf { _ -> it.surface.isNotEmpty() }
                            ?.let { failure -> AnkiPreflightResult.Failed(failure) }
                            ?: AnkiPreflightResult.Complete(
                                AnkiPreflightObservation(
                                    listOf(AnkiPreflightDeck(Long.MAX_VALUE, false)),
                                    if (it.surface.isEmpty()) emptyList() else duplicates,
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
                    resolveAnkiNotes = { ids ->
                        assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                        AnkiReadResult.Value(
                            Intent("test.notes.${ids.joinToString(",") { it.value.toString() }}")
                        )
                    },
                    resolveAnkiNote = {
                        assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                        AnkiReadResult.Value(Intent("test.note.${it.value}"))
                    },
                    invokeAnkiLive = {
                        assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                        SQLiteDatabase.openDatabase(
                                File(directory, "db.sqlite3").path,
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
        val prepare =
            Event.PrepareReaderAnkiExport(
                AnkiSelection.Effective(
                    selected,
                    core.view.effectiveReaderWord!!.details!!.lookup.revision,
                )
            )
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
        assertNull(core.view.ankiExport)
        send(
            Event.PrepareReaderAnkiExport(
                AnkiSelection.Effective(
                    selected,
                    core.view.effectiveReaderWord!!.details!!.lookup.revision,
                )
            )
        )
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
        await { it.ankiSetup == AnkiSetupPhase.Ready }
        send(Event.CloseSettings)
        compose.setContent {
            YomokuTheme {
                YomokuApp(
                    core.view,
                    core.sessionKey,
                    core::dispatch,
                    ankiDecks = core.ankiDecks.state,
                    ankiBrowser = core.ankiBrowser,
                )
            }
        }
        tapCat(last = true)
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()

    private fun entryTag(entry: AnkiEntryView): String =
        "anki_entry_" +
            when (val key = entry.key) {
                is AnkiEntryKey.Word -> ankiEntryTag(key)
                is AnkiEntryKey.Name -> ankiEntryTag(key)
            }

    @Test
    fun dictionaryAddFreezesTappedOccurrenceAcrossDismissalAndKeepsReaderState() {
        open()
        try {
            showDictionary()
            val original = core.view
            await { it.ankiEntries.any { entry -> entry.phase == AnkiEntryPhase.Add } }
            val entry = core.view.ankiEntries.first { it.phase == AnkiEntryPhase.Add }
            assertTrue(operations.isEmpty())
            click(entryTag(entry))
            val operation =
                checkNotNull(operations.poll(60, TimeUnit.SECONDS)) as AnkiLiveOperation.InsertNote
            await { it.ankiExport?.phase == AnkiExportPhase.Invoking }
            val accepted = core.view.ankiExport
            send(Event.DismissReaderWord)
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
            tapCat(last = false)
            assertEquals(accepted, core.view.ankiExport)
            assertEquals(Long.MAX_VALUE, operation.deckId)
            assertEquals("猫", operation.fields[0])
            assertEquals("猫と<strong class=\"yomoku-target\">猫</strong>。", operation.fields[3])
            duplicates = listOf(Long.MAX_VALUE)
            results.add(AnkiLiveResult.NoteAdded(Long.MAX_VALUE))
            await { it.ankiExport == null }
            // A different lookup owns fresh observations; the old write cannot relabel it.
            val current = core.view.effectiveReaderWord!!.details!!
            send(
                Event.LoadReaderAnkiEntries(
                    AnkiSelection.Effective(current.selection, current.lookup.revision)
                )
            )
            await {
                it.ankiExport == null &&
                    it.ankiEntries.any { entry ->
                        entry.phase == AnkiEntryPhase.Existing(listOf(Long.MAX_VALUE))
                    }
            }
            compose.onNodeWithTag("anki_export").assertDoesNotExist()
            assertEquals(original.reader, core.view.reader)
            assertEquals(original.effectiveReaderTokens, core.view.effectiveReaderTokens)
            assertEquals(original.statistics, core.view.statistics)
            assertEquals(original.completionNotice, core.view.completionNotice)
            tapCatAfterDismissal()
            val reopened = core.view.effectiveReaderWord!!.details!!
            assertEquals(
                original.effectiveReaderWord?.details,
                reopened.copy(
                    lookup =
                        reopened.lookup.copy(
                            revision = original.effectiveReaderWord!!.details!!.lookup.revision
                        )
                ),
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
        open()
        try {
            showDictionary()
            preflightFailure = AnkiExportError.PROVIDERUNAVAILABLE
            val selection =
                AnkiSelection.Effective(
                    core.view.effectiveReaderWord!!.selection!!,
                    core.view.effectiveReaderWord!!.details!!.lookup.revision,
                )
            send(Event.LoadReaderAnkiEntries(selection))
            await { it.ankiEntries.any { entry -> entry.phase is AnkiEntryPhase.Failed } }
            val entry = core.view.ankiEntries.first { it.phase is AnkiEntryPhase.Failed }
            compose.onAllNodesWithText("AnkiDroid is unavailable.").onFirst().assertExists()
            compose.onNodeWithTag("anki_export").assertDoesNotExist()
            preflightFailure = null
            duplicates = listOf(Long.MAX_VALUE, 9007199254740993L)
            click(entryTag(entry))
            await {
                it.ankiEntries.any { entry -> entry.phase == AnkiEntryPhase.Existing(duplicates) }
            }
            assertTrue(operations.isEmpty())
            assertTrue(preflightReads >= 2)
        } finally {
            close()
        }
    }

    @Test
    fun modelCreationStopsAfterAcknowledgementAndRestartCannotReplay() {
        createModel = true
        open()
        try {
            // Saving a deck creates a missing model in Settings before reader exports.
            send(Event.OpenSettings)
            send(Event.SetAnkiDefaultDeck(Long.MAX_VALUE))
            assertTrue(operations.poll(60, TimeUnit.SECONDS) is AnkiLiveOperation.CreateModel)
            results.add(AnkiLiveResult.ModelCreated(Long.MAX_VALUE))
            await { it.ankiSetup == AnkiSetupPhase.Ready && it.ankiExport == null }
            send(Event.RefreshLibrary)
            await { !it.storageBusy }
            assertTrue(operations.isEmpty())
            instrumentation.runOnMainSync { core.close() }
            open(useExecutor = false)
            assertNull(core.view.ankiExport)
            send(Event.DispatchAnkiLive)
            assertTrue(operations.isEmpty())
            send(Event.OpenSettings)
        } finally {
            close()
        }
    }

    @Test
    fun closeAtReadySuppressesQueuedInvocationWithoutRestartReplay() {
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
            send(Event.DispatchAnkiLive)
            assertTrue(operations.isEmpty())
        } finally {
            close()
        }
    }

    @Test
    fun inFlightCloseDropsLateResultAndRestartRequiresFreshQueries() {
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

    @Test
    fun dictionaryExistingNoteOpensExactIdsAndReturnsWithFrozenReaderAndHistory() {
        duplicates = listOf(Long.MAX_VALUE, 9007199254740993L)
        open()
        try {
            showDictionary()
            await {
                it.ankiEntries.any { entry -> entry.phase == AnkiEntryPhase.Existing(duplicates) }
            }
            val entry =
                core.view.ankiEntries.first { it.phase == AnkiEntryPhase.Existing(duplicates) }
            val frozen = core.view
            val opened = mutableListOf<String?>()
            instrumentation.runOnMainSync { core.ankiBrowser.attach { opened.add(it.action) } }
            click(entryTag(entry))
            compose.waitUntil(30_000) { opened.size == 1 }
            instrumentation.runOnMainSync {
                core.ankiBrowser.detach()
                core.ankiBrowser.attach { opened.add(it.action) }
            }
            assertEquals(listOf("test.notes.${duplicates.joinToString(",")}"), opened)
            assertNull(core.view.ankiExport)
            assertEquals(frozen.reader, core.view.reader)
            assertEquals(frozen.effectiveReaderWord, core.view.effectiveReaderWord)
            assertEquals(frozen.vocabulary, core.view.vocabulary)
            assertEquals(frozen.statistics, core.view.statistics)
            send(Event.OpenSettings)
            instrumentation.runOnMainSync { core.ankiBrowser.openEntry(entry) }
            executor.submit {}.get(30, TimeUnit.SECONDS)
            instrumentation.waitForIdleSync()
            assertEquals(1, opened.size)
            assertTrue(operations.isEmpty())
        } finally {
            close()
        }
    }
}
