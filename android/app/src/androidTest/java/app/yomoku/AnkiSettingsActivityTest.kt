package app.yomoku

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.AnkiSettingsPhase
import app.yomoku.shared.Event
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AnkiSettingsActivityTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test
    fun realActivityRetainsExplicitDiscoveryAndLocalSettingsAcrossRecreation() {
        lateinit var original: CoreSession
        compose.runOnUiThread {
            original = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session
            original.dispatch(Event.OpenSettings)
            assertEquals(AnkiDeckDiscoveryState.Unchecked, original.ankiDecks.state)
        }
        compose.waitUntil(30_000) { original.view.ankiSettings.phase == AnkiSettingsPhase.READY }
        compose.onNodeWithTag("anki_decks_check").performScrollTo().performClick()
        compose.waitUntil(30_000) { original.ankiDecks.state != AnkiDeckDiscoveryState.Loading }
        val settings = original.view.ankiSettings
        val discovery = original.ankiDecks.state
        assertNotEquals(AnkiDeckDiscoveryState.Unchecked, discovery)
        compose.activityRule.scenario.recreate()
        compose.runOnUiThread {
            val retained = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session
            assertSame(original, retained)
            assertEquals(settings, retained.view.ankiSettings)
            assertEquals(discovery, retained.ankiDecks.state)
            retained.dispatch(Event.CloseSettings)
            retained.dispatch(Event.OpenSettings)
        }
        compose.onNodeWithTag("anki_settings").performScrollTo().assertExists()
        assertEquals(settings, original.view.ankiSettings)
        assertEquals(discovery, original.ankiDecks.state)
    }
}
