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
class AnkiRecoveryPanelTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val events = mutableListOf<Event>()
    private val frame =
        mutableStateOf(
            AnkiRecoveryView(
                AnkiRecoveryPhase.UNRESOLVED,
                true,
                true,
                null,
                null,
                false,
                true,
                true,
            )
        )

    private fun render() {
        compose.setContent { YomokuTheme { AnkiRecoveryPanel(frame.value, events::add) } }
    }

    @Test
    fun recoveryActionsAreExplicitAndUseOnlyTypedReadEvents() {
        render()
        compose
            .onNodeWithTag("anki_recovery_status")
            .assertTextEquals("An earlier AnkiDroid operation needs checking.")
        assertTrue(events.isEmpty())
        compose.onNodeWithTag("anki_recovery_note").performClick()
        compose.onNodeWithTag("anki_recovery_model").performClick()
        assertEquals(
            listOf(
                Event.BeginAnkiRecoveryRead(AnkiRecoveryReadTarget.NOTE),
                Event.BeginAnkiRecoveryRead(AnkiRecoveryReadTarget.MODEL),
            ),
            events,
        )
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    phase = AnkiRecoveryPhase.CHECKING,
                    canRetry = true,
                    canCheckNote = false,
                    canCheckModel = false,
                )
        }
        compose.onNodeWithTag("anki_recovery_note").assertIsNotEnabled()
        compose.onNodeWithTag("anki_recovery_model").assertIsNotEnabled()
        compose.onNodeWithTag("anki_recovery_retry").performClick()
        assertEquals(Event.RetryAnkiRecovery, events.last())
        compose.runOnUiThread {
            frame.value = frame.value.copy(phase = AnkiRecoveryPhase.SAVING, canRetry = false)
        }
        compose.onNodeWithTag("anki_recovery_retry").assertDoesNotExist()
        compose.onNodeWithTag("anki_recovery_status").assertTextEquals("Saving recovery result…")
    }

    @Test
    fun recoveryErrorsAndResolvedStateNeverOfferMutationOrAutomaticRetry() {
        render()
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    phase = AnkiRecoveryPhase.FAILED,
                    storageError = DocumentError.UNAVAILABLE,
                    providerError = AnkiExportError.PERMISSIONDENIED,
                    canRetry = true,
                    canCheckNote = false,
                    canCheckModel = false,
                )
        }
        compose.onNodeWithText("AnkiDroid access permission is required.").assertIsDisplayed()
        compose.onNodeWithTag("anki_recovery_note").assertIsNotEnabled()
        assertTrue(events.isEmpty())
        compose.runOnUiThread {
            frame.value =
                AnkiRecoveryView(
                    AnkiRecoveryPhase.READY,
                    false,
                    false,
                    null,
                    null,
                    false,
                    false,
                    false,
                )
        }
        compose
            .onNodeWithTag("anki_recovery_status")
            .assertTextEquals("No pending AnkiDroid recovery.")
        compose.onNodeWithTag("anki_recovery_note").assertDoesNotExist()
        compose.onNodeWithTag("anki_recovery_model").assertDoesNotExist()
        compose.onNodeWithTag("anki_recovery_retry").assertDoesNotExist()
        assertTrue(events.isEmpty())
    }
}
