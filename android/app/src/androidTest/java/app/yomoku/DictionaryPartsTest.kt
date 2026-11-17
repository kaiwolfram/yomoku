package app.yomoku

import android.database.sqlite.SQLiteDatabase
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import java.io.File
import java.util.concurrent.CompletableFuture
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class DictionaryPartsTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private lateinit var core: CoreSession
    private val queries = AtomicInteger()
    private val sentences = CopyOnWriteArrayList<String>()

    private fun send(event: Event) = compose.runOnUiThread { core.dispatch(event) }

    private fun await(predicate: (ViewModel) -> Boolean) {
        compose.waitUntil(90_000) {
            check(core.view.documentError == null) { core.view.documentError.toString() }
            !core.view.storageBusy && predicate(core.view)
        }
        compose.waitForIdle()
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()

    private fun choose(surface: String, vocabulary: Boolean = false) {
        val lookup =
            if (vocabulary) core.view.vocabulary.word!!.lookup
            else core.view.effectiveReaderWord!!.details!!.lookup
        val index = lookup.parts.indexOfFirst { it.surface == surface }
        assertTrue(index >= 0)
        click("dictionary_part_$index")
        await {
            val current =
                if (vocabulary) it.vocabulary.word!!.lookup
                else it.effectiveReaderWord!!.details!!.lookup
            !current.loading && current.active == index.toULong()
        }
        compose.onNodeWithTag("dictionary_part_$index").assertIsSelected()
    }

    private fun addEntry() {
        await {
            it.ankiEntries.isNotEmpty() &&
                it.ankiEntries.all { entry -> entry.phase != AnkiEntryPhase.Checking }
        }
        val entry = core.view.ankiEntries.first { it.phase == AnkiEntryPhase.Add }
        click("anki_entry_${ankiEntryTag(entry.key)}")
        await { it.ankiEntries.any { entry -> entry.phase is AnkiEntryPhase.Existing } }
    }

    private fun checkStatusWrites(database: File, vocabulary: Boolean) {
        fun lookup() =
            if (vocabulary) core.view.vocabulary.word!!.lookup
            else core.view.effectiveReaderWord!!.details!!.lookup
        fun candidates() =
            if (vocabulary) core.view.vocabulary.word!!.candidates
            else core.view.effectiveReaderWord!!.details!!.candidates
        fun pending() =
            if (vocabulary) core.view.vocabulary.wordStatus != null
            else core.view.effectiveReaderStatus?.pending == true
        val selected = lookup()
        val definitions = candidates()
        val reads = queries.get()
        val retryTag = if (vocabulary) "vocabulary_word_status_retry" else "effective_status_retry"
        val expandedPart = selected.parts.indexOfFirst { !it.primary }
        fun assertRetained() {
            assertEquals(selected, lookup())
            assertEquals(definitions, candidates())
            assertEquals(reads, queries.get())
            compose.onNodeWithTag("dictionary_part_${selected.active}").assertIsSelected()
            compose.onNodeWithTag("dictionary_part_$expandedPart").assertExists()
            compose.onNodeWithTag("dictionary_more_parts").assertDoesNotExist()
            compose.onNodeWithTag("word_lookup_error").assertDoesNotExist()
            if (pending()) {
                compose.onNodeWithTag("dictionary_whole").assertIsNotEnabled().performClick()
                compose
                    .onNodeWithTag("dictionary_part_${selected.active}")
                    .assertIsNotEnabled()
                    .performClick()
                compose.onNodeWithTag("word_status_FAMILIAR").assertIsNotEnabled().performClick()
                assertEquals(selected, lookup())
            }
        }
        val lock =
            ProcessBuilder(
                    "/system/bin/app_process",
                    "/system/bin",
                    "app.yomoku.SqliteWriteLock",
                    database.path,
                )
                .apply {
                    environment()["CLASSPATH"] = instrumentation.context.applicationInfo.sourceDir
                    redirectErrorStream(true)
                }
                .start()
        try {
            CompletableFuture.runAsync {
                    val output = lock.inputStream.bufferedReader()
                    while (true) {
                        val line = output.readLine() ?: error("Writer exited before locking")
                        if (line == "locked") break
                    }
                }
                .get(20, TimeUnit.SECONDS)
            compose.onNodeWithTag("word_status_LEARNING").performScrollTo()
            val scroll =
                compose
                    .onNodeWithTag("reader_word_panel")
                    .fetchSemanticsNode()
                    .config[SemanticsProperties.VerticalScrollAxisRange]
                    .value()
            compose.onNodeWithTag("word_status_LEARNING").performClick()
            assertTrue(pending())
            assertRetained()
            assertEquals(
                scroll,
                compose
                    .onNodeWithTag("reader_word_panel")
                    .fetchSemanticsNode()
                    .config[SemanticsProperties.VerticalScrollAxisRange]
                    .value(),
            )
        } finally {
            lock.outputStream.close()
            if (!lock.waitFor(10, TimeUnit.SECONDS)) lock.destroyForcibly()
        }
        await { !pending() }
        compose.onNodeWithTag("word_status_LEARNING").assertIsSelected()
        assertRetained()
        SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use { sql ->
            sql.execSQL(
                "CREATE TRIGGER fail_parts_status BEFORE INSERT ON vocabulary_status_events BEGIN SELECT RAISE(ABORT, 'test status'); END"
            )
            try {
                click("word_status_KNOWN")
                await {
                    if (vocabulary) it.vocabulary.wordStatusError != null
                    else it.effectiveReaderStatus?.error != null
                }
                assertRetained()
                sql.execSQL("DROP TRIGGER fail_parts_status")
                click(retryTag)
                await { !pending() }
                compose.onNodeWithTag("word_status_KNOWN").assertIsSelected()
                assertRetained()
            } finally {
                sql.execSQL("DROP TRIGGER IF EXISTS fail_parts_status")
            }
        }
    }

    @Test
    fun readerAndVocabularyReplaceDefinitionsKeepOriginalStatusAndExportSelectedOccurrences() {
        val directory =
            File(instrumentation.targetContext.noBackupFilesDir, "parts-${System.nanoTime()}")
                .apply { mkdirs() }
        val database = File(directory, "user.sqlite3")
        compose.runOnUiThread {
            core =
                CoreSession(
                    database.path,
                    sudachiDictionaryProvider(instrumentation.targetContext),
                    readAnkiPreflight = { operation ->
                        if (operation.surface.isNotEmpty()) queries.incrementAndGet()
                        AnkiPreflightResult.Complete(
                            AnkiPreflightObservation(
                                listOf(AnkiPreflightDeck(7, false)),
                                emptyList(),
                                listOf(AnkiPreflightModel(9, operation.layout.name, true)),
                            )
                        )
                    },
                    invokeAnkiLive = { operation ->
                        val note = operation as AnkiLiveOperation.InsertNote
                        sentences.add(note.fields[3])
                        AnkiLiveResult.NoteAdded(88L + sentences.size)
                    },
                )
        }
        try {
            await { it.ankiSettings.phase == AnkiSettingsPhase.READY }
            send(Event.OpenSettings)
            send(Event.SetAnkiDefaultDeck(7))
            await { it.ankiSetup == AnkiSetupPhase.Ready }
            send(Event.CloseSettings)
            send(Event.CreatePaste("ふくろう便。"))
            await { it.documents.size == 1 }
            val id = core.view.documents.single().documentId
            send(Event.OpenReader(id))
            await { it.reader != null }
            compose.setContent {
                YomokuTheme { YomokuApp(core.view, core.sessionKey, core::dispatch) }
            }
            await { it.effectiveReaderTokens != null }
            val token = core.view.effectiveReaderTokens!!.tokens.single { it.surface == "ふくろう便" }
            fun tapWord() {
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
                            (it.item as? androidx.compose.ui.text.LinkAnnotation.Clickable)?.tag ==
                                "reader_token_${token.segmentIndex}"
                        }
                        .item
                compose.runOnUiThread { link.linkInteractionListener!!.onClick(link) }
                await { it.effectiveReaderWord?.details != null }
            }
            tapWord()
            compose.onNodeWithTag("word_surface").assertTextEquals("ふくろう便")
            compose.onNodeWithTag("word_no_match").assertExists()
            val before = core.view.effectiveReaderWord!!.details!!
            assertEquals(
                listOf("ふくろう", "便"),
                before.lookup.parts.filter { it.primary }.map { it.surface },
            )
            val shorter = before.lookup.parts.indexOfFirst { !it.primary }
            assertTrue(shorter >= 0)
            compose.onNodeWithTag("dictionary_part_$shorter").assertDoesNotExist()
            click("dictionary_more_parts")
            compose.onNodeWithTag("dictionary_part_$shorter").assertExists()
            choose("ふくろう")
            compose.onNodeWithTag("word_no_match").assertDoesNotExist()
            assertTrue(
                core.view.effectiveReaderWord!!.details!!.candidates.words.all { candidate ->
                    candidate.forms.all { it.form == "ふくろう" }
                }
            )
            choose("便")
            await {
                it.ankiEntries.isNotEmpty() &&
                    it.ankiEntries.all { entry -> entry.phase != AnkiEntryPhase.Checking }
            }
            val selected = core.view.effectiveReaderWord!!.details!!
            val reads = queries.get()
            checkStatusWrites(database, vocabulary = false)
            assertEquals(selected.lookup, core.view.effectiveReaderWord!!.details!!.lookup)
            assertEquals(reads, queries.get())
            assertEquals(before.encountered, core.view.effectiveReaderWord!!.details!!.encountered)
            addEntry()
            assertEquals(
                listOf("ふくろう<strong class=\"yomoku-target\">便</strong>。"),
                sentences.toList(),
            )
            click("dictionary_whole")
            await { it.effectiveReaderWord!!.details!!.lookup.active == null }
            compose.onNodeWithTag("word_no_match").assertExists()
            send(Event.DismissReaderWord)
            compose.waitForIdle()
            tapWord()
            assertNull(core.view.effectiveReaderWord!!.details!!.lookup.active)
            send(Event.DismissReaderWord)
            send(Event.CloseDocument)
            await { it.reader == null }
            send(Event.SelectDestination(Destination.VOCABULARY))
            await { it.vocabulary.matchingCount != null }
            val row = core.view.vocabulary.rows.single { it.surface == "ふくろう便" }
            click("vocabulary_row_${row.id.value}")
            await { it.vocabulary.word?.candidates != null }
            compose.onNodeWithTag("word_no_match").assertExists()
            choose("ふくろう", vocabulary = true)
            assertEquals(row, core.view.vocabulary.word!!.row)
            addEntry()
            assertEquals("", sentences.last())
            click("dictionary_more_parts")
            checkStatusWrites(database, vocabulary = true)
            click("dictionary_whole")
            await { it.vocabulary.word!!.lookup.active == null }
            compose.onNodeWithTag("word_no_match").assertExists()
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READONLY).use { sql
                ->
                sql.rawQuery("SELECT surface FROM vocabulary", null).use { rows ->
                    assertEquals(1, rows.count)
                    assertTrue(rows.moveToFirst())
                    assertEquals("ふくろう便", rows.getString(0))
                }
                sql.rawQuery("SELECT COUNT(*) FROM reading_occurrences", null).use { rows ->
                    assertTrue(rows.moveToFirst())
                    assertEquals(0, rows.getInt(0))
                }
            }
        } finally {
            compose.runOnUiThread {
                compose.activity.setContent {}
                core.close()
            }
        }
    }
}
