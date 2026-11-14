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
class AnkiSettingsPanelTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val settings = mutableStateOf(AnkiSettingsView(AnkiSettingsPhase.UNLOADED, null, null))
    private val discovery = mutableStateOf<AnkiDeckDiscoveryState>(AnkiDeckDiscoveryState.Unchecked)
    private val events = mutableListOf<Event>()
    private var checks = 0

    private fun render() {
        compose.setContent {
            YomokuTheme {
                AnkiSettingsPanel(
                    settings.value,
                    discovery.value,
                    events::add,
                    { checks++ },
                )
            }
        }
    }

    @Test
    fun actionsAreExplicitAndOnlyAcknowledgedReadySettingsAllowSelection() {
        render()
        assertTrue(events.isEmpty())
        assertEquals(0, checks)
        compose.onNodeWithText("No default deck configured.").assertDoesNotExist()
        compose.onNodeWithTag("anki_settings_load").performClick()
        assertEquals(Event.LoadAnkiSettings, events.last())
        compose.onNodeWithTag("anki_decks_check").assertIsNotEnabled()
        compose.runOnUiThread {
            settings.value = AnkiSettingsView(AnkiSettingsPhase.READY, null, null)
            discovery.value =
                AnkiDeckDiscoveryState.Ready(
                    listOf(
                        AnkiProviderDeck(Long.MAX_VALUE, "日本語", false),
                        AnkiProviderDeck(9007199254740993L, "日本語", false),
                        AnkiProviderDeck(42, "Filtered", true),
                    )
                )
        }
        compose.onNodeWithText("No default deck configured.").assertExists()
        compose.onNodeWithTag("anki_deck_42").assertDoesNotExist()
        compose.onNodeWithTag("anki_deck_${Long.MAX_VALUE}").performClick()
        assertEquals(Event.SetAnkiDefaultDeck(Long.MAX_VALUE), events.last())
        compose.runOnUiThread {
            settings.value = AnkiSettingsView(AnkiSettingsPhase.SAVING, 9007199254740993L, null)
        }
        compose.onNodeWithTag("anki_settings_deck").assertDoesNotExist()
        compose.onNodeWithTag("anki_deck_${Long.MAX_VALUE}").assertIsNotEnabled()
        compose.onNodeWithTag("anki_settings_clear").assertDoesNotExist()
        compose.onNodeWithTag("anki_settings_load").performClick()
        assertEquals(Event.RetryAnkiSettings, events.last())
        compose.runOnUiThread {
            settings.value = settings.value.copy(phase = AnkiSettingsPhase.READY)
        }
        compose.onNodeWithTag("anki_settings_clear").assertDoesNotExist()
        compose.onNodeWithTag("anki_settings_status").assertDoesNotExist()
    }

    @Test
    fun providerErrorsAndPermissionNeverInitiateAutomaticWork() {
        render()
        for (error in AnkiReadError.entries) {
            compose.runOnUiThread { discovery.value = AnkiDeckDiscoveryState.Failed(error) }
            compose.waitForIdle()
            if (error == AnkiReadError.PermissionDenied) {
                compose
                    .onNodeWithText(
                        "AnkiDroid database access lets Yomoku list decks, check existing notes and export words. You can keep reading without granting access."
                    )
                    .assertExists()
            } else compose.onNodeWithTag("anki_permission_request").assertDoesNotExist()
        }
        compose.runOnUiThread {
            discovery.value = AnkiDeckDiscoveryState.Loading
            settings.value =
                AnkiSettingsView(
                    AnkiSettingsPhase.FAILED,
                    Long.MAX_VALUE,
                    DocumentError.UNAVAILABLE,
                )
        }
        compose.onNodeWithTag("anki_decks_check").assertIsNotEnabled()
        compose.onNodeWithTag("anki_settings_clear").assertDoesNotExist()
        compose.onNodeWithTag("anki_settings_load").performClick()
        assertEquals(listOf(Event.RetryAnkiSettings), events)
        assertEquals(0, checks)
    }

    @Test
    fun missingAndFilteredDecksPreserveSavedChoiceWithoutSubstitution() {
        render()
        compose.runOnUiThread {
            settings.value = AnkiSettingsView(AnkiSettingsPhase.READY, Long.MAX_VALUE, null)
            discovery.value =
                AnkiDeckDiscoveryState.Ready(
                    listOf(AnkiProviderDeck(Long.MAX_VALUE, "Filtered", true))
                )
        }
        compose.onNodeWithText("No regular decks are available in AnkiDroid.").assertExists()
        compose
            .onNodeWithText("The saved deck is missing or filtered. Choose another deck.")
            .assertExists()
        compose.onNodeWithTag("anki_settings_deck").assertDoesNotExist()
        compose.onNodeWithTag("anki_deck_${Long.MAX_VALUE}").assertDoesNotExist()
        assertTrue(events.isEmpty())
        compose.onNodeWithTag("anki_settings_clear").assertDoesNotExist()
        assertTrue(events.isEmpty())
    }
}
