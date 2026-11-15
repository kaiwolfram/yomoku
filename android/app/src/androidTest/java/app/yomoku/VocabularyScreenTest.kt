package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class VocabularyScreenTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val events = mutableListOf<Event>()
    private val frame = mutableStateOf(vocabularyView())
    private val sessionKey = mutableStateOf("vocabulary-test")

    private fun render() {
        compose.setContent {
            YomokuTheme {
                YomokuApp(
                    frame.value,
                    sessionKey.value,
                    dispatch = { event ->
                        events.add(event)
                        if (event is Event.SetVocabularyQuery)
                            frame.value =
                                frame.value.copy(
                                    vocabulary = frame.value.vocabulary.copy(query = event.value)
                                )
                    },
                )
            }
        }
    }

    @Test
    fun sortRetainsRowsAndCountUntilAtomicReplacementIncludingRapidReturnToOriginalSort() {
        frame.value =
            frame.value.copy(
                vocabulary =
                    frame.value.vocabulary.copy(
                        matchingCount = 2UL,
                        rows = listOf(vocabularyRow(1), vocabularyRow(2)),
                    )
            )
        render()
        fun loading(sort: VocabularySort) {
            compose.runOnUiThread {
                frame.value =
                    frame.value.copy(
                        storageBusy = true,
                        vocabulary =
                            frame.value.vocabulary.copy(
                                query = frame.value.vocabulary.query.copy(sort = sort),
                                loading = true,
                                matchingCount = null,
                                rows = emptyList(),
                                nextOffset = null,
                            ),
                    )
            }
        }
        loading(VocabularySort.FREQUENCY)
        compose.onNodeWithTag("vocabulary_row_1").assertIsDisplayed().assertIsNotEnabled()
        compose.onNodeWithTag("vocabulary_count").assertTextEquals("Matching words: 2")
        compose.onNodeWithText("Loading vocabulary…").assertDoesNotExist()
        compose.mainClock.advanceTimeBy(200)
        compose.onNodeWithTag("app_loading").assertDoesNotExist()
        loading(VocabularySort.NEWEST)
        compose.onNodeWithTag("vocabulary_row_2").assertExists().assertIsNotEnabled()
        compose.mainClock.advanceTimeBy(150)
        compose.onNodeWithTag("app_loading").assertExists()
        assertFalse(events.contains(Event.LoadMoreVocabulary))
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    storageBusy = false,
                    vocabulary =
                        frame.value.vocabulary.copy(
                            loading = false,
                            matchingCount = 1UL,
                            rows = listOf(vocabularyRow(3)),
                        ),
                )
        }
        compose.onNodeWithTag("vocabulary_row_1").assertDoesNotExist()
        compose.onNodeWithTag("vocabulary_row_3").assertIsDisplayed().assertIsEnabled()
        compose.onNodeWithTag("vocabulary_count").assertTextEquals("Matching words: 1")
        compose.onNodeWithTag("app_loading").assertDoesNotExist()
        // A quick subsequent sort also retains the latest successful result.
        loading(VocabularySort.ALPHABETICAL)
        compose.onNodeWithTag("vocabulary_row_3").assertExists().assertIsNotEnabled()
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    storageBusy = false,
                    vocabulary =
                        frame.value.vocabulary.copy(
                            loading = false,
                            matchingCount = 0UL,
                            rows = emptyList(),
                        ),
                )
        }
        compose.onNodeWithTag("vocabulary_row_3").assertDoesNotExist()
        compose.onNodeWithTag("vocabulary_count").assertTextEquals("Matching words: 0")
        compose.onNodeWithTag("app_loading").assertDoesNotExist()
    }

    @Test
    fun statusPatchRemovalAndReentryPreserveVisibleWordAndPixelOffsetBeyondFirstPage() {
        val scroll = LazyListState()
        val rows = (1..300).map { vocabularyRow(it.toLong()) }
        frame.value =
            frame.value.copy(
                vocabulary =
                    frame.value.vocabulary.copy(
                        matchingCount = 305UL,
                        rows = rows,
                        nextOffset = 300UL,
                    )
            )
        compose.setContent {
            YomokuTheme { VocabularyScreen(frame.value, "anchor", scroll, events::add) }
        }
        compose.runOnUiThread { scroll.requestScrollToItem(151, 13) }
        compose.waitForIdle()
        fun assertAnchor(id: Long) {
            compose.runOnIdle {
                assertEquals(
                    id,
                    scroll.layoutInfo.visibleItemsInfo
                        .first { it.index == scroll.firstVisibleItemIndex }
                        .key,
                )
                assertEquals(13, scroll.firstVisibleItemScrollOffset)
            }
        }
        assertAnchor(151)
        fun update(
            next: List<VocabularyRow>,
            loading: Boolean = false,
            error: DocumentError? = null,
        ) {
            compose.runOnUiThread {
                frame.value =
                    frame.value.copy(
                        storageBusy = loading,
                        vocabulary =
                            frame.value.vocabulary.copy(
                                rows = next,
                                matchingCount = 305UL - (rows.size - next.size).toULong(),
                                nextOffset = next.size.toULong(),
                                loading = loading,
                                error = error,
                            ),
                    )
            }
            compose.waitForIdle()
        }
        val patched = rows.map {
            if (it.id.value == 151L) it.copy(status = VocabularyStatus.LEARNING) else it
        }
        update(patched)
        assertAnchor(151)
        // A refresh and a failed page retain the presentation and its position.
        update(patched, loading = true)
        assertAnchor(151)
        update(patched, error = DocumentError.UNAVAILABLE)
        assertAnchor(151)
        val beforeAnchorRemoved = patched.filter { it.id.value != 100L }
        update(beforeAnchorRemoved)
        assertAnchor(151)
        val anchorRemoved = beforeAnchorRemoved.filter { it.id.value != 151L }
        update(anchorRemoved)
        assertAnchor(152)
        update(patched)
        assertAnchor(152)
        assertFalse(events.contains(Event.LoadMoreVocabulary))
    }

    @Test
    fun errorsSearchFiltersAndSessionChangesClearRetainedSortPresentation() {
        render()
        for (reset in listOf("error", "search", "filter", "session")) {
            compose.runOnUiThread {
                frame.value =
                    vocabularyView()
                        .copy(
                            vocabulary =
                                vocabularyView()
                                    .vocabulary
                                    .copy(
                                        matchingCount = 1UL,
                                        rows = listOf(vocabularyRow(1)),
                                    )
                        )
            }
            compose.onNodeWithTag("vocabulary_row_1").assertExists()
            compose.runOnUiThread {
                frame.value =
                    frame.value.copy(
                        storageBusy = true,
                        vocabulary =
                            frame.value.vocabulary.copy(
                                query =
                                    frame.value.vocabulary.query.copy(
                                        sort = VocabularySort.FREQUENCY
                                    ),
                                loading = true,
                                matchingCount = null,
                                rows = emptyList(),
                            ),
                    )
            }
            compose.onNodeWithTag("vocabulary_row_1").assertExists()
            compose.runOnUiThread {
                val state = frame.value.vocabulary
                frame.value =
                    frame.value.copy(
                        vocabulary =
                            when (reset) {
                                "error" ->
                                    state.copy(loading = false, error = DocumentError.UNAVAILABLE)
                                "search" -> state.copy(query = state.query.copy(prefix = "猫"))
                                "filter" ->
                                    state.copy(
                                        query =
                                            state.query.copy(
                                                statuses = listOf(VocabularyStatus.KNOWN)
                                            )
                                    )
                                else -> state
                            }
                    )
                if (reset == "session") sessionKey.value = "new-session"
            }
            compose.onNodeWithTag("vocabulary_row_1").assertDoesNotExist()
            compose.onNodeWithTag("vocabulary_count").assertDoesNotExist()
            if (reset == "error") {
                compose.onNodeWithTag("vocabulary_error").assertExists()
                compose.runOnUiThread {
                    frame.value =
                        frame.value.copy(
                            vocabulary = frame.value.vocabulary.copy(loading = true, error = null)
                        )
                }
                compose.onNodeWithTag("vocabulary_row_1").assertDoesNotExist()
            }
        }
    }

    @Test
    fun statusFiltersShareOneRowAndEveryChipIsReachable() {
        render()
        val top = compose.onNodeWithTag("vocabulary_status_NEW").getUnclippedBoundsInRoot().top
        VocabularyStatus.entries.forEach { status ->
            val chip = compose.onNodeWithTag("vocabulary_status_${status.name}")
            assertEquals(top.value, chip.getUnclippedBoundsInRoot().top.value, 0.5f)
            chip.performScrollTo().assertIsDisplayed().assertIsEnabled().performClick()
            assertEquals(
                VocabularyStatus.entries.filter { it.ordinal > status.ordinal },
                frame.value.vocabulary.query.statuses,
            )
        }
    }

    @Test
    fun literalSearchAllStatusSubsetsSortsAndExactUnsignedLabels() {
        frame.value =
            frame.value.copy(
                vocabulary =
                    frame.value.vocabulary.copy(
                        matchingCount = ULong.MAX_VALUE,
                        rows = listOf(vocabularyRow(1).copy(encounteredCount = ULong.MAX_VALUE)),
                    )
            )
        render()
        compose
            .onNodeWithTag("vocabulary_count")
            .assertTextEquals("Matching words: ${ULong.MAX_VALUE}")
        compose.onNodeWithText("Encountered ${ULong.MAX_VALUE} times").assertIsDisplayed()
        val prefix = " 食べ%_𠮷"
        compose.onNodeWithTag("vocabulary_search").performTextReplacement(prefix)
        assertEquals(prefix, (events.last() as Event.SetVocabularyQuery).value.prefix)
        compose.onNodeWithTag("vocabulary_clear_search").performClick()
        assertEquals("", frame.value.vocabulary.query.prefix)
        VocabularyStatus.entries.forEach { status ->
            compose
                .onNodeWithTag("vocabulary_status_${status.name}")
                .performScrollTo()
                .assertIsSelected()
                .performClick()
            compose.onNodeWithTag("vocabulary_status_${status.name}").assertIsNotSelected()
        }
        assertTrue(frame.value.vocabulary.query.statuses.isEmpty())
        VocabularyStatus.entries.forEach { status ->
            compose
                .onNodeWithTag("vocabulary_status_${status.name}")
                .performScrollTo()
                .performClick()
        }
        assertEquals(VocabularyStatus.entries, frame.value.vocabulary.query.statuses)
        listOf(
                VocabularySort.NEWEST,
                VocabularySort.FREQUENCY,
                VocabularySort.LASTSEEN,
                VocabularySort.ALPHABETICAL,
            )
            .forEach { sort ->
                compose.onNodeWithTag("vocabulary_sort").performTouchInput { click() }
                compose.onNodeWithTag("vocabulary_sort_${sort.name}").performTouchInput { click() }
                assertEquals(sort, (events.last() as Event.SetVocabularyQuery).value.sort)
                compose.onNodeWithTag("vocabulary_reverse_sort").performClick()
                assertNotEquals(sort, frame.value.vocabulary.query.sort)
                compose.onNodeWithTag("vocabulary_sort").performClick()
                compose.onNodeWithTag("vocabulary_sort_${sort.name}").performClick()
                assertEquals(sort, frame.value.vocabulary.query.sort)
            }
    }

    @Test
    fun lazyLoadWaitsForIdleAndErrorsOnlyRetryExplicitlyIncludingAppend() {
        frame.value = frame.value.copy(storageBusy = true)
        render()
        compose.waitForIdle()
        assertTrue(events.isEmpty())
        compose.onNodeWithTag("vocabulary_search").assertIsNotEnabled()
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = false) }
        compose.waitForIdle()
        assertEquals(listOf(Event.LoadVocabulary), events)
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    vocabulary = frame.value.vocabulary.copy(error = DocumentError.UNAVAILABLE)
                )
        }
        compose.onNodeWithTag("vocabulary_error").assertIsDisplayed()
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = true) }
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = false) }
        compose.mainClock.advanceTimeBy(1000)
        assertEquals(listOf(Event.LoadVocabulary), events)
        compose.onNodeWithTag("vocabulary_retry").performClick()
        assertEquals(Event.RetryVocabulary, events.last())
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    vocabulary =
                        frame.value.vocabulary.copy(
                            matchingCount = 200UL,
                            rows = (1L..100L).map(::vocabularyRow),
                            nextOffset = 100UL,
                        )
                )
        }
        compose
            .onNodeWithTag("vocabulary_scroll")
            .performScrollToNode(hasTestTag("vocabulary_retry"))
        compose.onNodeWithTag("vocabulary_row_100").assertIsDisplayed()
        compose.waitForIdle()
        assertFalse(events.contains(Event.LoadMoreVocabulary))
        compose.onNodeWithTag("vocabulary_retry").performClick()
        assertEquals(2, events.count { it == Event.RetryVocabulary })
        compose.runOnUiThread { frame.value = frame.value.copy(settingsOpen = true) }
        compose.onNodeWithTag("vocabulary_error").assertDoesNotExist()
        compose.runOnUiThread { frame.value = frame.value.copy(settingsOpen = false) }
        compose
            .onNodeWithTag("vocabulary_scroll")
            .performScrollToNode(hasTestTag("vocabulary_retry"))
        compose.onNodeWithTag("vocabulary_retry").assertIsDisplayed()
        assertEquals(2, events.count { it == Event.RetryVocabulary })
        assertFalse(events.contains(Event.LoadMoreVocabulary))
    }

    @Test
    fun infiniteScrollLoadsOnlyWhenLastRowVisibleAndQueryResetsScroll() {
        frame.value =
            frame.value.copy(
                vocabulary =
                    frame.value.vocabulary.copy(
                        matchingCount = 205UL,
                        rows = (1L..100L).map(::vocabularyRow),
                        nextOffset = 100UL,
                    )
            )
        render()
        compose.waitForIdle()
        assertTrue(events.isEmpty())
        compose.onNodeWithTag("vocabulary_scroll").performScrollToKey(100L)
        compose.waitForIdle()
        assertEquals(listOf(Event.LoadMoreVocabulary), events)
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    storageBusy = true,
                    vocabulary = frame.value.vocabulary.copy(loading = true),
                )
        }
        compose.onNodeWithTag("vocabulary_search").assertDoesNotExist()
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    storageBusy = false,
                    vocabulary =
                        frame.value.vocabulary.copy(
                            loading = false,
                            rows = (1L..200L).map(::vocabularyRow),
                            nextOffset = 200UL,
                        ),
                )
        }
        compose.waitForIdle()
        assertEquals(1, events.size)
        compose.onNodeWithTag("vocabulary_scroll").performScrollToKey(200L)
        compose.waitForIdle()
        assertEquals(2, events.count { it == Event.LoadMoreVocabulary })
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    vocabulary =
                        frame.value.vocabulary.copy(
                            rows = (1L..205L).map(::vocabularyRow),
                            nextOffset = null,
                        )
                )
        }
        compose.onNodeWithTag("vocabulary_scroll").performScrollToKey(205L)
        compose.waitForIdle()
        assertEquals(2, events.size)
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    vocabulary =
                        frame.value.vocabulary.copy(
                            query = frame.value.vocabulary.query.copy(prefix = "none"),
                            matchingCount = 0UL,
                            rows = emptyList(),
                        )
                )
        }
        compose.onNodeWithTag("vocabulary_search").assertIsDisplayed()
        compose.onNodeWithText("No matching words.").assertIsDisplayed()
        compose.onNodeWithTag("vocabulary_count").assertTextEquals("Matching words: 0")
    }

    @Test
    fun rowTapOpensWordAndLongPressHasNoSelectionAction() {
        frame.value =
            frame.value.copy(
                vocabulary =
                    frame.value.vocabulary.copy(
                        matchingCount = 1UL,
                        rows = listOf(vocabularyRow(1)),
                    )
            )
        render()
        compose.onNodeWithTag("vocabulary_row_1").performClick()
        assertEquals(Event.OpenVocabularyWord(VocabularyId(1)), events.last())
        events.clear()
        compose
            .onNodeWithTag("vocabulary_row_1")
            .assert(
                SemanticsMatcher.keyNotDefined(
                    androidx.compose.ui.semantics.SemanticsActions.OnLongClick
                )
            )
        compose.onNodeWithTag("vocabulary_row_1").performTouchInput { longClick() }
        assertTrue(events.all { it == Event.OpenVocabularyWord(VocabularyId(1)) })
        compose.onNodeWithTag("vocabulary_select_all").assertDoesNotExist()
        compose.onNodeWithTag("vocabulary_selected_count").assertDoesNotExist()
    }

    @Test
    fun wordSheetSharesLookupAndStatusControlsAndRetainsDismissedWriteRetry() {
        val row = vocabularyRow(1).copy(encounteredCount = 7UL)
        frame.value =
            frame.value.copy(
                vocabulary =
                    frame.value.vocabulary.copy(
                        matchingCount = 1UL,
                        rows = listOf(row),
                        word =
                            VocabularyWordState(
                                DictionaryLookup(
                                    DictionaryQuery(row.surface),
                                    emptyList(),
                                    revision = 1UL,
                                    loading = false,
                                ),
                                VocabularyAnkiSelection(row.id, 1UL),
                                row,
                                error = DocumentError.UNAVAILABLE,
                            ),
                    )
            )
        render()
        compose.onNodeWithTag("word_surface").assertTextEquals(row.surface)
        compose.onNodeWithTag("word_lookup_error").assertExists()
        compose.onNodeWithTag("retry_word_details").performClick()
        assertEquals(Event.RetryVocabularyWord, events.last())
        compose.onNodeWithTag("word_status_NEW").performClick()
        assertEquals(Event.DismissVocabularyWord, events.last())
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    vocabulary =
                        frame.value.vocabulary.copy(
                            word =
                                VocabularyWordState(
                                    DictionaryLookup(
                                        DictionaryQuery(row.surface),
                                        emptyList(),
                                        revision = 1UL,
                                        loading = false,
                                    ),
                                    VocabularyAnkiSelection(row.id, 1UL),
                                    row,
                                    DictionaryCandidates(
                                        row.surface,
                                        words = emptyList(),
                                        names = emptyList(),
                                    ),
                                )
                        )
                )
        }
        compose.onNodeWithTag("word_encountered").assertTextEquals("Encountered 7 times")
        compose.onNodeWithTag("word_no_match").assertExists()
        compose.onNodeWithTag("vocabulary_delete_selected").assertDoesNotExist()
        val before = System.currentTimeMillis()
        compose.onNodeWithTag("word_status_LEARNING").performClick()
        val command = (events.last() as Event.SetVocabularyWordStatus).value
        assertEquals(row.id, command.id)
        assertEquals(VocabularyStatus.LEARNING, command.status)
        assertTrue(command.occurredAtUnixMs in before..System.currentTimeMillis())
        compose.onNodeWithTag("vocabulary_bulk_dialog").assertDoesNotExist()
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    vocabulary =
                        frame.value.vocabulary.copy(
                            wordStatus = command,
                            wordStatusError = DocumentError.UNAVAILABLE,
                        )
                )
        }
        compose.onNodeWithTag("word_status_KNOWN").assertIsNotEnabled()
        compose.onNodeWithTag("vocabulary_word_status_retry").performClick()
        assertEquals(Event.RetryVocabularyWordStatus, events.last())
        compose.onNodeWithTag("vocabulary_word_status_cancel").performClick()
        assertEquals(Event.CancelVocabularyWordStatus, events.last())
        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
            .sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_BACK)
        compose.waitForIdle()
        assertEquals(Event.DismissVocabularyWord, events.last())
        compose.runOnUiThread {
            frame.value = frame.value.copy(vocabulary = frame.value.vocabulary.copy(word = null))
        }
        compose.onNodeWithTag("vocabulary_word_status_retry").performClick()
        assertEquals(Event.RetryVocabularyWordStatus, events.last())
        compose.onNodeWithTag("vocabulary_word_status_cancel").performClick()
        assertEquals(Event.CancelVocabularyWordStatus, events.last())
    }
}

private fun vocabularyRow(id: Long) =
    VocabularyRow(VocabularyId(id), "語$id", VocabularyStatus.NEW, 1L, 0UL)

private fun vocabularyView() =
    ViewModel(
        ankiAddedNotice = null,
        ankiSettings = AnkiSettingsView(AnkiSettingsPhase.UNLOADED, null, null),
        ankiSetup = app.yomoku.shared.AnkiSetupPhase.Unchecked,
        ankiEntries = emptyList(),
        vocabulary =
            VocabularyState(
                VocabularyQuery("", VocabularyStatus.entries.toList(), VocabularySort.NEWEST),
                rows = emptyList(),
                loading = false,
            ),
        statistics = StatisticsState(StatisticsTimeframe.LASTWEEK, loading = false),
        destination = Destination.VOCABULARY,
        settingsOpen = false,
        documents = emptyList(),
        librarySort = LibrarySort.LASTREAD,
        storageBusy = false,
        contentRoute = ContentRoute.MAIN,
        completionSavePending = false,
    )
