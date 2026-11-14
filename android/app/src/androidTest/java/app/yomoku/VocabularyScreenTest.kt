package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
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

    private fun render() {
        compose.setContent {
            YomokuTheme {
                YomokuApp(
                    frame.value,
                    "vocabulary-test",
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
                compose.onNodeWithTag("vocabulary_sort").performClick()
                compose.onNodeWithTag("vocabulary_sort_${sort.name}").performClick()
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
                        word = VocabularyWordState(row, error = DocumentError.UNAVAILABLE),
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
