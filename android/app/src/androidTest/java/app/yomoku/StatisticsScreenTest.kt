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
    fun overviewDrilldownsExactValuesAndAllTimeframeEvents() {
        render()
        compose.onNodeWithTag("statistics_card_KNOWN").assertDoesNotExist()
        compose.onNodeWithTag("statistics_timeframe_LASTYEAR").assertIsSelected()
        StatisticsTimeframe.entries.forEach { timeframe ->
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
            compose.onNodeWithTag("statistics_card_${chart.name}").performClick()
            compose.onNodeWithTag("statistics_overview").assertIsDisplayed()
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
            compose
                .onNodeWithTag("statistics_scroll")
                .performScrollToNode(hasTestTag("statistics_opening"))
            chart.values(statisticsFixture().opening).forEachIndexed { index, value ->
                compose
                    .onNodeWithTag("statistics_opening")
                    .onChildren()
                    .filter(hasText("${compose.activity.getString(chart.lines[index])}: $value"))
                    .assertCountEquals(1)
            }
            val last = fixture.buckets.last()
            val bucket = "statistics_bucket_${last.startUnixMs}"
            compose.onNodeWithTag("statistics_scroll").performScrollToNode(hasTestTag(bucket))
            chart.values(last.values).forEachIndexed { index, value ->
                compose
                    .onNodeWithTag(bucket)
                    .onChildren()
                    .filter(hasText("${compose.activity.getString(chart.lines[index])}: $value"))
                    .assertCountEquals(1)
            }
            compose
                .onNodeWithTag("statistics_scroll")
                .performScrollToNode(hasTestTag("statistics_overview"))
            compose.onNodeWithTag("statistics_overview").performClick()
        }
    }

    @Test
    fun visibleIdleLoadingUsesOneCutoffAndErrorNeedsExplicitRetry() {
        frame.value =
            frame.value.copy(
                storageBusy = true,
                statistics = StatisticsState(StatisticsTimeframe.LASTYEAR, loading = false),
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
                            StatisticsTimeframe.LASTYEAR,
                            StatisticsQuery(StatisticsTimeframe.LASTYEAR, requested.asOfUnixMs),
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
    fun detailBackAndSettingsKeepTheSelectedChart() {
        render()
        compose.onNodeWithTag("statistics_card_OCCURRENCES").performClick()
        compose.onNodeWithTag("statistics_overview").assertIsDisplayed()
        compose.runOnUiThread { frame.value = frame.value.copy(settingsOpen = true) }
        compose.onNodeWithTag("statistics_chart_OCCURRENCES").assertDoesNotExist()
        compose.runOnUiThread { frame.value = frame.value.copy(settingsOpen = false) }
        compose.onNodeWithTag("statistics_overview").assertIsDisplayed()
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithTag("statistics_overview").assertDoesNotExist()
        compose.onNodeWithTag("statistics_card_OCCURRENCES").assertIsDisplayed()
    }
}

private fun statisticsView(): ViewModel {
    val data = statisticsFixture()
    return ViewModel(
        ankiSettings = AnkiSettingsView(AnkiSettingsPhase.UNLOADED, null, null),
        vocabulary =
            VocabularyState(
                VocabularyQuery("", VocabularyStatus.entries.toList(), VocabularySort.NEWEST),
                rows = emptyList(),
                selectingAll = false,
                loading = false,
            ),
        statistics =
            StatisticsState(
                StatisticsTimeframe.LASTYEAR,
                StatisticsQuery(StatisticsTimeframe.LASTYEAR, data.endExclusiveUnixMs - 1),
                false,
                data = data,
            ),
        destination = Destination.STATISTICS,
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
}
