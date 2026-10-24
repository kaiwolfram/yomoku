package app.yomoku

import android.database.sqlite.SQLiteDatabase
import androidx.activity.compose.setContent
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderWordsTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private var testCore: CoreSession? = null

    private fun core() =
        testCore ?: ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun awaitWords() {
        compose.waitForIdle()
        compose.waitUntil(30_000) {
            val view = core().view
            !view.storageBusy &&
                !view.readingTimeSavePending &&
                !view.completionSavePending &&
                view.readerTime != null &&
                view.readerTokens != null
        }
        compose.waitForIdle()
    }

    private fun openDocument(): Long {
        compose.waitUntil(30_000) { !core().view.storageBusy }
        val title = "Words ${System.nanoTime()}"
        compose.runOnUiThread {
            core().dispatch(Event.CreatePaste("$title\n😀 𠮷野家 猫 猫 犬 か\u3099\u0000"))
        }
        compose.waitUntil(60_000) { !core().view.storageBusy }
        val id = core().view.documents.single { it.title == title }.documentId
        compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
        awaitWords()
        val cat = core().view.readerTokens!!.tokens.first { it.surface == "猫" }.vocabulary!!
        val key = key()
        compose.runOnUiThread {
            core()
                .dispatch(
                    Event.SetReaderStatus(
                        ReaderStatusChange(
                            key,
                            cat.id,
                            VocabularyStatus.NEW,
                            System.currentTimeMillis(),
                        )
                    )
                )
        }
        awaitWords()
        return id.value
    }

    private fun key(): ReaderTokenKey {
        val view = core().view
        val page = view.reader!!
        return ReaderTokenKey(page.documentId, page.anchor, view.readerAnalysis!!)
    }

    private fun database() =
        SQLiteDatabase.openDatabase(
                File(compose.activity.filesDir, "yomoku.sqlite3").path,
                null,
                SQLiteDatabase.OPEN_READWRITE,
            )
            .apply { setForeignKeyConstraintsEnabled(true) }

    private fun count(db: SQLiteDatabase, sql: String) =
        db.rawQuery(sql, null).use {
            it.moveToFirst()
            it.getLong(0)
        }

    private fun tapSurface(surface: String) {
        val page = core().view.reader!!
        val snapshot = core().view.readerTokens!!
        val span =
            readerDisplayTokens(page, key(), snapshot)!!.first { it.token.surface == surface }
        val node = compose.onNodeWithTag("reader_text").performScrollTo()
        val layouts = mutableListOf<androidx.compose.ui.text.TextLayoutResult>()
        node.fetchSemanticsNode().config[SemanticsActions.GetTextLayoutResult].action!!(layouts)
        val box = layouts.single().getBoundingBox(span.start)
        node.performTouchInput { click(Offset(box.center.x, box.center.y)) }
        compose.onNodeWithTag("reader_word_panel").assertIsDisplayed()
        val wide = compose.activity.resources.configuration.screenWidthDp >= 600
        compose
            .onNodeWithTag(if (wide) "word_side_panel" else "word_bottom_sheet")
            .assertIsDisplayed()
        compose.onNodeWithTag("word_surface").assertTextEquals(surface)
    }

    private fun assertCats(status: VocabularyStatus) {
        val cats = core().view.readerTokens!!.tokens.filter { it.surface == "猫" }
        assertEquals(2, cats.size)
        assertTrue(cats.all { it.vocabulary!!.status == status })
        compose.onNodeWithTag("word_status_$status").assertIsSelected()
        val text =
            compose
                .onNodeWithTag("reader_text")
                .fetchSemanticsNode()
                .config[SemanticsProperties.Text]
                .single()
        val catRanges =
            readerDisplayTokens(core().view.reader!!, key(), core().view.readerTokens!!)!!.filter {
                it.token.surface == "猫"
            }
        for (span in catRanges) {
            val link = text.getLinkAnnotations(span.start, span.end).single().item
            val dark =
                compose.activity.resources.configuration.uiMode and
                    android.content.res.Configuration.UI_MODE_NIGHT_MASK ==
                    android.content.res.Configuration.UI_MODE_NIGHT_YES
            assertEquals(vocabularyBackground(status, dark), link.styles!!.style!!.background)
        }
    }

    @Test
    fun tappingAndFiveChoicesPersistWithoutLookupHistoryAndFailedChoiceCanRepeat() {
        val id = openDocument()
        val original = core().view.reader!!
        val catId =
            core().view.readerTokens!!.tokens.first { it.surface == "猫" }.vocabulary!!.id.value
        database().use { db ->
            val eventsSql =
                "SELECT count(*) FROM vocabulary_status_events WHERE vocabulary_id = $catId"
            val readingSql = "SELECT count(*) FROM reading_completions WHERE document_id = $id"
            val baseline = count(db, eventsSql)
            tapSurface("猫")
            assertCats(VocabularyStatus.NEW)
            assertEquals(baseline, count(db, eventsSql))
            assertEquals(0, count(db, readingSql))
            for (status in
                listOf(
                    VocabularyStatus.NEW,
                    VocabularyStatus.LEARNING,
                    VocabularyStatus.FAMILIAR,
                    VocabularyStatus.KNOWN,
                    VocabularyStatus.IGNORED,
                    VocabularyStatus.NEW,
                )) {
                compose.onNodeWithTag("word_status_$status").performClick()
                awaitWords()
                assertCats(status)
                assertEquals(original, core().view.reader)
                assertEquals(0, count(db, readingSql))
            }
            assertEquals(baseline + 5, count(db, eventsSql))
            try {
                db.execSQL(
                    "CREATE TRIGGER fail_word_ui BEFORE UPDATE ON vocabulary WHEN OLD.id = $catId BEGIN SELECT RAISE(ABORT, 'test choice'); END"
                )
                compose.onNodeWithTag("word_status_LEARNING").performClick()
                compose.waitUntil(30_000) {
                    !core().view.storageBusy && core().view.documentError != null
                }
                assertCats(VocabularyStatus.NEW)
                assertEquals(baseline + 5, count(db, eventsSql))
            } finally {
                db.execSQL("DROP TRIGGER IF EXISTS fail_word_ui")
            }
            compose.waitForIdle()
            assertEquals(baseline + 5, count(db, eventsSql))
            compose.onNodeWithTag("word_status_LEARNING").performClick()
            awaitWords()
            assertCats(VocabularyStatus.LEARNING)
            assertNull(core().view.documentError)
            assertEquals(baseline + 6, count(db, eventsSql))
            compose.onNodeWithText("Close").performClick()
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
            compose.runOnUiThread { core().dispatch(Event.LoadReaderTokens(key())) }
            awaitWords()
            tapSurface("猫")
            assertCats(VocabularyStatus.LEARNING)
            assertEquals(baseline + 6, count(db, eventsSql))
            androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
                .sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_BACK)
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
            assertEquals(original, core().view.reader)
            tapSurface("猫")
            compose.activityRule.scenario.recreate()
            awaitWords()
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
            tapSurface("猫")
            assertCats(VocabularyStatus.LEARNING)
            assertEquals(0, count(db, readingSql))
        }
    }

    @Test
    fun popupShowsWholeSupplementarySurfaceAndDeletedVocabularyCannotBeRegisteredByTap() {
        openDocument()
        val supplementarySurface =
            core().view.readerTokens!!.tokens.first { it.surface.contains("𠮷") }.surface
        tapSurface(supplementarySurface)
        compose.onNodeWithText("Close").performClick()
        val cat = core().view.readerTokens!!.tokens.first { it.surface == "猫" }.vocabulary!!
        database().use { db ->
            db.execSQL("DELETE FROM vocabulary WHERE id = ${cat.id.value}")
            compose.runOnUiThread { core().dispatch(Event.LoadReaderTokens(key())) }
            awaitWords()
            tapSurface("猫")
            for (status in VocabularyStatus.entries) {
                compose.onNodeWithTag("word_status_$status").assertIsNotEnabled()
            }
            assertEquals(0, count(db, "SELECT count(*) FROM vocabulary WHERE surface = '猫'"))
        }
    }

    @Test
    fun popupWindowInteractionsExtendIdleDeadlineAndKeepAccumulatingPartialTime() {
        var now = 0L
        lateinit var session: CoreSession
        compose.runOnUiThread {
            session =
                CoreSession(
                    ":memory:",
                    sudachiDictionaryProvider(compose.activity),
                    monotonicMs = { now },
                )
            testCore = session
            session.setReaderActive(true)
            compose.activity.setContent {
                YomokuTheme {
                    ReaderScreen(session.view, session::dispatch, session::onReaderInteraction)
                }
            }
        }
        try {
            openDocument()
            val original = session.view.reader
            compose.runOnUiThread { now = 299_000L }
            tapSurface("猫")
            compose.runOnUiThread { now = 550_000L }
            // Touch metadata, without selecting a status. This window bypasses Activity input.
            compose.onNodeWithTag("word_surface").performTouchInput { click() }
            compose.runOnUiThread {
                now = 750_000L
                session.setReaderActive(false)
            }
            awaitWords()
            assertEquals(750_000L, session.view.readerTime!!.pending.value)
            assertEquals(original, session.view.reader)
            assertNull(session.view.completionNotice)
            assertTrue(
                session.view.readerTokens!!
                    .tokens
                    .filter { it.surface == "猫" }
                    .all { it.vocabulary!!.status == VocabularyStatus.NEW }
            )
        } finally {
            compose.runOnUiThread {
                compose.activity.setContent {}
                session.close()
                testCore = null
            }
            compose.waitUntil(30_000) { session.isClosed }
        }
    }

    @Test
    fun loadingWaitsForStorageAndErrorsRequireExplicitRetryWhilePendingSavesDisableChoices() {
        openDocument()
        val base = core().view
        val expectedKey = key()
        var frame by mutableStateOf(base.copy(readerTokens = null, storageBusy = true))
        val events = mutableListOf<Event>()
        compose.runOnUiThread {
            compose.activity.setContent {
                YomokuTheme { ReaderWords(frame, { events.add(it) }, {}) }
            }
        }
        compose.waitForIdle()
        assertTrue(events.isEmpty())
        compose.runOnUiThread { frame = frame.copy(storageBusy = false) }
        compose.waitForIdle()
        assertEquals(listOf(Event.LoadReaderTokens(expectedKey)), events)
        compose.runOnUiThread { frame = frame.copy(documentError = DocumentError.UNAVAILABLE) }
        compose.waitForIdle()
        assertEquals(1, events.size)
        // A queued clock save can clear the shared error; this must not replay the failed read.
        compose.runOnUiThread { frame = frame.copy(documentError = null) }
        compose.waitForIdle()
        assertEquals(1, events.size)
        compose.onNodeWithTag("retry_word_lookup").performClick()
        assertEquals(
            listOf(Event.LoadReaderTokens(expectedKey), Event.LoadReaderTokens(expectedKey)),
            events,
        )
        compose.runOnUiThread { frame = base }
        compose.waitForIdle()
        // Link listeners are also used by accessibility/keyboard activation.
        val text =
            compose
                .onNodeWithTag("reader_text")
                .fetchSemanticsNode()
                .config[SemanticsProperties.Text]
                .single()
        val cat =
            readerDisplayTokens(base.reader!!, expectedKey, base.readerTokens!!)!!.first {
                it.token.surface == "猫"
            }
        val link = text.getLinkAnnotations(cat.start, cat.end).single().item
        compose.runOnUiThread { link.linkInteractionListener!!.onClick(link) }
        compose.onNodeWithTag("reader_word_panel").assertIsDisplayed()
        compose.runOnUiThread { frame = frame.copy(readingTimeSavePending = true) }
        for (status in VocabularyStatus.entries) compose
            .onNodeWithTag("word_status_$status")
            .assertIsNotEnabled()
        compose.runOnUiThread { frame = frame.copy(readerTokens = null) }
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        compose.runOnUiThread { frame = base }
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        assertEquals(2, events.size)
    }
}
