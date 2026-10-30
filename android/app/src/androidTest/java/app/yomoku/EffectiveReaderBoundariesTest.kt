package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.shared.TextRange
import app.yomoku.theme.YomokuTheme
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class EffectiveReaderBoundariesTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val events = mutableListOf<Event>()
    private val owner = mutableStateOf("retained-session")
    private val frame = mutableStateOf(fixture())

    private fun anchor(start: Long, end: Long) =
        TextAnchor(
            SourceId(Long.MAX_VALUE),
            ChapterId(Long.MAX_VALUE),
            TextRange(TextOffset(start), TextOffset(end)),
        )

    private fun fixture(): ViewModel {
        val page =
            ReaderPage(
                DocumentId(Long.MAX_VALUE),
                "Boundaries",
                false,
                "か\u3099\u0000猫犬。鳥",
                anchor(4, 23),
                ReadingPosition(SourceId(Long.MAX_VALUE), ChapterId(Long.MAX_VALUE), TextOffset(4)),
                1UL,
                1UL,
                emptyList(),
                emptyList(),
                emptyList(),
            )
        val analysis = ReaderAnalysis.Available(AnalysisId(Long.MAX_VALUE))
        val identity =
            EffectivePageIdentity(
                ULong.MAX_VALUE,
                ReaderTokenKey(page.documentId, page.anchor, analysis),
            )
        return ViewModel(
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
            readerTime = ReadingTimeUpdate(page.documentId, page.position, PendingReadingTime(0)),
            readingTimeSavePending = false,
            completionSavePending = false,
            effectiveReaderTokens =
                EffectiveReaderPage(
                    identity,
                    emptyList(),
                    listOf(
                        OriginalBoundaryToken(
                            anchor(0, 11),
                            TextRange(TextOffset(0), TextOffset(7)),
                            "𠮷か\u3099\u0000",
                        ),
                        OriginalBoundaryToken(
                            anchor(11, 14),
                            TextRange(TextOffset(7), TextOffset(10)),
                            "猫",
                        ),
                        OriginalBoundaryToken(
                            anchor(14, 17),
                            TextRange(TextOffset(10), TextOffset(13)),
                            "犬",
                        ),
                        OriginalBoundaryToken(
                            anchor(20, 23),
                            TextRange(TextOffset(16), TextOffset(19)),
                            "鳥",
                        ),
                    ),
                ),
            effectiveReaderWord =
                EffectiveReaderWordState(busy = false, customEntryPending = false),
            effectiveReaderStatus = EffectiveStatusState(pending = false, busy = false),
            effectiveReaderOverride =
                EffectiveReaderOverrideState(pending = false, busy = false, reloadRequired = false),
            readerFurigana = emptyList(),
        )
    }

    private fun show(restoration: StateRestorationTester? = null) {
        val content: @androidx.compose.runtime.Composable () -> Unit = {
            YomokuTheme { YomokuApp(frame.value, owner.value, events::add) }
        }
        if (restoration != null) restoration.setContent(content) else compose.setContent(content)
        compose.waitForIdle()
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()

    private fun actions() =
        events.filterIsInstance<Event.EffectiveReaderOverride>().map { it.value }

    private fun open(): EffectiveOverrideAction.Open =
        actions().filterIsInstance<EffectiveOverrideAction.Open>().last()

    private fun prompt(): EffectiveOverridePrompt {
        val opened = open()
        return EffectiveOverridePrompt(
            EffectiveOverrideIdentity(opened.page, ULong.MAX_VALUE),
            opened.anchor,
            "𠮷か\u3099\u0000猫犬",
            listOf(EffectiveBoundaryChoice(SplitMode.B, listOf("𠮷か\u3099\u0000", "猫", "犬"))),
            listOf(
                EffectiveWinningRule(
                    OverrideId(Long.MAX_VALUE),
                    OverrideScope.ALLOCCURRENCES,
                    "猫犬",
                    listOf("猫", "犬"),
                )
            ),
            OverrideScope.ALLOCCURRENCES,
        )
    }

    private fun loadPrompt(): EffectiveOverridePrompt {
        click("boundary_select")
        click("boundary_token_2")
        click("boundary_token_0")
        click("boundary_open")
        val prompt = prompt()
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderOverride =
                        EffectiveReaderOverrideState(
                            prompt.identity,
                            prompt,
                            false,
                            false,
                            null,
                            false,
                        )
                )
        }
        return prompt
    }

    @Test
    fun originalCrossingSpanOpensEditorWithExactIdentityAndRoutesModesScopeSaveAndReverse() {
        show()
        val prompt = loadPrompt()
        assertEquals(anchor(0, 17), open().anchor)
        assertEquals(frame.value.effectiveReaderTokens!!.identity, open().page)
        compose.onNodeWithTag("boundary_surface").assertTextEquals("𠮷か\u3099\u0000猫犬")
        compose.onNodeWithTag("boundary_scope_ALLOCCURRENCES").assertIsSelected()
        click("boundary_mode_A")
        assertEquals(
            EffectiveOverrideAction.Generate(prompt.identity, SplitMode.A),
            actions().last(),
        )
        click("boundary_scope_THISOCCURRENCE")
        click("boundary_mode_B")
        val choice = actions().last() as EffectiveOverrideAction.Choose
        assertEquals(prompt.identity, choice.identity)
        assertEquals(OverrideScope.THISOCCURRENCE, choice.scope)
        click("boundary_merge_all")
        click("boundary_save")
        val save = actions().last() as EffectiveOverrideAction.Save
        assertEquals(prompt.identity, save.identity)
        assertEquals(listOf(prompt.surface), save.parts)
        assertEquals(OverrideScope.THISOCCURRENCE, save.scope)
        click("boundary_reverse_${Long.MAX_VALUE}")
        assertEquals(
            EffectiveOverrideAction.Reverse(prompt.identity, OverrideId(Long.MAX_VALUE)),
            actions().last(),
        )
        click("boundary_close")
        assertEquals(EffectiveOverrideAction.Dismiss(prompt.identity), actions().last())
    }

    @Test
    fun gapsAndMalformedOrMissingOriginalDataDisableSelectionWithoutEffectiveFallback() {
        show()
        click("boundary_select")
        click("boundary_token_0")
        click("boundary_token_3")
        compose.onNodeWithTag("boundary_selection_gap").assertExists()
        compose.onNodeWithTag("boundary_open").assertIsNotEnabled()
        click("boundary_selection_close")
        assertTrue(actions().isEmpty())
        val ready = frame.value
        val snapshot = ready.effectiveReaderTokens!!
        for (originals in
            listOf(
                emptyList(),
                snapshot.originalBoundaries.reversed(),
                listOf(snapshot.originalBoundaries.first().copy(surface = "違う")),
            )) {
            compose.runOnUiThread {
                frame.value =
                    ready.copy(
                        effectiveReaderTokens = snapshot.copy(originalBoundaries = originals)
                    )
            }
            compose.onNodeWithTag("boundary_select").performScrollTo().assertIsNotEnabled()
        }
        compose.runOnUiThread {
            frame.value =
                ready.copy(
                    effectiveReaderTokens =
                        snapshot.copy(
                            tokens =
                                listOf(
                                    EffectivePageToken(
                                        0UL,
                                        anchor(11, 14),
                                        TextRange(TextOffset(7), TextOffset(10)),
                                        "犬",
                                        EffectiveTokenEligibility.MANUALJAPANESESPAN,
                                        null,
                                        null,
                                        null,
                                    )
                                )
                        )
                )
        }
        compose.onNodeWithTag("boundary_select").assertIsNotEnabled()
        compose.runOnUiThread { frame.value = ready.copy(readerAnalysis = ReaderAnalysis.Missing) }
        compose.onNodeWithTag("boundary_select").assertIsNotEnabled()
    }

    @Test
    fun acceptedFailureRemainsEditableForExactRetryCancelAndExplicitReloadAfterRetirement() {
        show()
        val prompt = loadPrompt()
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderTokens = null,
                    effectiveReaderOverride =
                        EffectiveReaderOverrideState(
                            prompt.identity,
                            null,
                            true,
                            false,
                            EffectiveOverrideFailure.Storage(DocumentError.UNAVAILABLE),
                            false,
                        ),
                )
        }
        click("boundary_retry")
        assertEquals(EffectiveOverrideAction.Retry(prompt.identity), actions().last())
        click("boundary_cancel")
        assertEquals(EffectiveOverrideAction.Cancel(prompt.identity), actions().last())
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderOverride =
                        EffectiveReaderOverrideState(
                            prompt.identity,
                            null,
                            false,
                            false,
                            EffectiveOverrideFailure.StaleContext,
                            true,
                        )
                )
        }
        val loads = events.filterIsInstance<Event.LoadReaderTokens>().size
        click("boundary_reload")
        assertEquals(EffectiveOverrideAction.Dismiss(prompt.identity), actions().last())
        assertEquals(loads + 1, events.filterIsInstance<Event.LoadReaderTokens>().size)
        assertEquals(
            prompt.identity.page.base,
            events.filterIsInstance<Event.LoadReaderTokens>().last().value,
        )
    }

    @Test
    fun selectionRestoresOnlyForSamePageAndNativeSession() {
        val restoration = StateRestorationTester(compose)
        show(restoration)
        click("boundary_select")
        click("boundary_token_2")
        click("boundary_token_0")
        restoration.emulateSavedInstanceStateRestore()
        compose.onNodeWithTag("boundary_token_0").assertIsSelected()
        compose.onNodeWithTag("boundary_token_2").assertIsSelected()
        compose.runOnUiThread { owner.value = "replacement" }
        compose.onNodeWithTag("boundary_selection").assertDoesNotExist()
        click("boundary_select")
        click("boundary_token_0")
        compose.runOnUiThread {
            val snapshot = frame.value.effectiveReaderTokens!!
            frame.value =
                frame.value.copy(
                    effectiveReaderTokens =
                        snapshot.copy(identity = snapshot.identity.copy(revision = 1UL))
                )
        }
        compose.onNodeWithTag("boundary_selection").assertDoesNotExist()
        assertTrue(actions().isEmpty())
    }

    @Test
    fun fifoTimeAndMutationGatesDisableSpanSubmissionAndEditorWrites() {
        show()
        click("boundary_select")
        click("boundary_token_0")
        val ready = frame.value
        for (blocked in
            listOf(
                ready.copy(storageBusy = true),
                ready.copy(readingTimeSavePending = true),
                ready.copy(completionSavePending = true),
                ready.copy(
                    effectiveReaderWord =
                        ready.effectiveReaderWord!!.copy(customEntryPending = true)
                ),
                ready.copy(
                    effectiveReaderStatus = EffectiveStatusState(pending = true, busy = false)
                ),
                ready.copy(
                    effectiveReaderOverride = ready.effectiveReaderOverride!!.copy(busy = true)
                ),
            )) {
            compose.runOnUiThread { frame.value = blocked }
            compose.onNodeWithTag("boundary_open").assertIsNotEnabled()
        }
        assertTrue(actions().isEmpty())
        compose.runOnUiThread { frame.value = ready }
        click("boundary_selection_close")
        loadPrompt()
        compose.runOnUiThread { frame.value = frame.value.copy(readingTimeSavePending = true) }
        compose.onNodeWithTag("boundary_save").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithTag("boundary_mode_B").performScrollTo().assertIsNotEnabled()
    }
}
