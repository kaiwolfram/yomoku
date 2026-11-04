package app.yomoku

import android.content.Intent
import androidx.activity.ComponentActivity
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AnkiExportPanelTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val events = mutableListOf<Event>()
    private val frame =
        mutableStateOf(
            AnkiExportView(ULong.MAX_VALUE, "猫𠮷", Long.MAX_VALUE, AnkiExportPhase.Prepared)
        )

    private fun render() {
        compose.setContent { YomokuTheme { AnkiExportPanel(frame.value, events::add) } }
    }

    private fun phase(value: AnkiExportPhase) {
        compose.runOnUiThread { frame.value = frame.value.copy(phase = value) }
    }

    @Test
    fun explicitActionsCarryFrozenGenerationAndPhaseWithoutAutomaticWrites() {
        render()
        compose.onNodeWithTag("anki_export_surface").assertTextEquals("猫𠮷")
        compose.onNodeWithText("Target deck ID: ${Long.MAX_VALUE}").assertIsDisplayed()
        assertTrue(events.isEmpty())
        compose.onNodeWithTag("anki_export_check").performClick()
        assertEquals(Event.BeginAnkiExportPreflight, events.last())
        phase(AnkiExportPhase.ProviderFailed(AnkiExportError.INCOMPATIBLEMODEL))
        compose
            .onNodeWithText("The AnkiDroid model does not match the required layout.")
            .assertExists()
        compose.onNodeWithTag("anki_export_check").performClick()
        assertEquals(Event.RetryAnkiExportPreflight, events.last())
        for (plan in
            listOf(AnkiExportPhase.CreateModel, AnkiExportPhase.ReuseModel(Long.MAX_VALUE))) {
            phase(plan)
            val before = events.size
            compose.waitForIdle()
            assertEquals(before, events.size)
            compose.onNodeWithTag("anki_export_begin").performClick()
            assertEquals(Event.BeginPreparedAnkiExport(ULong.MAX_VALUE, plan), events.last())
        }
        compose.runOnUiThread { frame.value = frame.value.copy(preparationId = 42UL) }
        compose.onNodeWithTag("anki_export_dismiss").performClick()
        assertEquals(Event.ClearAnkiExport(42UL), events.last())
    }

    @Test
    fun busyAndUncertainOutcomesNeverReplayWrites() {
        render()
        for (busy in
            listOf(
                AnkiExportPhase.Checking,
                AnkiExportPhase.Ready,
                AnkiExportPhase.Invoking,
            )) {
            phase(busy)
            compose.onNodeWithTag("anki_export_begin").assertDoesNotExist()
            compose.onNodeWithTag("anki_export_check").assertDoesNotExist()
            if (busy != AnkiExportPhase.Checking)
                compose.onNodeWithTag("anki_export_dismiss").assertDoesNotExist()
        }
        val outcomes =
            listOf(
                AnkiExportPhase.NoteAdded(Long.MAX_VALUE) to "Added to AnkiDroid.",
                AnkiExportPhase.ModelCreated(Long.MAX_VALUE) to
                    "Note model created. Close and select the word again to add it.",
                AnkiExportPhase.NotSubmitted to "Nothing was submitted to AnkiDroid.",
                AnkiExportPhase.NoteUncertain(Long.MAX_VALUE) to
                    "Write interrupted. Check AnkiDroid, then check again before an explicit retry.",
                AnkiExportPhase.ModelUncertain(null) to
                    "Write interrupted. Check AnkiDroid, then check again before an explicit retry.",
                AnkiExportPhase.Existing(listOf(Long.MAX_VALUE, 9007199254740993L)) to
                    "Already in AnkiDroid. Note IDs: ${Long.MAX_VALUE}, 9007199254740993",
            )
        for ((outcome, message) in outcomes) {
            phase(outcome)
            compose.onNodeWithTag("anki_export_status").assertTextEquals(message)
            compose.onNodeWithTag("anki_export_begin").assertDoesNotExist()
            compose.onNodeWithTag("anki_export_check").assertDoesNotExist()
            compose.onNodeWithTag("anki_export_dismiss").assertExists()
        }
        assertTrue(events.isEmpty())
    }

    @Test
    fun browserActionsUseExactIdsShowFailuresAndRequireExplicitRetry() {
        val executor = Executors.newSingleThreadExecutor()
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val reads = mutableListOf<Long>()
        val launches = mutableListOf<Intent>()
        var available = false
        var interactions = 0
        lateinit var browser: AnkiNoteBrowser
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    phase = AnkiExportPhase.Existing(listOf(Long.MAX_VALUE, 9007199254740993L))
                )
            browser =
                AnkiNoteBrowser(
                    executor,
                    {
                        reads.add(it.value)
                        entered.countDown()
                        assertTrue(release.await(30, TimeUnit.SECONDS))
                        AnkiReadResult.Value(if (available) Intent("test.${it.value}") else null)
                    },
                ) {
                    frame.value
                }
            browser.attach(launches::add)
        }
        try {
            compose.setContent {
                YomokuTheme {
                    AnkiExportPanel(frame.value, events::add, browser) { interactions++ }
                }
            }
            assertTrue(reads.isEmpty())
            compose.onNodeWithTag("anki_note_open_${Long.MAX_VALUE}").performClick()
            assertTrue(entered.await(30, TimeUnit.SECONDS))
            compose.onNodeWithTag("anki_note_open_9007199254740993").assertIsNotEnabled()
            compose.onNodeWithTag("anki_note_browser_status").assertTextEquals("Opening AnkiDroid…")
            release.countDown()
            compose.waitUntil(30_000) { browser.state?.phase == AnkiBrowserPhase.Unavailable }
            compose
                .onNodeWithTag("anki_note_browser_status")
                .assertTextEquals(
                    "The AnkiDroid note browser is unavailable. You can try Open again."
                )
            compose.waitForIdle()
            assertEquals(listOf(Long.MAX_VALUE), reads)
            available = true
            compose.onNodeWithTag("anki_note_open_9007199254740993").performClick()
            compose.waitUntil(30_000) { launches.size == 1 }
            assertEquals("test.9007199254740993", launches.single().action)
            compose.onNodeWithTag("anki_note_browser_status").assertDoesNotExist()
            assertTrue(events.isEmpty())
            assertEquals(2, interactions)
        } finally {
            release.countDown()
            compose.runOnUiThread { browser.close() }
            executor.shutdown()
            assertTrue(executor.awaitTermination(30, TimeUnit.SECONDS))
        }
    }

    @Test
    fun browserControlsExcludeUnacknowledgedAndUncertainIds() {
        val executor = Executors.newSingleThreadExecutor()
        lateinit var browser: AnkiNoteBrowser
        compose.runOnUiThread {
            browser =
                AnkiNoteBrowser(executor, { error("Composition must not resolve") }) {
                    frame.value
                }
        }
        try {
            compose.setContent {
                YomokuTheme { AnkiExportPanel(frame.value, events::add, browser) }
            }
            for (value in
                listOf(
                    AnkiExportPhase.Prepared,
                    AnkiExportPhase.ModelCreated(Long.MAX_VALUE),
                    AnkiExportPhase.NoteUncertain(Long.MAX_VALUE),
                    AnkiExportPhase.ModelUncertain(Long.MAX_VALUE),
                )) {
                phase(value)
                compose.onNodeWithTag("anki_note_open_${Long.MAX_VALUE}").assertDoesNotExist()
            }
            phase(AnkiExportPhase.NoteAdded(Long.MAX_VALUE))
            compose.onNodeWithTag("anki_note_open_${Long.MAX_VALUE}").assertExists()
            assertTrue(events.isEmpty())
        } finally {
            compose.runOnUiThread { browser.close() }
            executor.shutdown()
            assertTrue(executor.awaitTermination(30, TimeUnit.SECONDS))
        }
    }
}
