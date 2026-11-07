package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.shared.TextRange
import app.yomoku.theme.YomokuTheme
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Exercises the actual shell with compact native projections, without activating production. */
@RunWith(AndroidJUnit4::class)
class EffectiveReaderNavigationTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val events = mutableListOf<Event>()
    private val owner = mutableStateOf("session-one")
    private val frame = mutableStateOf(fixture())

    private fun fixture(): ViewModel {
        val anchor =
            TextAnchor(
                SourceId(1),
                ChapterId(1),
                TextRange(TextOffset(6), TextOffset(9)),
            )
        val page =
            ReaderPage(
                DocumentId(1),
                "Compact navigation",
                false,
                "猫",
                anchor,
                ReadingPosition(anchor.sourceId, anchor.chapterId, anchor.range.start),
                2UL,
                3UL,
                emptyList(),
                emptyList(),
                emptyList(),
            )
        val analysis = ReaderAnalysis.Available(AnalysisId(1))
        val identity =
            EffectivePageIdentity(
                ULong.MAX_VALUE,
                ReaderTokenKey(page.documentId, anchor, analysis),
            )
        val token =
            EffectivePageToken(
                ULong.MAX_VALUE,
                anchor,
                TextRange(TextOffset(0), TextOffset(3)),
                "猫",
                EffectiveTokenEligibility.ANALYZERJAPANESE,
                null,
                null,
                null,
            )
        return ViewModel(
            ankiAddedNotice = null,
            ankiSettings = AnkiSettingsView(AnkiSettingsPhase.UNLOADED, null, null),
            ankiSetup = app.yomoku.shared.AnkiSetupPhase.Unchecked,
            ankiEntries = emptyList(),
            vocabulary =
                VocabularyState(
                    VocabularyQuery("", VocabularyStatus.entries.toList(), VocabularySort.NEWEST),
                    rows = emptyList(),
                    selectingAll = false,
                    loading = false,
                ),
            statistics = StatisticsState(StatisticsTimeframe.LASTYEAR, loading = false),
            destination = Destination.LIBRARY,
            settingsOpen = false,
            documents = emptyList(),
            libraryTitleQuery = "",
            librarySort = LibrarySort.LASTREAD,
            libraryDocumentCount = 0UL,
            storageBusy = false,
            contentRoute = ContentRoute.READER,
            reader = page,
            readerAnalysis = analysis,
            completionSavePending = false,
            effectiveReaderTokens = EffectiveReaderPage(identity, listOf(token)),
            effectiveReaderWord = EffectiveReaderWordState(busy = false),
            effectiveReaderStatus = EffectiveStatusState(pending = false, busy = false),
            readerFurigana = emptyList(),
        )
    }

    private fun show() {
        compose.setContent {
            YomokuTheme { YomokuApp(frame.value, owner.value, events::add) }
        }
        compose.waitForIdle()
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()

    private fun complete(): Event.CompleteEffectiveReaderPage =
        events.filterIsInstance<Event.CompleteEffectiveReaderPage>().last()

    @Test
    fun appUsesButtonsForCompactCompletionAndIgnoresHorizontalSwipes() {
        show()
        compose.onNodeWithTag("bottom_navigation").assertDoesNotExist()
        compose.onNodeWithTag("navigation_rail").assertDoesNotExist()
        val text =
            compose
                .onNodeWithTag("reader_text")
                .fetchSemanticsNode()
                .config[SemanticsProperties.Text]
                .single()
        val link = text.getLinkAnnotations(0, text.length).single().item
        compose.runOnUiThread { link.linkInteractionListener!!.onClick(link) }
        compose.waitForIdle()
        assertEquals(
            Event.LoadEffectiveReaderWord(
                EffectiveReaderSelection(
                    frame.value.effectiveReaderTokens!!.identity,
                    ULong.MAX_VALUE,
                )
            ),
            events.single(),
        )
        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
            .sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_BACK)
        click("next_page")
        val page = frame.value.reader!!
        assertEquals(frame.value.effectiveReaderTokens!!.identity, complete().identity)
        assertEquals(page.position, complete().position)
        assertTrue(complete().completedAtUnixMs > 0)
        click("previous_page")
        assertEquals(
            Event.PreviousReaderPage(page.documentId, page.anchor, page.position),
            events.last(),
        )
        val beforeSwipes = events.toList()
        compose.onNodeWithTag("reader_scroll").performTouchInput { swipeLeft() }
        compose.onNodeWithTag("reader_scroll").performTouchInput { swipeRight() }
        assertEquals(beforeSwipes, events)
        compose.runOnUiThread {
            frame.value = frame.value.copy(reader = page.copy(pageNumber = 3UL))
        }
        compose.onNodeWithText("Finish Book").performScrollTo().performClick()
        assertEquals(2, events.filterIsInstance<Event.CompleteEffectiveReaderPage>().size)
    }

    @Test
    fun missingRetiredForeignAndMalformedPagesNeverCompleteUsingOriginalB() {
        show()
        val ready = frame.value
        val snapshot = ready.effectiveReaderTokens!!
        val invalid =
            listOf(
                ready.copy(effectiveReaderTokens = null, documentError = DocumentError.UNAVAILABLE),
                ready.copy(effectiveReaderTokens = null),
                ready.copy(
                    readerAnalysis = ReaderAnalysis.Missing,
                    effectiveReaderTokens =
                        snapshot.copy(
                            identity =
                                snapshot.identity.copy(
                                    base =
                                        snapshot.identity.base.copy(
                                            analysis = ReaderAnalysis.Missing
                                        )
                                ),
                            tokens = emptyList(),
                        ),
                ),
                ready.copy(
                    effectiveReaderTokens =
                        snapshot.copy(
                            identity =
                                snapshot.identity.copy(
                                    base =
                                        snapshot.identity.base.copy(
                                            analysis = ReaderAnalysis.Available(AnalysisId(2))
                                        )
                                )
                        )
                ),
                ready.copy(
                    effectiveReaderTokens =
                        snapshot.copy(
                            tokens =
                                snapshot.tokens.map {
                                    it.copy(displayRange = TextRange(TextOffset(1), TextOffset(2)))
                                }
                        )
                ),
            )
        for (view in invalid) {
            compose.runOnUiThread { frame.value = view }
            compose.onNodeWithTag("next_page").performScrollTo().assertIsNotEnabled()
            compose.onNodeWithTag("previous_page").assertIsEnabled()
            compose.onNodeWithTag("reader_scroll").performTouchInput { swipeLeft() }
            click("previous_page")
            assertTrue(events.last() is Event.PreviousReaderPage)
        }
        assertTrue(
            events.none {
                it is Event.CompleteEffectiveReaderPage
            }
        )
        compose.runOnUiThread {
            // An empty valid partition (e.g. punctuation) can complete without word metadata.
            frame.value = ready.copy(effectiveReaderTokens = snapshot.copy(tokens = emptyList()))
        }
        click("next_page")
        assertEquals(snapshot.identity, complete().identity)
    }

    @Test
    fun retainedButtonSemanticsUseCurrentRevisionAndGates() {
        show()
        fun nextAction() =
            compose
                .onNodeWithTag("next_page")
                .fetchSemanticsNode()
                .config[SemanticsActions.OnClick]
                .action!!
        val old = nextAction()
        val ready = frame.value
        compose.runOnUiThread { frame.value = ready.copy(completionSavePending = true) }
        compose.waitForIdle()
        compose.runOnUiThread { old() }
        assertTrue(events.isEmpty())
        compose.runOnUiThread {
            frame.value =
                ready.copy(
                    effectiveReaderTokens =
                        ready.effectiveReaderTokens!!.copy(
                            identity = ready.effectiveReaderTokens.identity.copy(revision = 1UL)
                        )
                )
        }
        compose.waitForIdle()
        compose.runOnUiThread { old() }
        // Compose's retained semantics wrapper invokes the button's current callback.
        assertEquals(1UL, complete().identity.revision)
        compose.runOnUiThread {
            val snapshot = frame.value.effectiveReaderTokens!!
            frame.value =
                frame.value.copy(
                    effectiveReaderTokens =
                        snapshot.copy(identity = snapshot.identity.copy(revision = 2UL))
                )
        }
        // The button must use the new identity even when page geometry has not changed.
        click("next_page")
        assertEquals(2UL, complete().identity.revision)
        val text =
            compose
                .onNodeWithTag("reader_text")
                .fetchSemanticsNode()
                .config[SemanticsProperties.Text]
                .single()
        val link = text.getLinkAnnotations(0, text.length).single().item
        compose.runOnUiThread { link.linkInteractionListener!!.onClick(link) }
        compose.onNodeWithTag("reader_word_panel").assertExists()
        compose.runOnUiThread { owner.value = "replacement-session" }
        compose.waitForIdle()
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        click("next_page")
        assertEquals(3, events.filterIsInstance<Event.CompleteEffectiveReaderPage>().size)
    }

    @Test
    fun longTitleUsesOneLineAccessibleBackAndCounterStaysCenteredForFinish() {
        frame.value =
            frame.value.copy(reader = frame.value.reader!!.copy(title = "長い本の題名".repeat(30)))
        show()
        val layouts = mutableListOf<androidx.compose.ui.text.TextLayoutResult>()
        compose.onNodeWithTag("screen_title").performSemanticsAction(
            SemanticsActions.GetTextLayoutResult
        ) {
            it(layouts)
        }
        assertEquals(1, layouts.single().lineCount)
        assertTrue(layouts.single().isLineEllipsized(0))
        fun checkCounter() {
            compose.onNodeWithTag("page_counter").performScrollTo()
            val reader = compose.onNodeWithTag("reader_scroll").fetchSemanticsNode().boundsInRoot
            val counter = compose.onNodeWithTag("page_counter").fetchSemanticsNode().boundsInRoot
            assertEquals(reader.center.x, counter.center.x, 1f)
            val previous = compose.onNodeWithTag("previous_page").getUnclippedBoundsInRoot()
            val next = compose.onNodeWithTag("next_page").getUnclippedBoundsInRoot()
            assertEquals(
                (previous.right - previous.left).value,
                (next.right - next.left).value,
                0.5f,
            )
        }
        checkCounter()
        compose.runOnUiThread {
            frame.value = frame.value.copy(reader = frame.value.reader!!.copy(pageNumber = 3UL))
        }
        compose.onNodeWithText("Finish Book").assertExists()
        checkCounter()
        compose.onNodeWithContentDescription("Back").performClick()
        assertEquals(Event.CloseDocument, events.last())
    }
}
