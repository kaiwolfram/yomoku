package app.yomoku

import android.database.sqlite.SQLiteDatabase
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
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
                view.effectiveReaderTokens != null
        }
        compose.waitForIdle()
    }

    private fun openDocument(suffix: String = ""): Long {
        compose.waitUntil(30_000) { !core().view.storageBusy }
        val title = "Words ${System.nanoTime()}"
        compose.runOnUiThread {
            core().dispatch(Event.CreatePaste("$title\n😀 𠮷野家 猫 猫 犬 東京 食べた か\u3099\u0000$suffix"))
        }
        compose.waitUntil(60_000) { !core().view.storageBusy }
        val id = core().view.documents.single { it.title == title }.documentId
        compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
        awaitWords()
        val snapshot = core().view.effectiveReaderTokens!!
        val token = snapshot.tokens.first { it.surface == "猫" }
        val cat = token.vocabulary!!
        compose.runOnUiThread {
            core()
                .dispatch(
                    Event.SetEffectiveReaderStatus(
                        EffectiveReaderSelection(snapshot.identity, token.segmentIndex),
                        cat.id,
                        VocabularyStatus.NEW,
                        System.currentTimeMillis(),
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

    private fun activateSurface(surface: String) {
        val page = core().view.reader!!
        val snapshot = core().view.effectiveReaderTokens!!
        val span =
            effectiveDisplayTokens(page, snapshot.identity, snapshot)!!.first {
                it.token.surface == surface
            }
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

    private fun tapSurface(surface: String) {
        activateSurface(surface)
        compose.waitUntil(30_000) {
            !core().view.storageBusy &&
                (core().view.effectiveReaderWord?.details != null ||
                    core().view.effectiveReaderWord?.error != null)
        }
        compose.waitForIdle()
    }

    private fun assertCats(status: VocabularyStatus) {
        val cats = core().view.effectiveReaderTokens!!.tokens.filter { it.surface == "猫" }
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
            effectiveDisplayTokens(
                    core().view.reader!!,
                    core().view.effectiveReaderTokens!!.identity,
                    core().view.effectiveReaderTokens!!,
                )!!
                .filter {
                    it.token.surface == "猫"
                }
        for (span in catRanges) {
            val link = text.getLinkAnnotations(span.start, span.end).single().item
            val dark =
                compose.activity.resources.configuration.uiMode and
                    android.content.res.Configuration.UI_MODE_NIGHT_MASK ==
                    android.content.res.Configuration.UI_MODE_NIGHT_YES
            assertEquals(androidx.compose.ui.text.TextLinkStyles(), link.styles)
            assertEquals(
                vocabularyBackground(status, dark),
                text.spanStyles
                    .first { it.start == span.start && it.end == span.end }
                    .item
                    .background,
            )
        }
    }

    @Test
    fun tappingAndFiveChoicesPersistWithoutLookupHistoryAndFailedChoiceCanRepeat() {
        val id = openDocument()
        val original = core().view.reader!!
        val catId =
            core()
                .view
                .effectiveReaderTokens!!
                .tokens
                .first { it.surface == "猫" }
                .vocabulary!!
                .id
                .value
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
                    !core().view.storageBusy && core().view.effectiveReaderStatus?.error != null
                }
                assertCats(VocabularyStatus.NEW)
                assertEquals(baseline + 5, count(db, eventsSql))
            } finally {
                db.execSQL("DROP TRIGGER IF EXISTS fail_word_ui")
            }
            compose.waitForIdle()
            assertEquals(baseline + 5, count(db, eventsSql))
            compose.onNodeWithTag("word_status_LEARNING").assertIsNotEnabled()
            compose.onNodeWithTag("effective_status_retry").performScrollTo().performClick()
            awaitWords()
            assertCats(VocabularyStatus.LEARNING)
            assertNull(core().view.effectiveReaderStatus?.error)
            assertEquals(baseline + 6, count(db, eventsSql))
            compose.onNodeWithText("Close").performScrollTo().performClick()
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
            core().view.effectiveReaderTokens!!.tokens.first { it.surface.contains("𠮷") }.surface
        tapSurface(supplementarySurface)
        compose.onNodeWithText("Close").performScrollTo().performClick()
        val cat =
            core().view.effectiveReaderTokens!!.tokens.first { it.surface == "猫" }.vocabulary!!
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
                    ReaderScreen(
                        session.view,
                        session.sessionKey,
                        session::dispatch,
                        session::onReaderInteraction,
                    )
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
                session.view.effectiveReaderTokens!!
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
    fun dictionaryShowsSeparateWordsNamesAndLifetimeCountsAndDismissesOnInvalidation() {
        val id = openDocument()
        val original = core().view.reader!!
        database().use { db ->
            val before = count(db, "SELECT count(*) FROM vocabulary_status_events")
            val completions = count(db, "SELECT count(*) FROM reading_completions")
            val tokyoId =
                core()
                    .view
                    .effectiveReaderTokens!!
                    .tokens
                    .first { it.surface == "東京" }
                    .vocabulary!!
                    .id
                    .value
            val encountered =
                count(db, "SELECT count(*) FROM reading_occurrences WHERE vocabulary_id = $tokyoId")
            tapSurface("東京")
            val details = core().view.effectiveReaderWord!!.details!!
            assertTrue(details.candidates.words.isNotEmpty())
            assertTrue(details.candidates.names.isNotEmpty())
            compose
                .onNodeWithTag("word_encountered")
                .assertTextEquals("Encountered $encountered times")
            compose.onNodeWithTag("word_definitions").assertExists()
            compose.onNodeWithTag("word_names").performScrollTo().assertIsDisplayed()
            for (candidate in details.candidates.words) {
                for (index in candidate.entry.senses.indices) {
                    val node = compose.onNodeWithTag("word_sense_${candidate.entryId}_$index")
                    if (applicableSenseReadings(candidate, index).isEmpty())
                        node.assertDoesNotExist()
                    else node.assertExists()
                }
            }
            assertEquals(original, core().view.reader)
            assertEquals(before, count(db, "SELECT count(*) FROM vocabulary_status_events"))
            assertEquals(completions, count(db, "SELECT count(*) FROM reading_completions"))
            // Refresh retires even a same-key selected result without replaying lookup.
            compose.runOnUiThread { core().dispatch(Event.LoadReaderTokens(key())) }
            awaitWords()
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
            assertNull(core().view.effectiveReaderWord?.selection)
            tapSurface("食べ")
            assertEquals("食べる", core().view.effectiveReaderWord!!.details!!.token.dictionaryForm)
            compose.onNodeWithTag("word_definitions").assertExists()
            assertTrue(
                compose
                    .onAllNodesWithText("Ichidan verb", substring = true)
                    .fetchSemanticsNodes()
                    .isNotEmpty()
            )
            compose.runOnUiThread {
                core()
                    .dispatch(
                        Event.CompleteEffectiveReaderPage(
                            core().view.effectiveReaderTokens!!.identity,
                            original.position,
                            System.currentTimeMillis(),
                        )
                    )
            }
            compose.waitUntil(30_000) {
                !core().view.storageBusy && core().view.contentRoute == ContentRoute.MAIN
            }
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
            compose.runOnUiThread { core().dispatch(Event.OpenReader(DocumentId(id))) }
            awaitWords()
            tapSurface("東京")
            compose
                .onNodeWithTag("word_encountered")
                .assertTextEquals("Encountered ${encountered + 1} times")
            compose.runOnUiThread { core().dispatch(Event.OpenSettings) }
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
            compose.waitUntil(30_000) { !core().view.storageBusy }
            compose.runOnUiThread { core().dispatch(Event.SetPageLength(PageLength.CHARACTERS250)) }
            compose.waitUntil(30_000) { !core().view.storageBusy }
            compose.runOnUiThread { core().dispatch(Event.CloseSettings) }
            awaitWords()
            assertNull(core().view.effectiveReaderWord?.selection)
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
            tapSurface("東京")
            compose.activityRule.scenario.recreate()
            awaitWords()
            assertNull(core().view.effectiveReaderWord?.selection)
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        }
    }

    @Test
    fun closingDuringLookupDiscardsLateDetailsAndDoesNotReplayTheRequest() {
        val publisher = sudachiDictionaryProvider(compose.activity)
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val provider =
            object : DictionaryProvider {
                override fun openVerifiedDictionary() = publisher.openVerifiedDictionary()

                override fun verifiedWordDictionaryPath(): String {
                    entered.countDown()
                    check(release.await(30, TimeUnit.SECONDS))
                    return publisher.verifiedWordDictionaryPath()
                }
            }
        lateinit var session: CoreSession
        compose.runOnUiThread {
            session = CoreSession(":memory:", provider)
            testCore = session
            session.setReaderActive(true)
            compose.activity.setContent {
                YomokuTheme {
                    ReaderScreen(
                        session.view,
                        session.sessionKey,
                        session::dispatch,
                        session::onReaderInteraction,
                    )
                }
            }
        }
        try {
            openDocument()
            activateSurface("東京")
            assertTrue(entered.await(30, TimeUnit.SECONDS))
            assertNull(session.view.effectiveReaderWord?.details)
            compose.onNodeWithText("Loading dictionary…").assertExists()
            compose.onNodeWithText("Close").performScrollTo().performClick()
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
            assertNull(session.view.effectiveReaderWord?.selection)
            release.countDown()
            awaitWords()
            assertNull(session.view.effectiveReaderWord?.details)
            assertNull(session.view.effectiveReaderWord?.error)
            compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
            tapSurface("東京")
            compose.onNodeWithTag("word_definitions").assertExists()
            compose.onNodeWithTag("word_names").assertExists()
        } finally {
            release.countDown()
            compose.runOnUiThread {
                compose.activity.setContent {}
                session.close()
                testCore = null
            }
            compose.waitUntil(30_000) { session.isClosed }
        }
    }

    @Test
    fun movingBothDirectionsDismissesDetailsAndNeverRestoresTheSelection() {
        openDocument("\n${"猫 ".repeat(1600)}。\n${"犬 ".repeat(1600)}。")
        val first = core().view.reader!!
        assertTrue(first.totalPages > 1UL)
        tapSurface("東京")
        compose.runOnUiThread {
            core()
                .dispatch(
                    Event.CompleteEffectiveReaderPage(
                        core().view.effectiveReaderTokens!!.identity,
                        first.position,
                        System.currentTimeMillis(),
                    )
                )
        }
        awaitWords()
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        assertNull(core().view.effectiveReaderWord?.selection)
        val second = core().view.reader!!
        assertEquals(2UL, second.pageNumber)
        tapSurface(core().view.effectiveReaderTokens!!.tokens.first().surface)
        compose.runOnUiThread {
            core()
                .dispatch(
                    Event.PreviousReaderPage(second.documentId, second.anchor, second.position)
                )
        }
        awaitWords()
        assertEquals(first.anchor, core().view.reader!!.anchor)
        assertNull(core().view.effectiveReaderWord?.selection)
        assertNull(core().view.effectiveReaderWord?.details)
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
    }

    @Test
    fun dictionaryFailureNeedsExplicitRetryAndEmptyMatchesAreSuccessful() {
        val publisher = sudachiDictionaryProvider(compose.activity)
        val fail = AtomicBoolean(true)
        val calls = AtomicInteger()
        val provider =
            object : DictionaryProvider {
                override fun openVerifiedDictionary() = publisher.openVerifiedDictionary()

                override fun verifiedWordDictionaryPath(): String {
                    calls.incrementAndGet()
                    return if (fail.get()) "" else publisher.verifiedWordDictionaryPath()
                }
            }
        lateinit var session: CoreSession
        compose.runOnUiThread {
            session = CoreSession(":memory:", provider)
            testCore = session
            session.setReaderActive(true)
            compose.activity.setContent {
                YomokuTheme {
                    ReaderScreen(
                        session.view,
                        session.sessionKey,
                        session::dispatch,
                        session::onReaderInteraction,
                    )
                }
            }
        }
        try {
            openDocument()
            tapSurface("猫")
            compose.onNodeWithTag("word_lookup_error").assertExists()
            compose.onNodeWithTag("word_no_match").assertDoesNotExist()
            assertEquals(1, calls.get())
            fail.set(false)
            compose.runOnUiThread { session.setReaderActive(false) }
            awaitWords()
            compose.onNodeWithTag("word_lookup_error").assertExists()
            assertEquals(1, calls.get())
            compose.onNodeWithTag("retry_word_details").performScrollTo().performClick()
            compose.waitUntil(30_000) {
                !session.view.storageBusy && session.view.effectiveReaderWord?.details != null
            }
            compose.onNodeWithTag("word_definitions").assertExists()
            assertEquals(2, calls.get())
            // Use a successful empty candidate payload to check the distinct no-match view.
            val base = session.view
            val empty =
                base.effectiveReaderWord!!
                    .details!!
                    .copy(
                        candidates =
                            base.effectiveReaderWord.details.candidates.copy(
                                words = emptyList(),
                                names = emptyList(),
                            )
                    )
            var frame by mutableStateOf(base)
            val events = mutableListOf<Event>()
            compose.runOnUiThread {
                compose.activity.setContent {
                    YomokuTheme {
                        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                            EffectiveReaderWords(frame, core().sessionKey, { events.add(it) }, {})
                        }
                    }
                }
            }
            val text =
                compose
                    .onNodeWithTag("reader_text")
                    .fetchSemanticsNode()
                    .config[SemanticsProperties.Text]
                    .single()
            val span =
                effectiveDisplayTokens(
                        base.reader!!,
                        base.effectiveReaderTokens!!.identity,
                        base.effectiveReaderTokens,
                    )!!
                    .first {
                        it.token.surface == "猫"
                    }
            val link = text.getLinkAnnotations(span.start, span.end).single().item
            compose.runOnUiThread { link.linkInteractionListener!!.onClick(link) }
            compose.runOnUiThread {
                frame =
                    frame.copy(
                        effectiveReaderWord = frame.effectiveReaderWord!!.copy(details = empty)
                    )
            }
            compose.onNodeWithTag("word_no_match").performScrollTo().assertIsDisplayed()
            compose.onNodeWithTag("word_encountered").assertTextEquals("Encountered 0 times")
            compose.onNodeWithTag("word_lookup_error").assertDoesNotExist()
            compose.onNodeWithTag("retry_word_details").assertDoesNotExist()
            assertEquals(1, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
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
        var frame by mutableStateOf(base.copy(effectiveReaderTokens = null, storageBusy = true))
        val events = mutableListOf<Event>()
        compose.runOnUiThread {
            compose.activity.setContent {
                YomokuTheme {
                    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                        EffectiveReaderWords(
                            frame,
                            core().sessionKey,
                            {
                                events.add(it)
                                if (it is Event.LoadEffectiveReaderWord) {
                                    frame =
                                        frame.copy(
                                            effectiveReaderWord =
                                                frame.effectiveReaderWord!!.copy(
                                                    selection = it.value
                                                )
                                        )
                                }
                            },
                            {},
                        )
                    }
                }
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
        compose.onNodeWithTag("retry_word_lookup").performScrollTo().performClick()
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
            effectiveDisplayTokens(
                    base.reader!!,
                    base.effectiveReaderTokens!!.identity,
                    base.effectiveReaderTokens,
                )!!
                .first {
                    it.token.surface == "猫"
                }
        val link = text.getLinkAnnotations(cat.start, cat.end).single().item
        compose.runOnUiThread {
            frame = frame.copy(readingTimeSavePending = true)
            link.linkInteractionListener!!.onClick(link)
        }
        compose.onNodeWithTag("reader_word_panel").assertIsDisplayed()
        assertTrue(events.filterIsInstance<Event.LoadEffectiveReaderWord>().isEmpty())
        compose.runOnUiThread { frame = frame.copy(readingTimeSavePending = false) }
        compose.waitForIdle()
        assertEquals(1, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
        compose.runOnUiThread { frame = frame.copy(readingTimeSavePending = true) }
        for (status in VocabularyStatus.entries) compose
            .onNodeWithTag("word_status_$status")
            .assertIsNotEnabled()
        compose.runOnUiThread { frame = frame.copy(effectiveReaderTokens = null) }
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        compose.runOnUiThread { frame = base }
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        assertEquals(1, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
        assertEquals(2, events.filterIsInstance<Event.LoadReaderTokens>().size)
    }
}
