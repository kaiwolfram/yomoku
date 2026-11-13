package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
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
class StatisticsScreenTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val events = mutableListOf<Event>()
    private val frame = mutableStateOf(statisticsView())

    private fun render() {
        compose.setContent {
            YomokuTheme { YomokuApp(frame.value, "statistics-test", events::add) }
        }
    }

    @Test
    fun persistedIntervalIsVisibleOnEntryAndSelectionPreservesBothScrollPositions() {
        frame.value =
            frame.value.copy(
                statistics = frame.value.statistics.copy(timeframe = StatisticsTimeframe.ALLTIME)
            )
        render()
        compose.onNodeWithTag("statistics_timeframe_ALLTIME").assertIsDisplayed().assertIsSelected()
        compose
            .onNodeWithTag("statistics_timeframes")
            .performScrollToNode(hasTestTag("statistics_timeframe_LASTWEEK"))
        compose.onNodeWithTag("statistics_scroll").performSemanticsAction(
            SemanticsActions.ScrollBy
        ) {
            it(0f, 12f)
        }
        compose.waitForIdle()
        fun horizontal() =
            compose
                .onNodeWithTag("statistics_timeframes")
                .fetchSemanticsNode()
                .config[SemanticsProperties.HorizontalScrollAxisRange]
                .value()
        fun vertical() =
            compose
                .onNodeWithTag("statistics_scroll")
                .fetchSemanticsNode()
                .config[SemanticsProperties.VerticalScrollAxisRange]
                .value()
        val x = horizontal()
        val y = vertical()
        compose.onNodeWithTag("statistics_timeframe_LASTMONTH").performClick()
        assertEquals(
            StatisticsTimeframe.LASTMONTH,
            (events.last() as Event.SetStatisticsTimeframe).timeframe,
        )
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    statistics =
                        frame.value.statistics.copy(timeframe = StatisticsTimeframe.LASTMONTH)
                )
        }
        compose.waitForIdle()
        compose.onNodeWithTag("statistics_timeframe_LASTMONTH").assertIsSelected()
        assertEquals(x, horizontal(), 0.5f)
        assertEquals(y, vertical(), 0.5f)
    }

    @Test
    fun overviewChartsHaveNoDetailActionsAndAllTimeframeEvents() {
        render()
        compose.onNodeWithTag("statistics_card_KNOWN").assertDoesNotExist()
        compose.onNodeWithTag("statistics_timeframe_LASTWEEK").assertIsSelected()
        StatisticsTimeframe.entries.forEach { timeframe ->
            compose
                .onNodeWithTag("statistics_timeframes")
                .performScrollToNode(hasTestTag("statistics_timeframe_${timeframe.name}"))
            compose.onNodeWithTag("statistics_timeframe_${timeframe.name}").performClick()
            val event = events.last() as Event.SetStatisticsTimeframe
            assertEquals(timeframe, event.timeframe)
            assertTrue(event.asOfUnixMs > 0)
            compose.runOnUiThread {
                val data = statisticsFixture(timeframe)
                frame.value =
                    frame.value.copy(
                        statistics =
                            StatisticsState(
                                timeframe,
                                StatisticsQuery(timeframe, data.endExclusiveUnixMs - 1),
                                false,
                                data = data,
                            )
                    )
            }
            compose.onNodeWithTag("statistics_timeframe_${timeframe.name}").assertIsSelected()
        }
        StatisticsChart.entries.forEach { chart ->
            compose
                .onNodeWithTag("statistics_scroll")
                .performScrollToNode(hasTestTag("statistics_card_${chart.name}"))
            compose.onNodeWithTag("statistics_card_${chart.name}").assertHasNoClickAction()
            compose.onNodeWithText("View details").assertDoesNotExist()
            val fixture = statisticsFixture()
            val description =
                compose.activity.getString(chart.title) +
                    ": " +
                    chart.lines
                        .mapIndexed { index, label ->
                            "${compose.activity.getString(label)} ${chart.values(fixture.opening)[index]} → ${chart.values(fixture.buckets.last().values)[index]}"
                        }
                        .joinToString(", ")
            compose
                .onNodeWithTag("statistics_scroll")
                .performScrollToNode(hasTestTag("statistics_chart_${chart.name}"))
            compose
                .onNodeWithTag("statistics_chart_${chart.name}")
                .assertContentDescriptionEquals(description)
        }
    }

    @Test
    fun visibleIdleLoadingUsesOneCutoffAndErrorNeedsExplicitRetry() {
        frame.value =
            frame.value.copy(
                storageBusy = true,
                statistics = StatisticsState(StatisticsTimeframe.LASTWEEK, loading = false),
            )
        render()
        compose.waitForIdle()
        assertTrue(events.isEmpty())
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = false) }
        compose.waitForIdle()
        val requested = events.single() as Event.LoadStatistics
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = true) }
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = false) }
        compose.waitForIdle()
        assertEquals(requested, events.last())
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    statistics =
                        StatisticsState(
                            StatisticsTimeframe.LASTWEEK,
                            StatisticsQuery(StatisticsTimeframe.LASTWEEK, requested.asOfUnixMs),
                            false,
                            DocumentError.UNAVAILABLE,
                        )
                )
        }
        compose.onNodeWithTag("statistics_error").assertIsDisplayed()
        val count = events.size
        compose.mainClock.advanceTimeBy(1000)
        compose.waitForIdle()
        assertEquals(count, events.size)
        assertFalse(events.contains(Event.RetryStatistics))
        compose.onNodeWithText("Retry").performClick()
        assertEquals(Event.RetryStatistics, events.last())
        compose.runOnUiThread { frame.value = frame.value.copy(settingsOpen = true) }
        compose.onNodeWithTag("statistics_error").assertDoesNotExist()
        compose.waitForIdle()
        assertEquals(Event.RetryStatistics, events.last())
        compose.runOnUiThread { frame.value = frame.value.copy(settingsOpen = false) }
        compose.onNodeWithTag("statistics_error").assertIsDisplayed()
        assertEquals(requested, events.last())
        assertEquals(1, events.count { it == Event.RetryStatistics })
    }

    @Test
    fun settingsKeepTheOverview() {
        render()
        compose.onNodeWithTag("statistics_card_OCCURRENCES").assertIsDisplayed()
        compose.runOnUiThread { frame.value = frame.value.copy(settingsOpen = true) }
        compose.onNodeWithTag("statistics_chart_OCCURRENCES").assertDoesNotExist()
        compose.runOnUiThread { frame.value = frame.value.copy(settingsOpen = false) }
        compose.onNodeWithTag("statistics_card_OCCURRENCES").assertIsDisplayed()
        compose.onNodeWithText("View details").assertDoesNotExist()
    }
}

private fun statisticsView(): ViewModel {
    val data = statisticsFixture()
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
        statistics =
            StatisticsState(
                StatisticsTimeframe.LASTWEEK,
                StatisticsQuery(StatisticsTimeframe.LASTWEEK, data.endExclusiveUnixMs - 1),
                false,
                data = data,
            ),
        destination = Destination.STATISTICS,
        settingsOpen = false,
        documents = emptyList(),
        librarySort = LibrarySort.LASTREAD,
        storageBusy = false,
        contentRoute = ContentRoute.MAIN,
        completionSavePending = false,
        readerFurigana = emptyList(),
    )
}
