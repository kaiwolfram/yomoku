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
        compose.setContent {
            YomokuTheme { if (rememberAppLoadingVisible(loading.value)) AppLoadingIndicator() }
        }
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
        compose.setContent { if (rememberAppLoadingVisible(loading.value)) AppLoadingIndicator() }
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
    fun continuousWorkAcrossSourcesKeepsDelayAndSpinnerReplacesLibraryAddWithoutMovement() {
        val view = mutableStateOf(idleView().copy(storageBusy = true))
        val importing = mutableStateOf(false)
        compose.mainClock.autoAdvance = false
        compose.setContent {
            YomokuTheme {
                YomokuApp(view.value, "loading-test", {}, importActive = importing.value)
            }
        }
        val add = compose.onNodeWithTag("library_add").getUnclippedBoundsInRoot()
        compose.mainClock.advanceTimeBy(240)
        compose.runOnUiThread {
            view.value = view.value.copy(storageBusy = false)
            importing.value = true
        }
        compose.mainClock.advanceTimeBy(96)
        val spinner = compose.onNodeWithTag("app_loading")
        spinner.assertExists()
        compose.onAllNodesWithTag("app_loading").assertCountEquals(1)
        compose.onNodeWithTag("library_add").assertDoesNotExist()
        val spinnerBounds = spinner.getUnclippedBoundsInRoot()
        assertEquals(
            (add.left.value + add.right.value) / 2f,
            (spinnerBounds.left.value + spinnerBounds.right.value) / 2f,
            0.5f,
        )
        assertEquals(
            (add.top.value + add.bottom.value) / 2f,
            (spinnerBounds.top.value + spinnerBounds.bottom.value) / 2f,
            0.5f,
        )
        compose.runOnUiThread { importing.value = false }
        compose.mainClock.advanceTimeByFrame()
        spinner.assertDoesNotExist()
        compose.onNodeWithTag("library_add").assertIsDisplayed().assertIsEnabled()
        assertEquals(add, compose.onNodeWithTag("library_add").getUnclippedBoundsInRoot())
    }

    @Test
    fun readerSpinnerIsImmediatelyLeftOfSettingsAndPopupSharesLimitsBusyAndErrorStates() {
        val frame =
            mutableStateOf(
                idleView()
                    .copy(
                        contentRoute = ContentRoute.READER,
                        readerTypography =
                            ReaderTypography(ReaderFontSize.SP16, ReaderLineSpacing.PERCENT150),
                    )
            )
        val events = mutableListOf<Event>()
        compose.setContent { YomokuTheme { YomokuApp(frame.value, "reader-menu", events::add) } }
        compose.onNodeWithTag("reader_settings").performTouchInput { click() }
        compose.onNodeWithTag("reader_font_size_increase").performTouchInput { click() }
        assertEquals(
            Event.SetReaderTypography(
                ReaderTypography(ReaderFontSize.SP18, ReaderLineSpacing.PERCENT150)
            ),
            events.last(),
        )
        compose.onNodeWithTag("reader_typography_popup").assertIsDisplayed()
        for (busy in listOf("storage", "completion")) {
            compose.runOnUiThread {
                frame.value =
                    frame.value.copy(
                        storageBusy = busy == "storage",
                        completionSavePending = busy == "completion",
                    )
            }
            compose.onNodeWithTag("reader_font_size_increase").assertIsNotEnabled()
            compose.onNodeWithTag("reader_line_spacing_decrease").assertIsNotEnabled()
        }
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    storageBusy = false,
                    completionSavePending = false,
                    documentError = DocumentError.UNAVAILABLE,
                    readerTypography =
                        ReaderTypography(ReaderFontSize.SP24, ReaderLineSpacing.PERCENT200),
                )
        }
        // The page also reports the error; inspect the popup's copy.
        compose
            .onNode(
                hasTestTag("document_error") and
                    hasAnyAncestor(hasTestTag("reader_typography_popup"))
            )
            .assertIsDisplayed()
        compose.onNodeWithTag("reader_font_size_increase").assertIsNotEnabled()
        compose.onNodeWithTag("reader_line_spacing_increase").assertIsNotEnabled()
        compose.onNodeWithTag("reader_font_size_decrease").assertIsEnabled().performClick()
        assertTrue(events.last() is Event.SetReaderTypography)
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    readerTypography =
                        ReaderTypography(ReaderFontSize.SP14, ReaderLineSpacing.PERCENT125)
                )
        }
        compose.onNodeWithTag("reader_font_size_decrease").assertIsNotEnabled()
        compose.onNodeWithTag("reader_line_spacing_decrease").assertIsNotEnabled()
        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
            .sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_BACK)
        compose.onNodeWithTag("reader_typography_popup").assertDoesNotExist()
        compose.onNodeWithTag("reader_settings").performClick()
        compose.onNodeWithTag("reader_typography_popup").assertIsDisplayed()
        tapBelowPopup(compose.activity)
        compose.onNodeWithTag("reader_typography_popup").assertDoesNotExist()
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = true) }
        compose.mainClock.advanceTimeBy(320)
        val spinner = compose.onNodeWithTag("app_loading").getUnclippedBoundsInRoot()
        val settings = compose.onNodeWithTag("reader_settings").getUnclippedBoundsInRoot()
        assertEquals(
            48f,
            (settings.left.value + settings.right.value) / 2f -
                (spinner.left.value + spinner.right.value) / 2f,
            0.5f,
        )
        compose.onNodeWithTag("reader_settings").performClick()
        compose.runOnUiThread {
            frame.value = frame.value.copy(contentRoute = ContentRoute.MAIN, storageBusy = false)
        }
        compose.onNodeWithTag("reader_typography_popup").assertDoesNotExist()
        compose.runOnUiThread { frame.value = frame.value.copy(contentRoute = ContentRoute.READER) }
        compose.onNodeWithTag("reader_typography_popup").assertDoesNotExist()
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
        )
}
