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
                .assertIsSelected()
                .performClick()
            compose.onNodeWithTag("vocabulary_status_${status.name}").assertIsNotSelected()
        }
        assertTrue(frame.value.vocabulary.query.statuses.isEmpty())
        VocabularyStatus.entries.forEach { status ->
            compose.onNodeWithTag("vocabulary_status_${status.name}").performClick()
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
    fun selectionMembershipFollowsIdsAcrossRowSortAndEqualSizeReplacement() {
        frame.value =
            frame.value.copy(
                vocabulary =
                    frame.value.vocabulary.copy(
                        matchingCount = 3UL,
                        rows = listOf(Long.MAX_VALUE, 7L, 1L).map(::vocabularyRow),
                        selection = listOf(VocabularyId(1), VocabularyId(Long.MAX_VALUE)),
                    )
            )
        render()
        val search = compose.onNodeWithTag("vocabulary_search").getUnclippedBoundsInRoot()
        val actions = compose.onNodeWithTag("vocabulary_selected_count").getUnclippedBoundsInRoot()
        val filters = compose.onNodeWithTag("vocabulary_status_NEW").getUnclippedBoundsInRoot()
        assertTrue(search.bottom <= actions.top)
        assertTrue(actions.bottom <= filters.top)
        compose.onNodeWithTag("vocabulary_scroll").performScrollToKey(Long.MAX_VALUE)
        compose.onNodeWithTag("vocabulary_row_${Long.MAX_VALUE}").assertIsSelected()
        compose.onNodeWithTag("vocabulary_row_7").assertIsNotSelected()
        compose.onNodeWithTag("vocabulary_row_1").assertIsSelected()
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    vocabulary =
                        frame.value.vocabulary.copy(
                            selection = listOf(VocabularyId(7), VocabularyId(Long.MAX_VALUE))
                        )
                )
        }
        compose.onNodeWithTag("vocabulary_selected_count").assertTextEquals("Selected words: 2")
        compose.onNodeWithTag("vocabulary_row_${Long.MAX_VALUE}").assertIsSelected()
        compose.onNodeWithTag("vocabulary_row_7").assertIsSelected()
        compose.onNodeWithTag("vocabulary_row_1").assertIsNotSelected().performClick()
        assertEquals(Event.ToggleVocabularySelection(VocabularyId(1)), events.last())
    }

    @Test
    fun rowSelectionAllStatusesAndConfirmationEchoTheRetainedCommand() {
        frame.value =
            frame.value.copy(
                vocabulary =
                    frame.value.vocabulary.copy(
                        matchingCount = 205UL,
                        rows = listOf(vocabularyRow(1)),
                    )
            )
        render()
        compose.onNodeWithTag("vocabulary_select_all").assertDoesNotExist()
        compose.onNodeWithTag("vocabulary_row_1").performTouchInput { longClick() }
        assertEquals(Event.ToggleVocabularySelection(VocabularyId(1)), events.last())
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    vocabulary =
                        frame.value.vocabulary.copy(
                            selection = listOf(VocabularyId(1), VocabularyId(999))
                        )
                )
        }
        compose.onNodeWithTag("vocabulary_selected_count").assertTextEquals("Selected words: 2")
        compose.onNodeWithTag("vocabulary_scroll").performScrollToKey(1L)
        compose.onNodeWithTag("vocabulary_row_1").assertIsSelected().performClick()
        assertEquals(Event.ToggleVocabularySelection(VocabularyId(1)), events.last())
        compose.onNodeWithTag("vocabulary_select_all").performScrollTo().performClick()
        assertEquals(Event.SelectAllVocabulary, events.last())
        VocabularyStatus.entries.forEach { status ->
            compose.onNodeWithTag("vocabulary_change_status").performScrollTo().performClick()
            val before = System.currentTimeMillis()
            compose.onNodeWithTag("vocabulary_bulk_status_${status.name}").performClick()
            val requested =
                (events.last() as Event.RequestVocabularyBulkAction).value
                    as VocabularyBulkAction.SetStatus
            assertEquals(status, requested.status)
            assertTrue(requested.occurredAtUnixMs in before..System.currentTimeMillis())
            val frozen =
                VocabularyBulkCommand(
                    listOf(VocabularyId(1), VocabularyId(999)),
                    VocabularyBulkAction.SetStatus(status, 9_007_199_254_740_993L),
                )
            compose.runOnUiThread {
                frame.value =
                    frame.value.copy(
                        vocabulary = frame.value.vocabulary.copy(confirmation = frozen)
                    )
            }
            compose.onNodeWithTag("vocabulary_confirm_bulk").performClick()
            assertEquals(Event.ConfirmVocabularyBulkAction(frozen), events.last())
            compose.onNodeWithTag("vocabulary_cancel_bulk").performClick()
            assertEquals(Event.CancelVocabularyBulkAction, events.last())
            compose.runOnUiThread {
                frame.value =
                    frame.value.copy(vocabulary = frame.value.vocabulary.copy(confirmation = null))
            }
        }
        compose.onNodeWithTag("vocabulary_delete_selected").performScrollTo().performClick()
        assertEquals(Event.RequestVocabularyBulkAction(VocabularyBulkAction.Delete), events.last())
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.waitForIdle()
        assertEquals(Event.CancelVocabularySelection, events.last())
    }

    @Test
    fun captureAndWriteFailuresRequireExplicitRetainedRetryAndGatePendingWork() {
        frame.value =
            frame.value.copy(
                vocabulary =
                    frame.value.vocabulary.copy(
                        matchingCount = 205UL,
                        rows = listOf(vocabularyRow(1)),
                        selection = listOf(VocabularyId(1)),
                        bulkError = DocumentError.UNAVAILABLE,
                    )
            )
        render()
        compose
            .onNodeWithTag("vocabulary_bulk_error")
            .assertTextEquals("Matching words could not be selected. Retry or cancel.")
        compose.onNodeWithTag("vocabulary_confirm_bulk").performClick()
        assertEquals(Event.RetryVocabularySelection, events.last())
        compose.onNodeWithTag("vocabulary_cancel_bulk").performClick()
        assertEquals(Event.CancelVocabularySelection, events.last())
        val command = VocabularyBulkCommand(listOf(VocabularyId(1)), VocabularyBulkAction.Delete)
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(vocabulary = frame.value.vocabulary.copy(confirmation = command))
        }
        compose
            .onNodeWithText(
                "Permanently delete selected words (1) and their encounter and status history? Statistics will be recalculated. Existing documents will not recreate these words automatically. This cannot be undone."
            )
            .assertExists()
        compose.onNodeWithTag("vocabulary_confirm_bulk").performClick()
        assertEquals(Event.RetryVocabularyBulkAction, events.last())
        val count = events.size
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = true) }
        compose.onNodeWithTag("vocabulary_confirm_bulk").assertIsNotEnabled()
        compose.onNodeWithTag("vocabulary_cancel_bulk").assertIsNotEnabled()
        compose.onNodeWithTag("vocabulary_bulk_loading").assertIsDisplayed()
        compose.mainClock.advanceTimeBy(1000)
        assertEquals(count, events.size)
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    vocabulary =
                        frame.value.vocabulary.copy(
                            confirmation = null,
                            bulkError = null,
                            selectingAll = true,
                        )
                )
        }
        compose.onNodeWithTag("vocabulary_confirm_bulk").assertDoesNotExist()
        compose.onNodeWithTag("vocabulary_cancel_bulk").assertIsNotEnabled()
        assertEquals(count, events.size)
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
                selectingAll = false,
                loading = false,
            ),
        statistics = StatisticsState(StatisticsTimeframe.LASTWEEK, loading = false),
        destination = Destination.VOCABULARY,
        settingsOpen = false,
        documents = emptyList(),
        libraryTitleQuery = "",
        librarySort = LibrarySort.LASTREAD,
        libraryDocumentCount = 0UL,
        storageBusy = false,
        contentRoute = ContentRoute.MAIN,
        completionSavePending = false,
        readerFurigana = emptyList(),
    )
