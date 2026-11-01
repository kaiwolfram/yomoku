package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
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
class ReaderAnkiPanelTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val key =
        ReaderTokenKey(
            DocumentId(Long.MAX_VALUE),
            TextAnchor(
                SourceId(1),
                ChapterId(2),
                TextRange(TextOffset(0), TextOffset(6)),
            ),
            ReaderAnalysis.Available(AnalysisId(Long.MAX_VALUE)),
        )
    private val original = AnkiSelection.Original(ReaderWordSelection(key, ULong.MAX_VALUE))
    private val effective =
        AnkiSelection.Effective(
            EffectiveReaderSelection(EffectivePageIdentity(ULong.MAX_VALUE, key), ULong.MAX_VALUE)
        )
    private val selection = mutableStateOf<AnkiSelection?>(original)
    private val settings =
        mutableStateOf(AnkiSettingsView(AnkiSettingsPhase.READY, Long.MAX_VALUE, null))
    private val discovery =
        mutableStateOf<AnkiDeckDiscoveryState>(
            AnkiDeckDiscoveryState.Ready(listOf(AnkiProviderDeck(Long.MAX_VALUE, "日本語", false)))
        )
    private val export = mutableStateOf<AnkiExportView?>(null)
    private val enabled = mutableStateOf(true)
    private val events = mutableListOf<Event>()
    private var interactions = 0

    private fun render() {
        compose.setContent {
            YomokuTheme {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    ReaderAnkiPanel(
                        selection.value,
                        settings.value,
                        export.value,
                        discovery.value,
                        enabled.value,
                        events::add,
                        { interactions++ },
                    )
                }
            }
        }
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()

    @Test
    fun preparationCarriesOnlyExactOriginalOrEffectiveSelectionAndRejectionDoesNotRetry() {
        render()
        assertTrue(events.isEmpty())
        for (value in listOf(original, effective)) {
            compose.runOnUiThread { selection.value = value }
            compose.onNodeWithTag("word_anki_not_prepared").assertDoesNotExist()
            click("word_anki_add")
            assertEquals(Event.PrepareReaderAnkiExport(value), events.last())
            // No accepted native projection: explain the rejected gate without guessing a cause.
            compose.onNodeWithTag("word_anki_not_prepared").assertExists()
            compose.waitForIdle()
            assertEquals(1, events.count { it == Event.PrepareReaderAnkiExport(value) })
        }
        compose.runOnUiThread { enabled.value = false }
        compose.onNodeWithTag("word_anki_add").assertIsNotEnabled()
        compose.onNodeWithTag("word_anki_settings").assertIsNotEnabled()
        compose.runOnUiThread { enabled.value = true }
        click("word_anki_settings")
        assertEquals(Event.OpenSettings, events.last())
        assertEquals(3, interactions)
        assertEquals(3, events.size)
    }

    @Test
    fun configurationAndProviderObservationsNeverStartReadsOrInferDecks() {
        render()
        for (phase in AnkiSettingsPhase.entries.filter { it != AnkiSettingsPhase.READY }) {
            compose.runOnUiThread { settings.value = settings.value.copy(phase = phase) }
            compose.onNodeWithTag("word_anki_add").assertDoesNotExist()
            compose.onNodeWithTag("word_anki_setup").assertExists()
        }
        compose.runOnUiThread {
            settings.value = AnkiSettingsView(AnkiSettingsPhase.READY, null, null)
        }
        compose.onNodeWithTag("word_anki_add").assertDoesNotExist()
        compose.runOnUiThread {
            settings.value = settings.value.copy(defaultDeckId = Long.MAX_VALUE)
        }
        val observations =
            listOf(
                AnkiDeckDiscoveryState.Unchecked,
                AnkiDeckDiscoveryState.Loading,
                AnkiDeckDiscoveryState.Ready(emptyList()),
                AnkiDeckDiscoveryState.Ready(listOf(AnkiProviderDeck(Long.MAX_VALUE, "日本語", true))),
                AnkiDeckDiscoveryState.Ready(
                    listOf(AnkiProviderDeck(9007199254740993L, "日本語", false))
                ),
            ) + AnkiReadError.entries.map { AnkiDeckDiscoveryState.Failed(it) }
        for (value in observations) {
            compose.runOnUiThread { discovery.value = value }
            compose.onNodeWithTag("word_anki_add").assertDoesNotExist()
            compose.onNodeWithTag("word_anki_settings").assertExists()
        }
        assertTrue(events.isEmpty())
        assertEquals(0, interactions)
        click("word_anki_settings")
        assertEquals(listOf(Event.OpenSettings), events)
    }

    @Test
    fun frozenExportRemainsIndependentOfSelectionConfigurationAndProviderChanges() {
        render()
        compose.runOnUiThread {
            export.value = AnkiExportView(42UL, "食べた𠮷", Long.MAX_VALUE, AnkiExportPhase.Prepared)
            selection.value = effective
            settings.value = AnkiSettingsView(AnkiSettingsPhase.UNLOADED, null, null)
            discovery.value = AnkiDeckDiscoveryState.Failed(AnkiReadError.ProviderUnavailable)
        }
        compose.onNodeWithTag("word_anki_add").assertDoesNotExist()
        compose.onNodeWithTag("anki_export_surface").assertTextEquals("食べた𠮷")
        assertTrue(events.isEmpty())
        click("anki_export_check")
        assertEquals(Event.BeginAnkiExportPreflight, events.last())
        for (phase in
            listOf(AnkiExportPhase.CreateModel, AnkiExportPhase.ReuseModel(Long.MAX_VALUE))) {
            compose.runOnUiThread { export.value = export.value!!.copy(phase = phase) }
            click("anki_export_begin")
            assertEquals(Event.BeginPreparedAnkiExport(42UL, phase), events.last())
        }
        compose.runOnUiThread {
            export.value = export.value!!.copy(phase = AnkiExportPhase.NoteUncertain(null))
            selection.value = null
        }
        compose.onNodeWithTag("anki_export_begin").assertDoesNotExist()
        compose.onNodeWithTag("anki_export_check").assertDoesNotExist()
        click("word_anki_settings")
        assertEquals(Event.OpenSettings, events.last())
        click("anki_export_dismiss")
        assertEquals(Event.ClearAnkiExport(42UL), events.last())
        assertEquals(5, events.size)
    }
}
