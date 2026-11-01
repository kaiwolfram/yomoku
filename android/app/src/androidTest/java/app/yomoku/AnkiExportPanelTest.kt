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
    fun busyUncertainAndDurableOutcomesNeverOfferWriteRetry() {
        render()
        for (busy in
            listOf(
                AnkiExportPhase.Checking,
                AnkiExportPhase.Beginning,
                AnkiExportPhase.Ready,
                AnkiExportPhase.Invoking,
                AnkiExportPhase.Saving,
                AnkiExportPhase.StorageFailed(DocumentError.UNAVAILABLE),
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
                    "The note may have been added. Use Check note in AnkiDroid recovery before another export.",
                AnkiExportPhase.ModelUncertain(null) to
                    "The note model may have been created. Use Check model in AnkiDroid recovery before another export.",
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
}
