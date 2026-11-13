package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.semantics.ProgressBarRangeInfo
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
class AppLoadingIndicatorTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Test
    fun delaysFor300MillisecondsAndDisappearsOnCompletionWithoutMinimumDisplayTime() {
        val loading = mutableStateOf(true)
        compose.mainClock.autoAdvance = false
        compose.setContent { YomokuTheme { AppLoadingIndicator(loading.value) } }
        compose.mainClock.advanceTimeBy(299, ignoreFrameDuration = true)
        compose.onNodeWithTag("app_loading").assertDoesNotExist()
        compose.mainClock.advanceTimeBy(1, ignoreFrameDuration = true)
        compose.mainClock.advanceTimeByFrame()
        compose
            .onNodeWithTag("app_loading")
            .assertIsDisplayed()
            .assertContentDescriptionEquals("Loading")
            .assertRangeInfoEquals(ProgressBarRangeInfo.Indeterminate)
        compose.runOnUiThread { loading.value = false }
        compose.mainClock.advanceTimeByFrame()
        compose.onNodeWithTag("app_loading").assertDoesNotExist()
    }

    @Test
    fun shortOperationsCancelPendingDisplayAndNextOperationGetsItsOwnDelay() {
        val loading = mutableStateOf(true)
        compose.mainClock.autoAdvance = false
        compose.setContent { AppLoadingIndicator(loading.value) }
        compose.mainClock.advanceTimeBy(200)
        compose.runOnUiThread { loading.value = false }
        compose.mainClock.advanceTimeByFrame()
        compose.mainClock.advanceTimeBy(400)
        compose.onNodeWithTag("app_loading").assertDoesNotExist()
        compose.runOnUiThread { loading.value = true }
        compose.mainClock.advanceTimeByFrame()
        compose.mainClock.advanceTimeBy(299, ignoreFrameDuration = true)
        compose.onNodeWithTag("app_loading").assertDoesNotExist()
        compose.mainClock.advanceTimeBy(32)
        compose.onNodeWithTag("app_loading").assertExists()
    }

    @Test
    fun continuousWorkAcrossSourcesKeepsDelayAndSpinnerIsAfterLibraryAdd() {
        val view = mutableStateOf(idleView().copy(storageBusy = true))
        val importing = mutableStateOf(false)
        compose.mainClock.autoAdvance = false
        compose.setContent {
            YomokuTheme {
                YomokuApp(view.value, "loading-test", {}, importActive = importing.value)
            }
        }
        compose.mainClock.advanceTimeBy(240)
        compose.runOnUiThread {
            view.value = view.value.copy(storageBusy = false)
            importing.value = true
        }
        compose.mainClock.advanceTimeBy(96)
        val spinner = compose.onNodeWithTag("app_loading")
        spinner.assertExists()
        compose.onAllNodesWithTag("app_loading").assertCountEquals(1)
        val add = compose.onNodeWithTag("library_add").fetchSemanticsNode().boundsInRoot
        assertTrue(spinner.fetchSemanticsNode().boundsInRoot.left >= add.right)
        compose.runOnUiThread { importing.value = false }
        compose.mainClock.advanceTimeByFrame()
        spinner.assertDoesNotExist()
    }

    @Test
    fun aggregatesAllExistingBusyStatesWithoutTreatingIdleOrFailedStatesAsWork() {
        val idle = idleView()
        assertFalse(appIsLoading(idle, AnkiDeckDiscoveryState.Unchecked, false))
        val states =
            listOf(
                idle.copy(storageBusy = true),
                idle.copy(vocabulary = idle.vocabulary.copy(loading = true)),
                idle.copy(statistics = idle.statistics.copy(loading = true)),
                idle.copy(ankiSettings = idle.ankiSettings.copy(phase = AnkiSettingsPhase.LOADING)),
                idle.copy(ankiSettings = idle.ankiSettings.copy(phase = AnkiSettingsPhase.SAVING)),
                idle.copy(ankiSetup = AnkiSetupPhase.Checking),
            ) +
                listOf(AnkiExportPhase.Checking, AnkiExportPhase.Ready, AnkiExportPhase.Invoking)
                    .map {
                        idle.copy(ankiExport = AnkiExportView(1UL, "", 7, it))
                    }
        for (view in states) assertTrue(appIsLoading(view, AnkiDeckDiscoveryState.Unchecked, false))
        assertTrue(appIsLoading(idle, AnkiDeckDiscoveryState.Loading, false))
        assertTrue(appIsLoading(idle, AnkiDeckDiscoveryState.Unchecked, true))
        assertFalse(
            appIsLoading(
                idle.copy(
                    ankiSetup = AnkiSetupPhase.Failed(AnkiExportError.PROVIDERFAILURE),
                    ankiSettings = idle.ankiSettings.copy(phase = AnkiSettingsPhase.FAILED),
                ),
                AnkiDeckDiscoveryState.Failed(AnkiReadError.PermissionDenied),
                false,
            )
        )
    }

    private fun idleView() =
        ViewModel(
            ankiSettings = AnkiSettingsView(AnkiSettingsPhase.READY, null, null),
            ankiSetup = AnkiSetupPhase.Unchecked,
            ankiEntries = emptyList(),
            vocabulary =
                VocabularyState(
                    VocabularyQuery("", VocabularyStatus.entries.toList(), VocabularySort.NEWEST),
                    rows = emptyList(),
                    selectingAll = false,
                    loading = false,
                ),
            statistics = StatisticsState(StatisticsTimeframe.LASTWEEK, loading = false),
            destination = Destination.LIBRARY,
            settingsOpen = false,
            documents = emptyList(),
            librarySort = LibrarySort.LASTREAD,
            storageBusy = false,
            contentRoute = ContentRoute.MAIN,
            completionSavePending = false,
            readerFurigana = emptyList(),
        )
}
