package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.graphics.toPixelMap
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
    private val setupBusy = mutableStateOf(false)

    private fun render() {
        compose.setContent {
            YomokuTheme {
                AnkiSettingsPanel(
                    settings.value,
                    discovery.value,
                    events::add,
                    { checks++ },
                    setupBusy.value,
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
        compose.onNodeWithTag("anki_settings_load").assertDoesNotExist()
        compose.onNodeWithTag("anki_settings_status").assertDoesNotExist()
        compose.onNodeWithText("Saving default deck…").assertDoesNotExist()
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
                    .assertDoesNotExist()
            } else compose.onNodeWithTag("anki_permission_request").assertDoesNotExist()
        }
        compose.runOnUiThread {
            discovery.value = AnkiDeckDiscoveryState.Loading()
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
    fun checkingAndSavingKeepDeckGeometryWithoutTransientTextOrEnabledActions() {
        val decks =
            listOf(AnkiProviderDeck(1, "Deck one", false), AnkiProviderDeck(2, "Deck two", false))
        settings.value = AnkiSettingsView(AnkiSettingsPhase.READY, 1, null)
        discovery.value = AnkiDeckDiscoveryState.Ready(decks)
        render()
        compose
            .onNodeWithText("Choose a default deck for future word exports.")
            .assertDoesNotExist()
        val first = compose.onNodeWithTag("anki_deck_1").getUnclippedBoundsInRoot()
        val second = compose.onNodeWithTag("anki_deck_2").getUnclippedBoundsInRoot()
        assertTrue(first.bottom.value - first.top.value >= 48f)
        assertEquals(4f, second.top.value - first.bottom.value, 0.5f)
        val checkBounds = compose.onNodeWithTag("anki_decks_check").getUnclippedBoundsInRoot()
        val checkImage = compose.onNodeWithTag("anki_decks_check").captureToImage().toPixelMap()
        val label =
            compose
                .onNodeWithTag("anki_deck_label_1", useUnmergedTree = true)
                .getUnclippedBoundsInRoot()
        assertEquals(32f, label.left.value - first.left.value, 0.5f)
        for (phase in 0..2) {
            compose.runOnUiThread {
                settings.value =
                    settings.value.copy(
                        phase =
                            if (phase == 0) AnkiSettingsPhase.SAVING else AnkiSettingsPhase.READY
                    )
                discovery.value =
                    if (phase == 1) AnkiDeckDiscoveryState.Loading(decks)
                    else AnkiDeckDiscoveryState.Ready(decks)
                setupBusy.value = phase == 2
            }
            compose.onNodeWithTag("anki_deck_1").assertIsNotEnabled()
            compose.onNodeWithTag("anki_deck_2").assertIsNotEnabled()
            assertEquals(first, compose.onNodeWithTag("anki_deck_1").getUnclippedBoundsInRoot())
            assertEquals(second, compose.onNodeWithTag("anki_deck_2").getUnclippedBoundsInRoot())
            compose.onNodeWithTag("anki_decks_check").assertIsNotEnabled()
            assertEquals(
                checkBounds,
                compose.onNodeWithTag("anki_decks_check").getUnclippedBoundsInRoot(),
            )
            val pendingImage =
                compose.onNodeWithTag("anki_decks_check").captureToImage().toPixelMap()
            for (y in 0 until checkImage.height) for (x in 0 until checkImage.width) {
                assertEquals(checkImage[x, y], pendingImage[x, y])
            }
            compose.onNodeWithTag("anki_settings_status").assertDoesNotExist()
            compose.onNodeWithTag("anki_settings_load").assertDoesNotExist()
        }
        compose.runOnUiThread {
            setupBusy.value = false
            discovery.value = AnkiDeckDiscoveryState.Failed(AnkiReadError.ProviderFailure, decks)
        }
        compose.onNodeWithTag("anki_deck_1").assertIsNotEnabled()
        compose.onNodeWithTag("anki_decks_check").assertIsEnabled().performClick()
        assertEquals(1, checks)
    }

    @Test
    fun choosingFirstDeckKeepsCheckPositionAsMissingDeckMessageDisappears() {
        settings.value = AnkiSettingsView(AnkiSettingsPhase.READY, null, null)
        discovery.value = AnkiDeckDiscoveryState.Ready(listOf(AnkiProviderDeck(1, "Deck", false)))
        render()
        val bounds = compose.onNodeWithTag("anki_decks_check").getUnclippedBoundsInRoot()
        compose.onNodeWithTag("anki_deck_1").performClick()
        assertEquals(Event.SetAnkiDefaultDeck(1), events.last())
        compose.runOnUiThread {
            settings.value = AnkiSettingsView(AnkiSettingsPhase.SAVING, 1, null)
        }
        assertEquals(bounds, compose.onNodeWithTag("anki_decks_check").getUnclippedBoundsInRoot())
        compose.onNodeWithText("No default deck configured.").assertDoesNotExist()
        compose.onNodeWithTag("anki_decks_check").assertIsNotEnabled()
    }

    @Test
    fun settingsLoadAndModelCheckHaveNoTemporaryText() {
        compose.setContent {
            YomokuTheme {
                AnkiSettingsPanel(settings.value, discovery.value, events::add, {})
                AnkiModelSetupPanel(AnkiSetupPhase.Checking)
                AnkiModelSetupPanel(AnkiSetupPhase.Unchecked)
            }
        }
        compose.runOnUiThread {
            settings.value = settings.value.copy(phase = AnkiSettingsPhase.LOADING)
        }
        compose.onNodeWithTag("anki_settings_status").assertDoesNotExist()
        compose.onNodeWithTag("anki_settings_load").assertDoesNotExist()
        compose.onNodeWithTag("anki_model_setup").assertDoesNotExist()
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
