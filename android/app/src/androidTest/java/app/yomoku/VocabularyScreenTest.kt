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
        VocabularySort.entries.forEach { sort ->
            compose.onNodeWithTag("vocabulary_sort").performClick()
            compose.onNodeWithTag("vocabulary_sort_${sort.name}").performClick()
            assertEquals(sort, (events.last() as Event.SetVocabularyQuery).value.sort)
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
}

private fun vocabularyRow(id: Long) =
    VocabularyRow(VocabularyId(id), "語$id", VocabularyStatus.NEW, 1L, 0UL)

private fun vocabularyView() =
    ViewModel(
        vocabulary =
            VocabularyState(
                VocabularyQuery("", VocabularyStatus.entries.toList(), VocabularySort.NEWEST),
                rows = emptyList(),
                selectingAll = false,
                loading = false,
            ),
        statistics = StatisticsState(StatisticsTimeframe.LASTYEAR, loading = false),
        destination = Destination.VOCABULARY,
        settingsOpen = false,
        documents = emptyList(),
        libraryTitleQuery = "",
        librarySort = LibrarySort.LASTREAD,
        libraryDocumentCount = 0UL,
        storageBusy = false,
        contentRoute = ContentRoute.MAIN,
        readingTimeSavePending = false,
        completionSavePending = false,
        readerFurigana = emptyList(),
    )
