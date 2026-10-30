package app.yomoku

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class StatisticsProductionTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun idle() {
        compose.waitForIdle()
        compose.waitUntil(90_000) {
            val view = core().view
            !view.storageBusy &&
                !view.readingTimeSavePending &&
                !view.completionSavePending &&
                (view.destination != Destination.STATISTICS ||
                    view.statistics.data != null ||
                    view.statistics.error != null)
        }
        assertNull(core().view.statistics.error)
    }

    private fun visit(): StatisticsBuckets {
        compose.onNode(hasText("Statistics") and hasClickAction()).performClick()
        idle()
        return checkNotNull(core().view.statistics.data)
    }

    private fun totals() = checkNotNull(core().view.statistics.data).buckets.last().values

    private fun open(id: DocumentId): EffectiveReaderPage {
        compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
        compose.waitUntil(90_000) {
            core().view.effectiveReaderTokens?.identity?.base?.documentId == id &&
                core().view.readerTime != null &&
                !core().view.storageBusy
        }
        return checkNotNull(core().view.effectiveReaderTokens)
    }

    @Test
    fun chartsRefreshAfterImportManualStatusReadingAndRecreationInEveryTimeframe() {
        idle()
        visit()
        val before = totals()
        compose.onNode(hasText("Library") and hasClickAction()).performClick()
        idle()
        compose.runOnUiThread {
            core().dispatch(Event.CreatePaste("Statistics-${System.nanoTime()}\n猫 猫 犬"))
        }
        idle()
        val id = checkNotNull(core().view.importSummary).documentId
        compose.runOnUiThread { core().dispatch(Event.DismissImportSummary) }
        visit()
        val imported = totals()
        assertEquals(before.reading, imported.reading)
        assertTrue(imported.statuses.new >= before.statuses.new)

        compose.onNode(hasText("Library") and hasClickAction()).performClick()
        idle()
        val page = open(id)
        assertEquals(3, page.tokens.size)
        val token = page.tokens.first()
        val old = checkNotNull(token.vocabulary).status
        val chosen =
            if (old == VocabularyStatus.KNOWN) VocabularyStatus.LEARNING else VocabularyStatus.KNOWN
        compose.runOnUiThread {
            core()
                .dispatch(
                    Event.SetEffectiveReaderStatus(
                        EffectiveReaderSelection(page.identity, token.segmentIndex),
                        checkNotNull(token.vocabulary).id,
                        chosen,
                        System.currentTimeMillis(),
                    )
                )
        }
        idle()
        compose.runOnUiThread { core().dispatch(Event.CloseDocument) }
        idle()
        visit()
        val changed = totals()
        assertEquals(imported.reading, changed.reading)
        assertEquals(
            if (chosen == VocabularyStatus.KNOWN) imported.statuses.known + 1UL
            else imported.statuses.known - 1UL,
            changed.statuses.known,
        )
        if (chosen == VocabularyStatus.KNOWN) assertTrue(changed.knownDictionaryForms > 0UL)
        else assertTrue(changed.knownDictionaryForms <= imported.knownDictionaryForms)

        compose.onNode(hasText("Library") and hasClickAction()).performClick()
        idle()
        open(id)
        compose.onNodeWithTag("reader_scroll").performScrollToNode(hasTestTag("next_page"))
        compose.onNodeWithTag("next_page").performClick()
        compose.waitUntil(90_000) { core().view.contentRoute == ContentRoute.MAIN }
        idle()
        visit()
        val completed = totals()
        assertEquals(
            before.reading.japaneseOccurrences + 3UL,
            completed.reading.japaneseOccurrences,
        )

        StatisticsTimeframe.entries.forEach { timeframe ->
            compose.onNodeWithTag("statistics_timeframe_${timeframe.name}").performClick()
            idle()
            val data = checkNotNull(core().view.statistics.data)
            assertEquals(timeframe, data.timeframe)
            assertEquals(completed, totals())
            val graph = StatisticsDataset(data, StatisticsChart.OCCURRENCES)
            assertEquals(
                data.opening.reading.japaneseOccurrences,
                graph.points.first().values.single(),
            )
            assertEquals(completed.reading.japaneseOccurrences, graph.points.last().values.single())
        }

        compose.onNodeWithTag("statistics_card_OCCURRENCES").performClick()
        compose.onNodeWithTag("statistics_overview").assertIsDisplayed()
        val original = core()
        val query = original.view.statistics.query
        compose.activityRule.scenario.recreate()
        idle()
        assertSame(original, core())
        assertEquals(query, core().view.statistics.query)
        assertEquals(StatisticsTimeframe.ALLTIME, core().view.statistics.timeframe)
        compose.onNodeWithTag("statistics_overview").assertIsDisplayed()
        compose.onNodeWithText("Settings").performClick()
        compose.onNodeWithText("Back").performClick()
        idle()
        compose.onNodeWithTag("statistics_overview").assertIsDisplayed()
        assertEquals(query, core().view.statistics.query)
        assertEquals(completed, totals())
    }
}
