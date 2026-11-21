package app.yomoku

import android.content.res.Configuration
import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.unit.dp
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
    fun mainScreenContentStartsAtToolbarBottom() {
        frame.value =
            frame.value.copy(
                documents =
                    (1L..2L).map {
                        LibraryDocument(
                            documentId = DocumentId(it),
                            title = "Book $it",
                            progressPercent = 0UL,
                            isRead = false,
                        )
                    },
                readerTypography =
                    ReaderTypography(ReaderFontSize.SP16, ReaderLineSpacing.PERCENT150),
                pageLength = PageLength.CHARACTERS250,
            )
        render()
        fun assertTop(node: SemanticsNodeInteraction) {
            val toolbar = compose.onNodeWithTag("app_toolbar").getUnclippedBoundsInRoot()
            assertEquals(toolbar.bottom.value, node.getUnclippedBoundsInRoot().top.value, 0.5f)
        }
        assertTop(compose.onNodeWithTag("statistics_header"))
        compose.runOnUiThread { frame.value = frame.value.copy(destination = Destination.LIBRARY) }
        assertTop(compose.onNodeWithTag("library_sort_controls"))
        compose.runOnUiThread {
            frame.value = frame.value.copy(destination = Destination.VOCABULARY)
        }
        assertTop(compose.onNodeWithTag("vocabulary_search"))
        compose.runOnUiThread { frame.value = frame.value.copy(settingsOpen = true) }
        assertTop(compose.onNodeWithText("Reader"))
    }

    @Test
    fun persistedIntervalIsVisibleAndDropdownSelectionPreservesVerticalScroll() {
        frame.value =
            frame.value.copy(
                statistics = frame.value.statistics.copy(timeframe = StatisticsTimeframe.ALLTIME)
            )
        render()
        compose.onNodeWithTag("statistics_timeframe").assertTextContains("All time")
        compose.onNodeWithTag("statistics_scroll").performSemanticsAction(
            SemanticsActions.ScrollBy
        ) {
            it(0f, 12f)
        }
        compose.waitForIdle()
        fun vertical() =
            compose
                .onNodeWithTag("statistics_scroll")
                .fetchSemanticsNode()
                .config[SemanticsProperties.VerticalScrollAxisRange]
                .value()
        val y = vertical()
        compose.onNodeWithTag("statistics_timeframe").performTouchInput { click() }
        compose.onNodeWithTag("statistics_timeframe_ALLTIME").assertIsSelected()
        compose.onNodeWithTag("statistics_timeframe_LASTMONTH").performClick()
        compose.onNodeWithTag("statistics_timeframe_LASTMONTH").assertDoesNotExist()
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
        compose.onNodeWithTag("statistics_timeframe").assertTextContains("Last month")
        assertEquals(y, vertical(), 0.5f)
    }

    @Test
    fun shortViewportKeepsChartAnchorAcrossPendingSelectionsAndFailureRetry() {
        compose.setContent {
            YomokuTheme {
                Box(Modifier.height(260.dp)) {
                    StatisticsScreen(frame.value, "short-statistics", events::add)
                }
            }
        }
        val list = compose.onNodeWithTag("statistics_scroll")
        val chart = compose.onNodeWithTag("statistics_card_STATUSES")
        list.performScrollToKey(StatisticsChart.STATUSES.name)
        fun chartTop() = chart.assertIsDisplayed().getUnclippedBoundsInRoot().top.value
        val top = chartTop()
        val previous = checkNotNull(frame.value.statistics.data)
        fun show(
            timeframe: StatisticsTimeframe,
            loading: Boolean,
            data: StatisticsBuckets,
            error: DocumentError? = null,
        ) {
            compose.runOnUiThread {
                frame.value =
                    frame.value.copy(
                        storageBusy = loading,
                        statistics =
                            StatisticsState(
                                timeframe,
                                StatisticsQuery(timeframe, data.endExclusiveUnixMs - 1),
                                loading,
                                error,
                                data,
                            ),
                    )
            }
        }
        show(StatisticsTimeframe.LASTMONTH, true, previous)
        assertEquals(top, chartTop(), 0.5f)
        show(StatisticsTimeframe.LASTYEAR, true, previous)
        assertEquals(top, chartTop(), 0.5f)
        show(StatisticsTimeframe.ALLTIME, false, previous, DocumentError.UNAVAILABLE)
        compose.onNodeWithTag("statistics_error").assertIsDisplayed()
        compose.onNodeWithText("Retry").performClick()
        assertEquals(Event.RetryStatistics, events.last())
        show(StatisticsTimeframe.ALLTIME, true, previous)
        assertEquals(top, chartTop(), 0.5f)
        show(StatisticsTimeframe.ALLTIME, false, statisticsFixture(StatisticsTimeframe.ALLTIME))
        assertEquals(top, chartTop(), 0.5f)
        list.performScrollToKey("header")
        compose.onNodeWithText("Monthly · UTC · lifetime values").assertIsDisplayed()
    }

    @Test
    fun overviewChartsHaveNoDetailActionsAndAllTimeframeEvents() {
        render()
        compose.onNodeWithTag("statistics_card_KNOWN").assertDoesNotExist()
        compose.onNodeWithTag("statistics_timeframe").assertTextContains("Last week")
        captureUiTrial(compose, "statistics")
        StatisticsTimeframe.entries.forEach { timeframe ->
            compose.onNodeWithTag("statistics_timeframe").performTouchInput { click() }
            compose.onNodeWithTag("statistics_timeframe_${timeframe.name}").performTouchInput {
                click()
            }
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
            compose.onNodeWithTag("statistics_timeframe_${timeframe.name}").assertDoesNotExist()
            compose.onNodeWithTag("statistics_timeframe").performClick()
            compose.onNodeWithTag("statistics_timeframe_${timeframe.name}").assertIsSelected()
            if (timeframe == StatisticsTimeframe.LASTWEEK)
                captureUiTrial(compose, "statistics-menu")
            androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
                .sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_BACK)
            compose.onNodeWithTag("statistics_timeframe_${timeframe.name}").assertDoesNotExist()
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
    fun wordStatusOverlapUsesLatestCountsAndStatusOrder() {
        val fixture = statisticsFixture()
        val sharedCounts = WordStatusCounts(30UL, 30UL, 30UL, 30UL, 30UL)
        val lastBucket = fixture.buckets.last()
        val cases =
            listOf(
                WordStatusCounts(20UL, 35UL, 45UL, 25UL, 25UL) to VocabularyStatus.FAMILIAR,
                WordStatusCounts(20UL, 40UL, 40UL, 25UL, 25UL) to VocabularyStatus.LEARNING,
                sharedCounts to VocabularyStatus.NEW,
                WordStatusCounts(20UL, 30UL, 35UL, 25UL, 40UL) to VocabularyStatus.IGNORED,
            )
        render()
        val chartTag = "statistics_chart_STATUSES"
        compose.onNodeWithTag("statistics_scroll").performScrollToNode(hasTestTag(chartTag))
        val dark =
            (compose.activity.resources.configuration.uiMode and
                Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES
        for ((latestCounts, expectedStatus) in cases) {
            val data =
                fixture.copy(
                    opening = fixture.opening.copy(statuses = sharedCounts),
                    buckets =
                        listOf(
                            fixture.buckets.first(),
                            lastBucket.copy(
                                values = lastBucket.values.copy(statuses = latestCounts)
                            ),
                        ),
                )
            compose.runOnUiThread {
                frame.value =
                    frame.value.copy(statistics = frame.value.statistics.copy(data = data))
            }
            val pixels = compose.onNodeWithTag(chartTag).captureToImage().toPixelMap()
            // The first segment spans a quarter of the plot. Its shared count of 30 is midway
            // between the extrema of 10 and 50, which the first bucket supplies in every case.
            assertEquals(
                "Top status for $latestCounts should be $expectedStatus",
                vocabularyBackground(expectedStatus, dark),
                pixels[pixels.width / 8, pixels.height / 2],
            )
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
    fun dropdownDismissesOnOutsideTapAndWhenLoadingGuardDisablesIt() {
        render()
        compose.onNodeWithTag("statistics_timeframe").performTouchInput { click() }
        compose.onNodeWithTag("statistics_timeframe_LASTWEEK").assertIsSelected()
        tapBelowPopup(compose.activity)
        compose.onNodeWithTag("statistics_timeframe_LASTWEEK").assertDoesNotExist()
        compose.onNodeWithTag("statistics_timeframe").performClick()
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = true) }
        compose.onNodeWithTag("statistics_timeframe_LASTWEEK").assertDoesNotExist()
        compose.onNodeWithTag("statistics_timeframe").assertIsNotEnabled()
        compose.runOnUiThread {
            frame.value = frame.value.copy(statistics = frame.value.statistics.copy(loading = true))
        }
        compose.onNodeWithTag("statistics_timeframe").assertIsEnabled().performTouchInput {
            click()
        }
        compose.onNodeWithTag("statistics_timeframe_LASTMONTH").performTouchInput { click() }
        assertEquals(
            StatisticsTimeframe.LASTMONTH,
            (events.last() as Event.SetStatisticsTimeframe).timeframe,
        )
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
    )
}
